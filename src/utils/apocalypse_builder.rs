use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;
use crate::AppData;
use crate::database::DatabaseError;
use async_recursion::async_recursion;
use serde::{Serialize, Deserialize};


#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApocalypseState {
    uuids: Vec<Uuid>,
    names: Vec<String>,
    discords: Vec<String>,
    ips: Vec<IpAddr>,
    i: i64,
}


#[async_recursion]
pub async fn get_name_associations(name: String, data: Arc<AppData>, state: &mut ApocalypseState) -> Result<(), DatabaseError> {
    state.i += 1;
    if let Some(uuid) = data.db.select_players_uuid_by_name(&name).await? {
        if !state.uuids.contains(&uuid) {
            state.uuids.push(uuid.clone());
            get_uuid_associations(uuid, data.clone(), state).await?;
        }
    }

    Ok(())
}

#[async_recursion]
pub async fn get_uuid_associations(uuid: Uuid, data: Arc<AppData>, state: &mut ApocalypseState) -> Result<(), DatabaseError> {
    state.i += 1;
    if let Some(name) = data.db.select_player_name(&uuid).await? {
        if !state.names.contains(&name) {
            state.names.push(name.clone());
            get_name_associations(name, data.clone(), state).await?;
        }
    }

    for ip in data.db.select_all_player_sessions_ips(&uuid).await? {
        if !state.ips.contains(&ip) {
            state.ips.push(ip.clone());
            get_ip_associations(ip, data.clone(), state).await?;
        }
    }

    for discord in data.db.select_player_discord(&uuid).await? {
        if !state.discords.contains(&discord) {
            state.discords.push(discord.clone());
            get_discord_associations(discord, data.clone(), state).await?;
        }
    }


    Ok(())
}


#[async_recursion]
pub async fn get_ip_associations(ip: IpAddr, data: Arc<AppData>, state: &mut ApocalypseState) -> Result<(), DatabaseError> {
    state.i += 1;

    for uuid in data.db.select_players_with_session_ip(&ip).await? {
        if !state.uuids.contains(&uuid) {
            state.uuids.push(uuid.clone());
            get_uuid_associations(uuid, data.clone(), state).await?;
        }
    }

    Ok(())
}

#[async_recursion]
pub async fn get_discord_associations(discord: String, data: Arc<AppData>, state: &mut ApocalypseState) -> Result<(), DatabaseError> {
    state.i += 1;

    for uuid in data.db.select_players_uuid_by_discord(&discord).await? {
        if !state.uuids.contains(&uuid) {
            state.uuids.push(uuid.clone());
            get_uuid_associations(uuid, data.clone(), state).await?;
        }
    }
    Ok(())
}