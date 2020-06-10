use parity_scale_codec::alloc::collections::hash_map::RandomState;
use parity_scale_codec::alloc::collections::HashMap;
use parity_scale_codec::alloc::sync::Arc;
use sc_client_api::backend::{BlockImportOperation, Finalizer};
use sc_client_api::{Backend, ClientImportOperation, NewBlockState, TransactionFor};
use sp_blockchain::Error::Consensus;
use sp_blockchain::{
    Backend as BlockchainBackend, BlockStatus, CachedHeaderMetadata, Error as BlockchainError,
    HeaderBackend, HeaderMetadata, Info, Result as BlockchainResult,
};
use sp_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, BlockStatus as ImportBlockStatus,
    Error as ConsensusError, ImportResult,
};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};

pub struct Client<B> {
    backend: Arc<B>,
}

impl<B> Client<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend: backend.clone(),
        }
    }
}

impl<B> Clone for Client<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
        }
    }
}

impl<B, Block> HeaderMetadata<Block> for Client<B>
where
    Block: BlockT,
    B: Backend<Block>,
{
    /// Error used in case the header metadata is not found.
    type Error = BlockchainError;

    fn header_metadata(
        &self,
        hash: Block::Hash,
    ) -> Result<CachedHeaderMetadata<Block>, Self::Error> {
        self.backend.blockchain().header_metadata(hash)
    }

    fn insert_header_metadata(&self, hash: Block::Hash, metadata: CachedHeaderMetadata<Block>) {
        self.backend
            .blockchain()
            .insert_header_metadata(hash, metadata)
    }

    fn remove_header_metadata(&self, hash: Block::Hash) {
        self.backend.blockchain().remove_header_metadata(hash)
    }
}

impl<B, Block> HeaderBackend<Block> for Client<B>
where
    Block: BlockT,
    B: Sync + Send + Backend<Block>,
{
    /// Get block header. Returns `None` if block is not found.
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        self.backend.blockchain().header(id)
    }

    /// Get blockchain info.
    fn info(&self) -> Info<Block> {
        self.backend.blockchain().info()
    }

    /// Get block status.
    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        self.backend.blockchain().status(id)
    }

    /// Get block number by hash. Returns `None` if the header is not in the chain.
    fn number(
        &self,
        hash: Block::Hash,
    ) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
        self.backend.blockchain().number(hash)
    }

    /// Get block hash by number. Returns `None` if the header is not in the chain.
    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        self.backend.blockchain().hash(number)
    }
}

impl<B, Block> HeaderBackend<Block> for &Client<B>
where
    Block: BlockT,
    B: Sync + Send + Backend<Block>,
{
    /// Get block header. Returns `None` if block is not found.
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        (**self).header(id)
    }

    /// Get blockchain info.
    fn info(&self) -> Info<Block> {
        (**self).info()
    }

    /// Get block status.
    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        (**self).status(id)
    }

    /// Get block number by hash. Returns `None` if the header is not in the chain.
    fn number(
        &self,
        hash: Block::Hash,
    ) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
        (**self).number(hash)
    }

    /// Get block hash by number. Returns `None` if the header is not in the chain.
    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        (**self).hash(number)
    }
}

impl<B, Block> BlockImport<Block> for &Client<B>
where
    Block: BlockT,
    B: Backend<Block>,
{
    type Error = ConsensusError;
    type Transaction = TransactionFor<B, Block>;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        let BlockCheckParams {
            hash,
            number,
            parent_hash,
            allow_missing_state,
            import_existing,
        } = block;

        let block_status = |id: &BlockId<Block>| -> BlockchainResult<ImportBlockStatus> {
            let hash_and_number = match id.clone() {
                BlockId::Hash(hash) => self.backend.blockchain().number(hash)?.map(|n| (hash, n)),
                BlockId::Number(n) => self.backend.blockchain().hash(n)?.map(|hash| (hash, n)),
            };
            match hash_and_number {
                Some((hash, number)) => {
                    if self.backend.have_state_at(&hash, number) {
                        Ok(ImportBlockStatus::InChainWithState)
                    } else {
                        Ok(ImportBlockStatus::InChainPruned)
                    }
                }
                None => Ok(ImportBlockStatus::Unknown),
            }
        };

        match block_status(&BlockId::Hash(hash))
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?
        {
            ImportBlockStatus::InChainWithState | ImportBlockStatus::Queued if !import_existing => {
                return Ok(ImportResult::AlreadyInChain)
            }
            ImportBlockStatus::InChainWithState
            | ImportBlockStatus::Queued
            | ImportBlockStatus::Unknown => {}
            ImportBlockStatus::InChainPruned => return Ok(ImportResult::AlreadyInChain),
            ImportBlockStatus::KnownBad => return Ok(ImportResult::KnownBad),
        }

        match block_status(&BlockId::Hash(parent_hash))
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?
        {
            ImportBlockStatus::InChainWithState | ImportBlockStatus::Queued => {}
            ImportBlockStatus::Unknown => return Ok(ImportResult::UnknownParent),
            ImportBlockStatus::InChainPruned if allow_missing_state => {}
            ImportBlockStatus::InChainPruned => return Ok(ImportResult::MissingState),
            ImportBlockStatus::KnownBad => return Ok(ImportResult::KnownBad),
        }
        Ok(ImportResult::imported(false))
    }

    fn import_block(
        &mut self,
        block: BlockImportParams<Block, Self::Transaction>,
        cache: HashMap<[u8; 4], Vec<u8>, RandomState>,
    ) -> Result<ImportResult, Self::Error> {
        let BlockImportParams {
            origin,
            header,
            justification,
            post_digests,
            body,
            storage_changes,
            finalized,
            auxiliary,
            fork_choice,
            intermediates,
            import_existing,
            ..
        } = block;

        let mut operation: ClientImportOperation<Block, B> = ClientImportOperation {
            op: self
                .backend
                .begin_operation()
                .map_err(|e| ConsensusError::ClientImport(e.to_string()))?,
            notify_imported: None,
            notify_finalized: Vec::new(),
        };

        assert!(justification.is_some() && finalized || justification.is_none());

        if !intermediates.is_empty() {
            return Err(BlockchainError::IncompletePipeline)
                .map_err(|e| ConsensusError::ClientImport(e.to_string()).into());
        }

        let hash = header.hash();
        let parent_hash = header.parent_hash();
        let status: BlockStatus = self
            .backend
            .blockchain()
            .status(BlockId::Hash(hash))
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;

        match status {
            BlockStatus::InChain => return Ok(ImportResult::AlreadyInChain),
            BlockStatus::Unknown => {}
        }

        operation
            .op
            .set_block_data(
                header.clone(),
                body,
                justification,
                // It's always best as we are only interested in one fork
                NewBlockState::Best,
            )
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;

        self.backend
            .commit_operation(operation.op)
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;

        Ok(ImportResult::imported(true))
    }
}

