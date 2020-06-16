use crate::msg::{LatestHeightResponse, HandleMsg, InitMsg, QueryMsg};
use crate::{ingest_finalized_header, initialize_db};
use crate::types::{ConsensusState, ClientState, Header, SignedBlock, Block};
use sp_runtime::traits::{Header as HeaderT, Block as BlockT};
use crate::common::types::light_authority_set::LightAuthoritySet;
use parity_scale_codec::Decode;
use sp_runtime::Justification;

use cosmwasm::errors::{contract_err, Result};
use cosmwasm::traits::{Api, Extern, ReadonlyStorage, Storage};
use cosmwasm::types::{log, Env, Response};
use cw_storage::{serialize, Singleton, ReadonlySingleton, singleton, singleton_read};

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_MESSAGES: &[u8] = b"messages";

pub const KEY_STATE_CONS: &[u8] = b"consensus_state";
pub const KEY_STATE_CLIENT: &[u8] = b"client_state";

pub fn consensus_state<S: Storage>(storage: &mut S) -> Singleton<S, ConsensusState> {
    singleton(storage, KEY_STATE_CONS)
}

pub fn consensus_state_ro<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, ConsensusState> {
    singleton_read(storage, KEY_STATE_CONS)
}

pub fn client_state<S: Storage>(storage: &mut S) -> Singleton<S, ClientState> {
    singleton(storage, KEY_STATE_CLIENT)
}

pub fn client_state_ro<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, ClientState> {
    singleton_read(storage, KEY_STATE_CLIENT)
}

pub fn init<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    _env: Env,
    msg: InitMsg,
) -> Result<Response> {

    // Check name, symbol, decimals
    if !is_valid_identifier(&msg.name) {
        return contract_err("Name is not in the expected format (8-20 lowercase UTF-8 bytes)");
    }

    let block = SignedBlock::decode(&mut msg.block.as_bytes()).ok().unwrap();
    let authset = LightAuthoritySet::decode(&mut msg.authority_set.as_bytes()).ok().unwrap();
    let head = block.block.header;

    let state_bytes = initialize_db(head.clone(), authset).ok().unwrap();
    let client = ClientState {
        name: msg.name,
        height: head.number.clone(),
        hash: head.hash().clone().as_bytes().to_vec(),
        commitment_root: head.state_root().as_bytes().to_vec(),
        frozen_height: None,
        state: state_bytes,
    };

    client_state(&mut deps.storage).save(&client)?;

    Ok(Response::default())
}

pub fn handle<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    msg: HandleMsg,
) -> Result<Response> {
    match msg {
        HandleMsg::UpdateClient { block, authority_set } => try_block(deps, env, &block, &authority_set),
    }
}

pub fn query<S: Storage, A: Api>(deps: &Extern<S, A>, msg: QueryMsg) -> Result<Vec<u8>> {
    match msg {
        QueryMsg::LatestHeight { } => {
            let client = client_state_ro(&deps.storage).load()?;

            let out = serialize(&LatestHeightResponse {
                height: client.height,
                hash: client.hash,
            })?;
            Ok(out)
        }
    }
}

fn try_block<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    _env: Env,
    block: &String,
    _authority_set: &String,
) -> Result<Response> {

    let client = client_state(&mut deps.storage).load()?;

    let block = SignedBlock::decode(&mut block.as_bytes()).ok().unwrap();

    let head = block.block.header.clone();

    let (result, ibc_data) = ingest_finalized_header(
        client.state,
        head.clone(),
        block.justification,
        1
    ).unwrap();

    let new_client = ClientState {
        height: head.number.clone(),
        hash: head.hash().as_bytes().to_vec(),
        commitment_root: head.state_root().as_bytes().to_vec(),
        state: ibc_data,
        ..client
    };

    client_state(&mut deps.storage).save(&new_client)?;

    let res = Response {
        messages: vec![],
        log: vec![
            log("action", "block"),
            log("height", &head.number.to_string())
        ],
        data: None,
    };
    Ok(res)
}


fn is_valid_identifier(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 8 || bytes.len() > 20 {
        return false; // length invalid
    }
    for byte in bytes {
        if byte > &122 || byte < &97 {
            return false; // not lowercase ascii
        }
    }
    return true;
}

