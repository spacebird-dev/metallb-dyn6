# metallb-dyn6

Dynamic IPv6 Prefix support for [MetalLB](https://metallb.universe.tf/).

This utility enables MetalLB to manage an IPv6 address pool with a dynamically changing prefix.
It synchronizes the network part of an Ipv6 Address Range in a MetalLB IPAddressPool with a dynamically retrieved Ipv6 network.
It ensures that the address range in the pool always corresponds to the currently assigned network and automatically reloads MetaLLB when required.

## Use Case & Concepts

The main use case for this tool is running an Ipv6-enabled k8s cluster from a residential ISP connection with no static Ipv6 Prefix.

Consider the following situation:
You want to host an IPv6-enabled k8s cluster and have gotten an Ipv6 prefix from your ISP, say `2001:db8:aaaa::/48`.
To make your cluster-hosted services accessible from the internet, you'd assign them addresses from within that prefix (and setup your router, but that's outside the scope of this README).
For example, your `IPAddressPool` could look like this:

```yaml
apiVersion: metallb.io/v1beta1
kind: IPAddressPool
metadata:
  namespace: metallb-system
spec:
  addresses:
    # Some private IPv4 addresses
    - 10.10.1.10-10.10.1.99
    # A range from within your ISP-assigned Prefix
    - 2001:db8:aaaa::1000-2001:db8:aaaa::1999
```

This works great, as long as the prefix assigned to you is **static** - that is, it does not change.

However, many consumer ISPs regularly change the assigned prefix (for example when the connection gets interrupted).
This means that after a power outage, your assigned prefix might suddenly be `2001:db8:ffff::/48`, resulting in an incorrect MetalLB configuration and thus no internet access.

Unfortunately, as `IPAddressPool` resources are static, MetalLB has no native way to address this.
This is where `metallb-dyn6` comes in.

`metallb-dyn6` addresses this issue by listening to changes in the prefix, replacing the range in the `IPAddressPool` with one based on the new prefix whenever a change occurs.
It does this by performing the following actions:

1. First, it queries a *source* for the IPv6 prefix, which simply tells `metallb-dyn6` what prefix to use. Right now, the only available source is `my-ip`, which queries the [MyIP API](https://www.my-ip.io/) for your current public IPV6 address.
    - `metallb-dyn6`s design is modular, so more sources can easily be added in the future.
2. It then compares the Prefix stored in the `IPAddresspool` with the one retrieved from the source. If there is a mismatch, it updates the `IPAddressPool` to match the prefix retrieved from the source.
3. Finally, it forces MetalLB to accept this new configuration by deleting all of its pods and waiting for them to be recreated (this is the [officially recommended way to do this](https://github.com/metallb/metallb/issues/348#issuecomment-442218138)).


## Installation

The officially recommanded way to install `metallb-dyn6` is through Helm.

First, install the repository like so:

`helm repo add spacebird https://charts.spacebird.dev`

Then, install the chart:

```sh
helm install metallb-dyn6 spacebird/metallb-dyn6 \
  --namespace metallb-system
  --set "metallb.hostRange=::1000-::1999"
  --set "metallb.pool=my-ipaddress-pool-name"
```

---
⚠️ **IMPORTANT NOTES** ⚠️

- You **must** install `metallb-dyn6` into the same namespace as your `metallb` installation.
- The parameters `metallb.hostRange` and `metallb.pool` are **required**.
- `metallb.hostRange` must be a dash-separated range - it cannot be a `/xx` CIDR-style range.

---

For a full list of possible values, please see the [Helm chart docs](https://github.com/spacebird-dev/charts/tree/main/charts/metallb-dyn6)

### Subnet override

Sometimes, the IPv6 network returned from the source may not have the correct subnet applied.
For example, you may have a separate subnet configured for just your k8s cluster services traffic that you'd like MetalLB to use.

In those cases, you can override parts of the returned network with the `subnetOverride` values.
For example, to always override the last 8 bits of the network with `cd:`, you would supply the following values:

```yaml
dyn6:
  subnetOverride:
    enabled: true
    prefixLen: 56 # how many of the original network bits to keep
    override: "0:0:0:00cd::" # note the leading zeros - they are required
```

## Development

This tool is built in Rust, using standard `cargo` tooling.
You may want to install the `just` command runner to run the recipes in the [`Justfile`](./Justfile).
`cross` is used for cross-compilation.

### Release Management

Draft Releases are automatically managed through the `version-maintenance` workflow.
Follow the instructions in those drafts to publish a new release.
