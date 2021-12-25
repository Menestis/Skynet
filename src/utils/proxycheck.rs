use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use reqwest::{Client, Error};
use tracing::{error, instrument, warn};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use crate::log::debug;
use crate::utils::proxycheck::ProxyCheckError::ProxyCheck;

#[derive(Serialize, Deserialize)]
struct ProxyCheckResponse {
    status: ProxyCheckStatus,
    message: Option<String>,
    #[serde(flatten)]
    ips: HashMap<IpAddr, ProxyCheckIpResponse>,
}

#[derive(Serialize, Deserialize)]
enum ProxyCheckStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "denied")]
    Denied,
    #[serde(rename = "error")]
    Error,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyCheckIpResponse {
    pub proxy: String,
    #[serde(rename = "type")]
    pub ip_type: Option<String>,
    pub provider: Option<String>,
    pub risk: i32,
}

#[derive(Error, Debug)]
pub enum ProxyCheckError {
    #[error(transparent)]
    Reqwest(#[from] Error),
    #[error("Proxycheck : {0}")]
    ProxyCheck(String),
}


#[instrument(skip(client, api_key, addr), level = "debug")]
pub async fn check_ip(client: &Client, api_key: &str, addr: &IpAddr) -> Result<ProxyCheckIpResponse, ProxyCheckError> {
    let response = client.get(format!("https://proxycheck.io/v2/{}?vpn=1&risk=1&seen=1&tag=login&key={}", addr, api_key)).send().await?;
    let mut proxy_check_response = response.json::<ProxyCheckResponse>().await?;
    match proxy_check_response.status {
        ProxyCheckStatus::Ok => {
            match proxy_check_response.ips.remove(addr) {
                None => Err(ProxyCheck("Provided ip was not found in result".to_string())),
                Some(res) => Ok(res)
            }
        }
        ProxyCheckStatus::Warning => {
            match proxy_check_response.ips.remove(addr) {
                None => {
                    match proxy_check_response.message {
                        None => Err(ProxyCheck("Provided ip was not found in result, proxycheck.io added a warning about this query but no message was given".to_string())),
                        Some(msg) => Err(ProxyCheckError::ProxyCheck(msg))
                    }
                }
                Some(res) => Ok(res)
            }
        }
        ProxyCheckStatus::Denied | ProxyCheckStatus::Error =>
            Err(ProxyCheckError::ProxyCheck(proxy_check_response.message.unwrap_or("No message".to_string())))
    }
}


impl ProxyCheckIpResponse {
    pub fn is_risk(&self) -> bool {
        let proxy = self.proxy == "yes";
        let vpn = self.ip_type.as_ref().map_or(false, |ty| ty == "VPN");
        return (proxy && !vpn) || (proxy && self.risk > 33) || self.risk > 66;
    }
}

impl Display for ProxyCheckIpResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let proxy = self.proxy == "yes";
        let vpn = self.ip_type.as_ref().map_or(false, |ty| ty == "VPN");
        write!(f, "Proxy : {}, VPN : {}, Risk : {}", proxy, vpn, self.risk)
    }
}

