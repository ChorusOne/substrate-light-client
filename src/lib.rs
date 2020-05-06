mod block_import_wrapper;
mod client;
mod common;
mod db;
mod dummy_objs;
mod genesis;
mod import_queue;
mod runtime;
mod types;
mod verifier;

use crate::import_queue::setup_block_processor;
use crate::types::{Block, Header};
use sp_consensus::import_queue::{BlockImportResult, IncomingBlock};
use sp_runtime::traits::NumberFor;
use sp_runtime::Justification;

// WASM entry point
fn ingest_finalized_header(
    encoded_data: Vec<u8>,
    finalized_header: Header,
    justification: Option<Justification>,
) -> Result<BlockImportResult<NumberFor<Block>>, String> {
    let (mut block_processor_fn, ibc_data) =
        setup_block_processor(encoded_data).map_err(|e| format!("{}", e))?;
    let incoming_block = IncomingBlock {
        hash: finalized_header.hash(),
        header: Some(finalized_header),
        body: None,
        justification,
        origin: None,
        allow_missing_state: false,
        import_existing: false,
    };
    block_processor_fn(incoming_block)
}
