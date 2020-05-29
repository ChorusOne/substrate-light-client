mod block_import_wrapper;
mod block_processor;
mod client;
mod common;
mod db;
mod dummy_objs;
mod genesis;
mod runtime;
mod storage;
mod types;
mod verifier;

use crate::block_processor::setup_block_processor;
use crate::common::{
    fetch_light_authority_set, fetch_next_authority_change, initialize_backend,
    insert_light_authority_set, LightAuthoritySet, Status, NUM_COLUMNS,
};
use crate::db::create;
use crate::genesis::GenesisData;
use crate::types::{Block, Header};
use parity_scale_codec::Encode;
use sc_client_api::{Backend, BlockImportOperation, NewBlockState};
use sp_api::BlockId;
use sp_blockchain::{Error as BlockchainError, HeaderBackend, Info};
use sp_consensus::import_queue::{BlockImportResult, IncomingBlock};
use sp_runtime::traits::{Block as BlockT, NumberFor};
use sp_runtime::Justification;

// WASM entry point need to call this function
pub fn initialize_db(
    initial_header: Header,
    initial_authority_set: LightAuthoritySet,
) -> Result<Vec<u8>, BlockchainError> {
    let db = create(NUM_COLUMNS);
    let new_data = crate::db::Data {
        db,
        genesis_data: GenesisData {},
    };
    let empty_data = new_data.encode();
    let (backend, data) = initialize_backend(empty_data, 1)?;
    insert_light_authority_set(backend.clone(), initial_authority_set)?;

    // Ingest initial header
    let mut backend_op: sc_client::light::backend::ImportOperation<Block, storage::Storage> =
        backend.begin_operation()?;
    backend_op.set_block_data(initial_header, None, None, NewBlockState::Best)?;
    backend.commit_operation(backend_op)?;

    Ok(data.encode())
}

pub fn current_status<Block>(encoded_data: Vec<u8>) -> Result<Status<Block>, BlockchainError>
where
    Block: BlockT,
{
    let (backend, _) = initialize_backend(encoded_data, 1)?;
    let possible_light_authority_set = fetch_light_authority_set(backend.clone())?;
    let mut possible_finalized_header: Option<Block::Header> = None;
    let mut possible_best_header: Option<Block::Header> = None;
    let info: Info<Block> = backend.blockchain().info();
    if info.finalized_hash != Default::default() {
        possible_finalized_header = backend
            .blockchain()
            .header(BlockId::<Block>::Hash(info.finalized_hash))?;
    }
    if info.best_hash != Default::default() {
        possible_best_header = backend
            .blockchain()
            .header(BlockId::<Block>::Hash(info.best_hash))?;
    }
    let possible_next_change_in_authority = fetch_next_authority_change(backend.clone())?;

    Ok(Status {
        possible_finalized_header,
        possible_light_authority_set,
        possible_next_change_in_authority,
        possible_best_header,
    })
}

// WASM entry point need to call this function
pub fn ingest_finalized_header(
    encoded_data: Vec<u8>,
    finalized_header: Header,
    justification: Option<Justification>,
    max_non_finalized_blocks_allowed: u64,
) -> Result<(BlockImportResult<NumberFor<Block>>, Vec<u8>), String> {
    let (mut block_processor_fn, data) =
        setup_block_processor(encoded_data, max_non_finalized_blocks_allowed)
            .map_err(|e| format!("{}", e))?;
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
    let block_import_response = block_processor_fn(incoming_block)?;
    match &block_import_response {
        BlockImportResult::ImportedKnown(_) => {}
        BlockImportResult::ImportedUnknown(_, aux, _) => {
            if aux.bad_justification || aux.needs_finality_proof {
                return Err(format!(
                    "Error: {}",
                    "Justification is invalid or authority set is not updated."
                ));
            }
        }
    }
    Ok((block_import_response, data.encode()))
}

