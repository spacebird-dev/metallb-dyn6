use either::Either::Left;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{DeleteParams, ListParams, Patch, PatchParams},
    core::{object::HasSpec, ObjectMeta},
    runtime::{conditions::is_deleted, wait::await_condition},
    Api, Client,
};
use thiserror::Error;
use tracing::{event, instrument, Level};
use types::ranges::MetalLbAddressRange;
use v1beta1::ipaddresspool::{IPAddressPool, IPAddressPoolStatus};

mod v1beta1;

#[derive(Debug)]
pub struct AddressPoolUpdater {
    pool_name: String,
    pool_api: Api<IPAddressPool>,
    pod_api: Api<Pod>,
}

#[derive(Error, Debug)]
#[error("Error while accessing the k8s API: {msg}")]
pub struct K8sError {
    msg: String,
}
impl From<kube::Error> for K8sError {
    fn from(value: kube::Error) -> Self {
        K8sError {
            msg: value.to_string(),
        }
    }
}

impl AddressPoolUpdater {
    #[instrument]
    pub async fn new(pool_name: String, metallb_namespace: String) -> Result<Self, K8sError> {
        event!(
            Level::DEBUG,
            msg = "Creating k8s Client for MetalLB access",
            pool = &pool_name
        );
        let client = Client::try_default().await?;

        let updater = AddressPoolUpdater {
            pool_name: pool_name.clone(),
            pool_api: Api::namespaced(client.clone(), &metallb_namespace),
            pod_api: Api::namespaced(client, &metallb_namespace),
        };
        event!(
            Level::INFO,
            msg = "Created k8s Client for Pool",
            pool_name = &pool_name
        );
        let pool = updater.get_pool().await?;
        event!(Level::DEBUG, ?pool);
        Ok(updater)
    }

    pub async fn get_addresses(&self) -> Result<Vec<MetalLbAddressRange>, K8sError> {
        let raw_addresses = self.get_pool().await?.spec.addresses;
        raw_addresses
            .iter()
            .map(|a| {
                a.parse::<MetalLbAddressRange>().map_err(|e| K8sError {
                    msg: format!("Error while parsing IP pool addresses: {}", e),
                })
            })
            .collect::<Result<Vec<_>, K8sError>>()
    }

    pub async fn set_addresses(
        &self,
        addresses: Vec<MetalLbAddressRange>,
        force_reload: bool,
    ) -> Result<(), K8sError> {
        let mut pool = self.get_pool().await?;
        let spec = pool.spec_mut();
        spec.addresses = addresses.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        let patch = IPAddressPool {
            metadata: ObjectMeta {
                ..Default::default()
            },
            spec: spec.clone(),
            status: Some(IPAddressPoolStatus {}),
        };

        self.pool_api
            .patch(
                &self.pool_name,
                &PatchParams::default(),
                &Patch::Merge(&patch),
            )
            .await
            .map(|_| ())?;
        if force_reload {
            self.force_reset_metallb().await?;
        }
        Ok(())
    }

    async fn get_pool(&self) -> Result<IPAddressPool, K8sError> {
        self.pool_api
            .get(&self.pool_name)
            .await
            .map_err(|e| K8sError {
                msg: format!("Error reading pool: {}", e),
            })
    }

    /// Forcibly delete all pods within the MetalLB namespace.
    /// This is required to get MetalLB to accept a new configuration, as documented here:
    /// https://github.com/metallb/metallb/issues/308
    #[instrument(skip(self))]
    async fn force_reset_metallb(&self) -> Result<(), K8sError> {
        event!(
            Level::INFO,
            msg = "Forcibly deleting MetalLB pods to pick up new addresses"
        );
        if let Left(del) = self
            .pod_api
            .delete_collection(&DeleteParams::default(), &ListParams::default())
            .await?
        {
            for l in del {
                let name = l.metadata.name.unwrap();
                event!(Level::DEBUG, msg = "Waiting for pod deletion", pod = name);
                await_condition(
                    self.pod_api.clone(),
                    &name,
                    is_deleted(&l.metadata.uid.unwrap()),
                )
                .await
                .map_err(|e| K8sError { msg: e.to_string() })?;
            }
        }
        Ok(())
    }
}
