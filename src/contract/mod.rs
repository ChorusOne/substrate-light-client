pub mod msg;
mod types;

use cosmwasm::errors::{contract_err, dyn_contract_err, Result};
use cosmwasm::traits::{Api, Extern, ReadonlyStorage, Storage};
use cosmwasm::types::{log, Env, Response};
use cw_storage::{serialize, singleton, singleton_read, ReadonlySingleton, Singleton};
use parity_scale_codec::Decode;
use sp_finality_grandpa::AuthorityList;
use sp_runtime::traits::Header as HeaderT;

use crate::common::types::light_authority_set::LightAuthoritySet;
use crate::contract::types::{ConsensusState, ContractState, H256};
use crate::light_state::{current_status, ingest_finalized_header, initialize_state};
use crate::msg::{HandleMsg, InitMsg, LatestHeightResponse, QueryMsg};
use crate::types::{Block, SignedBlock};

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_MESSAGES: &[u8] = b"messages";

pub const KEY_STATE_CONS: &[u8] = b"consensus_state";
pub const KEY_STATE_CLIENT: &[u8] = b"client_state";

fn consensus_state<S: Storage>(storage: &mut S) -> Singleton<S, ConsensusState> {
    singleton(storage, KEY_STATE_CONS)
}

fn consensus_state_ro<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, ConsensusState> {
    singleton_read(storage, KEY_STATE_CONS)
}

fn contract_state<S: Storage>(storage: &mut S) -> Singleton<S, ContractState> {
    singleton(storage, KEY_STATE_CLIENT)
}

fn read_only_contract_state<S: ReadonlyStorage>(
    storage: &S,
) -> ReadonlySingleton<S, ContractState> {
    singleton_read(storage, KEY_STATE_CLIENT)
}

pub(crate) fn init<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    _env: Env,
    msg: InitMsg,
) -> Result<Response> {
    // Check name, symbol, decimals
    if !is_valid_identifier(&msg.name) {
        return contract_err("Name is not in the expected format (8-20 lowercase UTF-8 bytes)");
    }

    let block_bytes = match hex::decode(&msg.block[2..]) {
        Ok(bytes) => bytes,
        Err(_) => return dyn_contract_err("Unable to decode block hex".to_string()),
    };

    let block = match SignedBlock::decode(&mut block_bytes.as_slice()) {
        Ok(block) => block,
        Err(_) => return dyn_contract_err("SignedBlock::decode()".to_string()),
    };

    let auth_bytes = match hex::decode(&msg.authority_set[2..]) {
        Ok(bytes) => bytes,
        Err(_) => return dyn_contract_err("Unable to decode authority_set hex".to_string()),
    };

    let authset = match AuthorityList::decode(&mut auth_bytes.as_slice()) {
        Ok(authset) => authset,
        Err(_) => return dyn_contract_err("AuthorityList::decode()".to_string()),
    };
    let head = block.block.header;

    let authority_set = LightAuthoritySet::new(0, authset);

    let light_client_data = match initialize_state(head.clone(), authority_set) {
        Ok(state_bytes) => state_bytes,
        Err(_) => return dyn_contract_err("initialize_state()".to_string()),
    };

    let mut new_contract_state =
        update_contract_state(ContractState::default(), light_client_data.clone())?;
    new_contract_state.name = msg.name;
    new_contract_state.light_client_data = light_client_data;
    new_contract_state.max_non_finalized_blocks_allowed = msg.max_non_finalized_blocks_allowed;

    contract_state(&mut deps.storage).save(&new_contract_state)?;

    Ok(Response::default())
}

pub(crate) fn handle<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    msg: HandleMsg,
) -> Result<Response> {
    match msg {
        HandleMsg::IngestBlock {
            block,
            authority_set,
        } => try_block(deps, env, &block, &authority_set),
    }
}

