use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::Range,
    str::FromStr,
};

use ipnet::{IpNet, Ipv4Net, Ipv6AddrRange, Ipv6Net};
use thiserror::Error;

const PREFIX_MASK: u128 = 0xffff_ffff_ffff_ffff_0000_0000_0000_0000;

#[derive(Error, Debug, PartialEq, Eq, Hash)]
pub enum RangeParseError {
    #[error("Invalid Input Format")]
    UnknownFormat,
    #[error("Prefix must be Empty")]
    PrefixNotEmpty,
    #[error("Invalid range (inverse or zero-sized?)")]
    InvalidRange,
}

/// An address range as accepted by MetalLBs IPAddressPool resource.
/// Can either be a Network with a CIDR mask or a dash-separated doublet of addresses
#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub enum MetalLbAddressRange {
    V4Cidr(Ipv4Net),
    V6Cidr(Ipv6Net),
    V4Range(V4Range),
    V6Range(V6Range),
}

impl FromStr for MetalLbAddressRange {
    type Err = RangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((l, r)) = s.split_once('-') {
            if let (Ok(left), Ok(right)) = (l.parse::<Ipv4Addr>(), r.parse::<Ipv4Addr>()) {
                Ok(MetalLbAddressRange::V4Range(V4Range {
                    start: left,
                    end: right,
                }))
            } else if let (Ok(left), Ok(right)) = (l.parse::<Ipv6Addr>(), r.parse::<Ipv6Addr>()) {
                Ok(MetalLbAddressRange::V6Range(V6Range {
                    start: left,
                    end: right,
                }))
            } else {
                Err(RangeParseError::UnknownFormat)
            }
        } else if let Ok(v4r) = s.parse::<Ipv4Net>() {
            Ok(MetalLbAddressRange::V4Cidr(v4r))
        } else if let Ok(v6r) = s.parse::<Ipv6Net>() {
            Ok(MetalLbAddressRange::V6Cidr(v6r))
        } else {
            Err(RangeParseError::UnknownFormat)
        }
    }
}

impl Display for MetalLbAddressRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetalLbAddressRange::V4Cidr(r) => r.fmt(f),
            MetalLbAddressRange::V6Cidr(r) => r.fmt(f),
            MetalLbAddressRange::V4Range(r) => r.fmt(f),
            MetalLbAddressRange::V6Range(r) => r.fmt(f),
        }
    }
}

impl MetalLbAddressRange {
    /// Returns the first address of the range
    pub fn start(&self) -> IpAddr {
        match self {
            // MetalLB Address pools are actual pools, so they can include .0 and .255
            Self::V4Cidr(net) => IpAddr::V4(net.network()),
            Self::V6Cidr(net) => IpAddr::V6(net.network()),
            Self::V4Range(v4r) => IpAddr::V4(v4r.start),
            Self::V6Range(v6r) => IpAddr::V6(v6r.start),
        }
    }

