use sp_finality_grandpa::AuthorityList;
use parity_scale_codec::{Encode, Decode};

#[derive(Encode, Decode, Default)]
pub struct GenesisData {
    pub grandpa_authority_set: AuthorityList,
    pub grandpa_authority_set_id: u64
}