use std::collections::HashMap;
use std::net::IpAddr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub uuid: Uuid,
    pub username: String,
    pub power: i32,
    pub locale: String,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub currency: i32,
    pub premium_currency: i32,
    pub proxy: Option<Uuid>,
    pub server: Option<Uuid>,
    pub blocked: Vec<Uuid>,
    pub inventory: HashMap<String, i32>,
    pub properties: HashMap<String, String>,
    pub ban: Option<Ban>,
    pub discord_id: Option<String>,
    pub mute: Option<Mute>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ban {
    pub id: Uuid,
    pub start: String,
    pub end: Option<String>,
    pub issuer: Option<Uuid>,
    pub reason: Option<String>,
    pub ip: Option<IpAddr>,
    pub target: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mute {
    pub id: Uuid,
    pub start: String,
    pub end: Option<String>,
    pub issuer: Option<Uuid>,
    pub reason: Option<String>,
    pub target: Option<Uuid>,
    pub remaining: Option<i64>
}
