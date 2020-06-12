use parity_scale_codec::{Decode, Encode};
use sp_finality_grandpa::ScheduledChange;
use sp_runtime::traits::{Block as BlockT, NumberFor};

#[derive(Encode, Decode)]
pub struct NextChangeInAuthority<Block>
where
    Block: BlockT,
{
    pub next_change_at: NumberFor<Block>,
    pub change: ScheduledChange<NumberFor<Block>>,
}

impl<Block> NextChangeInAuthority<Block>
where
    Block: BlockT,
{
    pub fn new(
        next_change_at: NumberFor<Block>,
        change: ScheduledChange<NumberFor<Block>>,
    ) -> Self {
        Self {
            next_change_at,
            change,
        }
    }
}
