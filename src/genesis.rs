use parity_scale_codec::{Decode, Encode};
use sp_finality_grandpa::AuthorityList;

#[derive(Encode, Decode, Clone)]
pub struct GenesisData {
    pub grandpa_authority_set: AuthorityList,
    pub grandpa_authority_set_id: u64,
}
