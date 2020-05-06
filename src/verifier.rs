use crate::common::{
    LightAuthoritySet, NextChangeInAuthority, GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY,
    LIGHT_AUTHORITY_SET_KEY, NEXT_CHANGE_IN_AUTHORITY_KEY,
};
use parity_scale_codec::alloc::borrow::Cow;
use parity_scale_codec::alloc::sync::Arc;
use parity_scale_codec::{Decode, Encode};
use sc_client_api::AuxStore;
use sp_consensus::import_queue::Verifier;
use sp_consensus::{BlockImportParams, BlockOrigin};
use sp_finality_grandpa::{ConsensusLog, ScheduledChange, GRANDPA_ENGINE_ID};
use sp_runtime::generic::OpaqueDigestItemId;
use sp_runtime::traits::Header;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use std::marker::PhantomData;

fn find_scheduled_change<B: BlockT>(header: &B::Header) -> Option<ScheduledChange<NumberFor<B>>> {
    let id = OpaqueDigestItemId::Consensus(&GRANDPA_ENGINE_ID);

    let filter_log = |log: ConsensusLog<NumberFor<B>>| match log {
        ConsensusLog::ScheduledChange(change) => Some(change),
        _ => None,
    };

    // find the first consensus digest with the right ID which converts to
    // the right kind of consensus log.
    header
        .digest()
        .convert_first(|l| l.try_to(id).and_then(filter_log))
}

pub struct GrandpaVerifier<Client, Block> {
    client: Arc<Client>,
    _phantom: PhantomData<Block>,
}

impl<Block, Client> GrandpaVerifier<Client, Block>
where
    Client: AuxStore + Send + Sync,
    Block: BlockT,
{
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client: client.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn insert_authority_set(
        &self,
        light_authority_set: LightAuthoritySet,
    ) -> Result<(), String> {
        self.client
            .insert_aux(
                &[(
                    LIGHT_AUTHORITY_SET_KEY,
                    light_authority_set.encode().as_slice(),
                )],
                &[],
            )
            .map_err(|err| format!("{}", err))
    }

    pub fn fetch_stored_authority_set(&self) -> Result<Option<LightAuthoritySet>, String> {
        let encoded_possible_light_authority_set = self
            .client
            .get_aux(LIGHT_AUTHORITY_SET_KEY)
            .map_err(|err| format!("{}", err))?;

        if encoded_possible_light_authority_set.is_none() {
            return Ok(None);
        }

        let encoded_light_authority_set = encoded_possible_light_authority_set.unwrap();

        let light_authority_set =
            LightAuthoritySet::decode(&mut encoded_light_authority_set.as_slice())
                .map_err(|err| format!("{}", err))?;

        Ok(Some(light_authority_set))
    }

    pub fn delete_stored_next_authority_change(&self) -> Result<(), String> {
        self.client
            .insert_aux(&[], &[NEXT_CHANGE_IN_AUTHORITY_KEY])
            .map_err(|err| format!("{}", err))
    }

    pub fn fetch_stored_next_authority_change(
        &self,
    ) -> Result<Option<NextChangeInAuthority<Block>>, String> {
        let encoded_next_possible_authority_change = self
            .client
            .get_aux(NEXT_CHANGE_IN_AUTHORITY_KEY)
            .map_err(|err| format!("{}", err))?;

        if encoded_next_possible_authority_change.is_none() {
            return Ok(None);
        }

        let encoded_authority_change = encoded_next_possible_authority_change.unwrap();

        let next_change_in_authority: NextChangeInAuthority<Block> =
            NextChangeInAuthority::decode(&mut encoded_authority_change.as_slice())
                .map_err(|err| format!("{}", err))?;

        Ok(Some(next_change_in_authority))
    }
}

impl<Block, Client> Verifier<Block> for GrandpaVerifier<Client, Block>
where
    Client: AuxStore + Send + Sync,
    Block: BlockT,
{
    fn verify(
        &mut self,
        origin: BlockOrigin,
        header: <Block as BlockT>::Header,
        justification: Option<Vec<u8>>,
        body: Option<Vec<<Block as BlockT>::Extrinsic>>,
    ) -> Result<
        (
            BlockImportParams<Block, ()>,
            Option<Vec<([u8; 4], Vec<u8>)>>,
        ),
        String,
    > {
        let (possible_authority_change, scheduled_change_exists) = {
            let possible_authority_change = self.fetch_stored_next_authority_change()?;
            match possible_authority_change {
                Some(authority_change) => {
                    if authority_change.next_change_at == *header.number() {
                        self.delete_stored_next_authority_change()?;
                        (Some(authority_change), false)
                    } else {
                        (None, true)
                    }
                }
                None => (None, false),
            }
        };

        let found_scheduled_authority_change = find_scheduled_change::<Block>(&header);
        let possible_next_authority_change: Option<NextChangeInAuthority<Block>> =
            match found_scheduled_authority_change {
                Some(scheduled_change) => {
                    if scheduled_change_exists {
                        Err("Scheduled change already exists.")
                    } else {
                        Ok(Some(NextChangeInAuthority::new(
                            *header.number() + scheduled_change.delay,
                            scheduled_change,
                        )))
                    }
                }
                None => Ok(None),
            }?;

        let mut block_import_params: BlockImportParams<Block, ()> =
            BlockImportParams::new(BlockOrigin::NetworkBroadcast, header);
        if let Some(next_authority_change) = possible_next_authority_change {
            block_import_params.intermediates.insert(
                Cow::from(GRANDPA_AUTHORITY_CHANGE_INTERMEDIATE_KEY),
                Box::new(next_authority_change),
            );
        }

        if let Some(authority_change) = possible_authority_change {
            let possible_current_authority_set = self.fetch_stored_authority_set()?;
            let current_authority_set = if possible_current_authority_set.is_none() {
                Err("No previous authority set found")
            } else {
                Ok(possible_current_authority_set.unwrap())
            }?;
            let next_authority_set = LightAuthoritySet::construct_next_authority_set(
                &current_authority_set,
                authority_change.change.next_authorities,
            );
            self.insert_authority_set(next_authority_set)?;
        }

        Ok((block_import_params, None))
    }
}
