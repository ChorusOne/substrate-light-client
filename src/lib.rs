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
    let (backend, ibc_data) = initialize_backend(empty_data, 1)?;
    insert_light_authority_set(backend.clone(), initial_authority_set)?;

    // Ingest initial header
    let mut backend_op: sc_client::light::backend::ImportOperation<Block, storage::IBCStorage> =
        backend.begin_operation()?;
    backend_op.set_block_data(initial_header, None, None, NewBlockState::Best)?;
    backend.commit_operation(backend_op)?;

    Ok(ibc_data.encode())
}

// WASM entry point need to call this function
pub fn ingest_finalized_header(
    encoded_data: Vec<u8>,
    finalized_header: Header,
    justification: Option<Justification>,
    max_non_finalized_blocks_allowed: u64,
) -> Result<(BlockImportResult<NumberFor<Block>>, Vec<u8>), String> {
    let (mut block_processor_fn, ibc_data) =
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
    Ok((block_processor_fn(incoming_block)?, ibc_data.encode()))
}

#[cfg(test)]
mod tests {
    use crate::common::{
        fetch_light_authority_set, fetch_next_authority_change, initialize_backend,
        LightAuthoritySet,
    };
    use crate::storage::IBCStorage;
    use crate::types::{Block, Header};
    use crate::{ingest_finalized_header, initialize_db};
    use clear_on_drop::clear::Clear;
    use finality_grandpa::{Commit, SignedPrecommit};
    use parity_scale_codec::alloc::sync::Arc;
    use parity_scale_codec::{Decode, Encode};
    use sc_client_api::Storage;
    use sc_finality_grandpa::{AuthorityId, Message, Precommit};
    use sp_blockchain::Backend;
    use sp_core::crypto::Public;
    use sp_core::ed25519;
    use sp_finality_grandpa::{
        AuthorityList, AuthoritySignature, ScheduledChange, GRANDPA_ENGINE_ID,
    };
    use sp_keyring::Ed25519Keyring;
    use sp_runtime::traits::{Block as BlockT, HashFor, Header as HeaderT, NumberFor, One};
    use sp_runtime::DigestItem;

