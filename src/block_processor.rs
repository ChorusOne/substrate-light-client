use crate::block_import_wrapper::BlockImportWrapper;
use crate::client::Client;
use crate::db;
use crate::dummy_objs::DummyGenesisGrandpaAuthoritySetProvider;
use crate::dummy_objs::{DummyCallExecutor, DummySpawnHandle};
use crate::runtime::RuntimeApiConstructor;
use crate::types::Block;
use crate::verifier::GrandpaVerifier;
use parity_scale_codec::Decode;
use sc_chain_spec::{ChainType, GenericChainSpec, NoExtension};
use sc_client::light::backend::Backend;
use sc_client_api::FetchChecker;
use sc_client_db::light::LightStorage;
use sc_client_db::{DatabaseSettings, DatabaseSettingsSrc, PruningMode};
use sc_finality_grandpa as grandpa;
use sp_blockchain::Result as ClientResult;
use sp_consensus::import_queue::{import_single_block, BlockImportResult, IncomingBlock};
use sp_consensus::BlockOrigin;
use sp_runtime::traits::{BlakeTwo256, NumberFor};
use std::marker::PhantomData;
use std::sync::Arc;

pub type BlockProcessor<B> =
    Box<dyn FnMut(IncomingBlock<B>) -> Result<BlockImportResult<NumberFor<B>>, String>>;

pub fn setup_block_processor(
    encoded_data: Vec<u8>,
) -> ClientResult<(BlockProcessor<Block>, db::IBCData)> {
    let ibc_data = db::IBCData::decode(&mut encoded_data.as_slice()).unwrap();

    // This dummy genesis provider will panic, if auxiliary storage
    // does not contain authority set at LIGHT_AUTHORITY_SET_KEY.
    let dummy_grandpa_genesis_authority_set_provider = DummyGenesisGrandpaAuthoritySetProvider {};

    let light_storage = LightStorage::new(DatabaseSettings {
        state_cache_size: 2048,
        state_cache_child_ratio: Some((20, 100)),
        pruning: PruningMode::keep_blocks(256),
        source: DatabaseSettingsSrc::Custom(Arc::new(ibc_data.db.clone())),
    })?;

    let light_blockchain = sc_client::light::new_light_blockchain(light_storage);
    let backend = sc_client::light::new_light_backend(light_blockchain.clone());

    // We are never going to execute any extrinsic, so we use dummy implementation
    let executor: DummyCallExecutor<Block, LightStorage<Block>> = DummyCallExecutor {
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

    // Custom client implementation with dummy runtime
    let client: Arc<
        Client<_, _, RuntimeApiConstructor, DummyCallExecutor<Block, LightStorage<Block>>>,
    > = Arc::new(Client {
        backend: backend.clone(),
        _phantom: PhantomData,
        _phantom2: PhantomData,
        _phantom3: PhantomData,
        aux_store_write_enabled: true,
    });

    // This is to prevent grandpa light import queue to accidentally
    // re-write authority set
    let read_only_aux_store_client = Arc::new(client.clone_with_read_only_aux_store());

    let light_data_checker = Arc::new(sc_client::light::new_fetch_checker::<_, Block, _>(
        light_blockchain.clone(),
        executor.clone(),
        Box::new(DummySpawnHandle {}),
    ));

    let fetch_checker: Arc<dyn FetchChecker<Block>> = light_data_checker.clone();

    // We need to re-initialize grandpa light import queue because
    // current version read/write authority set from private field instead of
    // auxiliary storage.
    let block_processor_fn = Box::new(move |incoming_block: IncomingBlock<Block>| {
        let grandpa_block_import = grandpa::light_block_import(
            read_only_aux_store_client.clone(),
            backend.clone(),
            &dummy_grandpa_genesis_authority_set_provider,
            Arc::new(fetch_checker.clone()),
        )
        .map_err(|e| format!("{}", e))?;
        let mut grandpa_verifier = GrandpaVerifier::new(client.clone());
        let mut block_import_wrapper: BlockImportWrapper<
            _,
            Block,
            Backend<LightStorage<Block>, BlakeTwo256>,
            _,
        > = BlockImportWrapper::new(grandpa_block_import.clone(), client.clone());
        import_single_block(
            &mut block_import_wrapper,
            BlockOrigin::NetworkBroadcast,
            incoming_block,
            &mut grandpa_verifier,
        )
        .map_err(|e| format!("{:?}", e))
    });

    Ok((block_processor_fn, ibc_data))
}
