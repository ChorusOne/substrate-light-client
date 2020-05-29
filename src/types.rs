use sp_runtime::codec::{Decode, Encode};

use sp_runtime::{
    //generic,
    traits::BlakeTwo256,
    OpaqueExtrinsic, RuntimeDebug,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;

pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;

use sp_finality_grandpa::{AuthorityList, SetId};

pub type BlockNumber = u32;
pub type H256 = Vec<u8>;

#[derive(Clone, Default, Encode, Decode, RuntimeDebug, Serialize, Deserialize)]
pub struct ClientState {
    pub name: String,
    pub height: BlockNumber,
    pub hash: H256,
    pub commitment_root: H256,
    pub state: Vec<u8>,
    pub frozen_height: Option<BlockNumber>,
}

#[derive(Clone, Default, Encode, Decode, RuntimeDebug, Serialize, Deserialize)]
pub struct ConsensusState {
    pub set_id: Option<SetId>,
    pub authorities: Option<AuthorityList>,
    pub commitment_root: Option<H256>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Serialize, Debug, Deserialize, JsonSchema)]
pub struct JsonHeader {
    pub height: BlockNumber,
    pub header: String,
    pub block_hash: H256,
    pub commitment_root: H256,
    pub justification: Option<String>,
    pub authority_set: Option<String>,
    //pub authorities_proof: Option<Vec<Vec<u8>>>,
}

