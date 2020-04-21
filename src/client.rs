use sc_client_api::backend::{LockImportRun, Finalizer, AuxStore};
use sp_blockchain::{Error, HeaderMetadata, CachedHeaderMetadata, HeaderBackend, BlockStatus, Result as BlockchainResult, Info, ProvideCache, Cache};
use sc_client_api::{ClientImportOperation, Backend, TransactionFor, call_executor::ExecutorProvider, CallExecutor, BlockchainEvents, ImportNotifications, FinalityNotifications, StorageEventStream};
use sp_api::{ProvideRuntimeApi, ApiRef, ConstructRuntimeApi, CallApiAt, CallApiAtParams};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use parity_scale_codec::alloc::sync::Arc;
use std::marker::PhantomData;
use sp_core::NativeOrEncoded;
use sp_version::RuntimeVersion;
use parity_scale_codec::{Encode, Decode};
use std::panic::UnwindSafe;
use sp_api::Core;
use sp_consensus::{BlockImport, BlockCheckParams, ImportResult, BlockImportParams, Error as ConsensusError};
use parity_scale_codec::alloc::collections::hash_map::RandomState;
use parity_scale_codec::alloc::collections::HashMap;
use sc_client_api::execution_extensions::ExecutionExtensions;
use sp_storage::StorageKey;

pub struct Client<B, Block, RA, E> {
    pub backend: Arc<B>,
    pub _phantom: PhantomData<RA>,
    pub _phantom2: PhantomData<Block>,
    pub _phantom3: PhantomData<E>
}

impl<B, Block, RA, E> Clone for Client<B, Block, RA, E> {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

impl<B, Block, RA, E> LockImportRun<Block, B> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block>  {
    fn lock_import_and_run<R, Err, F>(&self, f: F) -> Result<R, Err> where
        F: FnOnce(&mut ClientImportOperation<Block, B>) -> Result<R, Err>,
        Err: From<sp_blockchain::Error> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> Finalizer<Block, B> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block> {
    fn apply_finality(&self, operation: &mut ClientImportOperation<Block, B>, id: BlockId<Block>, justification: Option<Vec<u8>>, notify: bool) -> BlockchainResult<()> {
        unimplemented!()
    }

    fn finalize_block(&self, id: BlockId<Block>, justification: Option<Vec<u8>>, notify: bool) -> BlockchainResult<()> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> Finalizer<Block, B> for &Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block> {
    fn apply_finality(&self, operation: &mut ClientImportOperation<Block, B>, id: BlockId<Block>, justification: Option<Vec<u8>>, notify: bool) -> BlockchainResult<()> {
        (**self).apply_finality(operation, id, justification, notify)
    }

    fn finalize_block(&self, id: BlockId<Block>, justification: Option<Vec<u8>>, notify: bool) -> BlockchainResult<()> {
        (**self).finalize_block(id, justification, notify)
    }
}



impl<B, Block, RA, E> AuxStore for Client<B, Block, RA, E> {
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item=&'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item=&'a &'b [u8]>,
    >(&self, insert: I, delete: D) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_aux(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> AuxStore for &Client<B, Block, RA, E> {
    fn insert_aux<
        'a,
        'b: 'a,
        'c: 'a,
        I: IntoIterator<Item=&'a (&'c [u8], &'c [u8])>,
        D: IntoIterator<Item=&'a &'b [u8]>,
    >(&self, insert: I, delete: D) -> Result<(), Error> {
        (**self).insert_aux(insert, delete)
    }

    fn get_aux(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        (**self).get_aux(key)
    }
}

impl<B, Block, RA, E> HeaderMetadata<Block> for Client<B, Block, RA, E> where Block: BlockT {
    /// Error used in case the header metadata is not found.
    type Error = sp_blockchain::Error;

    fn header_metadata(&self, hash: Block::Hash) -> Result<CachedHeaderMetadata<Block>, Self::Error> {
        unimplemented!();
    }

    fn insert_header_metadata(&self, hash: Block::Hash, header_metadata: CachedHeaderMetadata<Block>) {
        unimplemented!();
    }


    fn remove_header_metadata(&self, hash: Block::Hash) {
        unimplemented!();
    }
}

impl<B, Block, RA, E> HeaderBackend<Block> for Client<B, Block, RA, E> where Block: BlockT, RA: Sync + Send, B: Sync + Send, E: Sync + Send {
    /// Get block header. Returns `None` if block is not found.
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        unimplemented!();
    }

    /// Get blockchain info.
    fn info(&self) -> Info<Block> {
        unimplemented!();
    }

    /// Get block status.
    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        unimplemented!();
    }

    /// Get block number by hash. Returns `None` if the header is not in the chain.
    fn number(&self, hash: Block::Hash) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
        unimplemented!();
    }

    /// Get block hash by number. Returns `None` if the header is not in the chain.
    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        unimplemented!();
    }
}

impl<B, Block, RA, E> HeaderBackend<Block> for &Client<B, Block, RA, E> where Block: BlockT, RA: Sync + Send, B: Sync + Send, E: Sync + Send {
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
    fn number(&self, hash: Block::Hash) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
        (**self).number(hash)
    }

