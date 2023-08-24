use std::net::Ipv6Addr;

// Temporary until https://github.com/rust-lang/rust/issues/27709 gets stabilized
pub fn is_unicast_global(addr: &Ipv6Addr) -> bool {
    // is_unicast
    !addr.is_multicast()
        // is_unicast_link_local
        && (addr.segments()[0] & 0xffc0) != 0xfe80
        // is_unique_local
        && (addr.segments()[0] & 0xfe00) != 0xfc00
        // is_documentation
        && !((addr.segments()[0] == 0x2001) && (addr.segments()[1] == 0xdb8))
        // is_bnenchmarking
        && !((addr.segments()[0] == 0x2001) && (addr.segments()[1] == 0x2) && (addr.segments()[2] == 0))
        && !addr.is_loopback()
        && !addr.is_unspecified()
}
