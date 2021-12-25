use std::collections::HashMap;
use std::net::IpAddr;

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(tag = "event")]
pub enum ServerEvent {
    NewRoute {
        addr: IpAddr,
        id: Uuid,
        description: String,
        name: String,
        kind: String,
        properties: HashMap<String, String>,
    },
    DeleteRoute {
        id: Uuid,
        name: String,
    },
    ServerStarted {
        addr: IpAddr,
        id: Uuid,
        description: String,
        name: String,
        kind: String,
        properties: HashMap<String, String>,
    },
    UpdateProxyConfig {
        //TODO
    },
    MovePlayer {
        #[serde(skip)]
        proxy: Uuid,
        server: Uuid,
        player: Uuid,
    },
    AdminMovePlayer {
        server: Uuid,
        player: Uuid,
    },
    MovePlayerToAvailable {
        #[serde(skip)]
        proxy: Uuid,
        kind: String,
        player: Uuid,
    },
    DisconnectPlayer {
        #[serde(skip)]
        proxy: Uuid,
        player: Uuid,
    },
}


impl ServerEvent {
    pub fn route(&self) -> String {
        match self {
            ServerEvent::NewRoute { .. } => "proxy.servers.routes.new".to_string(),
            ServerEvent::DeleteRoute { .. } => "proxy.servers.routes.delete".to_string(),
            ServerEvent::ServerStarted { .. } => "proxy.servers.routes.started".to_string(),
            ServerEvent::UpdateProxyConfig { .. } => "disabled".to_string(),
            ServerEvent::MovePlayer { proxy, .. } => proxy.to_string(),
            ServerEvent::AdminMovePlayer { server, .. } => server.to_string(),
            ServerEvent::MovePlayerToAvailable { proxy, .. } => proxy.to_string(),
            ServerEvent::DisconnectPlayer { proxy, .. } => proxy.to_string()
        }
    }
    pub fn direct(&self) -> bool {
        match self {
            ServerEvent::MovePlayer { .. } |
            ServerEvent::AdminMovePlayer { .. } |
            ServerEvent::MovePlayerToAvailable { .. } |
            ServerEvent::DisconnectPlayer { .. } => true,
            _ => false
        }
    }
}