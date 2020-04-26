use core::result;
use std::cell::RefCell;

use parity_scale_codec::Decode;
use sc_client::StorageProof;
use sc_consensus_babe::{BabeApi, BabeConfiguration};
use sp_api::{ApiErrorExt, ApiExt, ApiRef, CallApiAt, CallApiAtParams, ChangesTrieState, ConstructRuntimeApi, Core, ExecutionContext, InitializeBlock, InMemoryBackend, NativeOrEncoded, RuntimeApiInfo, StorageChanges};
use sp_block_builder::BlockBuilder;
use sp_blockchain::Error;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, HashFor, NumberFor, Zero};

pub struct RuntimeApiExt {
    pub babe_configuration: Option<BabeConfiguration>
}

impl ApiErrorExt for RuntimeApiExt {
    type Error = Error;
}

impl<Block> ApiExt<Block> for RuntimeApiExt where Block: BlockT {
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

impl<Block> BabeApi<Block> for RuntimeApiExt where Block: BlockT {
    fn BabeApi_configuration_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<BabeConfiguration>, <Self as sp_api::ApiErrorExt>::Error> {
        match &self.babe_configuration {
            Some(babe_configuration) => Ok(NativeOrEncoded::Native(babe_configuration.clone())),
            _ => unimplemented!()
        }
    }
    fn BabeApi_current_epoch_start_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<u64>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
}

impl<Block> Core<Block> for RuntimeApiExt where Block: BlockT {
    fn Core_version_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<sp_version::RuntimeVersion>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn Core_execute_block_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<Block>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<()>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn Core_initialize_block_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<&<Block as sp_runtime::traits::Block>::Header>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<()>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
}

impl<Block> BlockBuilder<Block> for RuntimeApiExt where Block: BlockT {
    fn BlockBuilder_apply_extrinsic_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<<Block as sp_runtime::traits::Block>::Extrinsic>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<std::result::Result<std::result::Result<(), sp_runtime::DispatchError>, sp_runtime::transaction_validity::TransactionValidityError>>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_finalize_block_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<<Block as sp_runtime::traits::Block>::Header>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_inherent_extrinsics_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<sp_inherents::InherentData>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<std::vec::Vec<<Block as sp_runtime::traits::Block>::Extrinsic>>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_check_inherents_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<(Block, sp_inherents::InherentData)>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<sp_inherents::CheckInherentsResult>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
    fn BlockBuilder_random_seed_runtime_api_impl(&self, _: &BlockId<Block>, _: sp_core::ExecutionContext, _: std::option::Option<()>, _: std::vec::Vec<u8>) -> std::result::Result<sp_core::NativeOrEncoded<<Block as sp_runtime::traits::Block>::Hash>, <Self as sp_api::ApiErrorExt>::Error> { unimplemented!() }
}

pub struct RuntimeApiConstructor {

}

impl<Block, C> ConstructRuntimeApi<Block, C> for RuntimeApiConstructor where Block: BlockT, C: CallApiAt<Block, Error=sp_blockchain::Error> {
    type RuntimeApi = RuntimeApiExt;

    // We are abusing `call_api_at` to get genesis configuration into ApiExt object
    fn construct_runtime_api(call: &C) -> ApiRef<Self::RuntimeApi> {
        let call_api_params = CallApiAtParams::<_, _, fn() -> _, _> {
            core_api: &Self::RuntimeApi{
                babe_configuration: None
            },
            at: &BlockId::number(Zero::zero()),
            function: "genesis_config",
            native_call: None,
            arguments: vec![],
            overlayed_changes: &RefCell::new(Default::default()),
            storage_transaction_cache: &RefCell::new(Default::default()),
            initialize_block: InitializeBlock::Skip,
            context: ExecutionContext::Importing,
            recorder: &None,
        };

        let response: NativeOrEncoded<BabeConfiguration> = call.call_api_at(call_api_params).unwrap();

        let babe_configuration = match response {
            NativeOrEncoded::Encoded(mut encoded_babe_configuration) => BabeConfiguration::decode(&mut encoded_babe_configuration.as_slice()).unwrap(),
            _ => unimplemented!()
        };

        RuntimeApiExt {
            babe_configuration: Some(babe_configuration)
        }.into()
    }
}
