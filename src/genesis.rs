use sp_finality_grandpa::AuthorityList;
use parity_scale_codec::{Encode, Decode};
use sc_finality_grandpa::GenesisAuthoritySetProvider;
use sp_blockchain::Error;
use sp_runtime::traits::Block as BlockT;
use sc_consensus_babe::BabeConfiguration;

#[derive(Encode, Decode, Clone)]
pub struct GenesisData {
    pub grandpa_authority_set: AuthorityList,
    pub grandpa_authority_set_id: u64,
}
