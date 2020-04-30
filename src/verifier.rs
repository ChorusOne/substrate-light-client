use sp_consensus::import_queue::Verifier;
use sp_consensus::{BlockOrigin, BlockImportParams};
use parity_scale_codec::alloc::sync::Arc;
use parity_scale_codec::{Encode, Decode};
use sc_client_api::AuxStore;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use sp_runtime::generic::OpaqueDigestItemId;
use sp_finality_grandpa::{GRANDPA_ENGINE_ID, ConsensusLog, ScheduledChange};
use sp_runtime::traits::Header;
use std::io::Write;
use parity_scale_codec::alloc::borrow::Cow;
use crate::common::{NEXT_CHANGE_IN_AUTHORITY_KEY, GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY, NextChangeInAuthority};

fn find_scheduled_change<B: BlockT>(header: &B::Header)
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

pub struct GrandpaVerifier<Client> {
    client: Arc<Client>
}

impl<Client> GrandpaVerifier<Client> where Client: AuxStore + Send + Sync  {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client: client.clone()
        }
    }
}

impl<Block, Client> Verifier<Block> for GrandpaVerifier<Client> where Client: AuxStore + Send + Sync, Block: BlockT {
    fn verify(&mut self, origin: BlockOrigin, header: <Block as BlockT>::Header, justification: Option<Vec<u8>>, body: Option<Vec<<Block as BlockT>::Extrinsic>>) -> Result<(BlockImportParams<Block, ()>, Option<Vec<([u8; 4], Vec<u8>)>>), String> {

        let (possible_authority_change, scheduled_change_exists) = {
            let encoded_possible_authority_change = self.client.get_aux(NEXT_CHANGE_IN_AUTHORITY_KEY).map_err(|err| format!("{}", err))?;
            match encoded_possible_authority_change {
                Some(encoded_authority_change) => {
                    let next_change_in_authority: NextChangeInAuthority<Block> = NextChangeInAuthority::decode(&mut encoded_authority_change.as_slice()).map_err(|err| format!("{}", err))?;
                    if next_change_in_authority.next_change_at == *header.number() {
                        self.client.insert_aux(&[], &[NEXT_CHANGE_IN_AUTHORITY_KEY]).map_err(|err| format!("{}", err))?;
                        (Some(next_change_in_authority), false)
                    } else {
                        (None, true)
                    }
                },
                None => (None, false)
            }
        };

        let found_scheduled_authority_change = find_scheduled_change::<Block>(&header);
        let possible_next_authority_change: Option<NextChangeInAuthority<Block>> = match found_scheduled_authority_change {
            Some(scheduled_change) => {
                if scheduled_change_exists {
                    Err("Scheduled change already exists.")
                } else {
                    Ok(Some(NextChangeInAuthority::new(*header.number() + scheduled_change.delay, scheduled_change)))
                }
            }
            None => Ok(None)
        }?;

        let mut block_import_params: BlockImportParams<Block, ()> = BlockImportParams::new(BlockOrigin::NetworkBroadcast, header);
        if let Some(next_authority_change) = possible_next_authority_change {
            block_import_params.intermediates.insert(Cow::from(GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY), Box::new(next_authority_change));
        }

        Ok((block_import_params, None))
    }
}