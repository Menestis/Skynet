use std::sync::Arc;
use chrono::Duration;
use reqwest::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::{instrument};
use uuid::Uuid;
use warp::body::json;
use crate::web::rejections::ApiError;
use crate::web::with_auth;
use serde::{Serialize, Deserialize};
use crate::messenger::servers_events::ServerEvent;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"players"/Uuid/"move")).and(with_auth(data.clone(), "move-player")).and(super::with_data(data.clone()).and(json::<PlayerMove>())).and_then(move_player)
        .or(warp::get().and(path!("api"/"players")).and(with_auth(data.clone(), "get-online-players")).and(super::with_data(data.clone())).and_then(get_online))
        .or(warp::post().and(path!("api"/"players"/Uuid/"ban")).and(with_auth(data.clone(), "ban-player")).and(super::with_data(data.clone())).and(json::<PlayerBan>()).and_then(ban_player))

    //.or(warp::patch().and(path!("api"/"players"/Uuid)).and(with_auth(data.clone(), "patch-online-player")).and(super::with_data(data.clone())).and(json::<PlayerPatch>()).and_then(patch_player))
}


#[instrument(skip(data), level = "debug")]
async fn get_online(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    Ok(reply::json(&data.db.select_online_players_reduced_info().await.map_err(ApiError::from)?).into_response())
}

// #[derive(Debug, Serialize, Deserialize)]
// struct PlayerPatch {
//     currency: Option<i32>,
//     premium_currency: Option<i32>,
//     blocked: Option<Vec<Uuid>>,
//     // friend_policy: Option<FriendPoly>,
//     friends: Option<Vec<Uuid>>,
//     groups: Option<Vec<String>>,
//     suffix: Option<String>,
//     prefix: Option<String>,
//     locale: Option<String>,
//     ban: Option<PlayerBan>,
//
//     disconnected: Option<bool>,
//     server: Option<Uuid>,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// struct PlayerBan {
//     reason: String,
//     duration: Option<i32>,
//     issuer: Option<Uuid>,
//     #[serde(default)]
//     ip: bool
// }
//
// #[instrument(skip(data), level = "debug")]
// async fn patch_player(uuid: Uuid, data: Arc<AppData>, patch: PlayerPatch) -> Result<impl Reply, Rejection> {
//     let info = match data.db.select_full_player_info(&uuid).await.map_err(ApiError::from)? {
//         None => return Ok(StatusCode::NOT_FOUND.into_response()),
//         Some(info) => info
//     };
//
//
//
//     data.db.update_full_player_info(info)
//
//
//
//
//     Ok(reply::reply().into_response())
// }


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
    } else {
        if request.unban {
            //TODO
            return Ok(reply().into_response());
        } else {
            let duration = request.duration.map(|t| Duration::seconds(t as i64));
            data.db.insert_ban(&uuid, request.reason.as_ref(), request.issuer.as_ref(), duration.as_ref()).await.map_err(ApiError::from)?;
        }
    }

    if let Some(proxy) = data.db.select_online_player_proxy(&uuid).await.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::DisconnectPlayer { proxy, player: uuid }).await.map_err(ApiError::from)?;
    }


    Ok(reply().into_response())
}


