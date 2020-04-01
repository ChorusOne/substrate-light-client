use sc_consensus_babe::{ConsensusLog as BabeConsensusLog, NextEpochDescriptor, CompatibleDigestItem, BABE_ENGINE_ID};
use sp_finality_grandpa::{ScheduledChange, GRANDPA_ENGINE_ID, ConsensusLog as GrandpaConsensusLog};
use sp_runtime::traits::{Block as BlockT, NumberFor, DigestItemFor};
use sp_runtime::generic::OpaqueDigestItemId;

fn find_scheduled_change<B: BlockT>(header: &B::Header)
                                    -> Option<ScheduledChange<NumberFor<B>>>
{
    let id = OpaqueDigestItemId::Consensus(&GRANDPA_ENGINE_ID);

    let filter_log = |log: GrandpaConsensusLog<NumberFor<B>>| match log {
        GrandpaConsensusLog::ScheduledChange(change) => Some(change),
        _ => None,
    };

    // find the first consensus digest with the right ID which converts to
    // the right kind of consensus log.
    header.digest().convert_first(|l| l.try_to(id).and_then(filter_log))
}

fn find_forced_change<B: BlockT>(header: &B::Header)
                                 -> Option<(NumberFor<B>, ScheduledChange<NumberFor<B>>)>
{
    let id = OpaqueDigestItemId::Consensus(&GRANDPA_ENGINE_ID);

    let filter_log = |log: GrandpaConsensusLog<NumberFor<B>>| match log {
        GrandpaConsensusLog::ForcedChange(delay, change) => Some((delay, change)),
        _ => None,
    };

    // find the first consensus digest with the right ID which converts to
    // the right kind of consensus log.
    header.digest().convert_first(|l| l.try_to(id).and_then(filter_log))
}

/// Extract the BABE epoch change digest from the given header, if it exists.
fn find_next_epoch_digest<B: BlockT>(header: &B::Header)
                                     -> Result<Option<NextEpochDescriptor>, &str>
    where DigestItemFor<B>: CompatibleDigestItem,
{
    let mut epoch_digest: Option<_> = None;
    for log in header.digest().logs() {
        trace!(target: "ibc_module", "Checking log {:?}, looking for epoch change digest.", log);
        let log = log.try_to::<BabeConsensusLog>(OpaqueDigestItemId::Consensus(&BABE_ENGINE_ID));
        match (log, epoch_digest.is_some()) {
            (Some(BabeConsensusLog::NextEpochData(_)), true) => return Err("multiple epoch change digest detected"),
            (Some(BabeConsensusLog::NextEpochData(epoch)), false) => epoch_digest = Some(epoch),
            _ => trace!(target: "ibc_module", "Ignoring digest not meant for us"),
        }
    }

    Ok(epoch_digest)
}

fn ingest_finalized_header<B: BlockT>(finalized_header: &B::Header) -> Result<(), Err> {

}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
