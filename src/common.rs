use sp_finality_grandpa::{ScheduledChange, AuthorityList};
use sp_runtime::traits::{NumberFor, Block as BlockT};
use parity_scale_codec::{Encode, Decode};

// Purposely shorthanded name just to save few bytes of storage
pub const NEXT_CHANGE_IN_AUTHORITY_KEY: &'static [u8] = b"ibc_nca";
pub static GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY: &[u8] = b"grandpa_nca";


/// LightAuthoritySet is saved under this key in aux storage.
pub const LIGHT_AUTHORITY_SET_KEY: &[u8] = b"grandpa_voters";
/// Latest authority set tracker.
#[derive(Debug, Encode, Decode)]
pub struct LightAuthoritySet {
    set_id: u64,
    authorities: AuthorityList,
}

impl LightAuthoritySet {
    pub fn new(set_id: u64, authorities: AuthorityList) -> Self {
        Self {
            set_id,
            authorities
        }
    }

    pub fn construct_next_authority_set(prev_authority_set: &LightAuthoritySet, new_authority_list: AuthorityList) -> Self {
        Self {
            set_id: prev_authority_set.set_id + 1,
            authorities: new_authority_list
        }
    }
}

#[derive(Encode, Decode)]
pub struct NextChangeInAuthority<Block> where Block: BlockT {
    pub next_change_at: NumberFor<Block>,
    pub change: ScheduledChange<NumberFor<Block>>
}

impl<Block> NextChangeInAuthority<Block> where Block: BlockT {
    pub fn new(next_change_at: NumberFor<Block>, change: ScheduledChange<NumberFor<Block>>) -> Self {
        Self {
            next_change_at,
            change
        }
    }
}