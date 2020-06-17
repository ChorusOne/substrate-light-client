use crate::types::BlockNumber;
use serde::{Deserialize, Serialize};
use sp_finality_grandpa::{AuthorityList, SetId};

// This type is similar to primitive_types::H256 and
// redeclared here to simplify state variables and
// make them independent to parity types.
pub type H256 = Vec<u8>;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ContractState {
    pub name: String,
    pub best_header_height: BlockNumber,
    pub best_header_hash: H256,
    pub last_finalized_header_hash: H256,
    pub best_header_commitment_root: H256,
    pub light_client_data: Vec<u8>,
    pub frozen_height: Option<BlockNumber>,
    pub max_non_finalized_blocks_allowed: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ConsensusState {
    pub set_id: Option<SetId>,
    pub authorities: Option<AuthorityList>,
    pub commitment_root: Option<H256>,
}
