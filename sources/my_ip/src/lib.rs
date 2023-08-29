use std::{net::Ipv6Addr, time::Duration};

use async_trait::async_trait;
use ipnet::Ipv6Net;
use prefix_source::addr_to_network;
use prefix_source::{PrefixSource, SourceError};
use reqwest::Client;
use serde::Deserialize;

const MY_IP_URL: &str = "https://api6.my-ip.io/ip.json";

#[derive(Deserialize)]
enum MyIpType {
    IPv6,
}
#[derive(Deserialize)]
#[allow(dead_code)]
struct MyIpResponse {
    success: bool,
    ip: Ipv6Addr,
    r#type: MyIpType,
}

#[derive(Debug, Clone)]
pub struct MyIpSource {
    client: Client,
}

impl MyIpSource {
    pub fn new() -> Self {
        MyIpSource {
            client: Client::new(),
        }
    }
}

impl Default for MyIpSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PrefixSource for MyIpSource {
    async fn get(&self) -> Result<Ipv6Net, SourceError> {
        let ip = self
            .client
            .get(MY_IP_URL)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| SourceError { msg: e.to_string() })?
            .json::<MyIpResponse>()
            .await
            .map_err(|e| SourceError { msg: e.to_string() })?
            .ip;
        Ok(addr_to_network(ip))
    }
}
