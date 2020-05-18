mod block_import_wrapper;
mod block_processor;
mod client;
mod common;
mod db;
mod dummy_objs;
mod genesis;
mod runtime;
mod types;
mod verifier;

use crate::block_processor::setup_block_processor;
use crate::common::{
    initialize_backend, insert_light_authority_set, LightAuthoritySet, NUM_COLUMNS,
};
use crate::db::create;
use crate::genesis::GenesisData;
use crate::types::{Block, Header};
use parity_scale_codec::Encode;
use sc_client_api::{Backend, BlockImportOperation, NewBlockState};
use sp_blockchain::Error as BlockchainError;
use sp_consensus::import_queue::{BlockImportResult, IncomingBlock};
use sp_runtime::traits::NumberFor;
use sp_runtime::Justification;

// WASM entry point need to call this function
fn initialize_db(
    initial_header: Header,
    initial_authority_set: LightAuthoritySet,
) -> Result<Vec<u8>, BlockchainError> {
    let db = create(NUM_COLUMNS);
    let new_ibc_data = crate::db::IBCData {
        db,
        genesis_data: GenesisData {},
    };
    let empty_data = new_ibc_data.encode();
    let (mut backend, ibc_data) = initialize_backend(empty_data)?;
    insert_light_authority_set(backend.clone(), initial_authority_set)?;

    // Ingest initial header
    let mut backend_op = backend.begin_operation()?;
    backend_op.set_block_data(initial_header, None, None, NewBlockState::Best)?;
    backend.commit_operation(backend_op)?;

    Ok(ibc_data.encode())
}

// WASM entry point need to call this function
fn ingest_finalized_header(
    encoded_data: Vec<u8>,
    finalized_header: Header,
    justification: Option<Justification>,
) -> Result<(BlockImportResult<NumberFor<Block>>, Vec<u8>), String> {
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

    // We aren't returning updated db data from block processor function directly, because
    // in future we might want to call it for multiple blocks per tx.
    Ok((block_processor_fn(incoming_block)?, ibc_data.encode()))
}
