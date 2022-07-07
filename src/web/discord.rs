use std::sync::Arc;
use rand::{Rng, thread_rng};
use serde::Serialize;
use tokio::join;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::{instrument};
use uuid::Uuid;
use warp::body::json;
use warp::http::StatusCode;
use crate::messenger::servers_events::ServerEvent;
use crate::structures::discord::Message;
use crate::web::{with_auth, with_data};
use crate::web::rejections::ApiError;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::get().and(path!("api"/"discord"/"link"/Uuid)).and(with_auth(data.clone(), "create-discord-link")).and(with_data(data.clone())).and_then(create_link)
        .or(warp::post().and(path!("api"/"discord"/"link"/String)).and(with_auth(data.clone(), "complete-discord-link")).and(json::<String>()).and(with_data(data.clone())).and_then(complete_link))
        .or(warp::delete().and(path!("api"/"discord"/"link"/String)).and(with_auth(data.clone(), "delete-discord-link")).and(with_data(data.clone())).and_then(delete_link))
        .or(warp::post().and(path!("api"/"discord"/"webhook"/String)).and(with_auth(data.clone(), "webhook")).and(json::<String>()).and(with_data(data.clone())).and_then(call_webhook))
}


#[instrument(skip(data))]
async fn create_link(uuid: Uuid, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    if let None = data.db.select_player_username(&uuid).await.map_err(ApiError::from)? {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let x: i32 = thread_rng().gen_range(1000..9999);
    data.db.insert_discord_link(&x.to_string(), &uuid).await.map_err(ApiError::from)?;
    Ok(reply::json(&x.to_string()).into_response())
}

#[derive(Serialize, Debug)]
pub struct Leaderboard {
    label: String,
    leaderboard: Vec<String>,
}


#[instrument(skip(data))]
async fn delete_link(discord: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let uuid = match data.db.select_player_uuid_by_discord(&discord).await.map_err(ApiError::from)? {
        None => return Ok(reply().into_response()),
        Some(uuid) => uuid
    };
    let (proxy, server) = join!(data.db.select_online_player_proxy(&uuid), data.db.select_online_player_server(&uuid));

    if let Some(proxy) = proxy.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::InvalidatePlayer { server: proxy, uuid }).await.map_err(ApiError::from)?;
    };
    if let Some(server) = server.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::InvalidatePlayer { server, uuid }).await.map_err(ApiError::from)?;
    };

    Ok(warp::reply().into_response())
}

#[instrument(skip(data))]
async fn complete_link(link: String, discord: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let uuid = match data.db.select_discord_link(&link).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(uuid) => uuid
    };
    data.db.delete_discord_link(&link).await.map_err(ApiError::from)?;
    data.db.set_player_discord_id(&uuid, Some(&discord)).await.map_err(ApiError::from)?;

    let (proxy, server) = join!(data.db.select_online_player_proxy(&uuid), data.db.select_online_player_server(&uuid));

    if let Some(proxy) = proxy.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::InvalidatePlayer { server: proxy, uuid }).await.map_err(ApiError::from)?;
    };
    if let Some(server) = server.map_err(ApiError::from)? {
        data.msgr.send_event(&ServerEvent::InvalidatePlayer { server, uuid }).await.map_err(ApiError::from)?;
    };

    Ok(warp::reply().into_response())
}


#[instrument(skip(data))]
async fn call_webhook(webhook: String, msg: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let webhook = match data.db.select_webhook(&webhook).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(url) => url
    };

    let message = match serde_json::from_str(&msg) {
        Ok(msg) => msg,
        Err(_err) => {
            let mut message = Message::new();
            message.content(&msg);
            message
        }
    };

    data.client.post(webhook).json(&message).send().await.map_err(ApiError::from)?;

    Ok(warp::reply().into_response())
}