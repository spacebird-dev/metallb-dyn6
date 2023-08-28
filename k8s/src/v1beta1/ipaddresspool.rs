use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// MetalLB IPAddressPool CRD specification, as needed to interact with k8s.
/// This was manually generated, but could in theory be automated using openapi-generation
#[derive(Deserialize, Serialize, Debug, Clone, CustomResource, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
#[kube(group = "metallb.io", version = "v1beta1", kind = "IPAddressPool")]
pub(crate) struct IpAddressPoolSpec {
    pub(crate) addresses: Vec<String>,
    pub(crate) auto_assign: Option<bool>,
    pub(crate) avoid_buggy_ips: Option<bool>,
    pub(crate) service_allocation: Option<ServiceAllocationSpec>,
}

#[derive(Deserialize, Serialize, Debug, Clone, CustomResource, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
#[kube(group = "metallb.io", version = "v1beta1", kind = "ServiceAllocation")]
pub(crate) struct ServiceAllocationSpec {
    pub(crate) priority: u32,
    pub(crate) namespaces: Vec<String>,
    pub(crate) namespace_selectors: Vec<LabelSelector>,
    pub(crate) service_selectors: Vec<LabelSelector>,
}
