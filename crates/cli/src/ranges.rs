use std::{collections::HashSet, hash::RandomState};

use ipnet::Ipv6Net;
use metallb_dyn6_k8s::ranges::{MetalLbAddressRange, V6HostRange, V6Range};
use tracing::info;

use crate::subnet_override::SubnetOverride;

/// Calculate the new address range list to apply to the pool.
/// If no changes are needed, the return value is None.
pub(crate) fn calculate_changed_ranges(
    current: &[MetalLbAddressRange],
    prefix_net: Ipv6Net,
    host_range: V6HostRange,
    subnet_override: Option<SubnetOverride>,
) -> Option<Vec<MetalLbAddressRange>> {
    let prefix_net = subnet_override.map_or(prefix_net, |ovr| ovr.apply(prefix_net));

    let desired_v6_range =
        MetalLbAddressRange::V6Range(V6Range::from_host_range(prefix_net, host_range));
    info!(msg = "Desired address range", range = ?desired_v6_range);

    let mut desired_ranges = current
        .iter()
        .filter(|r| {
            // Remove any pre-existing IPv6 address ranges
            !matches!(
                r,
                MetalLbAddressRange::V6Cidr(_) | MetalLbAddressRange::V6Range(_)
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    desired_ranges.push(desired_v6_range);
    info!(
        desired_ranges = ?desired_ranges
    );

    if HashSet::<_, RandomState>::from_iter(&desired_ranges) == HashSet::from_iter(current) {
        None
    } else {
        Some(desired_ranges)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        sync::LazyLock,
    };

    use ipnet::Ipv4Net;

    use super::*;

    static DESIRED_PREFIX_NET: Ipv6Net =
        Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0xdb8, 0xdead, 0xbeef, 0, 0, 0, 0), 64);
    static DESIRED_HOST_RANGE: LazyLock<V6HostRange> =
        LazyLock::new(|| "::0-::1000".parse().unwrap());
    static DESIRED_V6_RANGE: LazyLock<MetalLbAddressRange> = LazyLock::new(|| {
        MetalLbAddressRange::V6Range(V6Range::from_host_range(
            DESIRED_PREFIX_NET,
            *DESIRED_HOST_RANGE,
        ))
    });
    static V4_RANGE: MetalLbAddressRange =
        MetalLbAddressRange::V4Cidr(Ipv4Net::new_assert(Ipv4Addr::new(10, 0, 0, 0), 24));

    #[test]
    fn missing_desired_range_gets_added() {
        let current = vec![V4_RANGE];
        let calculated =
            calculate_changed_ranges(&current, DESIRED_PREFIX_NET, *DESIRED_HOST_RANGE, None)
                .map(HashSet::<_, RandomState>::from_iter);
        assert_eq!(
            calculated,
            Some(HashSet::from_iter(vec![V4_RANGE, *DESIRED_V6_RANGE]))
        );
    }

    #[test]
    fn desired_range_prefix_gets_updated() {
        let current = vec![
            V4_RANGE,
            MetalLbAddressRange::V6Range(V6Range::from_host_range(
                Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0xdb8, 0xd00f, 0xffff, 0, 0, 0, 0), 64),
                *DESIRED_HOST_RANGE,
            )),
        ];
        let calculated =
            calculate_changed_ranges(&current, DESIRED_PREFIX_NET, *DESIRED_HOST_RANGE, None)
                .map(HashSet::<_, RandomState>::from_iter);
        assert_eq!(
            calculated,
            Some(HashSet::from_iter(vec![V4_RANGE, *DESIRED_V6_RANGE]))
        );
    }

    #[test]
    fn desired_range_hosts_range_gets_update() {
        let current = vec![
            V4_RANGE,
            MetalLbAddressRange::V6Range(V6Range::from_host_range(
                DESIRED_PREFIX_NET,
                "::2000-::2100".parse().unwrap(),
            )),
        ];
        let calculated =
            calculate_changed_ranges(&current, DESIRED_PREFIX_NET, *DESIRED_HOST_RANGE, None)
                .map(HashSet::<_, RandomState>::from_iter);
        assert_eq!(
            calculated,
            Some(HashSet::from_iter(vec![V4_RANGE, *DESIRED_V6_RANGE]))
        );
    }

    #[test]
    fn matching_desired_range_recognized() {
        let current = vec![V4_RANGE, *DESIRED_V6_RANGE];
        let calculated =
            calculate_changed_ranges(&current, DESIRED_PREFIX_NET, *DESIRED_HOST_RANGE, None)
                .map(HashSet::<_, RandomState>::from_iter);
        assert_eq!(calculated, None);
    }

    #[test]
    fn extra_v6_prefixes_removed() {
        let current = vec![
            V4_RANGE,
            MetalLbAddressRange::V6Range(V6Range::from_host_range(
                Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0xdb8, 0xd00f, 0xffff, 0, 0, 0, 0), 64),
                *DESIRED_HOST_RANGE,
            )),
            MetalLbAddressRange::V6Range(V6Range::from_host_range(
                Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0xdb8, 0x4405, 0x417, 0, 0, 0, 0), 64),
                "::2000-::2100".parse().unwrap(),
            )),
        ];
        let calculated =
            calculate_changed_ranges(&current, DESIRED_PREFIX_NET, *DESIRED_HOST_RANGE, None)
                .map(HashSet::<_, RandomState>::from_iter);
        assert_eq!(
            calculated,
            Some(HashSet::from_iter(vec![V4_RANGE, *DESIRED_V6_RANGE]))
        );
    }

    #[test]
    fn matching_range_extra_v6_prefixes_removed() {
        let current = vec![
            V4_RANGE,
            *DESIRED_V6_RANGE,
            MetalLbAddressRange::V6Range(V6Range::from_host_range(
                Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0xdb8, 0x4405, 0x417, 0, 0, 0, 0), 64),
                "::2000-::2100".parse().unwrap(),
            )),
        ];
        let calculated =
            calculate_changed_ranges(&current, DESIRED_PREFIX_NET, *DESIRED_HOST_RANGE, None)
                .map(HashSet::<_, RandomState>::from_iter);
        assert_eq!(
            calculated,
            Some(HashSet::from_iter(vec![V4_RANGE, *DESIRED_V6_RANGE]))
        );
    }
}
