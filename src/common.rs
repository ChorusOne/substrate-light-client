use crate::db;
use crate::types::Block;
use parity_scale_codec::alloc::sync::Arc;
use parity_scale_codec::{Decode, Encode};
use sc_client::light::backend::Backend;
use sc_client_api::AuxStore;
use sc_client_db::light::LightStorage;
use sc_client_db::{DatabaseSettings, DatabaseSettingsSrc, PruningMode};
use sp_blockchain::Error as BlockchainError;
use sp_finality_grandpa::{AuthorityList, ScheduledChange};
use sp_runtime::traits::{Block as BlockT, HashFor, NumberFor};

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
            authorities,
        }
    }

    pub fn construct_next_authority_set(
        prev_authority_set: &LightAuthoritySet,
        new_authority_list: AuthorityList,
    ) -> Self {
        Self {
            set_id: prev_authority_set.set_id + 1,
            authorities: new_authority_list,
        }
    }
}

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

pub fn initialize_backend(
    encoded_data: Vec<u8>,
) -> Result<
    (
        Arc<Backend<LightStorage<Block>, HashFor<Block>>>,
        db::IBCData,
    ),
    BlockchainError,
> {
    let ibc_data = db::IBCData::decode(&mut encoded_data.as_slice()).unwrap();

    let light_storage = LightStorage::new(DatabaseSettings {
        state_cache_size: 2048,
        state_cache_child_ratio: Some((20, 100)),
        pruning: PruningMode::keep_blocks(256),
        source: DatabaseSettingsSrc::Custom(Arc::new(ibc_data.db.clone())),
    })?;

    let light_blockchain = sc_client::light::new_light_blockchain(light_storage);
    Ok((
        sc_client::light::new_light_backend(light_blockchain.clone()),
        ibc_data,
    ))
}

pub fn store_next_authority_change<AS, Block>(
    aux_store: Arc<AS>,
    next_authority_change: &NextChangeInAuthority<Block>,
) -> Result<(), BlockchainError>
where
    AS: AuxStore,
    Block: BlockT,
{
    aux_store.insert_aux(
        &[(
            NEXT_CHANGE_IN_AUTHORITY_KEY,
            next_authority_change.encode().as_slice(),
        )],
        &[],
    )
}

pub fn delete_next_authority_change<AS>(aux_store: Arc<AS>) -> Result<(), BlockchainError>
where
    AS: AuxStore,
{
    aux_store.insert_aux(&[], &[NEXT_CHANGE_IN_AUTHORITY_KEY])
}

pub fn fetch_next_authority_change<AS, Block>(
    aux_store: Arc<AS>,
) -> Result<Option<NextChangeInAuthority<Block>>, BlockchainError>
where
    AS: AuxStore,
    Block: BlockT,
{
    let encoded_next_possible_authority_change = aux_store.get_aux(NEXT_CHANGE_IN_AUTHORITY_KEY)?;

    if encoded_next_possible_authority_change.is_none() {
        return Ok(None);
    }

    let encoded_authority_change = encoded_next_possible_authority_change.unwrap();

    let next_change_in_authority: NextChangeInAuthority<Block> =
        NextChangeInAuthority::decode(&mut encoded_authority_change.as_slice()).map_err(|err| {
            BlockchainError::Backend(format!(
                "Unable to decode next change in authority. DB might be corrupted. Underlying Error: {}",
                err.what()
            ))
        })?;

    Ok(Some(next_change_in_authority))
}

pub fn insert_light_authority_set<AS>(
    aux_store: Arc<AS>,
    light_authority_set: LightAuthoritySet,
) -> Result<(), BlockchainError>
where
    AS: AuxStore,
{
    aux_store.insert_aux(
        &[(
            LIGHT_AUTHORITY_SET_KEY,
            light_authority_set.encode().as_slice(),
        )],
        &[],
    )
}

pub fn fetch_light_authority_set<AS>(
    aux_store: Arc<AS>,
) -> Result<Option<LightAuthoritySet>, BlockchainError>
where
    AS: AuxStore,
{
    let encoded_possible_light_authority_set = aux_store.get_aux(LIGHT_AUTHORITY_SET_KEY)?;

    if encoded_possible_light_authority_set.is_none() {
        return Ok(None);
    }

    let encoded_light_authority_set = encoded_possible_light_authority_set.unwrap();

    let light_authority_set =
        LightAuthoritySet::decode(&mut encoded_light_authority_set.as_slice()).map_err(|err| {
            BlockchainError::Backend(format!(
                "Unable to decode light authority set. DB might be corrupted. Underlying Error: {}",
                err.what()
            ))
        })?;

    Ok(Some(light_authority_set))
}