    /// Returns the last address of the range
    pub fn end(&self) -> IpAddr {
        match self {
            Self::V4Cidr(net) => IpAddr::V4(net.broadcast()),
            Self::V6Cidr(net) => IpAddr::V6(net.broadcast()),
            Self::V4Range(v4r) => IpAddr::V4(v4r.end),
            Self::V6Range(v6r) => IpAddr::V6(v6r.end),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub struct V4Range {
    start: Ipv4Addr,
    end: Ipv4Addr,
}

impl Display for V4Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub struct V6Range {
    start: Ipv6Addr,
    end: Ipv6Addr,
}

impl Display for V6Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl V6Range {
    /// Create a dash-separated V6 range from a prefix and a host-address range
    pub fn from_host_range(prefix: Ipv6Net, host_range: V6HostRange) -> Self {
        V6Range {
            start: Ipv6Addr::from(u128::from(prefix.network()) | u128::from(host_range.start)),
            end: Ipv6Addr::from(u128::from(prefix.network()) | u128::from(host_range.end)),
        }
    }
}

/// A range of Ipv6 host address parts for insertion into a MetalLB Ipv6 address range.
/// For example, the host range ::1000-::1999 can be combined with a prefix to produce a full address range.
#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub struct V6HostRange {
    start: Ipv6Addr,
    end: Ipv6Addr,
}

impl FromStr for V6HostRange {
    type Err = RangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((startstr, endstr)) = s.split_once('-') else {
            return Err(RangeParseError::UnknownFormat);
        };

        let (Ok(start), Ok(end)) = (startstr.parse::<Ipv6Addr>(), endstr.parse::<Ipv6Addr>()) else {
            return Err(RangeParseError::UnknownFormat);
        };

        if (u128::from(start) & PREFIX_MASK != 0) && (u128::from(end) & PREFIX_MASK != 0) {
            return Err(RangeParseError::PrefixNotEmpty);
        }

        if start >= end {
            return Err(RangeParseError::InvalidRange);
        }

        Ok(V6HostRange { start, end })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_range_parsing() {
        assert_eq!(
            "::1000-::1999".parse::<V6HostRange>().unwrap(),
            V6HostRange {
                start: "::1000".parse().unwrap(),
                end: "::1999".parse().unwrap()
            }
        )
    }

    #[test]
    fn test_host_range_errors_on_nonempty_prefix() {
        assert_eq!(
            "a::1000-a::1999".parse::<V6HostRange>().unwrap_err(),
            RangeParseError::PrefixNotEmpty
        )
    }

    #[test]
    fn test_host_range_errors_on_backwards_range() {
        assert_eq!(
            "::1999-::1000".parse::<V6HostRange>().unwrap_err(),
            RangeParseError::InvalidRange
        )
    }

    #[test]
    fn test_host_range_errors_on_zero_range() {
        assert_eq!(
            "::1000-::1000".parse::<V6HostRange>().unwrap_err(),
            RangeParseError::InvalidRange
        )
    }

    #[test]
    fn test_address_range_v4_cidr() {
        let range = "192.0.2.0/24".parse::<MetalLbAddressRange>().unwrap();
        assert_eq!(
            range,
            MetalLbAddressRange::V4Cidr(Ipv4Net::new("192.0.2.0".parse().unwrap(), 24).unwrap())
        );
        assert_eq!(range.start(), "192.0.2.0".parse::<Ipv4Addr>().unwrap());
        assert_eq!(range.end(), "192.0.2.255".parse::<Ipv4Addr>().unwrap());
    }

    #[test]
    fn test_address_range_v4_range() {
        let range = "192.0.2.10-192.0.2.200"
            .parse::<MetalLbAddressRange>()
            .unwrap();
        assert_eq!(
            range,
            MetalLbAddressRange::V4Range(V4Range {
                start: "192.0.2.10".parse().unwrap(),
                end: "192.0.2.200".parse().unwrap()
            })
        );
        assert_eq!(range.start(), "192.0.2.10".parse::<Ipv4Addr>().unwrap());
        assert_eq!(range.end(), "192.0.2.200".parse::<Ipv4Addr>().unwrap());
    }

    #[test]
    fn test_address_range_v6_cidr() {
        let range = "2001:db8:dead:beef::/96"
            .parse::<MetalLbAddressRange>()
            .unwrap();
        assert_eq!(
            range,
            MetalLbAddressRange::V6Cidr(
                Ipv6Net::new("2001:db8:dead:beef::".parse().unwrap(), 96).unwrap()
            )
        );
        assert_eq!(
            range.start(),
            "2001:db8:dead:beef::".parse::<Ipv6Addr>().unwrap()
        );
        assert_eq!(
            range.end(),
            "2001:db8:dead:beef::ffff:ffff".parse::<Ipv6Addr>().unwrap()
        );
    }

    #[test]
    fn test_address_range_v6_range() {
        let range = "2001:db8:dead:beef::-2001:db8:dead:beef:ffff::"
            .parse::<MetalLbAddressRange>()
            .unwrap();
        assert_eq!(
            "2001:db8:dead:beef::-2001:db8:dead:beef:ffff::"
                .parse::<MetalLbAddressRange>()
                .unwrap(),
            MetalLbAddressRange::V6Range(V6Range {
                start: "2001:db8:dead:beef::".parse().unwrap(),
                end: "2001:db8:dead:beef:ffff::".parse().unwrap()
            })
        );
        assert_eq!(
            range.start(),
            "2001:db8:dead:beef::".parse::<Ipv6Addr>().unwrap()
        );
        assert_eq!(
            range.end(),
            "2001:db8:dead:beef:ffff::".parse::<Ipv6Addr>().unwrap()
        )
    }

    #[test]
    fn test_address_range_from_host_range() {
        let range = V6Range::from_host_range(
            Ipv6Net::new("2001:db8:dead:beef::".parse().unwrap(), 64).unwrap(),
            "::1000-::1999".parse::<V6HostRange>().unwrap(),
        );

        assert_eq!(
            range.start,
            "2001:db8:dead:beef::1000".parse::<Ipv6Addr>().unwrap()
        );
        assert_eq!(
            range.end,
            "2001:db8:dead:beef::1999".parse::<Ipv6Addr>().unwrap()
        );
    }

    #[test]
    fn test_address_range_to_string() {
        let range = V6Range::from_host_range(
            Ipv6Net::new("2001:db8:dead:beef::".parse().unwrap(), 64).unwrap(),
            "::1000-::1999".parse::<V6HostRange>().unwrap(),
        );

        assert_eq!(
            range.to_string(),
            "2001:db8:dead:beef::1000-2001:db8:dead:beef::1999"
        );
    }
}
