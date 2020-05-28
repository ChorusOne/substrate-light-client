use core::result;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::panic::UnwindSafe;

use parity_scale_codec::alloc::collections::HashMap;
use parity_scale_codec::{Decode, Encode};
use sc_client::{CallExecutor, ExecutionStrategy, StorageProof};
use sc_client_api::light::Storage as LightStorage;
use sc_client_api::{
    ChangesProof, FetchChecker, RemoteBodyRequest, RemoteCallRequest, RemoteChangesRequest,
    RemoteHeaderRequest, RemoteReadChildRequest, RemoteReadRequest,
};
use sc_finality_grandpa::GenesisAuthoritySetProvider;
use sp_api::{
    InitializeBlock, NativeOrEncoded, OverlayedChanges, ProofRecorder, StorageTransactionCache,
};
use sp_blockchain::Error;
use sp_blockchain::Error as ClientError;
use sp_externalities::Extensions;
use sp_finality_grandpa::AuthorityList;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, HashFor, NumberFor};
use sp_state_machine::ExecutionManager;
use sp_version::{NativeVersion, RuntimeVersion};

#[derive(Clone)]
pub struct DummyGenesisGrandpaAuthoritySetProvider {}

impl<Block> GenesisAuthoritySetProvider<Block> for DummyGenesisGrandpaAuthoritySetProvider
where
    Block: BlockT,
{
    fn get(&self) -> Result<AuthorityList, Error> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct DummyFetchChecker;

impl<Block: BlockT> FetchChecker<Block> for DummyFetchChecker {
    fn check_header_proof(
        &self,
        _request: &RemoteHeaderRequest<Block::Header>,
        _remote_header: Option<Block::Header>,
        _remote_proof: StorageProof,
    ) -> Result<Block::Header, ClientError> {
        unimplemented!()
    }

    fn check_read_proof(
        &self,
        _request: &RemoteReadRequest<Block::Header>,
        _remote_proof: StorageProof,
    ) -> Result<HashMap<Vec<u8>, Option<Vec<u8>>>, ClientError> {
        unimplemented!()
    }

    fn check_read_child_proof(
        &self,
        _request: &RemoteReadChildRequest<Block::Header>,
        _remote_proof: StorageProof,
    ) -> Result<HashMap<Vec<u8>, Option<Vec<u8>>>, ClientError> {
        unimplemented!()
    }

    fn check_execution_proof(
        &self,
        _request: &RemoteCallRequest<Block::Header>,
        _remote_proof: StorageProof,
    ) -> Result<Vec<u8>, ClientError> {
        unimplemented!()
    }

    fn check_changes_proof(
        &self,
        _request: &RemoteChangesRequest<Block::Header>,
        _remote_proof: ChangesProof<Block::Header>,
    ) -> Result<Vec<(NumberFor<Block>, u32)>, ClientError> {
        unimplemented!()
    }

    fn check_body_proof(
        &self,
        _request: &RemoteBodyRequest<Block::Header>,
        _body: Vec<Block::Extrinsic>,
    ) -> Result<Vec<Block::Extrinsic>, ClientError> {
        unimplemented!()
    }
}

pub struct DummyCallExecutor<B: BlockT, Storage: LightStorage<B>> {
    pub _phantom: PhantomData<B>,
    pub _phantom2: PhantomData<Storage>,
}

impl<B, Storage> Clone for DummyCallExecutor<B, Storage>
where
    B: BlockT,
    Storage: LightStorage<B>,
{
    fn clone(&self) -> Self {
        DummyCallExecutor {
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<B, Storage> CallExecutor<B> for DummyCallExecutor<B, Storage>
where
    B: BlockT,
    Storage: LightStorage<B>,
{
    /// Externalities error type.
    type Error = Error;

    /// The backend used by the node.
    type Backend = sc_client::light::backend::Backend<Storage, HashFor<B>>;

    /// Execute a call to a contract on top of state in a block of given hash.
    ///
    /// No changes are made.
    fn call(
        &self,
        id: &BlockId<B>,
        method: &str,
        call_data: &[u8],
        strategy: ExecutionStrategy,
        extensions: Option<Extensions>,
    ) -> Result<Vec<u8>, sp_blockchain::Error> {
        unimplemented!();
    }

    /// Execute a contextual call on top of state in a block of a given hash.
    ///
    /// No changes are made.
    /// Before executing the method, passed header is installed as the current header
    /// of the execution context.
    fn contextual_call<
        'a,
        IB: Fn() -> sp_blockchain::Result<()>,
        EM: Fn(
            Result<NativeOrEncoded<R>, Self::Error>,
            Result<NativeOrEncoded<R>, Self::Error>,
        ) -> Result<NativeOrEncoded<R>, Self::Error>,
        R: Encode + Decode + PartialEq,
        NC: FnOnce() -> result::Result<R, String> + UnwindSafe,
    >(
        &self,
        initialize_block_fn: IB,
        at: &BlockId<B>,
        method: &str,
        call_data: &[u8],
        changes: &RefCell<OverlayedChanges>,
        storage_transaction_cache: Option<
            &RefCell<
                StorageTransactionCache<
                    B,
                    <Self::Backend as sc_client_api::backend::Backend<B>>::State,
                >,
            >,
        >,
        initialize_block: InitializeBlock<'a, B>,
        execution_manager: ExecutionManager<EM>,
        native_call: Option<NC>,
        proof_recorder: &Option<ProofRecorder<B>>,
        extensions: Option<Extensions>,
    ) -> sp_blockchain::Result<NativeOrEncoded<R>>
    where
        ExecutionManager<EM>: Clone,
    {
        unimplemented!();
    }

    /// Extract RuntimeVersion of given block
    ///
    /// No changes are made.
    fn runtime_version(&self, id: &BlockId<B>) -> Result<RuntimeVersion, sp_blockchain::Error> {
        unimplemented!();
    }

    /// Execute a call to a contract on top of given state, gathering execution proof.
    ///
    /// No changes are made.
    fn prove_at_state<S: sp_state_machine::Backend<HashFor<B>>>(
        &self,
        mut state: S,
        overlay: &mut OverlayedChanges,
        method: &str,
        call_data: &[u8],
    ) -> Result<(Vec<u8>, StorageProof), sp_blockchain::Error> {
        let trie_state = state.as_trie_backend().ok_or_else(|| {
            Box::new(sp_state_machine::ExecutionError::UnableToGenerateProof)
                as Box<dyn sp_state_machine::Error>
        })?;
        self.prove_at_trie_state(trie_state, overlay, method, call_data)
    }

    /// Execute a call to a contract on top of given trie state, gathering execution proof.
    ///
    /// No changes are made.
    fn prove_at_trie_state<S: sp_state_machine::TrieBackendStorage<HashFor<B>>>(
        &self,
        trie_state: &sp_state_machine::TrieBackend<S, HashFor<B>>,
        overlay: &mut OverlayedChanges,
        method: &str,
        call_data: &[u8],
    ) -> Result<(Vec<u8>, StorageProof), sp_blockchain::Error> {
        unimplemented!();
    }

    /// Get runtime version if supported.
    fn native_runtime_version(&self) -> Option<&NativeVersion> {
        unimplemented!();
    }
}
