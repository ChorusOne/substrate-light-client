use std::marker::PhantomData;
use std::panic::UnwindSafe;

use parity_scale_codec::alloc::collections::hash_map::RandomState;
use parity_scale_codec::alloc::collections::HashMap;
use parity_scale_codec::alloc::sync::Arc;
use parity_scale_codec::{Decode, Encode};
use sc_client::apply_aux;
use sc_client_api::backend::{AuxStore, BlockImportOperation, Finalizer, LockImportRun};
use sc_client_api::execution_extensions::ExecutionExtensions;
use sc_client_api::{
    backend, call_executor::ExecutorProvider, Backend, BlockchainEvents, CallExecutor,
    ClientImportOperation, FinalityNotifications, ImportNotifications, NewBlockState,
    StorageEventStream, TransactionFor,
};
use sp_api::Core;
use sp_api::{ApiRef, CallApiAt, CallApiAtParams, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_blockchain::{
    Backend as BlockchainBackend, BlockStatus, CachedHeaderMetadata, Error as BlockchainError,
    HeaderBackend, HeaderMetadata, Info, Result as BlockchainResult,
};
use sp_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, BlockStatus as ImportBlockStatus,
    Error as ConsensusError, ImportResult,
};
use sp_core::NativeOrEncoded;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use sp_storage::StorageKey;
use sp_version::RuntimeVersion;

pub struct Client<B, Block, RA, E> {
    pub backend: Arc<B>,
    pub _phantom: PhantomData<RA>,
    pub _phantom2: PhantomData<Block>,
    pub _phantom3: PhantomData<E>,
    pub aux_store_write_enabled: bool,
}

impl<B, Block, RA, E> Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block>,
{
    pub fn block_status(&self, id: &BlockId<Block>) -> BlockchainResult<ImportBlockStatus> {
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
    }
}

impl<B, Block, RA, E> Client<B, Block, RA, E> {
    pub fn clone_with_read_write_aux_store(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            _phantom: self._phantom.clone(),
            _phantom2: self._phantom2.clone(),
            _phantom3: self._phantom3.clone(),
            aux_store_write_enabled: true,
        }
    }

    pub fn clone_with_read_only_aux_store(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            _phantom: self._phantom.clone(),
            _phantom2: self._phantom2.clone(),
            _phantom3: self._phantom3.clone(),
            aux_store_write_enabled: false,
        }
    }
}

impl<B, Block, RA, E> Clone for Client<B, Block, RA, E> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            _phantom: self._phantom.clone(),
            _phantom2: self._phantom2.clone(),
            _phantom3: self._phantom3.clone(),
            aux_store_write_enabled: self.aux_store_write_enabled,
        }
    }
}

impl<B, Block, RA, E> LockImportRun<Block, B> for &Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block>,
{
    fn lock_import_and_run<R, Err, F>(&self, f: F) -> Result<R, Err>
    where
        F: FnOnce(&mut ClientImportOperation<Block, B>) -> Result<R, Err>,
        Err: From<BlockchainError>,
    {
        (**self).lock_import_and_run(f)
    }
}

impl<B, Block, RA, E> LockImportRun<Block, B> for Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block>,
{
    fn lock_import_and_run<R, Err, F>(&self, f: F) -> Result<R, Err>
    where
        F: FnOnce(&mut ClientImportOperation<Block, B>) -> Result<R, Err>,
        Err: From<BlockchainError>,
    {
        let inner = || {
            let _import_lock = self.backend.get_import_lock().write();

            let mut op = ClientImportOperation {
                op: self.backend.begin_operation()?,
                notify_imported: None,
                notify_finalized: Vec::new(),
            };

            let r = f(&mut op)?;

            let ClientImportOperation {
                op,
                notify_imported,
                notify_finalized,
            } = op;
            self.backend.commit_operation(op)?;

            Ok(r)
        };

        inner()
    }
}

impl<B, Block, RA, E> AuxStore for Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block> + backend::AuxStore,
{
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item = &'a &'b [u8]>,
    >(
        &self,
        insert: I,
        delete: D,
    ) -> BlockchainResult<()> {
        if !self.aux_store_write_enabled {
            Ok(())
        } else {
            self.lock_import_and_run(|op| apply_aux(op, insert, delete))
        }
    }

    fn get_aux(&self, key: &[u8]) -> BlockchainResult<Option<Vec<u8>>> {
        backend::AuxStore::get_aux(&*self.backend, key)
    }
}

impl<B, Block, RA, E> AuxStore for &Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block> + backend::AuxStore,
{
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item = &'a &'b [u8]>,
    >(
        &self,
        insert: I,
        delete: D,
    ) -> BlockchainResult<()> {
        (**self).insert_aux(insert, delete)
    }

