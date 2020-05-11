use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{H256, Header, BlockNumber};

use cosmwasm::types::HumanAddr;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub header: Header,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    UpdateClient {
        header: Header,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryMsg {
    LatestHeight { }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct LatestHeightResponse {
    pub height: BlockNumber,
    pub hash: H256,
}
