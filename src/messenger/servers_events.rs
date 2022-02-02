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
    /*    UpdateProxyConfig {
            //TODO
        },*/
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
    InvalidatePlayer {
        #[serde(skip)]
        server: Uuid,
        uuid: Uuid,
    },
}


impl ServerEvent {
    pub fn route(&self) -> String {
        use ServerEvent::*;

        match self {
            NewRoute { .. } => "proxy.servers.routes.new".to_string(),
            DeleteRoute { .. } => "proxy.servers.routes.delete".to_string(),
            ServerStarted { .. } => "proxy.servers.routes.started".to_string(),
            //ServerEvent::UpdateProxyConfig { .. } => "disabled".to_string(),
            MovePlayer { proxy, .. } => proxy.to_string(),
            AdminMovePlayer { server, .. } => server.to_string(),
            MovePlayerToAvailable { proxy, .. } => proxy.to_string(),
            DisconnectPlayer { proxy, .. } => proxy.to_string(),
            InvalidatePlayer { server, .. } => server.to_string(),
        }
    }
    pub fn direct(&self) -> bool {
        use ServerEvent::*;
        match self {
            MovePlayer { .. } |
            AdminMovePlayer { .. } |
            MovePlayerToAvailable { .. } |
            DisconnectPlayer { .. } | 
            InvalidatePlayer { .. } => true,
            _ => false
        }
    }
}