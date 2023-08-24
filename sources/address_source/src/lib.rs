use std::net::Ipv6Addr;

use thiserror::Error;

#[derive(Error, Debug)]
#[error("Could not retrieve IPv6 address from source: {msg}")]
pub struct SourceError {
    pub msg: String,
}

/// A [PrefixSource] provides a IPv6 Address that dyn6 can then extract the prefix for the MetalLB address pool from.
pub trait AddressSource {
    fn get(&self) -> Result<Ipv6Addr, SourceError>;
}
