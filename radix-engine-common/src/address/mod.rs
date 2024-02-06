mod decoder;
mod display;
mod encoder;
mod errors;
mod hrpset;

pub use decoder::*;
pub use display::*;
pub use encoder::*;
pub use errors::*;
pub use hrpset::*;

pub mod test_addresses {
    use crate::types::{NodeId, ResourceAddress};

    // The system addresses are defined in `radix-engine-interface`, but some
    // tests have a need for some placeholder addresses - so define them here so
    // we only need to update one place if they change in future.

    pub const FUNGIBLE_RESOURCE: ResourceAddress = ResourceAddress::new_or_panic([
        93, 254, 253
    ]);
    pub const FUNGIBLE_RESOURCE_NODE_ID: NodeId = FUNGIBLE_RESOURCE.into_node_id();
    pub const FUNGIBLE_RESOURCE_SIM_ADDRESS: &'static str =
        "resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3";
    pub const FUNGIBLE_RESOURCE_NO_NETWORK_STRING: &'static str =
        "NodeId(5da66318c6318c61f5a61b4c6318c6318cf794aa8d295f14e6318c6318c6)";
    pub const FUNGIBLE_RESOURCE_HEX_STRING: &'static str =
        "5da66318c6318c61f5a61b4c6318c6318cf794aa8d295f14e6318c6318c6";

    pub const NON_FUNGIBLE_RESOURCE: ResourceAddress = ResourceAddress::new_or_panic([
        154, 254, 253
    ]);
    pub const NON_FUNGIBLE_RESOURCE_NODE_ID: NodeId = FUNGIBLE_RESOURCE.into_node_id();
    pub const NON_FUNGIBLE_RESOURCE_SIM_ADDRESS: &'static str =
        "resource_sim1ngktvyeenvvqetnqwysevcx5fyvl6hqe36y3rkhdfdn6uzvt5366ha";
}
