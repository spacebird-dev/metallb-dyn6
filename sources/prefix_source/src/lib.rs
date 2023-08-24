use ipnet::Ipv6Net;
use thiserror::Error;

pub const PREFIX_LENGTH: u8 = 64;

#[derive(Error, Debug)]
#[error("Could not retrieve IPv6 address from source: {msg}")]
pub struct SourceError {
    pub msg: String,
}

#[derive(Error, Debug)]
pub enum PrefixError {
    #[error("Invalid Prefix length: {0}. Must be {PREFIX_LENGTH}")]
    InvalidPrefixLength(u8),
}

/// Represents the Ipv6 prefix to use in MetalLB.
/// A [SourcePrefix] is guaranteed to be of length 64, conforming to standard IPv6 GUA networks.
#[non_exhaustive]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SourcePrefix {
    /// The Ipv6 Network
    pub net: Ipv6Net,
}

impl TryFrom<Ipv6Net> for SourcePrefix {
    type Error = PrefixError;

    fn try_from(value: Ipv6Net) -> Result<Self, Self::Error> {
        if value.prefix_len() != PREFIX_LENGTH {
            Err(PrefixError::InvalidPrefixLength(value.prefix_len()))
        } else {
            Ok(SourcePrefix { net: value })
        }
    }
}

/// A [PrefixSource] provides a IPv6 Prefix (Network) that should be used for the MetalLB address pool.
/// This prefix must be a standard IPv6-formatted prefix, that is, it must have a length of /64.
/// This network is then combined with the host range and fed to MetalLB
pub trait PrefixSource {
    fn get(&self) -> Result<SourcePrefix, SourceError>;
}
