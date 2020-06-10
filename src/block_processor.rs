use crate::block_import_wrapper::BlockImportWrapper;
use crate::client::Client;
use crate::common::initialize_backend;
use crate::db;
use crate::grandpa_block_import::GrandpaLightBlockImport;
use crate::runtime::RuntimeApiConstructor;
use crate::types::Block;
use crate::verifier::GrandpaVerifier;
use sp_blockchain::Result as ClientResult;
use sp_consensus::import_queue::{import_single_block, BlockImportResult, IncomingBlock};
use sp_consensus::BlockOrigin;
use sp_runtime::traits::NumberFor;
use std::sync::Arc;

pub type BlockProcessor<B> =
    Box<dyn FnMut(IncomingBlock<B>) -> Result<BlockImportResult<NumberFor<B>>, String>>;

pub fn setup_block_processor(
    encoded_data: Vec<u8>,
    max_non_finalized_blocks_allowed: u64,
) -> ClientResult<(BlockProcessor<Block>, db::Data)> {
    let (backend, data) = initialize_backend(encoded_data, max_non_finalized_blocks_allowed)?;

    // Custom client implementation with dummy runtime
    let client = Arc::new(Client::new(backend.clone()));

    // We need to re-initialize grandpa light import queue because
    // current version read/write authority set from private field instead of
    // auxiliary storage.
    let block_processor_fn = Box::new(move |incoming_block: IncomingBlock<Block>| {
        let grandpa_block_import = GrandpaLightBlockImport::new(client.clone(), backend.clone());
        let mut grandpa_verifier = GrandpaVerifier::new(backend.clone());
        let mut block_import_wrapper: BlockImportWrapper<_, _> =
            BlockImportWrapper::new(grandpa_block_import.clone(), backend.clone());
        import_single_block(
            &mut block_import_wrapper,
            BlockOrigin::NetworkBroadcast,
            incoming_block,
            &mut grandpa_verifier,
        )
        .map_err(|e| format!("{:?}", e))
    });

    Ok((block_processor_fn, data))
}
