use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use warp::{Filter, path, Rejection, Reply, reply};
use warp::reply::{json, Json};
use crate::AppData;
use serde::Serialize;
use uuid::Uuid;
use crate::web::rejections::{ApiError, handle_rejection};
use tracing::*;

pub mod rejections;
pub mod login;
pub mod status;
pub mod sessions;
pub mod proxy;
pub mod server;
pub mod registration;
pub mod players;

pub async fn create_task(addr: SocketAddr, data: Arc<AppData>) -> impl Future<Output=()> {
    let mut r = data.shutdown_receiver.clone();

    let routes = documentation_filter()
        .or(shutdown_filter(data.clone()))
        .or(login::filter(data.clone()))
        .or(sessions::filter(data.clone()))
        .or(proxy::filter(data.clone()))
        .or(server::filter(data.clone()))
        .or(registration::filter(data.clone()))
        .or(players::filter(data.clone()))
        .or(status::filter(data.clone()))
        .recover(handle_rejection);

    warp::serve(routes).bind_with_graceful_shutdown(addr, async move { let _ = r.changed().await; }).1
}

fn with_auth(data: Arc<AppData>, permission: &'static str) -> impl Filter<Extract=(), Error=Rejection> + Clone {
    warp::header::<String>("Authorization").and(with_data(data.clone())).map(|uuid, data| (uuid, permission.to_string(), data)).untuple_one().and_then(check_authorization).untuple_one()
}

#[instrument(skip(data))]
async fn check_authorization(key: String, permission: String, data: Arc<AppData>) -> Result<(), Rejection> {
    if key.starts_with("Server ") {
        let key = Uuid::parse_str(key.trim_start_matches("Server ")).map_err(ApiError::from)?;

        if data.db.get_cached_api_group("server").map(|grp| grp.permissions.as_ref()).flatten().map(|perms| perms.contains(&permission)).unwrap_or(false)
            && data.db.select_server_kind_by_key(&key).await.map_err(ApiError::from)?.map(|t| t != "proxy").unwrap_or_default() {
            Ok(())
        } else {
            Err(ApiError::Authorization.into())
        }
    } else if key.starts_with("Proxy ") {
        let key = Uuid::parse_str(key.trim_start_matches("Proxy ")).map_err(ApiError::from)?;
        if data.db.get_cached_api_group("proxy").map(|grp| grp.permissions.as_ref()).flatten().map(|perms| perms.contains(&permission)).unwrap_or(false)
            && data.db.select_server_kind_by_key(&key).await.map_err(ApiError::from)?.map(|t| t == "proxy").unwrap_or_default() {
            Ok(())
        } else {
            Err(ApiError::Authorization.into())
        }
    } else {
        let key = Uuid::parse_str(&key).map_err(ApiError::from)?;
        if data.db.has_route_permission(key, &permission).await.map_err(ApiError::Database)? {
            Ok(())
        } else {
            Err(ApiError::Authorization.into())
        }
    }
}


#[derive(Debug, Serialize)]
pub struct JsonMessage {
    message: String,
}

pub fn _json_reply(message: String) -> Json {
    json(&JsonMessage { message })
}

pub fn with_data(data: Arc<AppData>) -> impl Filter<Extract=(Arc<AppData>, ), Error=Infallible> + Clone {
    warp::any().map(move || data.clone())
}


fn shutdown_filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"shutdown")).and(with_auth(data.clone(), "shutdown")).and(with_data(data.clone())).and_then(shutdown)
}

#[instrument(skip(data))]
async fn shutdown(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    data.shutdown_sender.send(()).await.map_err(ApiError::Channel)?;
    Ok(reply())
}



fn documentation_filter() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::get().and(path!("api")).and_then(documentation)
}

async fn documentation() -> Result<impl Reply, Rejection> {
    Ok(reply::html(include_str!("../../docs/web/index.html")))
}
