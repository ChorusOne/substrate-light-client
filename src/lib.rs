mod block_import_wrapper;
mod block_processor;
mod client;
mod common;
mod db;
mod genesis;
mod grandpa_block_import;
mod justification;
mod light_state;
mod storage;
mod types;
mod verifier;

pub mod contract;
pub use contract::msg;

/// WASM methods exposed to be used by CosmWasm handler
/// All methods are thin wrapper around actual contract contained in
/// contract module.
#[cfg(target_arch = "wasm32")]
pub use cosmwasm::exports::{allocate, deallocate};

#[cfg(target_arch = "wasm32")]
pub use wasm::{handle, init, query};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::contract;
    use cosmwasm::{exports, imports};
    use std::ffi::c_void;

    /// WASM Entry point for contract::init
    #[no_mangle]
    pub extern "C" fn init(params_ptr: *mut c_void, msg_ptr: *mut c_void) -> *mut c_void {
        exports::do_init(
            &contract::init::<imports::ExternalStorage, imports::ExternalApi>,
            params_ptr,
            msg_ptr,
        )
    }

    /// WASM Entry point for contract::handle
    #[no_mangle]
    pub extern "C" fn handle(params_ptr: *mut c_void, msg_ptr: *mut c_void) -> *mut c_void {
        exports::do_handle(
            &contract::handle::<imports::ExternalStorage, imports::ExternalApi>,
            params_ptr,
            msg_ptr,
        )
    }

    /// WASM Entry point for contract::query
    #[no_mangle]
    pub extern "C" fn query(msg_ptr: *mut c_void) -> *mut c_void {
        exports::do_query(
            &contract::query::<imports::ExternalStorage, imports::ExternalApi>,
            msg_ptr,
        )
    }
}
