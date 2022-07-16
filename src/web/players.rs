use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use chrono::Duration;
use reqwest::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::{AppData, Database};
use tracing::instrument;
use uuid::{Uuid};
use warp::body::json;
use crate::web::rejections::ApiError;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use crate::database::DatabaseError;
use crate::database::servers::ServerKind;
use crate::kubernetes::autoscale;
use crate::kubernetes::autoscale::Autoscale;
use crate::log::debug;
use crate::messenger::servers_events::ServerEvent;
use async_recursion::async_recursion;


pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"players"/Uuid/"stats")).and(with_auth(data.clone(), "player-stats")).and(with_data(data.clone())).and(json::<PlayerStats>()).and_then(post_stats)
        .or(warp::post().and(path!("api"/"players"/Uuid/"move")).and(with_auth(data.clone(), "move-player")).and(with_data(data.clone()).and(json::<PlayerMove>())).and_then(move_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"ban")).and(with_auth(data.clone(), "ban-player")).and(with_data(data.clone())).and(json::<PlayerBan>()).and_then(ban_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"mute")).and(with_auth(data.clone(), "mute-player")).and(with_data(data.clone())).and(json::<PlayerMute>()).and_then(mute_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"disconnect")).and(with_auth(data.clone(), "disconnect-player")).and(with_data(data.clone())).and_then(disconnect_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"transaction")).and(with_auth(data.clone(), "player-transaction")).and(with_data(data.clone())).and(json::<PlayerTransaction>()).and_then(player_transaction))
        .or(warp::get().and(path!("api"/"players")).and(with_auth(data.clone(), "get-online-players")).and(with_data(data.clone())).and_then(get_online))
        .or(warp::get().and(path!("api"/"players"/String)).and(with_auth(data.clone(), "get-player")).and(with_data(data.clone())).and_then(get_player))
        .or(warp::post().and(path!("api"/"players"/Uuid/"groups"/"update")).and(with_auth(data.clone(), "update-player-groups")).and(with_data(data.clone())).and(json::<PlayerGroupsUpdate>()).and_then(update_player_groups))
        .or(warp::post().and(path!("api"/"players"/Uuid/"inventory"/"transaction")).and(with_auth(data.clone(), "player-inventory-transaction")).and(with_data(data.clone())).and(json::<PlayerInventoryTransaction>()).and_then(player_inventory_transaction))
}


#[derive(Deserialize, Serialize, Debug)]
struct PlayerStats {
    server: Uuid,
    session: Uuid,
    game_kind: Option<String>,
    stats: HashMap<String, i32>,
}


