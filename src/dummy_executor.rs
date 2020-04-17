use sc_client::{CallExecutor, StorageProof, ExecutionStrategy};
use std::cell::RefCell;
use sp_runtime::generic::BlockId;
use core::result;
use std::panic::UnwindSafe;
use sp_version::{NativeVersion, RuntimeVersion};
use sp_state_machine::{OverlayedChanges, ExecutionManager};
use sp_runtime::traits::{Block as BlockT, HashFor};
use sp_api::{NativeOrEncoded, InitializeBlock, StorageTransactionCache, ProofRecorder};
use sp_externalities::{Extensions, Externalities};
use sc_client::light::fetcher::BlockchainStorage;
use sp_blockchain::Error;
use std::marker::PhantomData;
use parity_scale_codec::{Decode, Encode};
use sp_core::traits::{CodeExecutor, RuntimeCode, CallInWasm};
use sp_runtime::codec;


pub struct DummyCallExecutor<B: BlockT, Storage: BlockchainStorage<B>>{
    _phantom: PhantomData<B>,
    _phantom2: PhantomData<Storage>,
}

impl<B, Storage> Clone for DummyCallExecutor<B, Storage> where B: BlockT, Storage: BlockchainStorage<B> {
    fn clone(&self) -> Self {
        DummyCallExecutor {
            _phantom: PhantomData,
            _phantom2: PhantomData
        }
    }
}

impl<B, Storage> CallInWasm for DummyCallExecutor<B, Storage> where B: BlockT, Storage: BlockchainStorage<B> + 'static  {
    fn call_in_wasm(&self, wasm_code: &[u8], code_hash: Option<Vec<u8>>, method: &str, call_data: &[u8], ext: &mut dyn Externalities) -> Result<Vec<u8>, String> {
        unimplemented!()
    }
}

impl<B, Storage> CodeExecutor for DummyCallExecutor<B, Storage> where B: BlockT, Storage: BlockchainStorage<B> + 'static {
    type Error = Error;

    fn call<
        R: codec::Codec + PartialEq,
        NC: FnOnce() -> Result<R, String> + UnwindSafe,
    >(&self, ext: &mut dyn Externalities, runtime_code: &RuntimeCode<'_>, method: &str, data: &[u8], use_native: bool, native_call: Option<NC>) -> (Result<NativeOrEncoded<R>, Self::Error>, bool) {
        unimplemented!()
    }
}

impl<B, Storage> CallExecutor<B> for DummyCallExecutor<B, Storage> where B: BlockT, Storage: BlockchainStorage<B> {
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
            Result<NativeOrEncoded<R>, Self::Error>
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
        storage_transaction_cache: Option<&RefCell<
            StorageTransactionCache<B, <Self::Backend as sc_client_api::backend::Backend<B>>::State>,
        >>,
        initialize_block: InitializeBlock<'a, B>,
        execution_manager: ExecutionManager<EM>,
        native_call: Option<NC>,
        proof_recorder: &Option<ProofRecorder<B>>,
        extensions: Option<Extensions>,
    ) -> sp_blockchain::Result<NativeOrEncoded<R>> where ExecutionManager<EM>: Clone {
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
        call_data: &[u8]
    ) -> Result<(Vec<u8>, StorageProof), sp_blockchain::Error> {
        let trie_state = state.as_trie_backend()
            .ok_or_else(||
                Box::new(sp_state_machine::ExecutionError::UnableToGenerateProof)
                    as Box<dyn sp_state_machine::Error>
            )?;
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
        call_data: &[u8]
    ) -> Result<(Vec<u8>, StorageProof), sp_blockchain::Error> {
        unimplemented!();
    }

    /// Get runtime version if supported.
    fn native_runtime_version(&self) -> Option<&NativeVersion> {
        unimplemented!();
    }
}
