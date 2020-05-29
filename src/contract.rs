use crate::msg::{LatestHeightResponse, HandleMsg, InitMsg, QueryMsg};
use crate::{ingest_finalized_header, initialize_db};
use crate::types::{ConsensusState, ClientState ,JsonHeader, Header};
use crate::common::LightAuthoritySet;
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
    if !is_valid_name(&msg.name) {
        return contract_err("Name is not in the expected format (3-30 UTF-8 bytes)");
    }

    let head = Header::decode(&mut msg.header.header.as_bytes()).ok().unwrap();
    let authset = LightAuthoritySet::decode(&mut msg.header.authority_set.unwrap().as_bytes()).ok().unwrap();
    let state_bytes = initialize_db(head, authset).ok().unwrap();
    let client = ClientState {
        name: msg.name,
        height: msg.height,
        hash: [0].to_vec(),
        commitment_root: [0].to_vec(),
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
        HandleMsg::UpdateClient { height, header } => try_block(deps, env, &height, &header),
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
    height: &u32,
    header: &JsonHeader,
) -> Result<Response> {

    let client = client_state(&mut deps.storage).load()?;

    let head = Header::decode(&mut header.header.as_bytes()).ok().unwrap();
    let justification = Justification::decode(&mut header.justification.as_ref().unwrap().as_bytes()).ok().unwrap();
    let (result, ibc_data) = ingest_finalized_header(
        client.state,
        head,
        Some(justification),
        1
    ).unwrap();

    let new_client = ClientState {
        height: *height,
        hash: header.block_hash.clone(),
        commitment_root: header.commitment_root.clone(),
        state: ibc_data,
        ..client
    };

    client_state(&mut deps.storage).save(&new_client)?;

    let res = Response {
        messages: vec![],
        log: vec![
            log("action", "block"),
            log("height", &height.to_string())
        ],
        data: None,
    };
    Ok(res)
}


fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 30 {
        return false;
    }
    return true;
}

fn is_valid_message(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 6 {
        return false;
    }

    return true;
}
