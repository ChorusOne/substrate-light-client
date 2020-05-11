use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{LatestHeightResponse, HandleMsg, InitMsg, QueryMsg, ConsensusState, ClientState};
use crate::lib::ingest_finalized_header;

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

    // if !is_valid_commitment_root(&msg.header) {
    //     return contract_err("Message is too short!");
    // }
    let header = Header::Decode(&msg.header);


    let client_state = ClientState {
        name: msg.name,
        height: msg.header,
        hash: 0,
    };

    // generate consensus state based on provided header.
    // TODO: Parth to create fn to initialise db.
    let consensus_state = ConsensusState {

    };

    client_state(&mut deps.storage).save(&client_state)?;
    consensus_state(&mut deps.storage).save(&consensus_state)?;

    Ok(Response::default())
}

pub fn handle<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    msg: HandleMsg,
) -> Result<Response> {
    match msg {
        HandleMsg::Block { height, json } => try_block(deps, env, &height, &json),
    }
}

pub fn query<S: Storage, A: Api>(deps: &Extern<S, A>, msg: QueryMsg) -> Result<Vec<u8>> {
    match msg {
        QueryMsg::LatestHeight { } => {
            let client_state = client_state(&deps.storage).load()?;

            let out = serialize(&LatestHeightResponse {
                height: client_state.height,
                hash: client_state.hash,
            })?;
            Ok(out)
        }
    }
}

fn try_block<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    _env: Env,
    height: &u32,
    json: &str,
) -> Result<Response> {

    let client_state = client_state(&deps.storage).load()?;

    let header = JsonHeader.decode(json)?;

    let (mut result, ibc_data) = ingest_finalized_header(
        consensus_state.state,
        header.header,
        header.justification,
    );

    let new_client_state = Client_State {
        height: height,
        hash: header.block_hash,
        commitment_root: header.commitment_root,
        state: ibc_data,
        ..client_state
    };

    client_state(&mut deps.storage).save(&new_client_state)?;

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
