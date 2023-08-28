use std::{sync::Arc, time::Duration};

use anyhow::Result;
use clap::Parser;
use cli::Cli;


use k8s::AddressPoolUpdater;
use prefix_source::PrefixSource;
use subnet_override::SubnetOverride;
use tracing::{event, instrument, Level};
use types::ranges::{MetalLbAddressRange, V6HostRange, V6Range};

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
    tracing_subscriber::fmt().json().init();

    let cli = Cli::parse();
    let subnet_override = match (cli.subnet_override, cli.prefix_length) {
        (Some(or), Some(len)) => Some(subnet_override::validate_subnet_override(or, len)?),
        (None, None) => None,
        _ => unreachable!(), // Prevented by claps mutual requires
    };

    let source = get_source(&cli)?;

    let config = Arc::new(RuntimeConfig {
        source,
        pool: AddressPoolUpdater::new(cli.pool, cli.field_manager, cli.metallb_namespace).await?,
        subnet_override,
        host_range: cli.host_range,
        dry_run: cli.dy_run,
    });
    event!(Level::DEBUG, runtime_config = ?config);

    loop {
        let config = Arc::clone(&config);
        tokio::spawn(async move {
            let r = run(config).await;
            if let Err(e) = r {
                let text = e.to_string();
                event!(
                    Level::ERROR,
                    msg = "Run completed with errors",
                    error = text
                );
            }
        })
        .await?;
        tokio::time::sleep(Duration::from_secs(cli.update_interval)).await;
    }
}

#[instrument]
fn get_source(cli: &Cli) -> Result<Box<dyn PrefixSource>> {
    Ok(Box::new(match cli.source {
        cli::AddressSource::MyIp => {
            event!(Level::INFO, msg = "Using MyIP as address source");
            my_ip::MyIpSource::new()
        }
    }))
}

#[instrument]
async fn run(config: Arc<RuntimeConfig>) -> Result<()> {
    if config.dry_run {
        event!(
            Level::WARN,
            "Running in dry-run mode - no changes will be made"
        );
    }

    let mut prefix_net = config.source.get()?;
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

    let updated_ranges = current_ranges
        .iter()
        .map(|r| match r {
            // passthrough all ranges we don't care about
            MetalLbAddressRange::V4Cidr(r) => MetalLbAddressRange::V4Cidr(*r),
            MetalLbAddressRange::V6Cidr(r) => MetalLbAddressRange::V6Cidr(*r),
            MetalLbAddressRange::V4Range(r) => MetalLbAddressRange::V4Range(*r),
            // And replace any other ipv6-ranges with our desired one
            MetalLbAddressRange::V6Range(_) => desired_range,
        })
        .collect::<Vec<_>>();
    event!(
        Level::DEBUG,
        desired_ranges = ?updated_ranges
    );

    if !config.dry_run {
        config.pool.set_addresses(updated_ranges, true).await?;
    }

    event!(Level::INFO, msg = "Update successful");
    Ok(())
}