#[cfg(test)]
mod tests {
    use crate::common::{
        fetch_light_authority_set, fetch_next_authority_change, initialize_backend,
        LightAuthoritySet, NextChangeInAuthority,
    };
    use crate::storage::Storage;
    use crate::types::{Block, Header};
    use crate::{current_status, ingest_finalized_header, initialize_db};
    use clear_on_drop::clear::Clear;
    use finality_grandpa::{Commit, SignedPrecommit};
    use parity_scale_codec::alloc::sync::Arc;
    use parity_scale_codec::{Decode, Encode};
    use sc_client_api::Storage as StorageT;
    use sc_finality_grandpa::{AuthorityId, Message, Precommit};
    use sp_blockchain::Backend;
    use sp_core::crypto::Public;
    use sp_core::ed25519;
    use sp_finality_grandpa::{
        AuthorityList, AuthoritySignature, ScheduledChange, GRANDPA_ENGINE_ID,
    };
    use sp_keyring::Ed25519Keyring;
    use sp_runtime::traits::{Block as BlockT, HashFor, Header as HeaderT, NumberFor, One};
    use sp_runtime::{DigestItem, Justification};
    use std::hash::Hash;

    fn assert_successful_db_init(
        custom_authority_set: Option<LightAuthoritySet>,
    ) -> (Vec<u8>, Header) {
        let initial_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );

        let result = if custom_authority_set.is_none() {
            initialize_db(initial_header.clone(), LightAuthoritySet::new(0, vec![]))
        } else {
            initialize_db(initial_header.clone(), custom_authority_set.unwrap())
        };
        assert!(result.is_ok());
        let encoded_data = result.unwrap();
        assert!(encoded_data.len() > 0);
        // Best header need to be updated
        assert_best_header(encoded_data.clone(), &initial_header);

