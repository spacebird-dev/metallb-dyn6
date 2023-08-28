use std::{net::Ipv6Addr, time::Duration};

use ipnet::Ipv6Net;
use prefix_source::addr_to_network;
use prefix_source::{PrefixSource, SourceError};
use reqwest::blocking::Client;
use serde::Deserialize;

const MY_IP_URL: &str = "https://api6.my-ip.io/ip.json";

#[derive(Deserialize)]
enum MyIpType {
    Ipv6,
}
#[derive(Deserialize)]
#[allow(dead_code)]
struct MyIpResponse {
    sucess: bool,
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

impl PrefixSource for MyIpSource {
    fn get(&self) -> Result<Ipv6Net, SourceError> {
        let ip = self
            .client
            .get(MY_IP_URL)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| SourceError { msg: e.to_string() })?
            .json::<MyIpResponse>()
            .map_err(|e| SourceError { msg: e.to_string() })?
            .ip;
        Ok(addr_to_network(ip))
    }
}