    fn init_test_db(custom_authority_set: Option<LightAuthoritySet>) -> (Vec<u8>, Header) {
        let initial_header = Header::new(
            One::one(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );

        let data = if custom_authority_set.is_none() {
            initialize_db(initial_header.clone(), LightAuthoritySet::new(0, vec![])).unwrap()
        } else {
            initialize_db(initial_header.clone(), custom_authority_set.unwrap()).unwrap()
        };
        assert!(data.len() > 0);

        (data, initial_header)
    }

    fn create_next_header(header: Header) -> Header {
        let mut next_header = header.clone();
        next_header.number += 1;
        next_header.parent_hash = header.hash();
        next_header
    }

    #[test]
    fn test_initialize_db_success() {
        let (encoded_data, initial_header) = init_test_db(None);
        let mut next_header = create_next_header(initial_header);

        assert!(ingest_finalized_header(encoded_data, next_header, None, 256).is_ok());
    }

    #[test]
    fn test_initialize_db_non_sequential_block() {
        let (encoded_data, initial_header) = init_test_db(None);

        let mut next_header = create_next_header(initial_header);
        // Let's change number of block to be non sequential
        next_header.number += 1;

        assert_eq!(ingest_finalized_header(encoded_data, next_header, None, 256), Err(String::from("Other(ClientImport(\"Import failed: Did not finalize blocks in sequential order.\"))")));
    }

    #[test]
    fn test_initialize_db_wrong_parent_hash() {
        let (encoded_data, initial_header) = init_test_db(None);

        let mut next_header = create_next_header(initial_header);
        // Setting wrong parent hash
        next_header.parent_hash = Default::default();

        assert_eq!(
            ingest_finalized_header(encoded_data, next_header, None, 256),
            Err(String::from("UnknownParent"))
        );
    }

    #[test]
    fn test_authority_set_processing() {
        let (encoded_data, initial_header) = init_test_db(None);

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
        let encoded_data = ingest_finalized_header(encoded_data, next_header.clone(), None, 256)
            .unwrap()
            .1;

        // We should now have next schedule change in database
        let (backend, ibc_data) = initialize_backend(encoded_data, 256).unwrap();
        let possible_next_authority_change =
            fetch_next_authority_change::<_, Block>(backend.clone()).unwrap();
        assert!(possible_next_authority_change.is_some());
        let next_authority_change = possible_next_authority_change.unwrap();
        assert_eq!(next_authority_change.change, change);

        // Current authority set remains same
        let possible_current_authority_set = fetch_light_authority_set(backend.clone()).unwrap();
        assert!(possible_current_authority_set.is_some());
        let current_authority_set = possible_current_authority_set.unwrap();
        assert_eq!(current_authority_set.set_id(), 0);
        assert_eq!(current_authority_set.authorities(), vec![]);

        // It is not necessary to derive encoded data here,
        // we are doing it just for the sake of highlighting
        // how encoded data is updated.
        let encoded_data = ibc_data.encode();

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
        next_header.digest.clear();
        let result = ingest_finalized_header(encoded_data, next_header.clone(), None, 256);
        assert!(result.is_ok());
        // Updating encoded data
        let encoded_data = result.unwrap().1;

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
        let result = ingest_finalized_header(encoded_data, next_header.clone(), None, 256);
        assert!(result.is_ok());
        // Updating encoded data
        let encoded_data = result.unwrap().1;

        // Now, we have our authority set changed, and older NextChangeInAuthority struct replaced
        // by new change

        // Previous change has been overwritten by new change
        let (backend, _) = initialize_backend(encoded_data.clone(), 256).unwrap();
        let possible_next_authority_change =
            fetch_next_authority_change::<_, Block>(backend.clone()).unwrap();
        assert!(possible_next_authority_change.is_some());
        let next_authority_change = possible_next_authority_change.unwrap();
        assert_eq!(new_change, next_authority_change.change);

        // We now have authority set enacted as per previous change
        let possible_current_authority_set = fetch_light_authority_set(backend.clone()).unwrap();
        assert!(possible_current_authority_set.is_some());
        let current_authority_set = possible_current_authority_set.unwrap();
        // Last authority set had set_id of 0
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_eq!(current_authority_set.set_id(), 1);
        assert_eq!(current_authority_set.authorities(), change.next_authorities);

        // Now, a scenario where scheduled change isn't part of digest after two blocks delay
        // In this case new authority set will be enacted and aux entry will be removed

        let mut next_header = create_next_header(next_header.clone());
        // We don't need cloned digest
        next_header.digest.logs.clear();
        let result = ingest_finalized_header(encoded_data.clone(), next_header.clone(), None, 256);
        assert!(result.is_ok());
        // Updating encoded data
        let encoded_data = result.unwrap().1;

        // new change still same
        let (backend, _) = initialize_backend(encoded_data.clone(), 256).unwrap();
        let possible_next_authority_change =
            fetch_next_authority_change::<_, Block>(backend.clone()).unwrap();
        assert!(possible_next_authority_change.is_some());
        let next_authority_change = possible_next_authority_change.unwrap();
        assert_eq!(new_change, next_authority_change.change);

        // authority set still same
        let possible_current_authority_set = fetch_light_authority_set(backend.clone()).unwrap();
        assert!(possible_current_authority_set.is_some());
        let current_authority_set = possible_current_authority_set.unwrap();
        // Last authority set had set_id of 0
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_eq!(current_authority_set.set_id(), 1);
        assert_eq!(current_authority_set.authorities(), change.next_authorities);

        let mut next_header = create_next_header(next_header.clone());
        let result = ingest_finalized_header(encoded_data.clone(), next_header.clone(), None, 256);
        assert!(result.is_ok());
        // Updating encoded data
        let encoded_data = result.unwrap().1;

        // Now NextChangeInAuthority should be removed from db and authority set is changed
        let (backend, _) = initialize_backend(encoded_data.clone(), 256).unwrap();
        let possible_next_authority_change =
            fetch_next_authority_change::<_, Block>(backend.clone()).unwrap();
        assert!(possible_next_authority_change.is_none());

        // Brand new authority set
        let possible_current_authority_set = fetch_light_authority_set(backend.clone()).unwrap();
        assert!(possible_current_authority_set.is_some());
        let current_authority_set = possible_current_authority_set.unwrap();
        // Last authority set had set_id of 1
        // so while ingesting new authority set it
        // was incremented by 1.
        assert_eq!(current_authority_set.set_id(), 2);
        assert_eq!(
            current_authority_set.authorities(),
            new_change.next_authorities
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

        let (encoded_data, initial_header) = init_test_db(Some(genesis_authority_set.clone()));
        let first_header = create_next_header(initial_header.clone());
        let result = ingest_finalized_header(encoded_data, first_header.clone(), None, 256);
        assert!(result.is_ok());
        // Updated data with first header
        let encoded_data = result.unwrap().1;

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
        let result = ingest_finalized_header(
            encoded_data.clone(),
            second_header.clone(),
            justification,
            256,
        );
        assert!(result.is_ok());

        let encoded_data = result.unwrap().1;
        let (backend, _ibc_data) = initialize_backend(encoded_data, 256).unwrap();
        let storage = backend.blockchain().storage();
        assert!(Storage::<Block>::last_finalized(storage).unwrap() == second_header.hash());
    }
}
