use either::Either::Left;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{DeleteParams, ListParams, Patch, PatchParams},
    core::ObjectMeta,
    runtime::{conditions::is_deleted, wait::await_condition},
    Api, Client,
};
use thiserror::Error;
use tracing::{event, instrument, Level};

use crate::{
    ranges::MetalLbAddressRange,
    v1beta1::ipaddresspool::{IPAddressPool, IPAddressPoolSpec, IPAddressPoolStatus},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetalLbUpdaterConfig {
    pub namespace: String,
    pub ip_pool: String,
    pub label_selector: String,
}

#[derive(Debug)]
pub struct MetalLbUpdater {
    config: MetalLbUpdaterConfig,
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

impl MetalLbUpdater {
    #[instrument]
    pub async fn new(config: MetalLbUpdaterConfig) -> Result<Self, K8sError> {
        event!(
            Level::DEBUG,
            msg = "Creating k8s Client for MetalLB access",
            pool = ?config.ip_pool
        );
        let client = Client::try_default().await?;

        let updater = MetalLbUpdater {
            config: config.clone(),
            pool_api: Api::namespaced(client.clone(), &config.namespace),
            pod_api: Api::namespaced(client, &config.namespace),
        };
        event!(
            Level::INFO,
            msg = "Created k8s Client for Pool",
            pool_name = ?config.ip_pool
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

    pub async fn set_addresses(&self, addresses: Vec<MetalLbAddressRange>) -> Result<(), K8sError> {
        let original_pool: IPAddressPool = self.get_pool().await?;
        let mut new_spec = original_pool.clone().spec;
        new_spec.addresses = addresses.iter().map(|a| a.to_string()).collect::<Vec<_>>();

        self.patch_pool(&PatchParams::default(), new_spec).await?;
        event!(Level::INFO, msg = "Pool updated");

        if let Err(e) = self.force_reset_metallb().await {
            event!(
                Level::ERROR,
                msg = "Error while restarting MetalLB, reverting Pool change...",
                error = e.to_string()
            );
            self.patch_pool(&PatchParams::default(), original_pool.spec.clone())
                .await?;
            self.force_reset_metallb().await?;
            event!(Level::INFO, msg = "Pool change reverted");
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_pool(&self) -> Result<IPAddressPool, K8sError> {
        self.pool_api
            .get(&self.config.ip_pool)
            .await
            .map(|p| {
                event!(Level::DEBUG, pool = ?p);
                p
            })
            .map_err(|e| K8sError {
                msg: format!("Error reading pool: {}", e),
            })
    }

    #[instrument(skip(self))]
    async fn patch_pool(
        &self,
        params: &PatchParams,
        spec: IPAddressPoolSpec,
    ) -> Result<(), K8sError> {
        let patch = IPAddressPool {
            metadata: ObjectMeta {
                ..Default::default()
            },
            spec,
            status: Some(IPAddressPoolStatus {}),
        };

        event!(Level::DEBUG, patch = ?patch);

        self.pool_api
            .patch(&self.config.ip_pool, params, &Patch::Merge(&patch))
            .await
            .map(|p| {
                event!(Level::DEBUG, pool = ?p);
            })
            .map_err(Into::into)
    }

    /// Forcibly delete all pods within the MetalLB namespace.
    /// This is required to get MetalLB to accept a new configuration, as documented here:
    /// https://github.com/metallb/metallb/issues/308
    #[instrument(skip(self))]
    async fn force_reset_metallb(&self) -> Result<(), K8sError> {
        event!(
            Level::INFO,
            msg = "Forcibly deleting MetalLB pods to pick up new addresses",
            label_selector = ?self.config.label_selector
        );
        if let Left(del) = self
            .pod_api
            .delete_collection(
                &DeleteParams::default(),
                &ListParams {
                    label_selector: Some(self.config.label_selector.clone()),
                    ..Default::default()
                },
            )
            .await?
        {
            for l in del {
                let (Some(name), Some(uid)) = (l.metadata.name, l.metadata.uid) else {
                    event!(
                        Level::WARN,
                        msg = "Could not wait for pod deletion, metadata incomplete"
                    );
                    continue;
                };
                event!(Level::DEBUG, msg = "Waiting for pod deletion", pod = name);
                await_condition(self.pod_api.clone(), &name, is_deleted(&uid))
                    .await
                    .map_err(|e| K8sError { msg: e.to_string() })?;
            }
        }
        Ok(())
    }
}
