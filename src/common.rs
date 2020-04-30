use sp_finality_grandpa::ScheduledChange;
use sp_runtime::traits::{NumberFor, Block as BlockT};
use parity_scale_codec::{Encode, Decode};

// Purposely shorthanded name just to save few bytes of storage
pub const NEXT_CHANGE_IN_AUTHORITY_KEY: &'static [u8] = b"ibc_nca";
pub static GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY: &[u8] = b"grandpa_nca";

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