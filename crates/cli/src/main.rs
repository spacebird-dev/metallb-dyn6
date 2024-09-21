use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

use metallb_dyn6_k8s::{ranges::V6HostRange, MetalLbUpdater, MetalLbUpdaterConfig};
use metallb_dyn6_sources::{MyIpSource, NetworkSource};
use subnet_override::SubnetOverride;
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::EnvFilter;

mod cli;
mod ranges;
mod subnet_override;

#[derive(Debug)]
struct RuntimeConfig {
    source: Box<dyn NetworkSource>,
    pool: MetalLbUpdater,
    subnet_override: Option<SubnetOverride>,
    host_range: V6HostRange,
    dry_run: bool,
}

#[instrument(skip(cli))]
fn get_source(cli: &Cli) -> Result<Box<dyn NetworkSource>> {
    Ok(Box::new(match cli.source {
        cli::NetworkSource::MyIp => {
            info!(msg = "Using MyIP as address source");
            MyIpSource::new()
        }
    }))
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let subnet_override = match (cli.subnet_override, cli.prefix_length) {
        (Some(or), Some(len)) => Some(SubnetOverride::new(or, len)?),
        (None, None) => None,
        // Prevented by claps mutual requires
        _ => unreachable!("subnet_override or prefix_length must be specified together"),
    };

    let source = get_source(&cli)?;

    let config = RuntimeConfig {
        source,
        pool: MetalLbUpdater::new(MetalLbUpdaterConfig {
            ip_pool: cli.metallb_pool,
            namespace: cli.metallb_namespace,
            label_selector: cli.metallb_pods_label_selector,
        })
        .await?,
        subnet_override,
        host_range: cli.host_range,
        dry_run: cli.dry_run,
    };
    info!(runtime_config = ?config);

    if config.dry_run {
        warn!("Running in dry-run mode - no changes will be made");
    }

    loop {
        let r = run(&config).await;
        if let Err(e) = r {
            let text = e.to_string();
            error!(msg = "Run completed with errors", error = text);
        }
        tokio::time::sleep(Duration::from_secs(cli.update_interval)).await;
    }
}

#[instrument(skip(config))]
async fn run(config: &RuntimeConfig) -> Result<()> {
    let prefix_net = config.source.get().await?;
    info!(msg = "Retrieved dynamic prefix", prefix = ?prefix_net);
    assert_eq!(prefix_net.prefix_len(), 64);

    let current_ranges = config.pool.get_addresses().await?;
    debug!(current_ranges = ?current_ranges);

    let Some(desired_ranges) = ranges::calculate_changed_ranges(
        &current_ranges,
        prefix_net,
        config.host_range,
        config.subnet_override,
    ) else {
        info!("Desired address ranges match current ranges, nothing to do");
        return Ok(());
    };

    if !config.dry_run {
        config.pool.set_addresses(desired_ranges).await?;
    } else {
        info!("Skipping applying changes due to dry-run mode being enabled")
    }

    Ok(())
}
