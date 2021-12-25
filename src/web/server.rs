use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use futures::TryFutureExt;
use rand::Rng;
use reqwest::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::*;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use warp::body::json;
use crate::web::rejections::ApiError;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"procedure"/"server"/"login")).and(with_auth(data.clone(), "server-login")).and(with_data(data.clone())).and(json::<LoginRequest>()).and_then(login)
        .or(warp::post().and(path!("api"/"procedure"/"player"/"stats")).and(with_auth(data.clone(), "player-stats")).and(with_data(data.clone())).and(json::<PlayerStats>()).and_then(post_stats))
        .or(warp::post().and(path!("api"/"servers")).and(with_auth(data.clone(), "create-server")).and(with_data(data.clone())).and(json::<CreateServer>()).and_then(create_server))
        .or(warp::delete().and(path!("api"/"servers"/String)).and(with_auth(data.clone(), "delete-server")).and(with_data(data.clone())).and_then(delete_server))
        .or(warp::get().and(path!("api"/"servers")).and(with_auth(data.clone(), "get-all-servers")).and(with_data(data.clone())).and_then(get_all_servers))
}

#[derive(Deserialize, Debug)]
struct LoginRequest {
    uuid: Uuid,
    server: Uuid,
}

#[derive(Serialize, Debug)]
pub struct ServerLoginPlayerInfo {
    pub session: Uuid,
    pub proxy: Uuid,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub locale: String,
    pub permissions: Vec<String>,
    pub power: i32,
    pub currency: i32,
    pub premium_currency: i32,
    pub blocked: Vec<Uuid>,
    pub inventory: HashMap<String, i32>,
    pub properties: HashMap<String, String>,
}


#[instrument(skip(data))]
async fn login(data: Arc<AppData>, request: LoginRequest) -> Result<impl Reply, Rejection> {
    let kind = match data.db.select_server_kind(&request.server).await.map_err(ApiError::from)?.map(|kind| data.db.get_cached_kind(&kind)).flatten() {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };
    let player = match data.db.select_server_player_info(&request.uuid).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    let info = player.build_server_login_player_info(&data.db, &kind.name);

    data.db.update_player_online_sever_info(&request.uuid, request.server).await.map_err(ApiError::from)?;

    Ok(reply::json(&info).into_response())
}

#[derive(Deserialize, Serialize, Debug)]
struct PlayerStats {
    player: Uuid,
    session: Uuid,
    server: Uuid,
    stats: HashMap<String, i32>,
}


#[instrument(skip(data, request))]
async fn post_stats(data: Arc<AppData>, request: PlayerStats) -> Result<impl Reply, Rejection> {
    let kind = match data.db.select_server_kind(&request.server).await.map_err(ApiError::from)?.map(|kind| data.db.get_cached_kind(&kind)).flatten() {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    data.db.insert_stats(&request.player, &request.session, &kind.name, &request.stats).await.map_err(ApiError::from)?;

    Ok(reply().into_response())
}


#[derive(Deserialize, Serialize, Debug)]
struct CreateServer {
    kind: String,
    name: String,
    properties: Option<HashMap<String, String>>,
}


#[instrument(skip(data))]
async fn create_server(data: Arc<AppData>, request: CreateServer) -> Result<impl Reply, Rejection> {
    let kind = match data.db.get_cached_kind(&request.kind) {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    let name = format!("{}-{}-{}", kind.name, request.name, rand::thread_rng().gen_range(10000..99999));
    data.k8s.create_pod(&kind.name, &kind.image,
                        &name,
                        request.properties.unwrap_or_default()).await.map_err(ApiError::from)?;

    Ok(reply::json(&name).into_response())
}


#[instrument(skip(data))]
async fn delete_server_id(id: Uuid, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let (label, properties) = match data.db.select_server_label_and_properties(&id).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(srv) => srv,
    };
    if properties.get("protected").map(|t| t == "true").unwrap_or_default() {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }
    data.k8s.delete_pod(&label).await.map_err(ApiError::from)?;
    return Ok(reply().into_response());
}

#[instrument(skip(data))]
async fn delete_server(server: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    return match match Uuid::parse_str(&server) {
        Ok(uuid) => {
            data.db.select_server_label(&uuid).await.map_err(ApiError::from)?
        }
        Err(_) => {
            data.k8s.delete_pod(&server).await.map_err(ApiError::from)?;
            return Ok(reply().into_response());
        }
    } {
        None => {
            Ok(StatusCode::NOT_FOUND.into_response())
        }
        Some(label) => {
            data.k8s.delete_pod(&label).await.map_err(ApiError::from)?;
            Ok(reply().into_response())
        }
    };
}


#[instrument(skip(data))]
async fn get_all_servers(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    Ok(reply::json(&data.db.select_all_servers().await.map_err(ApiError::from)?))
}




