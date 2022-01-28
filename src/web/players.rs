use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use chrono::Duration;
use reqwest::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::{instrument};
use uuid::{Uuid};
use warp::body::json;
use crate::web::rejections::ApiError;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use crate::messenger::servers_events::ServerEvent;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"players"/Uuid/"stats")).and(with_auth(data.clone(), "player-stats")).and(with_data(data.clone())).and(json::<PlayerStats>()).and_then(post_stats)
        .or(warp::post().and(path!("api"/"players"/Uuid/"move")).and(with_auth(data.clone(), "move-player")).and(with_data(data.clone()).and(json::<PlayerMove>())).and_then(move_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"ban")).and(with_auth(data.clone(), "ban-player")).and(with_data(data.clone())).and(json::<PlayerBan>()).and_then(ban_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"disconnect")).and(with_auth(data.clone(), "disconnect-player")).and(with_data(data.clone())).and_then(disconnect_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"transaction")).and(with_auth(data.clone(), "make-transaction")).and(with_data(data.clone())).and(json::<PlayerTransaction>()).and_then(player_transaction))
        .or(warp::get().and(path!("api"/"players")).and(with_auth(data.clone(), "get-online-players")).and(with_data(data.clone())).and_then(get_online))
        .or(warp::get().and(path!("api"/"players"/String)).and(with_auth(data.clone(), "get-player")).and(with_data(data.clone())).and_then(get_player))
}


#[derive(Deserialize, Serialize, Debug)]
struct PlayerStats {
    server: Uuid,
    session: Uuid,
    stats: HashMap<String, i32>,
}


#[instrument(skip(data, request))]
async fn post_stats(uuid: Uuid, data: Arc<AppData>, request: PlayerStats) -> Result<impl Reply, Rejection> {
    let kind = match data.db.select_server_kind(&request.server).await.map_err(ApiError::from)?.map(|kind| data.db.get_cached_kind(&kind)).flatten() {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    data.db.insert_stats(&uuid, &request.session, &request.server, &kind.name, &request.stats).await.map_err(ApiError::from)?;

    Ok(reply().into_response())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum PlayerMove {
    Server {
        server: Uuid,
        #[serde(default)]
        admin_move: bool,
    },
    ServerKind {
        kind: String
    },
}

#[instrument(skip(data))]
async fn move_player(uuid: Uuid, data: Arc<AppData>, request: PlayerMove) -> Result<impl Reply, Rejection> {
    let proxy = match data.db.select_online_player_proxy(&uuid).await.map_err(ApiError::from)? {
        None => return Ok(reply::with_status("Player is not online or does not exists", StatusCode::NOT_FOUND).into_response()),
        Some(proxy) => proxy
    };

    match request {
        PlayerMove::Server { server, admin_move } => {
            if data.db.select_server_kind(&server).await.map_err(ApiError::from)?.is_none() {
                return Ok(reply::with_status("Requested server does not exist", StatusCode::NOT_FOUND).into_response());
            }

            if admin_move {
                data.msgr.send_event(&ServerEvent::AdminMovePlayer {
                    server,
                    player: uuid,
                }).await.map_err(ApiError::from)?;
            } else {
                data.msgr.send_event(&ServerEvent::MovePlayer {
                    proxy,
                    server,
                    player: uuid,
                }).await.map_err(ApiError::from)?;
            }
        }
        PlayerMove::ServerKind { kind } => {
            if data.db.get_cached_kind(&kind).is_none() {
                return Ok(reply::with_status("Requested server kind does not exist", StatusCode::NOT_FOUND).into_response());
            }
            data.msgr.send_event(&ServerEvent::MovePlayerToAvailable {
                proxy,
                kind,
                player: uuid,
            }).await.map_err(ApiError::from)?;
        }
    }


    Ok(reply::reply().into_response())
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerBan {
    duration: Option<i32>,
    reason: Option<String>,
    issuer: Option<Uuid>,
    #[serde(default)]
    ip: bool,
    #[serde(default)]
    unban: bool,
}

#[instrument(skip(data))]
async fn ban_player(uuid: Uuid, data: Arc<AppData>, request: PlayerBan) -> Result<impl Reply, Rejection> {
    if request.ip {
        if request.unban {
            //TODO
            return Ok(reply().into_response());
        } else {
            //TODO
            //get sessions
            //data.db.insert_ips_bans()
        }

        return Ok(reply().into_response());
    } else if request.unban {
        data.db.remove_player_ban(&uuid).await.map_err(ApiError::from)?;
        return Ok(reply().into_response());
    } else {
        let duration = request.duration.map(|t| Duration::seconds(t as i64));
        data.db.insert_ban(&uuid, request.reason.as_ref(), request.issuer.as_ref(), duration.as_ref()).await.map_err(ApiError::from)?;
    }

    if let Some(proxy) = data.db.select_online_player_proxy(&uuid).await.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::DisconnectPlayer { proxy, player: uuid }).await.map_err(ApiError::from)?;
    }


    Ok(reply().into_response())
}


#[instrument(skip(data))]
async fn get_online(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    Ok(reply::json(&data.db.select_online_players_reduced_info().await.map_err(ApiError::from)?).into_response())
}


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


#[instrument(skip(data))]
async fn get_player(player: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    match Uuid::parse_str(&player) {
        Ok(uuid) => Ok(reply::json(&match data.db.select_player_info(&uuid).await.map_err(ApiError::from)? {
            None => return Ok(StatusCode::NOT_FOUND.into_response()),
            Some(info) => info
        }.build_player_info(&data.db).await.map_err(ApiError::from)?).into_response()),
        Err(_) => {
            Ok(reply::json(&match data.db.select_player_info_by_name(&player).await.map_err(ApiError::from)? {
                None => return Ok(StatusCode::NOT_FOUND.into_response()),
                Some(info) => info
            }.build_player_info(&data.db).await.map_err(ApiError::from)?).into_response())
        }
    }
}

#[instrument(skip(data))]
async fn disconnect_player(uuid: Uuid, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    if let Some(proxy) = data.db.select_online_player_proxy(&uuid).await.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::DisconnectPlayer { proxy, player: uuid }).await.map_err(ApiError::from)?;
        Ok(StatusCode::OK)
    }else {
        Ok(StatusCode::NOT_FOUND)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerTransaction{
    #[serde(default)]
    currency: i32,
    #[serde(default)]
    premium_currency: i32
}

#[instrument(skip(data))]
async fn player_transaction(uuid: Uuid, data: Arc<AppData>, request: PlayerTransaction) -> Result<impl Reply, Rejection> {
    if let Some((currency, premium_currency)) = data.db.select_player_currencies(&uuid).await.map_err(ApiError::from)? {
        if currency >= request.currency && premium_currency >= request.premium_currency{
            data.db.set_player_currencies(&uuid, currency - request.currency, premium_currency - request.premium_currency).await.map_err(ApiError::from)?;
            Ok(reply::json(&true).into_response())
        }else {
            Ok(reply::json(&false).into_response())
        }
    }else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}