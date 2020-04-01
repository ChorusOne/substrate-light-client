use sc_consensus_babe::{ConsensusLog, NextEpochDescriptor, CompatibleDigestItem, BABE_ENGINE_ID};
use sp_runtime::traits::{Block as BlockT, DigestItemFor, Header};
use sp_runtime::generic::OpaqueDigestItemId;

/// Extract the BABE epoch change digest from the given header, if it exists.
pub(crate) fn find_next_epoch_digest<B: BlockT>(header: &B::Header)
                                     -> Result<Option<NextEpochDescriptor>, &str>
    where DigestItemFor<B>: CompatibleDigestItem,
{
    let mut epoch_digest: Option<_> = None;
    for log in header.digest().logs() {
        let log = log.try_to::<ConsensusLog>(OpaqueDigestItemId::Consensus(&BABE_ENGINE_ID));
        match (log, epoch_digest.is_some()) {
            (Some(ConsensusLog::NextEpochData(_)), true) => return Err("multiple epoch change digest detected"),
            (Some(ConsensusLog::NextEpochData(epoch)), false) => epoch_digest = Some(epoch),
            _ => (),
        }
    }

    Ok(epoch_digest)
}