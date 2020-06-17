use serde::{Deserialize, Serialize};
use sp_finality_grandpa::{AuthorityList, SetId};
use sp_runtime::{traits::BlakeTwo256, OpaqueExtrinsic, RuntimeDebug};

pub type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;

pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;

pub type SignedBlock = sp_runtime::generic::SignedBlock<Block>;

pub type BlockNumber = u32;
pub type H256 = Vec<u8>;

#[derive(Clone, Default, RuntimeDebug, Serialize, Deserialize)]
pub struct ClientState {
    pub name: String,
    pub height: BlockNumber,
    pub hash: H256,
    pub commitment_root: H256,
    pub state: Vec<u8>,
    pub frozen_height: Option<BlockNumber>,
}

#[derive(Clone, Default, RuntimeDebug, Serialize, Deserialize)]
pub struct ConsensusState {
    pub set_id: Option<SetId>,
    pub authorities: Option<AuthorityList>,
    pub commitment_root: Option<H256>,
}