#[instrument(skip(data, request))]
async fn post_stats(uuid: Uuid, data: Arc<AppData>, request: PlayerStats) -> Result<impl Reply, Rejection> {
    let kind = match data.db.select_server_kind(&request.server).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    data.db.insert_stats(&uuid, &request.session, &request.server, &kind, &request.stats, Some(request.game_kind.as_ref().unwrap_or(&kind))).await.map_err(ApiError::from)?;

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

#[derive(Debug, Serialize, Deserialize)]
enum PlayerMoveResponse {
    Ok,
    Failed,
    PlayerOffline,
    MissingServer,
    MissingServerKind,
    UnlinkedPlayer
}

#[instrument(skip(data), level = "info")]
async fn move_player(uuid: Uuid, data: Arc<AppData>, request: PlayerMove) -> Result<impl Reply, Rejection> {
    let (proxy, discord) = data.db.select_online_player_proxy_and_discord(&uuid).await.map_err(ApiError::from)?;
    let proxy = match proxy {
        None => return Ok(reply::json(&PlayerMoveResponse::PlayerOffline).into_response()),
        Some(t) => t
    };

    if discord.is_none(){
        return Ok(reply::json(&PlayerMoveResponse::UnlinkedPlayer).into_response())
    }


    debug!("Got player proxy {}", proxy);

    match request {
        PlayerMove::Server { server, admin_move } => {
            if data.db.select_server_kind(&server).await.map_err(ApiError::from)?.is_none() {
                return Ok(reply::json(&PlayerMoveResponse::MissingServer).into_response());
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
            let srv_kind = match data.db.select_server_kind_object(&kind).await.map_err(ApiError::from)? {
                None => return Ok(reply::json(&PlayerMoveResponse::MissingServerKind).into_response()),
                Some(kind) => kind
            };
            debug!("Got server kind {:?}", srv_kind);

            if !move_player_to_server_kind(data.clone(), &uuid, &srv_kind).await? {
                return Ok(reply::json(&PlayerMoveResponse::Failed).into_response());
            }
        }
    }


    Ok(reply::json(&PlayerMoveResponse::Ok).into_response())
}


pub async fn move_player_to_server_kind(data: Arc<AppData>, uuid: &Uuid, kind: &ServerKind) -> Result<bool, Rejection> {
    let servers = data.db.select_all_servers_by_kind(&kind.name).await.map_err(ApiError::from)?;
    debug!("Got servers with kind {}", kind.name);
    let autoscale = kind.autoscale.as_ref().map(|t| serde_json::from_str::<Autoscale>(t)).transpose().map_err(ApiError::from)?;
    debug!("Got autoscale for kind {}", kind.name);

    for srv in servers {
        if srv.state != "Waiting" && srv.state != "Idle" {
            continue;
        }
        if srv.properties.is_some() && srv.properties.as_ref().unwrap().contains_key("host") {
            continue;
        }
        let slots = if let Some(slots) = srv.properties.as_ref().map(|t| t.get("slots")).flatten().map(|t| t.parse::<i64>()).transpose().map_err(ApiError::from)? {
            slots
        } else if let Some(Autoscale::Simple { slots, .. }) = &autoscale {
            *slots
        } else {
            100
        };

        let players = data.db.select_player_count_by_server(&srv.id).await.map_err(ApiError::from)?;
        if players >= slots {
            continue;
        }

        let proxy = match data.db.select_online_player_proxy(&uuid).await.map_err(ApiError::from)? {
            None => return Ok(false),
            Some(proxy) => proxy
        };

        data.msgr.send_event(&ServerEvent::MovePlayer {
            proxy,
            server: srv.id.clone(),
            player: uuid.clone(),
        }).await.map_err(ApiError::from)?;

        return Ok(true);
    }

    if let Some(autoscale) = autoscale {
        if data.db.select_all_players_with_waiting_move_to(&kind.name, 1).await.map_err(ApiError::from)?.len() == 1 {
            debug!("Server already scaling : {}", kind.name);
            data.db.set_player_waiting_move_to(uuid, &kind.name).await.map_err(ApiError::from)?;
            return Ok(true)
        };

        autoscale::create_autoscale_server(data.clone(), &kind, &autoscale).await.map_err(ApiError::from)?;
        data.db.set_player_waiting_move_to(uuid, &kind.name).await.map_err(ApiError::from)?;

        Ok(true)
    } else {
        Ok(false)
    }
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

#[derive(Debug, Serialize, Deserialize)]
struct BanIpResult{
    players: Vec<Uuid>,
    ips: Vec<IpAddr>
}

#[instrument(skip(data))]
async fn ban_player(uuid: Uuid, data: Arc<AppData>, request: PlayerBan) -> Result<impl Reply, Rejection> {
    if request.ip {
        if request.unban {
            let ban = match match data.db.select_player_info(&uuid).await.map_err(ApiError::from)? {
                None => return Ok(StatusCode::NOT_FOUND.into_response()),
                Some(info) => info
            }.ban {
                None => return Ok(StatusCode::OK.into_response()),
                Some(ban) => ban
            };

            let players = data.db.select_players_from_ban(&ban).await.map_err(ApiError::from)?;
            let ips = data.db.select_ips_from_ban(&ban).await.map_err(ApiError::from)?;

            for player in &players{
                data.db.remove_player_ban(player).await.map_err(ApiError::from)?;
            }
            for ip in &ips {
                data.db.remove_ip_ban(&ip).await.map_err(ApiError::from)?;
            }

            return Ok(reply::json(&BanIpResult{ players, ips }).into_response());
        } else {
            let mut players = vec![uuid];
            let mut ips = Vec::new();

            ip_ban_recursive(uuid,  &mut ips, &mut players, &data.db).await.map_err(ApiError::from)?;

            let duration = request.duration.map(|t| Duration::seconds(t as i64));
            let reason = Some(request.reason.map(|r| format!("IPBan : {}", r)).unwrap_or("IPBan".to_string()));
            let ban_id = data.db.insert_ban_log(duration.as_ref(), Some(&uuid), None, request.issuer.as_ref(), reason.as_ref()).await.map_err(ApiError::from)?;
            for ip in &ips {
                data.db.insert_ip_ban_with_log(ip, reason.as_ref(), request.issuer.as_ref(), duration.as_ref(), false, &ban_id).await.map_err(ApiError::from)?;
            }

            for player in &players {
                data.db.insert_ban_with_log(player, reason.as_ref(), request.issuer.as_ref(), duration.as_ref(),&ban_id).await.map_err(ApiError::from)?;

                if let Some(proxy) = data.db.select_online_player_proxy(&uuid).await.map_err(ApiError::from)? {
                    data.msgr.send_event(&ServerEvent::DisconnectPlayer { proxy, player: uuid }).await.map_err(ApiError::from)?;
                }
            }


            return Ok(reply::json(&BanIpResult{ players, ips }).into_response());
        }
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

#[derive(Debug, Serialize, Deserialize)]
struct PlayerMute {
    duration: Option<i32>,
    reason: Option<String>,
    issuer: Option<Uuid>,
    #[serde(default)]
    unmute: bool,
}

#[instrument(skip(data))]
async fn mute_player(uuid: Uuid, data: Arc<AppData>, request: PlayerMute) -> Result<impl Reply, Rejection> {
    if request.unmute {
        data.db.remove_player_mute(&uuid).await.map_err(ApiError::from)?;
        return Ok(reply().into_response());
    } else {
        let duration = request.duration.map(|t| Duration::seconds(t as i64));
        data.db.insert_mute(&uuid, request.reason.as_ref(), request.issuer.as_ref(), duration.as_ref()).await.map_err(ApiError::from)?;
    }

    if let Some(server) = data.db.select_online_player_server(&uuid).await.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::InvalidatePlayer { server, uuid }).await.map_err(ApiError::from)?;
    }


    Ok(reply().into_response())
}

#[async_recursion]
pub async fn ip_ban_recursive(uuid: Uuid, ips: &mut Vec<IpAddr>, players: &mut Vec<Uuid>, db: &Database) -> Result<(), DatabaseError>{
    let player_ips = db.select_all_player_sessions_ips(&uuid).await?;

    for ip in &player_ips {
        if ips.contains(ip){
            continue
        }else {
            ips.push(ip.clone());
            let new_players = db.select_players_with_session_ip(ip).await?;

            for new_player in new_players {
                if !players.contains(&new_player) {
                    players.push(new_player);
                    ip_ban_recursive(new_player, ips, players, db).await?;
                }
            }
        }
    }


    Ok(())

}

#[instrument(skip(data))]
async fn get_online(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    Ok(reply::json(&data.db.select_online_players_reduced_info().await.map_err(ApiError::from)?).into_response())
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
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerTransaction {
    #[serde(default)]
    currency: i32,
    #[serde(default)]
    premium_currency: i32,
}

#[instrument(skip(data))]
async fn player_transaction(uuid: Uuid, data: Arc<AppData>, request: PlayerTransaction) -> Result<impl Reply, Rejection> {
    if let Some((currency, premium_currency)) = data.db.select_player_currencies(&uuid).await.map_err(ApiError::from)? {
        if currency + request.currency >= 0 && premium_currency + request.premium_currency >= 0 {
            data.db.set_player_currencies(&uuid, currency + request.currency, premium_currency + request.premium_currency).await.map_err(ApiError::from)?;
            if let Some(server) = data.db.select_online_player_server(&uuid).await.map_err(ApiError::from)? {
                data.msgr.send_event(&ServerEvent::InvalidatePlayer { server, uuid }).await.map_err(ApiError::from)?;
            }
            Ok(reply::json(&true).into_response())
        } else {
            Ok(reply::json(&false).into_response())
        }
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}

pub type PlayerGroupsUpdate = Vec<String>;

#[instrument(skip(data))]
async fn update_player_groups(uuid: Uuid, data: Arc<AppData>, request: PlayerGroupsUpdate) -> Result<impl Reply, Rejection> {
    if data.db.select_player_info(&uuid).await.map_err(ApiError::from)?.is_none() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    for x in &request {
        if x.starts_with("-") {
            data.db.remove_player_group(&uuid, &x).await.map_err(ApiError::from)?;
        } else {
            let (name, duration) = if x.contains("/") {
                let split: Vec<&str> = x.split("/").collect();
                (split[0].to_string(), split[1].parse::<i32>().map_err(ApiError::from)?)
            } else {
                (x.to_string(), -1)
            };

            if duration <= 0 {
                data.db.add_player_group(&uuid, &name).await.map_err(ApiError::from)?;
            } else {
                data.db.add_player_group_ttl(&uuid, &name, duration).await.map_err(ApiError::from)?;
            }
        }
    }

    if !request.is_empty() {
        if let Some(server) = data.db.select_online_player_server(&uuid).await.map_err(ApiError::from)? {
            data.msgr.send_event(&ServerEvent::InvalidatePlayer { server, uuid }).await.map_err(ApiError::from)?;
        }
    }

    Ok(StatusCode::OK.into_response())
}

pub type PlayerInventoryTransaction = HashMap<String, i32>;

#[instrument(skip(data))]
async fn player_inventory_transaction(uuid: Uuid, data: Arc<AppData>, request: PlayerInventoryTransaction) -> Result<impl Reply, Rejection> {
    let inv = match data.db.select_player_inventory(&uuid).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(inv) => inv
    };

    for (item, count) in &request {
        if *inv.get(item).unwrap_or(&0) + count < 0 {
            return Ok(reply::json(&false).into_response());
        }
    }

    for (item, count) in &request {
        data.db.set_player_inventory_item(&uuid, item, *inv.get(item.as_str()).unwrap_or(&0) + count).await.map_err(ApiError::from)?;
    }

    if !request.is_empty() {
        if let Some(server) = data.db.select_online_player_server(&uuid).await.map_err(ApiError::from)? {
            data.msgr.send_event(&ServerEvent::InvalidatePlayer { server, uuid }).await.map_err(ApiError::from)?;
        }
    }

    Ok(reply::json(&true).into_response())
}