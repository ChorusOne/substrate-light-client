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

pub struct RuntimeApiConstructor {

}

impl<Block, C> ConstructRuntimeApi<Block, C> for RuntimeApiConstructor where Block: BlockT, C: CallApiAt<Block, Error=sp_blockchain::Error> {
    type RuntimeApi = RuntimeApiExt;

    fn construct_runtime_api(call: &C) -> ApiRef<Self::RuntimeApi> {
        unimplemented!()
    }
}
