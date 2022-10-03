use std::collections::HashMap;
use std::sync::Arc;
use reqwest::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::*;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use warp::body::json;
#[cfg(feature = "kubernetes")]
use crate::kubernetes::autoscale;
#[cfg(feature = "kubernetes")]
use crate::kubernetes::servers::generate_server_name;
use crate::messenger::servers_events::ServerEvent;
use crate::web::rejections::ApiError;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"servers")).and(with_auth(data.clone(), "create-server")).and(with_data(data.clone())).and(json::<CreateServer>()).and_then(create_server)
        .or(warp::delete().and(path!("api"/"servers"/String)).and(with_auth(data.clone(), "delete-server")).and(with_data(data.clone())).and_then(delete_server))
        .or(warp::get().and(path!("api"/"servers")).and(with_auth(data.clone(), "get-all-servers")).and(with_data(data.clone())).and_then(get_all_servers))
        .or(warp::get().and(path!("api"/"onlinecount")).and(with_auth(data.clone(), "get-onlinecount")).and(with_data(data.clone())).and_then(get_onlinecount))
        .or(warp::post().and(path!("api"/"servers"/Uuid/"setstate")).and(with_auth(data.clone(), "set-server-state")).and(json::<String>()).and(with_data(data.clone())).and_then(set_server_state))
        .or(warp::post().and(path!("api"/"servers"/"broadcast")).and(with_auth(data.clone(), "broadcast")).and(json::<Broadcast>()).and(with_data(data.clone())).and_then(broadcast))
        .or(warp::post().and(path!("api"/"servers"/Uuid/"setdescription")).and(with_auth(data.clone(), "set-server-description")).and(json::<String>()).and(with_data(data.clone())).and_then(set_server_description))
        .or(warp::post().and(path!("api"/"servers"/Uuid/"playercount")).and(with_auth(data.clone(), "server-update-playercount")).and(json::<i32>()).and(with_data(data.clone())).and_then(update_playercount))
}


#[derive(Deserialize, Serialize, Debug)]
pub struct CreateServer {
    pub kind: String,
    pub name: String,
    pub properties: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ServerStartup {
    #[serde(default)]
    properties: HashMap<String, String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[instrument(skip(data))]
pub async fn create_server(data: Arc<AppData>, request: CreateServer) -> Result<impl Reply, Rejection> {
    let kind = match data.db.select_server_kind_object(&request.kind).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    let mut startup = match kind.startup {
        None => Ok(ServerStartup::default()),
        Some(startup) => serde_json::from_str::<ServerStartup>(&startup).map_err(ApiError::from),
    }?;

    for (k, v) in request.properties.unwrap_or_default() {
        startup.properties.insert(k, v);
    }

    let name = generate_server_name(&request.kind, &request.name);
    data.k8s.create_pod(&kind.name, &kind.image,
                        &name,
                        startup.properties, startup.env).await.map_err(ApiError::from)?;

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

#[instrument(skip(data))]
async fn get_onlinecount(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    Ok(reply::json(&data.db.select_setting("online_count").await.map_err(ApiError::from)?))
}


//["Idle", "Waiting", "Starting", "Playing"]
#[instrument(skip(data))]
async fn set_server_state(uuid: Uuid, state: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    data.db.update_server_state(&uuid, &state).await.map_err(ApiError::from)?;

    if state == "Idle" {
        autoscale::process_idling(data.clone(), uuid).await.map_err(ApiError::from)?;
    } else if state == "Waiting" {
        autoscale::process_waiting(data.clone(), uuid).await.map_err(ApiError::from)?;
    }
    data.msgr.send_event(&ServerEvent::ServerStateUpdate { server: uuid, state }).await.map_err(ApiError::from)?;
    Ok(reply::reply().into_response())
}

#[instrument(skip(data))]
async fn set_server_description(uuid: Uuid, description: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    data.db.update_server_description(&uuid, &description).await.map_err(ApiError::from)?;
    data.msgr.send_event(&ServerEvent::ServerDescriptionUpdate { server: uuid, description }).await.map_err(ApiError::from)?;

    Ok(reply::reply().into_response())
}


#[derive(Deserialize, Serialize, Debug)]
pub struct Broadcast {
    message: String,
    permission: Option<String>,
    server_kind: Option<String>,
}

#[instrument(skip(data))]
async fn broadcast(broadcast: Broadcast, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    data.msgr.send_event(&ServerEvent::Broadcast {
        message: broadcast.message,
        permission: broadcast.permission,
        server_kind: broadcast.server_kind,
    }).await.map_err(ApiError::from)?;

    Ok(reply::reply().into_response())
}


#[instrument(skip(data))]
async fn update_playercount(proxy: Uuid, count: i32, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    data.db.update_server_playercount(&proxy, count).await.map_err(ApiError::from)?;
    data.msgr.send_event(&ServerEvent::ServerCountUpdate { server: proxy, count }).await.map_err(ApiError::from)?;

    Ok(reply())
}