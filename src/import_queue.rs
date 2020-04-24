use crate::db;
use parity_scale_codec::Decode;
use sc_client_db::light::LightStorage;
use sc_client_db::{DatabaseSettings, PruningMode, DatabaseSettingsSrc};
use std::sync::Arc;
use sc_client::Client;
use crate::dummy_objs::{DummyCallExecutor, DummySpawnHandle, DummyConstructRuntimeApi};
use crate::types::Block;
use std::marker::PhantomData;
use sc_chain_spec::{GenericChainSpec, ChainType, ChainSpec, NoExtension};
use sp_runtime::OpaqueExtrinsic;
use sp_runtime::traits::BlakeTwo256;
use sc_finality_grandpa as grandpa;
use sc_network::config::OnDemand;
use sp_inherents::InherentDataProviders;
use std::error::Error;
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
    let client: Arc<crate::client::Client<_, _, crate::dummy_objs::DummyConstructRuntimeApi, DummyCallExecutor<Block, LightStorage<Block>>>> = Arc::new(crate::client::Client{
        backend: backend.clone(),
        _phantom: PhantomData,
        _phantom2: PhantomData,
        _phantom3: PhantomData
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