    /// Get block hash by number. Returns `None` if the header is not in the chain.
    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        (**self).hash(number)
    }
}

impl<B, Block, RA, E> ProvideRuntimeApi<Block> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block>, RA: ConstructRuntimeApi<Block, Self>  {
    type Api = <RA as ConstructRuntimeApi<Block, Self>>::RuntimeApi;

    fn runtime_api(&self) -> ApiRef<Self::Api> {
        RA::construct_runtime_api(self)
    }
}

impl<B, Block, RA, E> CallApiAt<Block> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block> {
    type Error = sp_blockchain::Error;
    type StateBackend = B::State;

    fn call_api_at<R: Encode + Decode + PartialEq, NC: FnOnce() -> std::result::Result<R, String> + UnwindSafe, C: Core<Block, Error=Self::Error>>(&self, params: CallApiAtParams<Block, C, NC, Self::StateBackend>) -> Result<NativeOrEncoded<R>, Self::Error> {
        unimplemented!()
    }

    fn runtime_version_at(&self, at: &BlockId<Block>) -> Result<RuntimeVersion, Self::Error> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> BlockImport<Block> for &Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block> {
    type Error = ConsensusError;
    type Transaction = TransactionFor<B, Block>;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        unimplemented!()
    }

    fn import_block(&mut self, block: BlockImportParams<Block, Self::Transaction>, cache: HashMap<[u8; 4], Vec<u8>, RandomState>) -> Result<ImportResult, Self::Error> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> BlockImport<Block> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block> {
    type Error = ConsensusError;
    type Transaction = TransactionFor<B, Block>;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        (&*self).check_block(block)
    }

    fn import_block(&mut self, block: BlockImportParams<Block, Self::Transaction>, new_cache: HashMap<[u8; 4], Vec<u8>, RandomState>) -> Result<ImportResult, Self::Error> {
        (&*self).import_block(block, new_cache)
    }
}

impl<B, Block, RA, E> ProvideCache<Block> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block> {
    fn cache(&self) -> Option<Arc<dyn Cache<Block>>> {
        unimplemented!()
    }
}

impl<B, Block, RA, E> ExecutorProvider<Block> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block>, E: CallExecutor<Block> {
    type Executor = E;

    fn executor(&self) -> &Self::Executor {
        unimplemented!()
    }

    fn execution_extensions(&self) -> &ExecutionExtensions<Block> {
        unimplemented!()
    }
}

impl <B, Block, RA, E> BlockchainEvents<Block> for Client<B, Block, RA, E> where Block: BlockT, B: Backend<Block> {
    fn import_notification_stream(&self) -> ImportNotifications<Block> {
        unimplemented!()
    }

    fn finality_notification_stream(&self) -> FinalityNotifications<Block> {
        unimplemented!()
    }

    fn storage_changes_notification_stream(&self, filter_keys: Option<&[StorageKey]>, child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>) -> Result<StorageEventStream<Block::Hash>, Error> {
        unimplemented!()
    }
}
