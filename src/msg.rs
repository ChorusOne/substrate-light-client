use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{H256, JsonHeader, BlockNumber};

//use cosmwasm::types::HumanAddr;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub height: u32,
    pub header: JsonHeader
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    UpdateClient {
        height: u32,
        header: JsonHeader,
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
