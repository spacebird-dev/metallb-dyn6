use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

use metallb_dyn6_k8s::{
    ranges::{MetalLbAddressRange, V6HostRange, V6Range},
    AddressPoolUpdater,
};
use metallb_dyn6_sources::{MyIpSource, PrefixSource};
use subnet_override::SubnetOverride;
use tracing::{event, instrument, Level};
use tracing_subscriber::EnvFilter;

mod cli;
mod subnet_override;

#[derive(Debug)]
struct RuntimeConfig {
    source: Box<dyn PrefixSource>,
    pool: AddressPoolUpdater,
    subnet_override: Option<SubnetOverride>,
    host_range: V6HostRange,
    dry_run: bool,
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let subnet_override = match (cli.subnet_override, cli.prefix_length) {
        (Some(or), Some(len)) => Some(subnet_override::validate_subnet_override(or, len)?),
        (None, None) => None,
        _ => unreachable!(), // Prevented by claps mutual requires
    };

    let source = get_source(&cli)?;

    let config = RuntimeConfig {
        source,
        pool: AddressPoolUpdater::new(cli.pool, cli.metallb_namespace).await?,
        subnet_override,
        host_range: cli.host_range,
        dry_run: cli.dry_run,
    };
    event!(Level::DEBUG, runtime_config = ?config);

    loop {
        let r = run(&config).await;
        if let Err(e) = r {
            let text = e.to_string();
            event!(
                Level::ERROR,
                msg = "Run completed with errors",
                error = text
            );
        }
        tokio::time::sleep(Duration::from_secs(cli.update_interval)).await;
    }
}

#[instrument(skip(cli))]
fn get_source(cli: &Cli) -> Result<Box<dyn PrefixSource>> {
    Ok(Box::new(match cli.source {
        cli::AddressSource::MyIp => {
            event!(Level::INFO, msg = "Using MyIP as address source");
            MyIpSource::new()
        }
    }))
}

#[instrument(skip(config))]
async fn run(config: &RuntimeConfig) -> Result<()> {
    if config.dry_run {
        event!(
            Level::WARN,
            "Running in dry-run mode - no changes will be made"
        );
    }

    let mut prefix_net = config.source.get().await?;
    event!(Level::INFO, msg = "Retrieved dynamic prefix", prefix = ?prefix_net);
    assert_eq!(prefix_net.prefix_len(), 64);

    if let Some(subnet_override) = &config.subnet_override {
        prefix_net = subnet_override::apply_subnet_override(&prefix_net, subnet_override)?;
    }

    let desired_range =
        MetalLbAddressRange::V6Range(V6Range::from_host_range(prefix_net, config.host_range));
    event!(Level::INFO, msg = "Desired address range", range = ?desired_range);

    let current_ranges = config.pool.get_addresses().await?;
    event!(Level::DEBUG, current_ranges = ?current_ranges);
    if current_ranges.contains(&desired_range) {
        event!(
            Level::INFO,
            msg = "Address range already present, nothing to do"
        );
        return Ok(());
    }

    let mut updated_ranges = current_ranges
        .into_iter()
        .filter(|r| {
            // Remove any pre-existing IPv6 address ranges
            !matches!(
                r,
                MetalLbAddressRange::V6Cidr(_) | MetalLbAddressRange::V6Range(_)
            )
        })
        .collect::<Vec<_>>();
    updated_ranges.push(desired_range);
    event!(
        Level::INFO,
        desired_ranges = ?updated_ranges
    );

    if !config.dry_run {
        config.pool.set_addresses(updated_ranges, true).await?;
        event!(Level::INFO, msg = "Pool successfully updated");
    }

    Ok(())
}
