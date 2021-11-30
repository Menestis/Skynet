use std::convert::Infallible;
use std::env::var;
use std::future::Future;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use warp::{Filter, path, Rejection, Reply, reply, Server};
use warp::reject::Reject;
use warp::reply::{json, Json};
use crate::AppData;
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tokio::sync::watch::Receiver;
use tracing::instrument;
use uuid::Uuid;
use warp::hyper::body::HttpBody;
use crate::web::rejections::{ApiError, handle_rejection};
use tracing::*;

mod rejections;
mod status;
mod login;

pub async fn create_task(addr: SocketAddr, data: Arc<AppData>) -> impl Future<Output=()> {
    let mut r = data.shutdown_receiver.clone();

    let routes = status::filter(data.clone())
        .or(login::filter(data.clone()))
        .or(shutdown_filter(data.clone()))
        .with(warp::log("requests"))
        .recover(handle_rejection);

    warp::serve(routes).bind_with_graceful_shutdown(addr, async move { let _ = r.changed().await; }).1
}

fn with_auth(data: Arc<AppData>, permission: &'static str) -> impl Filter<Extract=(), Error=Rejection> + Clone {
    warp::header::<Uuid>("Authorization").and(with_data(data.clone())).map(|uuid, data| (uuid, permission.to_string(), data)).untuple_one().and_then(check_authorization).untuple_one()
}

async fn check_authorization(key: Uuid, permission: String, data: Arc<AppData>) -> Result<(), Rejection> {
    if data.db.has_route_permission(key, permission).await.map_err(|e| ApiError::Database(e))? {
        Ok(())
    } else {
        Err(ApiError::Authorization.into())
    }
}


#[derive(Debug, Serialize)]
pub struct JsonMessage {
    message: String,
}

pub fn json_reply(message: String) -> Json {
    json(&JsonMessage { message })
}

pub fn with_data(data: Arc<AppData>) -> impl Filter<Extract=(Arc<AppData>, ), Error=Infallible> + Clone {
    warp::any().map(move || data.clone())
}


fn shutdown_filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"shutdown")).and(with_auth(data.clone(), "shutdown")).and(with_data(data.clone())).and_then(shutdown)
}

async fn shutdown(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    data.shutdown_sender.send(()).await.map_err(|e| ApiError::Channel(e))?;
    Ok(reply())
}