pub(crate) fn query<S: Storage, A: Api>(deps: &Extern<S, A>, msg: QueryMsg) -> Result<Vec<u8>> {
    match msg {
        QueryMsg::LatestHeight {} => {
            let state = read_only_contract_state(&deps.storage).load()?;

            let out = serialize(&LatestHeightResponse {
                best_header_height: state.height,
                best_header_hash: state.best_header_hash,
                last_finalized_header_hash: state.last_finalized_header_hash,
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
    let state = contract_state(&mut deps.storage).load()?;
    let block_bytes = match hex::decode(&block[2..]) {
        Ok(bytes) => bytes,
        Err(_) => return dyn_contract_err("Unable to decode block hex".to_string()),
    };

    let block = match SignedBlock::decode(&mut block_bytes.as_slice()) {
        Ok(block) => block,
        Err(_) => return dyn_contract_err("SignedBlock::decode()".to_string()),
    };

    let (_result, updated_light_client_data) = match ingest_finalized_header(
        state.light_client_data.clone(),
        block.block.header,
        block.justification,
        state.max_non_finalized_blocks_allowed,
    ) {
        Ok(result) => result,
        Err(e) => return dyn_contract_err(e.to_string()),
    };

    let new_contract_state = update_contract_state(state, updated_light_client_data)?;

    contract_state(&mut deps.storage).save(&new_contract_state)?;

    let res = Response {
        messages: vec![],
        log: vec![
            log("action", "block"),
            log("height", new_contract_state.height.to_string().as_ref()),
        ],
        data: None,
    };
    Ok(res)
}

fn update_contract_state(
    old_state: ContractState,
    new_light_client_data: Vec<u8>,
) -> Result<ContractState> {
    let light_client_status = match current_status::<Block>(new_light_client_data.clone()) {
        Ok(status) => status,
        Err(e) => return dyn_contract_err(e.to_string()),
    };

    let best_header_number = light_client_status
        .possible_best_header
        .as_ref()
        .map_or(0, |h| *h.number());
    let best_header_hash = light_client_status
        .possible_best_header
        .as_ref()
        .map_or(H256::default(), |h| h.hash().as_bytes().to_vec());
    let best_header_commitment_root = light_client_status
        .possible_best_header
        .as_ref()
        .map_or(H256::default(), |h| h.state_root().as_bytes().to_vec());
    let last_finalized_header_hash = light_client_status
        .possible_last_finalized_header
        .as_ref()
        .map_or(H256::default(), |h| h.hash().as_bytes().to_vec());

    Ok(ContractState {
        height: best_header_number,
        best_header_hash,
        last_finalized_header_hash,
        best_header_commitment_root,
        light_client_data: new_light_client_data,
        ..old_state
    })
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

#[cfg(test)]
mod tests {
    use crate::contract::{handle, init, read_only_contract_state};
    use crate::msg::{HandleMsg, InitMsg};
    use cosmwasm::mock::{MockApi, MockStorage};
    use cosmwasm::traits::Extern;
    use cosmwasm::types::Env;

    #[test]
    fn test_contract_init_and_update() {
        let mut storage = MockStorage::new();
        let api = MockApi::new(5);
        let mut extern_dep = Extern { storage, api };

        let init_msg = InitMsg{
            name: "testtesttest".into(),
            block: "0x5e9fc49076803d0ba88c719252ede5ae713d09367162d344e9b79ef3aac2efa03e620300fe518cc595e8f5ede8010cf6d26352f6a089ee52f992153a540c7b5d9b659ea272c9c1e535cf5ca49ab2d72059671d80f69c6dba7e6c0dca1e27c3832e873f2b08066175726120448dd10f0000000005617572610101fe734978fa3cb9804346988424124add53316e68e9dcd96a5dfc5a576fe61262031463e0e3a1cdb15538a763dddfbbdf2d3c47e3ecc72deebb3ba5ec59b1168204280402000bc0e95ebf720100".into(),
            authority_set: "0x0488dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee0100000000000000".to_string(),
            max_non_finalized_blocks_allowed: 256
        };
        let init_header_hash =
            hex::decode("f157283bcfe5ace5f3258bdb595ee8c6761394a56c8e73b6aaf734e6fb1e7c92")
                .expect("Hex decoding of init header hash failed");
        let result = init(&mut extern_dep, Env::default(), init_msg);
        assert!(result.is_ok());

        let contract_state = read_only_contract_state(&extern_dep.storage)
            .load()
            .expect("Contract state should exists");
        assert_eq!(contract_state.name, "testtesttest");
        assert!(contract_state.light_client_data.len() > 0);
        assert_eq!(contract_state.last_finalized_header_hash.len(), 0);
        assert_eq!(contract_state.best_header_hash, init_header_hash);

        let update_msg = HandleMsg::IngestBlock {
            block: "0xf157283bcfe5ace5f3258bdb595ee8c6761394a56c8e73b6aaf734e6fb1e7c92426203000ad92ba15285e38e29472d35c29a8e0097e0748fa66fca1b4c834e13f0604de6f7e776ac0632a86d967e1fc4694d51b15c06dadf6c2d0f60a0c661993ffa6d5308066175726120458dd10f00000000056175726101019c9a0a6afd95ff9b8a479bab6676867d19f388b187534394661f0b9ca540b86cd5847174d8b1075f61c01f3b0f5dfa8c643b15c226ebace6aa5aca43cd12ce8504280402000b30015fbf720100".to_string(),
            authority_set: "0x0488dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee0100000000000000".to_string(),
        };
        let next_header_hash =
            hex::decode("b17ad1a298edb7fa902ce240358ced980a1a1f9febe163152be5e66c377fa38c")
                .expect("Hex decoding of next header hash failed");
        let result = handle(&mut extern_dep, Env::default(), update_msg);
        assert!(result.is_ok());

        let contract_state = read_only_contract_state(&extern_dep.storage)
            .load()
            .expect("Contract state should exists");
        assert_eq!(contract_state.name, "testtesttest");
        assert!(contract_state.light_client_data.len() > 0);
        assert_eq!(contract_state.last_finalized_header_hash.len(), 0);
        assert_eq!(contract_state.best_header_hash, next_header_hash);
    }
}
