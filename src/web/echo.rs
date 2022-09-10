use std::net::IpAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use crate::web::{with_auth, with_data};
use warp::body::json;
use crate::web::rejections::ApiError;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"players"/Uuid/"echo")).and(with_auth(data.clone(), "echo")).and(json::<EchoUserDefinition>()).and(with_data(data.clone())).and_then(forward_echo)
        .or(warp::get().and(path!("api"/"servers"/Uuid/"echo"/"enable")).and(with_auth(data.clone(), "echo")).and(with_data(data.clone())).and_then(enable_echo))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EchoUserDefinition {
    pub ip: Option<IpAddr>,
    pub server: Uuid,
    pub username: Option<String>
}

pub static ECHO_URL: &str = "http://echo.echo:8888";

pub async fn forward_echo(player: Uuid, info: EchoUserDefinition, data: Arc<AppData>)-> Result<impl Reply, Rejection> {

    let enabled = data.db.select_player_echo_enabled(&player).await.map_err(ApiError::from)?;
    if !enabled {
        warn!("Enabled alpha feature echo for player {}", player);
        data.db.set_player_echo_enabled(&player, true).await.map_err(ApiError::from)?;
    }

    let client = &data.client;
    let code: u32 = client.post(format!("{}/players/{}", ECHO_URL, player)).header("Authorization", data.echo_key.to_string()).json(&info).send().await.map_err(ApiError::from)?.json().await.map_err(ApiError::from)?;

    Ok(reply::json(&code).into_response())
}

pub async fn enable_echo(server: Uuid, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    warn!("Enabling alpha feature echo for server {}",server);
    let client = &data.client;

    let key = client.post(format!("{}/servers/{}", ECHO_URL, server)).header("Authorization", data.echo_key.to_string()).send().await.map_err(ApiError::from)?.json::<Uuid>().await.map_err(ApiError::from)?;

    data.db.update_server_echo_key(&server, Some(key)).await.map_err(ApiError::from)?;

    Ok(reply::json(&key))
}
