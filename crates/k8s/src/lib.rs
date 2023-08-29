pub(crate) mod v1beta1;

pub mod ranges;
mod updater;

pub use updater::{AddressPoolUpdater, K8sError};
