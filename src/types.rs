use sp_runtime::{
    generic,
    traits::BlakeTwo256,
    OpaqueExtrinsic, RuntimeDebug,
};

pub type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;

use sp_finality_grandpa::{AuthorityList, SetId};

type BlockNumber = u32;
type H256 = Vec<u8>;

#[derive(Clone, Default, Encode, Decode, RuntimeDebug, Serialize, Deserialize)]
pub struct ClientState {
    pub height: BlockNumber,
    pub hash: H256,
    pub commitment_root: H256,
    pub state: Vec<u8>,
    frozen_height: Option<BlockNumber>,
}

#[derive(Clone, Default, Encode, Decode, RuntimeDebug, Serialize, Deserialize)]
pub struct ConsensusState {
    pub set_id: SetId,
    pub authorities: AuthorityList,
    pub commitment_root: H256,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Serialize, Debug, Deserialize, JsonSchema)]
pub struct JsonHeader {
    pub height: BlockNumber,
    pub block_hash: H256,
    pub commitment_root: H256,
    pub justification: Option<Vec<u8>>,
    //pub authorities_proof: Option<Vec<Vec<u8>>>,
}
