use serde::{Deserialize, Serialize};

// This type is similar to primitive_types::H256 and
// redeclared here to simplify state variables and
// make them independent to parity types.
pub type H256 = Vec<u8>;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ContractState {
    pub name: String,
    pub light_client_data: Vec<u8>,
    pub max_headers_allowed_to_store: u64,
}
