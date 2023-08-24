use std::net::Ipv6Addr;

use address_source::{AddressSource, SourceError};
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

pub struct IpifySource {
    client: Client,
}

impl IpifySource {
    pub fn new() -> Result<Self, SourceError> {
        Ok(IpifySource {
            client: Client::new(),
        })
    }
}

impl AddressSource for IpifySource {
    fn get(&self) -> Result<Ipv6Addr, SourceError> {
        Ok(self
            .client
            .get(MY_IP_URL)
            .send()
            .map_err(|e| SourceError { msg: e.to_string() })?
            .json::<MyIpResponse>()
            .map_err(|e| SourceError { msg: e.to_string() })?
            .ip)
    }
}
