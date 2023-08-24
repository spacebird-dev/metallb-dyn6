mod v6_unicast;

use interfaces::Interface;
use ipnet::Ipv6Net;
use prefix_source::{PrefixSource, SourceError, SourcePrefix, PREFIX_LENGTH};

pub struct InterfaceSource {
    iface: Interface,
}

impl InterfaceSource {
    pub fn new(iface_name: &str) -> Result<Self, SourceError> {
        match Interface::get_by_name(iface_name) {
            Ok(Some(i)) => Ok(InterfaceSource { iface: i }),
            Ok(None) => Err(SourceError {
                msg: format!("Interface {} does not exist", iface_name),
            }),
            Err(e) => Err(SourceError {
                msg: format!("Error while retrieving interfaces: {}", e),
            }),
        }
    }
}

impl PrefixSource for InterfaceSource {
    fn get(&self) -> Result<SourcePrefix, SourceError> {
        self.iface
            .addresses
            .iter()
            .filter_map(|a| match a.addr? {
                std::net::SocketAddr::V4(_) => None,
                std::net::SocketAddr::V6(a) => Some(*a.ip()),
            })
            .find(v6_unicast::is_unicast_global)
            .ok_or(SourceError {
                msg: format!("No global unicast address on interface {}", self.iface.name),
            })
            // The unwraps here are safe as we use a fixed prefix length as dictated by prefix_source
            .map(|a| Ipv6Net::new(a, PREFIX_LENGTH).unwrap().try_into().unwrap())
    }
}
