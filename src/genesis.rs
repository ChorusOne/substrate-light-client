use sp_finality_grandpa::AuthorityList;
use parity_scale_codec::{Encode, Decode};
use sc_finality_grandpa::GenesisAuthoritySetProvider;
use sp_blockchain::Error;
use sp_runtime::traits::Block as BlockT;
use sc_consensus_babe::BabeConfiguration;

#[derive(Encode, Decode)]
pub struct GenesisData {
    pub grandpa_authority_set: AuthorityList,
    pub grandpa_authority_set_id: u64,
    pub babe_configuration: BabeConfiguration
}

pub struct GenesisGrandpaAuthoritySetProvider {
    authority_set: AuthorityList,
    authority_set_id: u64
}

impl GenesisGrandpaAuthoritySetProvider {
    pub fn new(genesis_data: &GenesisData) -> Self {
        GenesisGrandpaAuthoritySetProvider{
            authority_set: genesis_data.grandpa_authority_set.clone(),
            authority_set_id: genesis_data.grandpa_authority_set_id
        }
    }
}

impl<Block> GenesisAuthoritySetProvider<Block> for GenesisGrandpaAuthoritySetProvider where Block: BlockT {
    fn get(&self) -> Result<AuthorityList, Error> {
        Ok(self.authority_set.clone())
    }
}