        (encoded_data, initial_header)
    }

    fn assert_successful_header_ingestion(
        encoded_data: Vec<u8>,
        header: Header,
        justification: Option<Justification>,
    ) -> Vec<u8> {
        let result = ingest_finalized_header(encoded_data, header.clone(), justification, 256);
        assert!(result.is_ok());
        let encoded_data = result.unwrap().1;
        // Best header need to be updated
        assert_best_header(encoded_data.clone(), &header);
        encoded_data
    }

    fn assert_failed_header_ingestion(
        encoded_data: Vec<u8>,
        header: Header,
        justification: Option<Justification>,
        expected_error: String,
    ) {
        let result = ingest_finalized_header(encoded_data, header.clone(), justification, 256);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), expected_error);
    }

    fn create_next_header(header: Header) -> Header {
        let mut next_header = header.clone();
        next_header.number += 1;
        next_header.parent_hash = header.hash();
        next_header
    }

    fn assert_best_header(encoded_data: Vec<u8>, expected_to_be_best_header: &Header) {
        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_best_header.is_some());
        assert_eq!(
            &status.possible_best_header.unwrap(),
            expected_to_be_best_header
        );
    }

    fn assert_finalized_header(encoded_data: Vec<u8>, expected_to_be_finalized: &Header) {
        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_finalized_header.is_some());
        assert_eq!(
            &status.possible_finalized_header.unwrap(),
            expected_to_be_finalized
        );
    }

    fn assert_authority_set(
        encoded_data: Vec<u8>,
        expected_light_authority_set: &LightAuthoritySet,
    ) {
        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_light_authority_set.is_some());
        let light_authority_set = status.possible_light_authority_set.unwrap();
        assert_eq!(
            light_authority_set.set_id(),
            expected_light_authority_set.set_id()
        );
        assert_eq!(
            light_authority_set.authorities(),
            expected_light_authority_set.authorities()
        );
    }

    fn assert_next_change_in_authority(
        encoded_data: Vec<u8>,
        scheduled_change: &ScheduledChange<NumberFor<Block>>,
    ) {
        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_next_change_in_authority.is_some());
        assert_eq!(
            &status.possible_next_change_in_authority.unwrap().change,
            scheduled_change
        );
    }

    fn assert_no_next_change_in_authority(encoded_data: Vec<u8>) {
        let result = current_status::<Block>(encoded_data.clone());
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.possible_next_change_in_authority.is_none());
    }

    #[test]
    fn test_initialize_db_success() {
        let (encoded_data, initial_header) = assert_successful_db_init(None);
        let mut next_header = create_next_header(initial_header);
        assert_successful_header_ingestion(encoded_data, next_header, None);
    }

    #[test]
    fn test_initialize_db_non_sequential_block() {
        let (encoded_data, initial_header) = assert_successful_db_init(None);

        let mut next_header = create_next_header(initial_header);
        // Let's change number of block to be non sequential
        next_header.number += 1;

        assert_failed_header_ingestion(encoded_data, next_header, None, String::from("Other(ClientImport(\"Import failed: Did not finalize blocks in sequential order.\"))"));
    }

    #[test]
    fn test_initialize_db_wrong_parent_hash() {
        let (encoded_data, initial_header) = assert_successful_db_init(None);

        let mut next_header = create_next_header(initial_header);
        // Setting wrong parent hash
        next_header.parent_hash = Default::default();

        assert_failed_header_ingestion(
            encoded_data,
            next_header,
            None,
            String::from("UnknownParent"),
        );
    }

    #[test]
    fn test_authority_set_processing() {
        println!("Starting Authority set processing test");
        let (encoded_data, initial_header) = assert_successful_db_init(None);

        let mut next_header = create_next_header(initial_header);

        // Let's push scheduled change
        let change = ScheduledChange {
            next_authorities: vec![
                (AuthorityId::from_slice(&[1; 32]), 3),
                (AuthorityId::from_slice(&[1; 32]), 3),
            ],
            delay: 2,
        };
        next_header.digest_mut().push(DigestItem::Consensus(
            GRANDPA_ENGINE_ID,
            sp_finality_grandpa::ConsensusLog::ScheduledChange(change.clone()).encode(),
        ));
        // Updating encoded data
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, next_header.clone(), None);

        // We should now have next schedule change in database
        assert_next_change_in_authority(encoded_data.clone(), &change);
        // Current authority set remains same
        assert_authority_set(encoded_data.clone(), &LightAuthoritySet::new(0, vec![]));

        // We cannot push another authority set while previous one exists
        let mut next_header = create_next_header(next_header);
        next_header.digest_mut().push(DigestItem::Consensus(
            GRANDPA_ENGINE_ID,
            sp_finality_grandpa::ConsensusLog::ScheduledChange(ScheduledChange {
                next_authorities: vec![
                    (AuthorityId::from_slice(&[2; 32]), 4),
                    (AuthorityId::from_slice(&[2; 32]), 4),
                ],
                delay: 4,
            })
            .encode(),
        ));
        assert_eq!(
            ingest_finalized_header(encoded_data.clone(), next_header.clone(), None, 256),
            Err(String::from(
                "VerificationFailed(None, \"Scheduled change already exists.\")"
            ))
        );
        // After clearing digest we should be able to ingest header
        next_header.digest.clear();
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, next_header.clone(), None);

        // We can push another authority set as new authority set will be enacted.
        let mut next_header = create_next_header(next_header);
        let new_change = ScheduledChange {
            next_authorities: vec![
                (AuthorityId::from_slice(&[3; 32]), 5),
                (AuthorityId::from_slice(&[3; 32]), 5),
            ],
            delay: 2,
        };
        next_header.digest_mut().push(DigestItem::Consensus(
            GRANDPA_ENGINE_ID,
            sp_finality_grandpa::ConsensusLog::ScheduledChange(new_change.clone()).encode(),
        ));

        let encoded_data =
            assert_successful_header_ingestion(encoded_data, next_header.clone(), None);

        // Now, we have our authority set changed, and older NextChangeInAuthority struct replaced
        // by new change

        // Previous change has been overwritten by new change
        assert_next_change_in_authority(encoded_data.clone(), &new_change);

        // We now have authority set enacted as per previous change
        // Last authority set had set_id of 0
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_authority_set(
            encoded_data.clone(),
            &LightAuthoritySet::new(1, change.next_authorities.clone()),
        );

        // Now, a scenario where scheduled change isn't part of digest after two blocks delay
        // In this case new authority set will be enacted and aux entry will be removed

        let mut next_header = create_next_header(next_header.clone());
        // We don't need cloned digest
        next_header.digest.logs.clear();
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, next_header.clone(), None);

        // new change still same
        assert_next_change_in_authority(encoded_data.clone(), &new_change);

        // authority set still same
        // Last authority set had set_id of 0
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_authority_set(
            encoded_data.clone(),
            &LightAuthoritySet::new(1, change.next_authorities.clone()),
        );

        let mut next_header = create_next_header(next_header.clone());
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, next_header.clone(), None);

        // Now NextChangeInAuthority should be removed from db and authority set is changed
        assert_no_next_change_in_authority(encoded_data.clone());

        // Brand new authority set
        // Last authority set had set_id of 1
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_authority_set(
            encoded_data.clone(),
            &LightAuthoritySet::new(2, new_change.next_authorities.clone()),
        );
    }

    #[derive(Encode, Decode)]
    pub struct GrandpaJustification<Block: BlockT> {
        round: u64,
        commit: Commit<Block::Hash, NumberFor<Block>, AuthoritySignature, AuthorityId>,
        votes_ancestries: Vec<Block::Header>,
    }

    fn make_ids(keys: &[Ed25519Keyring]) -> AuthorityList {
        keys.iter()
            .map(|key| key.clone().public().into())
            .map(|id| (id, 1))
            .collect()
    }

    #[test]
    fn test_finalization() {
        let peers = &[Ed25519Keyring::Alice];
        let voters = make_ids(peers);
        let genesis_authority_set = LightAuthoritySet::new(0, voters);

        let (encoded_data, initial_header) =
            assert_successful_db_init(Some(genesis_authority_set.clone()));
        let first_header = create_next_header(initial_header.clone());
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, first_header.clone(), None);

        // Now we will try to ingest a block with justification
        let second_header = create_next_header(first_header.clone());

        let round: u64 = 1;
        let set_id: u64 = 0;
        let precommit = Precommit::<Block> {
            target_hash: second_header.hash().clone(),
            target_number: *second_header.number(),
        };
        let msg = Message::<Block>::Precommit(precommit.clone());
        let mut encoded_msg: Vec<u8> = Vec::new();
        encoded_msg.clear();
        (&msg, round, set_id).encode_to(&mut encoded_msg);
        let signature = peers[0].sign(&encoded_msg[..]).into();
        let precommit = SignedPrecommit {
            precommit,
            signature,
            id: peers[0].public().into(),
        };
        let commit = Commit {
            target_hash: second_header.parent_hash().clone(),
            target_number: *second_header.number(),
            precommits: vec![precommit],
        };

        let grandpa_justification: GrandpaJustification<Block> = GrandpaJustification {
            round,
            commit,
            votes_ancestries: vec![second_header.clone()], // first_header.clone(), initial_header.clone()
        };

        let justification = Some(grandpa_justification.encode());

        // Let's ingest it.
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, second_header.clone(), justification);

        // Finalized header should be updated
        assert_finalized_header(encoded_data.clone(), &second_header);

        let third_header = create_next_header(second_header.clone());
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, third_header.clone(), None);

        let fourth_header = create_next_header(third_header.clone());
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, fourth_header.clone(), None);

        let fifth_header = create_next_header(fourth_header.clone());
        // Another justification, finalizing third, fourth and fifth header
        let round: u64 = 1;
        let set_id: u64 = 0;
        let precommit = Precommit::<Block> {
            target_hash: fifth_header.hash().clone(),
            target_number: *fifth_header.number(),
        };
        let msg = Message::<Block>::Precommit(precommit.clone());
        let mut encoded_msg: Vec<u8> = Vec::new();
        encoded_msg.clear();
        (&msg, round, set_id).encode_to(&mut encoded_msg);
        let signature = peers[0].sign(&encoded_msg[..]).into();
        let precommit = SignedPrecommit {
            precommit,
            signature,
            id: peers[0].public().into(),
        };
        let commit = Commit {
            target_hash: fifth_header.parent_hash().clone(),
            target_number: *fifth_header.number(),
            precommits: vec![precommit],
        };

        let grandpa_justification: GrandpaJustification<Block> = GrandpaJustification {
            round,
            commit,
            votes_ancestries: vec![fifth_header.clone()], // first_header.clone(), initial_header.clone()
        };

        let justification = Some(grandpa_justification.encode());
        let encoded_data =
            assert_successful_header_ingestion(encoded_data, fifth_header.clone(), justification);
        assert_finalized_header(encoded_data.clone(), &fifth_header);
    }
}
