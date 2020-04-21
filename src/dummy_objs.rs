use sc_client::{CallExecutor, StorageProof, ExecutionStrategy};
use std::cell::RefCell;
use sp_runtime::generic::BlockId;
use core::result;
use std::panic::UnwindSafe;
use sp_version::{NativeVersion, RuntimeVersion};
use sp_state_machine::{ExecutionManager, InMemoryBackend};
use sp_runtime::traits::{Block as BlockT, HashFor, NumberFor};
use sp_api::{Core, NativeOrEncoded, InitializeBlock, StorageTransactionCache, ProofRecorder, ConstructRuntimeApi, ApiRef, CallApiAt, ApiExt, ApiErrorExt, RuntimeApiInfo, StorageChanges, OverlayedChanges, ChangesTrieState};
use sp_externalities::{Extensions, Externalities};
use sc_client_api::light::Storage as LightStorage;
use sp_blockchain::Error;
use std::marker::PhantomData;
use parity_scale_codec::{Decode, Encode};
use sp_core::traits::{CodeExecutor, RuntimeCode, CallInWasm, CloneableSpawn};
use sp_runtime::codec;
use sp_block_builder::BlockBuilder;
use futures_task::{Spawn, SpawnError, FutureObj};
use sp_consensus_babe::BabeApi;

/**
  fn BlockBuilder_apply_extrinsic_runtime_api_impl(&self, _: &sp_runtime::generic::block::BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<<Block as sp_runtime::traits::Block>::Extrinsic>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<std::result::Result<std::result::Result<(), sp_runtime::DispatchError>, sp_runtime::transaction_validity::TransactionValidityError>>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
  fn BlockBuilder_finalize_block_runtime_api_impl(&self, _: &sp_runtime::generic::block::BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<<Block as sp_runtime::traits::Block>::Header>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
  fn BlockBuilder_inherent_extrinsics_runtime_api_impl(&self, _: &sp_runtime::generic::block::BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<sp_inherents::InherentData>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<std::vec::Vec<<Block as sp_runtime::traits::Block>::Extrinsic>>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
  fn BlockBuilder_check_inherents_runtime_api_impl(&self, _: &sp_runtime::generic::block::BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<(Block, sp_inherents::InherentData)>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<sp_inherents::CheckInherentsResult>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
  fn BlockBuilder_random_seed_runtime_api_impl(&self, _: &sp_runtime::generic::block::BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<<Block as sp_runtime::traits::Block>::Hash>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }

**/


pub struct DummySpawnHandle {

}

impl Spawn for DummySpawnHandle{
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        unimplemented!()
    }
}

impl CloneableSpawn for DummySpawnHandle{
    fn clone(&self) -> Box<dyn CloneableSpawn> {
        Box::new(DummySpawnHandle{})
    }
}

pub struct DummyApiExt {

}

impl ApiErrorExt for DummyApiExt {
    type Error = Error;
}

impl<Block> ApiExt<Block> for DummyApiExt where Block: BlockT {
    type StateBackend = InMemoryBackend<HashFor<Block>>;

    fn map_api_result<F: FnOnce(&Self) -> result::Result<R, E>, R, E>(&self, map_call: F) -> Result<R, E> where Self: Sized {
        unimplemented!()
    }

    fn has_api<A: RuntimeApiInfo + ?Sized>(&self, at: &BlockId<Block>) -> Result<bool, Self::Error> where Self: Sized {
        unimplemented!()
    }

    fn has_api_with<A: RuntimeApiInfo + ?Sized, P: Fn(u32) -> bool>(&self, at: &BlockId<Block>, pred: P) -> Result<bool, Self::Error> where Self: Sized {
        unimplemented!()
    }

    fn record_proof(&mut self) {
        unimplemented!()
    }

    fn extract_proof(&mut self) -> Option<StorageProof> {
        unimplemented!()
    }

    fn into_storage_changes(&self, backend: &Self::StateBackend, changes_trie_state: Option<&ChangesTrieState<HashFor<Block>, NumberFor<Block>>>, parent_hash: <Block as BlockT>::Hash) -> Result<StorageChanges<Self::StateBackend, Block>, String> where Self: Sized {
        unimplemented!()
    }
}

impl<Block> BabeApi<Block> for DummyApiExt where Block: BlockT {
    fn BabeApi_configuration_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<sp_consensus_babe::BabeConfiguration>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BabeApi_current_epoch_start_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<u64>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
}

impl<Block> Core<Block> for DummyApiExt where Block: BlockT {
    fn Core_version_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<sp_version::RuntimeVersion>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn Core_execute_block_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<Block>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<()>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn Core_initialize_block_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<&<Block as sp_runtime::traits::Block>::Header>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<()>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
}

impl<Block> BlockBuilder<Block> for DummyApiExt where Block: BlockT {
    fn BlockBuilder_apply_extrinsic_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<<Block as sp_runtime::traits::Block>::Extrinsic>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<std::result::Result<std::result::Result<(), sp_runtime::DispatchError>, sp_runtime::transaction_validity::TransactionValidityError>>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_finalize_block_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<<Block as sp_runtime::traits::Block>::Header>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_inherent_extrinsics_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<sp_inherents::InherentData>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<std::vec::Vec<<Block as sp_runtime::traits::Block>::Extrinsic>>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_check_inherents_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<(Block, sp_inherents::InherentData)>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<sp_inherents::CheckInherentsResult>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_random_seed_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<<Block as sp_runtime::traits::Block>::Hash>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
}

pub struct DummyConstructRuntimeApi {

}

impl<Block, C> ConstructRuntimeApi<Block, C> for DummyConstructRuntimeApi where Block: BlockT, C: CallApiAt<Block> {
    type RuntimeApi = DummyApiExt;

    fn construct_runtime_api(call: &C) -> ApiRef<Self::RuntimeApi> {
        unimplemented!()
    }
}


pub struct DummyCallExecutor<B: BlockT, Storage: LightStorage<B>>{
    pub _phantom: PhantomData<B>,
    pub _phantom2: PhantomData<Storage>,
}

impl<B, Storage> Clone for DummyCallExecutor<B, Storage> where B: BlockT, Storage: LightStorage<B> {
    fn clone(&self) -> Self {
        DummyCallExecutor {
            _phantom: PhantomData,
            _phantom2: PhantomData
        }
    }
}

impl<B, Storage> CallInWasm for DummyCallExecutor<B, Storage> where B: BlockT, Storage: LightStorage<B>   {
    fn call_in_wasm(&self, wasm_code: &[u8], code_hash: Option<Vec<u8>>, method: &str, call_data: &[u8], ext: &mut dyn Externalities) -> Result<Vec<u8>, String> {
        unimplemented!()
    }
}

impl<B, Storage> CodeExecutor for DummyCallExecutor<B, Storage> where B: BlockT, Storage: LightStorage<B> + 'static {
    type Error = Error;

    fn call<
        R: codec::Codec + PartialEq,
        NC: FnOnce() -> Result<R, String> + UnwindSafe,
    >(&self, ext: &mut dyn Externalities, runtime_code: &RuntimeCode<'_>, method: &str, data: &[u8], use_native: bool, native_call: Option<NC>) -> (Result<NativeOrEncoded<R>, Self::Error>, bool) {
        unimplemented!()
    }
}

impl<B, Storage> CallExecutor<B> for DummyCallExecutor<B, Storage> where B: BlockT, Storage: LightStorage<B> {
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
