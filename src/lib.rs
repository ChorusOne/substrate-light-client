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
use kvdb::KeyValueDB;
use parity_scale_codec::Encode;
use sc_client_api::{Backend, BlockImportOperation, NewBlockState};
use sp_blockchain::Error as BlockchainError;
use sp_consensus::import_queue::{BlockImportResult, IncomingBlock};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Header as HeaderT, NumberFor, Zero};
use sp_runtime::Justification;

// WASM entry point need to call this function
pub fn initialize_db(
    initial_header: Header,
    initial_authority_set: LightAuthoritySet,
) -> Result<Vec<u8>, BlockchainError> {
    let db = create(NUM_COLUMNS);
    let new_ibc_data = crate::db::IBCData {
        db,
        genesis_data: GenesisData {},
    };
    let empty_data = new_ibc_data.encode();
    let (backend, ibc_data) = initialize_backend(empty_data)?;
    insert_light_authority_set(backend.clone(), initial_authority_set)?;

    // Add dummy genesis hash
    // Note: This is very hacky way to fool into substrate backend that genesis hash is there, which utilize
    // several private values.
    // This is done to be able to read blockchain meta successfully.
    // One option to remove this hacky way is to not rely on the meta data provided by backend in our
    // custom client.
    let genesis_header = Header::new(
        Zero::zero(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let mut transaction = ibc_data.db.transaction();
    transaction.put(0, b"gen", &genesis_header.hash().0);
    ibc_data
        .db
        .write(transaction)
        .map_err(|e| BlockchainError::Backend(format!("{}", e)))?;

    // Ingest initial header
    let mut backend_op = backend.begin_operation()?;
    backend_op.set_block_data(initial_header, None, None, NewBlockState::Best)?;
    backend.commit_operation(backend_op)?;

    Ok(ibc_data.encode())
}

// WASM entry point need to call this function
pub fn ingest_finalized_header(
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

#[cfg(test)]
mod tests {
    use crate::common::LightAuthoritySet;
    use crate::types::Header;
    use crate::{ingest_finalized_header, initialize_db};
    use sp_runtime::traits::{Header as HeaderT, One, Zero};

    #[test]
    fn test_initialize_db_success() {
        let initial_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );

        let mut next_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        // Only header with number incremented by 1 and parent hash as initial header's hash
        // will be accepted.
        next_header.number += 1;
        next_header.parent_hash = initial_header.hash();

        let data = initialize_db(initial_header, LightAuthoritySet::new(0, vec![])).unwrap();
        assert!(data.len() > 0);

        assert!(ingest_finalized_header(data, next_header, None).is_ok());
    }

    #[test]
    fn test_initialize_db_non_sequential_block() {
        let initial_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );

        let mut next_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        // Only header with number incremented by 1 and parent hash as initial header's hash
        // will be accepted.
        next_header.number += 2;
        next_header.parent_hash = initial_header.hash();

        let data = initialize_db(initial_header, LightAuthoritySet::new(0, vec![])).unwrap();
        assert!(data.len() > 0);

        assert_eq!(ingest_finalized_header(data, next_header, None), Err(String::from("Other(ClientImport(\"Import failed: Did not finalize blocks in sequential order.\"))")));
    }

    #[test]
    fn test_initialize_db_wrong_parent_hash() {
        let initial_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );

        let mut next_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        // Only header with number incremented by 1 and parent hash as initial header's hash
        // will be accepted.
        next_header.number += 1;
        // We aren't setting parent hash

        let data = initialize_db(initial_header, LightAuthoritySet::new(0, vec![])).unwrap();
        assert!(data.len() > 0);

        assert_eq!(
            ingest_finalized_header(data, next_header, None),
            Err(String::from("UnknownParent"))
        );
    }
}
