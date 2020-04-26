use crate::db;
use parity_scale_codec::Decode;
use sc_client_db::light::LightStorage;
use sc_client_db::{DatabaseSettings, PruningMode, DatabaseSettingsSrc};
use std::sync::Arc;
use crate::dummy_objs::{DummyCallExecutor, DummySpawnHandle};
use crate::types::Block;
use std::marker::PhantomData;
use sc_chain_spec::{GenericChainSpec, ChainType, NoExtension};
use sc_finality_grandpa as grandpa;
use sc_network::config::OnDemand;
use sp_inherents::InherentDataProviders;
use crate::genesis::GenesisGrandpaAuthoritySetProvider;

// TODO: Clean this up and abstract away some parts
pub fn setup_import_queue(encoded_data: Vec<u8>)  {
    let ibc_data = db::IBCData::decode(&mut encoded_data.as_slice()).unwrap();
    let grandpa_genesis_authority_set_provider = GenesisGrandpaAuthoritySetProvider::new(&ibc_data.genesis_data);

    let light_storage = LightStorage::new(DatabaseSettings{
        state_cache_size: 2048,
        state_cache_child_ratio: Some((20, 100)),
        pruning: PruningMode::keep_blocks(256),
        source: DatabaseSettingsSrc::Custom(Arc::new(ibc_data.db))
    }).unwrap();

    let light_blockchain = sc_client::light::new_light_blockchain(light_storage);
    let backend = sc_client::light::new_light_backend(light_blockchain.clone());

    // We are never going to execute any extrinsic, so we provide dummy implementation
    let executor: DummyCallExecutor<Block, LightStorage<Block>> = DummyCallExecutor{
        _phantom: PhantomData,
        _phantom2: PhantomData,
    };

    let dummy_chain_spec: GenericChainSpec<(), NoExtension> = GenericChainSpec::from_genesis(
        "substrate_ibc_verification",
        "substrate_ibc_verification",
        ChainType::Custom(String::from("block_verifier")),
        || {},
        vec![],
        None,
        None,
        None,
        None,
    );

    // Custom Client implementation without runtime
    let client: Arc<crate::client::Client<_, _, crate::runtime::RuntimeApiConstructor, DummyCallExecutor<Block, LightStorage<Block>>>> = Arc::new(crate::client::Client{
        backend: backend.clone(),
        _phantom: PhantomData,
        _phantom2: PhantomData,
        _phantom3: PhantomData,
        babe_configuration: ibc_data.genesis_data.babe_configuration
    });

    let light_data_checker = Arc::new(
        sc_client::light::new_fetch_checker::<_, Block, _>(
            light_blockchain.clone(),
            executor.clone(),
            Box::new(DummySpawnHandle{}),
        ),
    );

    let fetcher = Arc::new(OnDemand::new(light_data_checker));

    let fetch_checker = fetcher.checker().clone();

    let grandpa_block_import = grandpa::light_block_import(
        client.clone(),
        backend,
        &grandpa_genesis_authority_set_provider,
        Arc::new(fetch_checker),
    ).unwrap();

    let finality_proof_import = grandpa_block_import.clone();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client).unwrap(),
        grandpa_block_import,
        client.clone(),
    ).unwrap();

    let inherent_data_providers = InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link,
        babe_block_import,
        None,
        Some(Box::new(finality_proof_import)),
        client.clone(),
        inherent_data_providers.clone(),
    ).unwrap();
}

#[cfg(test)]
mod tests {
    use crate::db::{create, IBCData};
    use crate::genesis::GenesisData;
    use sc_consensus_babe::{BabeConfiguration, BabeImportQueue};
    use kvdb::KeyValueDB;
    use sc_client_db::{PruningMode, DatabaseSettingsSrc, DatabaseSettings};
    use sc_client_db::light::LightStorage;
    use parity_scale_codec::alloc::sync::Arc;
    use crate::dummy_objs::DummyCallExecutor;
    use std::marker::PhantomData;
    use crate::types::Block;
    use sp_finality_grandpa::AuthorityId;
    use sp_core::crypto::Public;
    use sp_api::{ProvideRuntimeApi, ApiRef, ApiExt};
    use sc_consensus_babe::BabeApi;
    use sp_runtime::generic::BlockId;
    use std::ops::{Deref, DerefMut};
    use crate::client::Client;
    use sp_runtime::traits::{Block as BlockT, Zero};
    use sp_consensus::BlockImport;
    use sp_blockchain::{ProvideCache, HeaderBackend, HeaderMetadata};
    use sc_client_api::AuxStore;
    use crate::runtime::RuntimeApiConstructor;

    #[test]
    fn babe_configuration_fetch() {
        let db = create(2);
        let mut transaction = db.transaction();
        transaction.put(0, b"key1", b"horse");
        transaction.put(1, b"key2", b"pigeon");
        transaction.put(1, b"key3", b"cat");
        assert!(db.write(transaction).is_ok());

        let ibc_data = IBCData {
            db,
            genesis_data: GenesisData{
                grandpa_authority_set_id: 3,
                grandpa_authority_set: vec![(AuthorityId::from_slice(&[1; 32]), 5)],
                babe_configuration: BabeConfiguration{
                    slot_duration: 0,
                    epoch_length: 0,
                    c: (0, 0),
                    genesis_authorities: vec![],
                    randomness: [1; 32],
                    secondary_slots: false
                }
            }
        };

        let light_storage = LightStorage::new(DatabaseSettings{
            state_cache_size: 0,
            state_cache_child_ratio: Some((0, 0)),
            pruning: PruningMode::keep_blocks(0),
            source: DatabaseSettingsSrc::Custom(Arc::new(ibc_data.db))
        }).unwrap();

        let light_blockchain = sc_client::light::new_light_blockchain(light_storage);
        let backend = sc_client::light::new_light_backend(light_blockchain.clone());


        let client: Arc<crate::client::Client<sc_client::light::backend::Backend<sc_client_db::light::LightStorage<Block>, sp_runtime::traits::BlakeTwo256>, Block, RuntimeApiConstructor, DummyCallExecutor<Block, LightStorage<Block>>>> = Arc::new(crate::client::Client{
            backend: backend.clone(),
            _phantom: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
            babe_configuration: ibc_data.genesis_data.babe_configuration.clone()
        });

        // RuntimeApi returned by client should return same configuration we passed
        assert_eq!(call_babe_configuration(client).unwrap(), ibc_data.genesis_data.babe_configuration);
    }

    fn call_babe_configuration<Block: BlockT, Client>(
        client: Arc<Client>,
    ) -> std::result::Result<sc_consensus_babe::BabeConfiguration, sp_blockchain::Error> where
        Client: ProvideRuntimeApi<Block> + ProvideCache<Block> + Send + Sync + AuxStore + 'static,
        Client: HeaderBackend<Block> + HeaderMetadata<Block, Error = sp_blockchain::Error>,
        Client::Api: BabeApi<Block> + ApiExt<Block, Error = sp_blockchain::Error>,
    {
        client.runtime_api().configuration(&BlockId::number(Zero::zero()))
    }
}