    fn get_aux(&self, key: &[u8]) -> BlockchainResult<Option<Vec<u8>>> {
        (**self).get_aux(key)
    }
}

impl<B, Block, RA, E> HeaderMetadata<Block> for Client<B, Block, RA, E>
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

impl<B, Block, RA, E> HeaderBackend<Block> for Client<B, Block, RA, E>
where
    Block: BlockT,
    RA: Sync + Send,
    B: Sync + Send + Backend<Block>,
    E: Sync + Send,
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

impl<B, Block, RA, E> HeaderBackend<Block> for &Client<B, Block, RA, E>
where
    Block: BlockT,
    RA: Sync + Send,
    B: Sync + Send + Backend<Block>,
    E: Sync + Send,
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

impl<B, Block, RA, E> ProvideRuntimeApi<Block> for Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block>,
    RA: ConstructRuntimeApi<Block, Self>,
{
    type Api = <RA as ConstructRuntimeApi<Block, Self>>::RuntimeApi;

    fn runtime_api(&self) -> ApiRef<Self::Api> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> CallApiAt<Block> for Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block>,
{
    type Error = BlockchainError;
    type StateBackend = B::State;

    fn call_api_at<
        R: Encode + Decode + PartialEq,
        NC: FnOnce() -> std::result::Result<R, String> + UnwindSafe,
        C: Core<Block, Error = Self::Error>,
    >(
        &self,
        params: CallApiAtParams<Block, C, NC, Self::StateBackend>,
    ) -> Result<NativeOrEncoded<R>, Self::Error> {
        unimplemented!()
    }

    fn runtime_version_at(&self, at: &BlockId<Block>) -> Result<RuntimeVersion, Self::Error> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> BlockImport<Block> for &Client<B, Block, RA, E>
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

        match self
            .block_status(&BlockId::Hash(hash))
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

        match self
            .block_status(&BlockId::Hash(parent_hash))
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
        self.lock_import_and_run(|operation| {
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

            assert!(justification.is_some() && finalized || justification.is_none());

            if !intermediates.is_empty() {
                return Err(BlockchainError::IncompletePipeline);
            }

            let hash = header.hash();
            let status: BlockStatus = self.backend.blockchain().status(BlockId::Hash(hash))?;

            match status {
                BlockStatus::InChain => return Ok(ImportResult::AlreadyInChain),
                BlockStatus::Unknown => {}
            }

            let info = self.backend.blockchain().info();

            // the block is lower than our last finalized block so it must revert
            // finality, refusing import.
            if *header.number() <= info.finalized_number {
                return Err(BlockchainError::NotInFinalizedChain);
            }

            operation.op.set_block_data(
                header.clone(),
                body,
                justification,
                NewBlockState::Normal,
            )?;

            Ok(ImportResult::imported(false))
        })
        .map_err(|e| ConsensusError::ClientImport(e.to_string()).into())
    }
}

impl<B, Block, RA, E> BlockImport<Block> for Client<B, Block, RA, E>
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

impl<B, Block, RA, E> Finalizer<Block, B> for Client<B, Block, RA, E>
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

        if to_be_finalized == last_finalized {
            return Ok(());
        }

        let route_from_finalized =
            sp_blockchain::tree_route(self.backend.blockchain(), last_finalized, to_be_finalized)?;

        // Since we do not allow forks, retracted always needs to be empty and
        // enacted always need to be non-empty
        assert!(route_from_finalized.retracted().is_empty());
        assert!(!route_from_finalized.enacted().is_empty());

        let enacted = route_from_finalized.enacted();
        assert!(enacted.len() > 0);
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
        self.lock_import_and_run(|op| self.apply_finality(op, id, justification, notify))
    }
}

impl<B, Block, RA, E> Finalizer<Block, B> for &Client<B, Block, RA, E>
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

impl<B, Block, RA, E> ExecutorProvider<Block> for Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block>,
    E: CallExecutor<Block>,
{
    type Executor = E;

    fn executor(&self) -> &Self::Executor {
        unimplemented!()
    }

    fn execution_extensions(&self) -> &ExecutionExtensions<Block> {
        unimplemented!()
    }
}

// Blockchain Events are not being fired while importing block.
// So, no need to implement it.
impl<B, Block, RA, E> BlockchainEvents<Block> for Client<B, Block, RA, E>
where
    Block: BlockT,
    B: Backend<Block>,
{
    fn import_notification_stream(&self) -> ImportNotifications<Block> {
        unimplemented!()
    }

    fn finality_notification_stream(&self) -> FinalityNotifications<Block> {
        unimplemented!()
    }

    fn storage_changes_notification_stream(
        &self,
        filter_keys: Option<&[StorageKey]>,
        child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>,
    ) -> BlockchainResult<StorageEventStream<Block::Hash>> {
        unimplemented!()
    }
}
