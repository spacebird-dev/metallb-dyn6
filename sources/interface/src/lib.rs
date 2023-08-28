mod v6_unicast;



use interfaces::Interface;
use ipnet::Ipv6Net;
use prefix_source::{addr_to_network, PrefixSource, SourceError};

#[derive(Debug)]
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
    fn get(&self) -> Result<Ipv6Net, SourceError> {
        let addr = self
            .iface
            .addresses
            .iter()
            .filter_map(|a| match a.addr? {
                std::net::SocketAddr::V4(_) => None,
                std::net::SocketAddr::V6(a) => Some(*a.ip()),
            })
            .find(v6_unicast::is_unicast)
            .ok_or(SourceError {
                msg: format!("No global unicast address on interface {}", self.iface.name),
            })?;
        Ok(addr_to_network(addr))
    }
}
