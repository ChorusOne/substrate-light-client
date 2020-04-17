use crate::db;
use parity_scale_codec::Decode;
use sc_client_db::light::LightStorage;
use sc_client_db::{DatabaseSettings, PruningMode, DatabaseSettingsSrc};
use std::sync::Arc;
use sc_client::Client;
use crate::dummy_executor::DummyCallExecutor;
use std::marker::PhantomData;
use sc_chain_spec::GenericChainSpec;
use sp_runtime::{BuildStorage, OpaqueExtrinsic};
use sp_runtime::generic::{Block, Header};
use sp_runtime::traits::BlakeTwo256;
use sc_finality_grandpa as grandpa;
use sc_network::config::OnDemand;
use sp_inherents::InherentDataProviders;

// TODO: Clean this up and abstract away some parts
pub fn setup_import_queue(encoded_data: Vec<u8>) {
    let db = match encoded_data.len() {
        0 => db::create(1),
        _ => db::IBCDB::decode(&mut encoded_data.as_slice()).unwrap()
    };

    let light_storage = LightStorage::new(DatabaseSettings{
        state_cache_size: 2048,
        state_cache_child_ratio: Some((20, 100)),
        pruning: PruningMode::keep_blocks(256),
        source: DatabaseSettingsSrc::Custom(Arc::new(db))
    }).unwrap();

    let light_blockchain = sc_client::light::new_light_blockchain(light_storage);
    let backend = sc_client::light::new_light_backend(light_blockchain);

    // We are never going to execute any extrinsic, so we provide dummy implementation
    let executor = DummyCallExecutor{
        _phantom: PhantomData,
        _phantom2: PhantomData,
    };

    let client = Arc::new(Client::new(
        backend,
        executor.clone(),
        GenericChainSpec{
            client_spec: Default::default(),
            genesis: Default::default()
        }.build_storage().unwrap(),
        Default::default(),
        Default::default(),
        Default::default(),
        None,
    ).unwrap());

    let light_data_checker = Arc::new(
        sc_client::light::new_fetch_checker::<_, Block<Header<u32, BlakeTwo256>, OpaqueExtrinsic>, _>(
            light_blockchain.clone(),
            executor.clone(),
            Box::new(tasks_builder.spawn_handle()),
        ),
    );

    let fetcher = Arc::new(OnDemand::new(light_data_checker));

    let fetch_checker = fetcher.checker().clone();

    let grandpa_block_import = grandpa::light_block_import(
        client.clone(),
        backend,
        &(client.clone() as Arc<_>),
        Arc::new(fetch_checker),
    ).unwrap();

    let finality_proof_import = grandpa_block_import.clone();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
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