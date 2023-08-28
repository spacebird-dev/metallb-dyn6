use std::{mem::transmute, net::Ipv6Addr};

use anyhow::{bail, Context, Result};
use ipnet::Ipv6Net;

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct SubnetOverride {
    pub(crate) subnet: Ipv6Addr,
    pub(crate) prefix_length: u8,
}

// Ensure that the subnet override contains a valid subnet definition, given the host prefix length
pub(crate) fn validate_subnet_override(
    subnet: Ipv6Addr,
    prefix_length: u8,
) -> Result<SubnetOverride> {
    const HOST_MASK: u128 = 0x0000_0000_0000_0000_ffff_ffff_ffff_ffff;

    // should always be true as prefix_length is validated by clap
    assert!(prefix_length < 64);

    let prefix_mask = u128::from(
        Ipv6Net::new("ffff::".parse().unwrap(), prefix_length)
            .unwrap()
            .netmask(),
    );
    if (HOST_MASK | prefix_mask) & u128::from(subnet) != 0 {
        bail!("Subnet override must have empty prefix and host sections");
    } else {
        Ok(SubnetOverride {
            subnet,
            prefix_length,
        })
    }
}

pub(crate) fn apply_subnet_override(
    prefix: &Ipv6Net,
    subnet_override: &SubnetOverride,
) -> Result<Ipv6Net> {
    let truncated_addr = Ipv6Net::new(prefix.network(), subnet_override.prefix_length)?.network();
    let overriden_network =
        Ipv6Addr::from(u128::from(truncated_addr) | u128::from(subnet_override.subnet));
    Ok(Ipv6Net::new(overriden_network, 64).unwrap())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn validate_subnet_fails_on_incorrect_subnet() {
        validate_subnet_override("a:0:0:dead::".parse().unwrap(), 48).unwrap_err();
        validate_subnet_override("0:0:0:dead::f".parse().unwrap(), 48).unwrap_err();
        validate_subnet_override("0:0:0:dead::".parse().unwrap(), 56).unwrap_err();
    }

    #[test]
    fn validate_apply_subnet_override() {
        assert_eq!(
            apply_subnet_override(
                &Ipv6Net::new("2001:db8:dead:beef::".parse().unwrap(), 64).unwrap(),
                &SubnetOverride {
                    subnet: "0:0:0:d00f::".parse().unwrap(),
                    prefix_length: 48
                }
            )
            .unwrap(),
            Ipv6Net::new("2001:db8:dead:d00f::".parse().unwrap(), 64).unwrap()
        );

        assert_eq!(
            apply_subnet_override(
                &Ipv6Net::new("2001:db8:dead:beef::".parse().unwrap(), 64).unwrap(),
                &SubnetOverride {
                    subnet: "0:0:0:000f::".parse().unwrap(),
                    prefix_length: 56
                }
            )
            .unwrap(),
            Ipv6Net::new("2001:db8:dead:be0f::".parse().unwrap(), 64).unwrap()
        )
    }
}
