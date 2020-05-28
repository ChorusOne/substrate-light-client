use sp_runtime::traits::BlakeTwo256;
use sp_runtime::OpaqueExtrinsic;

pub type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;
