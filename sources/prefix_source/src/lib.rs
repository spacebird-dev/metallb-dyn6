use std::{fmt::Debug, net::Ipv6Addr};

use async_trait::async_trait;
use ipnet::Ipv6Net;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Hash, Clone)]
#[error("Could not retrieve IPv6 address from source: {msg}")]
pub struct SourceError {
    pub msg: String,
}

/// A [PrefixSource] provides a IPv6 Prefix that MetalLB can use to expose service
#[async_trait]
pub trait PrefixSource: Send + Debug + Sync {
    /// Return an available IPv6 Prefix for MetalLB.
    /// The prefix must have a length of /64, as is the case for a normal globally unique network.
    async fn get(&self) -> Result<Ipv6Net, SourceError>;
}

pub fn addr_to_network(addr: Ipv6Addr) -> Ipv6Net {
    // unwrap is safe here as we set the prefix length ourselves
    Ipv6Net::new(Ipv6Net::new(addr, 64).unwrap().network(), 64).unwrap()
}

#[cfg(test)]
mod tests {

    use ipnet::Ipv6Net;

    use super::*;

    #[test]
    fn test_add_to_prefix() {
        assert_eq!(
            addr_to_network("2001:db8:dead:beef:123:123:123:123".parse().unwrap()),
            Ipv6Net::new("2001:db8:dead:beef::".parse().unwrap(), 64).unwrap()
        )
    }
}
