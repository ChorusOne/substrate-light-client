mod db;
mod import_queue;
mod dummy_executor;

use sp_runtime::traits::{Block as BlockT};

// WASM entry point
fn ingest_finalized_header<B: BlockT>(finalized_header: &B::Header) -> Result<(), &str> {
    Ok(())
}
