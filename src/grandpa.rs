use sp_finality_grandpa::{ScheduledChange, GRANDPA_ENGINE_ID, ConsensusLog};
use sp_runtime::traits::{Block as BlockT, NumberFor, Header};
use sp_runtime::generic::OpaqueDigestItemId;

pub(crate) fn find_scheduled_change<B: BlockT>(header: &B::Header)
                                    -> Option<ScheduledChange<NumberFor<B>>>
{
    let id = OpaqueDigestItemId::Consensus(&GRANDPA_ENGINE_ID);

    let filter_log = |log: ConsensusLog<NumberFor<B>>| match log {
        ConsensusLog::ScheduledChange(change) => Some(change),
        _ => None,
    };

    // find the first consensus digest with the right ID which converts to
    // the right kind of consensus log.
    header.digest().convert_first(|l| l.try_to(id).and_then(filter_log))
}

pub(crate) fn find_forced_change<B: BlockT>(header: &B::Header)
                                 -> Option<(NumberFor<B>, ScheduledChange<NumberFor<B>>)>
{
    let id = OpaqueDigestItemId::Consensus(&GRANDPA_ENGINE_ID);

    let filter_log = |log: ConsensusLog<NumberFor<B>>| match log {
        ConsensusLog::ForcedChange(delay, change) => Some((delay, change)),
        _ => None,
    };

    // find the first consensus digest with the right ID which converts to
    // the right kind of consensus log.
    header.digest().convert_first(|l| l.try_to(id).and_then(filter_log))
}
