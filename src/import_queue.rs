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
use crate::dummy_objs::DummyGenesisGrandpaAuthoritySetProvider;
use sc_consensus_babe::BabeImportQueue;
use sp_runtime::traits::{BlakeTwo256, NumberFor};
use sp_api::TransactionFor;
use crate::client::Client;
use sc_client::light::backend::Backend;
use crate::runtime::RuntimeApiConstructor;
use sp_blockchain::Result as ClientResult;
use crate::verifier::GrandpaVerifier;
use crate::block_import_wrapper::BlockImportWrapper;
use sp_consensus::import_queue::{IncomingBlock, import_single_block, BlockImportResult, BlockImportError};
use sp_consensus::{ImportResult, BlockOrigin, Error};

pub type BlockProcessor<B> = Box<dyn FnMut(IncomingBlock<B>) -> Result<BlockImportResult<NumberFor<B>>, BlockImportError>>;


pub fn setup_block_processor(encoded_data: Vec<u8>) -> ClientResult<(BlockProcessor<Block>, db::IBCData)> {
    let ibc_data = db::IBCData::decode(&mut encoded_data.as_slice()).unwrap();

    // This dummy genesis provider will panic, So pre-requisite is to have
    // light authority set inside auxiliary storage beforehand.
    let dummy_grandpa_genesis_authority_set_provider = DummyGenesisGrandpaAuthoritySetProvider{};

    let light_storage = LightStorage::new(DatabaseSettings{
        state_cache_size: 2048,
        state_cache_child_ratio: Some((20, 100)),
        pruning: PruningMode::keep_blocks(256),
        source: DatabaseSettingsSrc::Custom(Arc::new(ibc_data.db.clone()))
    })?;

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
    let client: Arc<Client<_, _, RuntimeApiConstructor, DummyCallExecutor<Block, LightStorage<Block>>>> = Arc::new(Client{
        backend: backend.clone(),
        _phantom: PhantomData,
        _phantom2: PhantomData,
        _phantom3: PhantomData,
        aux_store_write_enabled: true,
    });

    let read_only_aux_store_client = Arc::new(client.clone_with_read_only_aux_store());

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
        read_only_aux_store_client.clone(),
        backend,
        &dummy_grandpa_genesis_authority_set_provider,
        Arc::new(fetch_checker),
    )?;

    let mut grandpa_verifier = GrandpaVerifier::new(client.clone());
    let mut block_import_wrapper: BlockImportWrapper<_, Block, Backend<LightStorage<Block>, BlakeTwo256>, _> = BlockImportWrapper::new(grandpa_block_import.clone(), client.clone());

    Ok((Box::new(move |incoming_block: IncomingBlock<Block>| {
        import_single_block(&mut block_import_wrapper, BlockOrigin::NetworkBroadcast, incoming_block, &mut grandpa_verifier)
    }), ibc_data))
}

#[cfg(test)]
mod tests {
    use crate::db::{create, IBCData};
    use crate::genesis::GenesisData;
    use sc_consensus_babe::BabeConfiguration;
    use kvdb::KeyValueDB;
    use sc_client_db::{PruningMode, DatabaseSettingsSrc, DatabaseSettings};
    use sc_client_db::light::LightStorage;
    use parity_scale_codec::alloc::sync::Arc;
    use crate::dummy_objs::DummyCallExecutor;
    use std::marker::PhantomData;
    use crate::types::Block;
    use sp_finality_grandpa::AuthorityId;
    use sp_core::crypto::Public;
    use crate::client::Client;
    use sp_runtime::traits::{Block as BlockT, BlakeTwo256};
    use crate::runtime::RuntimeApiConstructor;
    use std::ops::Deref;

    #[test]
    fn babe_configuration_fetch() {
        let db = create(11);
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


        let client: Arc<Client<sc_client::light::backend::Backend<LightStorage<Block>, BlakeTwo256>, Block, RuntimeApiConstructor, DummyCallExecutor<Block, LightStorage<Block>>>> = Arc::new(Client{
            backend: backend.clone(),
            _phantom: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
            babe_configuration: ibc_data.genesis_data.babe_configuration.clone()
        });

        // RuntimeApi returned by client should return same configuration we passed
        let computed_babe_config = sc_consensus_babe::Config::get_or_compute(&*client).unwrap();
        assert_eq!(*(computed_babe_config.deref()), ibc_data.genesis_data.babe_configuration);
    }
}
