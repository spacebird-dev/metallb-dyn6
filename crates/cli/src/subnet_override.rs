use std::net::Ipv6Addr;

use anyhow::{bail, Result};
use ipnet::Ipv6Net;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A mask to override a the last n bits of an IPv6 network prefix
pub(crate) struct SubnetOverride {
    subnet: Ipv6Addr,
    /// Length of the prefix, guaranteed to be less than 64
    prefix_length: u8,
}

impl SubnetOverride {
    pub(crate) fn new(subnet: Ipv6Addr, prefix_length: u8) -> Result<SubnetOverride> {
        let prefix_mask = if prefix_length >= 64 {
            bail!("Prefix length must be <64");
        } else {
            u128::from(u64::MAX >> prefix_length) << 64
        };

        if u128::from(subnet) & !prefix_mask != 0 {
            bail!("Subnet override must have empty prefix and host sections");
        } else {
            Ok(SubnetOverride {
                subnet,
                prefix_length,
            })
        }
    }
    pub(crate) fn apply(&self, prefix: Ipv6Net) -> Ipv6Net {
        let truncated_addr = Ipv6Net::new_assert(prefix.network(), self.prefix_length).network();
        let overriden_network =
            Ipv6Addr::from(u128::from(truncated_addr) | u128::from(self.subnet));
        Ipv6Net::new_assert(overriden_network, 64)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn nonzero_network_bits_fails() {
        SubnetOverride::new(Ipv6Addr::new(0xa, 0, 0, 0xdead, 0, 0, 0, 0), 48).unwrap_err();
        SubnetOverride::new(Ipv6Addr::new(0, 0, 0, 0xdead, 0, 0, 0, 0), 56).unwrap_err();
    }

    #[test]
    fn nonzero_host_bits_fails() {
        SubnetOverride::new(Ipv6Addr::new(0, 0, 0, 0xdead, 0, 0, 0, 0xf), 48).unwrap_err();
    }

    #[test]
    fn overrides_subnet() -> Result<()> {
        let r#override = SubnetOverride::new(Ipv6Addr::new(0, 0, 0, 0xdead, 0, 0, 0, 0), 48)?;
        assert_eq!(
            r#override.apply(Ipv6Net::new_assert(
                Ipv6Addr::new(0x2001, 0xdb8, 0, 0xbeef, 0, 0, 0, 0),
                64
            )),
            Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0xdb8, 0, 0xdead, 0, 0, 0, 0), 64)
        );
        Ok(())
    }
}
