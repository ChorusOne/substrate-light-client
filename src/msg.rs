use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{H256, BlockNumber};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub block: String,
    pub authority_set: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    UpdateClient {
        block: String,
        authority_set: String,
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
