use crate::justification::{GrandpaJustification, ProvableJustification};
use finality_grandpa::BlockNumberOps;
use parity_scale_codec::{Decode, Encode};
use sc_client_api::backend::Backend;
use sc_client_api::AuxStore;
use sc_client_api::{Finalizer, TransactionFor};
use sp_api::BlockId;
use sp_blockchain::{well_known_cache_keys, Error as ClientError, HeaderBackend};
use sp_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, Error as ConsensusError, ImportResult,
    ImportedAux,
};
use sp_finality_grandpa::AuthorityList;
use sp_runtime::traits::{Block as BlockT, DigestFor, Header, NumberFor};
use sp_runtime::Justification;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Latest authority set tracker.
#[derive(Debug, Encode, Decode)]
struct LightAuthoritySet {
    set_id: u64,
    authorities: AuthorityList,
}

/// A light block-import handler for GRANDPA.
///
/// It is responsible for:
/// - checking GRANDPA justifications;
/// - fetching finality proofs for blocks that are enacting consensus changes.
pub struct GrandpaLightBlockImport<BE, Client> {
    client: Arc<Client>,
    backend: Arc<BE>,
}

impl<BE, Client> GrandpaLightBlockImport<BE, Client> {
    pub fn new(client: Arc<Client>, backend: Arc<BE>) -> Self {
        Self {
            client: client.clone(),
            backend: backend.clone(),
        }
    }
}

impl<BE, Client> Clone for GrandpaLightBlockImport<BE, Client> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            backend: self.backend.clone(),
        }
    }
}

impl<BE, Block: BlockT, Client> BlockImport<Block> for GrandpaLightBlockImport<BE, Client>
where
    NumberFor<Block>: BlockNumberOps,
    DigestFor<Block>: Encode,
    BE: Backend<Block> + 'static,
    for<'a> &'a Client: HeaderBackend<Block>
        + BlockImport<Block, Error = ConsensusError, Transaction = TransactionFor<BE, Block>>
        + Finalizer<Block, BE>,
{
    type Error = ConsensusError;
    type Transaction = TransactionFor<BE, Block>;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        self.client.check_block(block)
    }

    fn import_block(
        &mut self,
        block: BlockImportParams<Block, Self::Transaction>,
        new_cache: HashMap<well_known_cache_keys::Id, Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        do_import_block::<_, _, _, GrandpaJustification<Block>>(
            &*self.client,
            self.backend.clone(),
            block,
            new_cache,
        )
    }
}

/// Try to import new block.
fn do_import_block<B, C, Block: BlockT, J>(
    mut client: C,
    mut backend: Arc<B>,
    mut block: BlockImportParams<Block, TransactionFor<B, Block>>,
    new_cache: HashMap<well_known_cache_keys::Id, Vec<u8>>,
) -> Result<ImportResult, ConsensusError>
where
    C: HeaderBackend<Block>
        + Finalizer<Block, B>
        + BlockImport<Block, Transaction = TransactionFor<B, Block>>
        + Clone,
    B: Backend<Block> + 'static,
    NumberFor<Block>: finality_grandpa::BlockNumberOps,
    DigestFor<Block>: Encode,
    J: ProvableJustification<Block>,
{
    let hash = block.post_hash();
    let number = block.header.number().clone();

    // we don't want to finalize on `inner.import_block`
    let justification = block.justification.take();
    let import_result = client.import_block(block, new_cache);

    let mut imported_aux = match import_result {
        Ok(ImportResult::Imported(aux)) => aux,
        Ok(r) => return Ok(r),
        Err(e) => return Err(ConsensusError::ClientImport(e.to_string()).into()),
    };

    match justification {
        Some(justification) => {
            do_import_justification::<_, _, _, J>(client, backend, hash, number, justification)
        }
        None => Ok(ImportResult::Imported(imported_aux)),
    }
}

/// Try to import justification.
fn do_import_justification<B, C, Block: BlockT, J>(
    client: C,
    backend: Arc<B>,
    hash: Block::Hash,
    number: NumberFor<Block>,
    justification: Justification,
) -> Result<ImportResult, ConsensusError>
where
    C: HeaderBackend<Block> + Finalizer<Block, B> + Clone,
    B: Backend<Block> + 'static,
    NumberFor<Block>: finality_grandpa::BlockNumberOps,
    J: ProvableJustification<Block>,
{
    let possible_light_authority_set = crate::common::fetch_light_authority_set(backend)
        .map_err(|e| ConsensusError::Other(Box::new(e)))?;
    if possible_light_authority_set.is_none() {
        return Err(ConsensusError::InvalidAuthoritiesSet);
    }
    let light_authority_set = possible_light_authority_set.unwrap();

    // Verify if justification is valid and it finalizes correct block
    let justification = J::decode_and_verify_finalization(
        &justification,
        light_authority_set.set_id(),
        (hash, number),
        &light_authority_set.authorities(),
    );

    // BadJustification error means that justification has been successfully decoded, but
    // it isn't valid within current authority set
    let justification = match justification {
        Err(ClientError::BadJustification(_)) => {
            let mut imported_aux = ImportedAux::default();
            imported_aux.needs_finality_proof = true;
            return Ok(ImportResult::Imported(imported_aux));
        }
        Err(e) => {
            return Err(ConsensusError::ClientImport(e.to_string()).into());
        }
        Ok(justification) => justification,
    };

    // finalize the block
    do_finalize_block(client, hash, number, justification.encode())
}

/// Finalize the block.
fn do_finalize_block<B, C, Block: BlockT>(
    client: C,
    hash: Block::Hash,
    number: NumberFor<Block>,
    justification: Justification,
) -> Result<ImportResult, ConsensusError>
where
    C: HeaderBackend<Block> + Finalizer<Block, B> + Clone,
    B: Backend<Block> + 'static,
    NumberFor<Block>: finality_grandpa::BlockNumberOps,
{
    // finalize the block
    client
        .finalize_block(BlockId::Hash(hash), Some(justification), true)
        .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;

    // we just finalized this block, so if we were importing it, it is now the new best
    Ok(ImportResult::imported(true))
}