impl<B, Block> BlockImport<Block> for Client<B>
where
    Block: BlockT,
    B: Backend<Block>,
{
    type Error = ConsensusError;
    type Transaction = TransactionFor<B, Block>;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        (&*self).check_block(block)
    }

    fn import_block(
        &mut self,
        block: BlockImportParams<Block, Self::Transaction>,
        new_cache: HashMap<[u8; 4], Vec<u8>, RandomState>,
    ) -> Result<ImportResult, Self::Error> {
        (&*self).import_block(block, new_cache)
    }
}

impl<B, Block> Finalizer<Block, B> for Client<B>
where
    Block: BlockT,
    B: Backend<Block>,
{
    fn apply_finality(
        &self,
        operation: &mut ClientImportOperation<Block, B>,
        id: BlockId<Block>,
        justification: Option<Vec<u8>>,
        notify: bool,
    ) -> BlockchainResult<()> {
        let to_be_finalized = self.backend.blockchain().expect_block_hash_from_id(&id)?;
        let last_finalized = self.backend.blockchain().last_finalized()?;
        let first_set_of_blocks_to_be_finalized = last_finalized == Default::default();

        let tree_route_from = if first_set_of_blocks_to_be_finalized {
            let info = self.backend.blockchain().info();
            info.genesis_hash
        } else {
            last_finalized
        };

        if !first_set_of_blocks_to_be_finalized && to_be_finalized == last_finalized {
            return Ok(());
        }

        let route_to_be_finalized =
            sp_blockchain::tree_route(self.backend.blockchain(), tree_route_from, to_be_finalized)?;

        // Since we do not allow forks, retracted always needs to be empty and
        // enacted always need to be non-empty
        assert!(route_to_be_finalized.retracted().is_empty());
        assert!(!route_to_be_finalized.enacted().is_empty());

        let enacted = route_to_be_finalized.enacted();
        assert!(enacted.len() > 0);

        if first_set_of_blocks_to_be_finalized {
            operation
                .op
                .mark_finalized(BlockId::Hash(tree_route_from), None)?;
        }

        for finalize_new in &enacted[..enacted.len() - 1] {
            operation
                .op
                .mark_finalized(BlockId::Hash(finalize_new.hash), None)?;
        }

        assert_eq!(enacted.last().map(|e| e.hash), Some(to_be_finalized));
        operation
            .op
            .mark_finalized(BlockId::Hash(to_be_finalized), justification)?;

        Ok(())
    }

    fn finalize_block(
        &self,
        id: BlockId<Block>,
        justification: Option<Vec<u8>>,
        notify: bool,
    ) -> BlockchainResult<()> {
        let mut operation: ClientImportOperation<Block, B> = ClientImportOperation {
            op: self.backend.begin_operation()?,
            notify_imported: None,
            notify_finalized: Vec::new(),
        };
        let result = self.apply_finality(&mut operation, id, justification, notify);
        self.backend
            .commit_operation(operation.op)
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;
        result
    }
}

impl<B, Block> Finalizer<Block, B> for &Client<B>
where
    Block: BlockT,
    B: Backend<Block>,
{
    fn apply_finality(
        &self,
        operation: &mut ClientImportOperation<Block, B>,
        id: BlockId<Block>,
        justification: Option<Vec<u8>>,
        notify: bool,
    ) -> BlockchainResult<()> {
        (**self).apply_finality(operation, id, justification, notify)
    }

    fn finalize_block(
        &self,
        id: BlockId<Block>,
        justification: Option<Vec<u8>>,
        notify: bool,
    ) -> BlockchainResult<()> {
        (**self).finalize_block(id, justification, notify)
    }
}
