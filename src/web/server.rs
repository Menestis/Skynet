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
use crate::kubernetes::autoscale;
use crate::kubernetes::servers::generate_server_name;
use crate::web::rejections::ApiError;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"servers")).and(with_auth(data.clone(), "create-server")).and(with_data(data.clone())).and(json::<CreateServer>()).and_then(create_server)
        .or(warp::delete().and(path!("api"/"servers"/String)).and(with_auth(data.clone(), "delete-server")).and(with_data(data.clone())).and_then(delete_server))
        .or(warp::get().and(path!("api"/"servers")).and(with_auth(data.clone(), "get-all-servers")).and(with_data(data.clone())).and_then(get_all_servers))
        .or(warp::post().and(path!("api"/"servers"/Uuid/"setstate")).and(with_auth(data.clone(), "set-server-state")).and(json::<String>()).and(with_data(data.clone())).and_then(set_server_state))
}


#[derive(Deserialize, Serialize, Debug)]
pub struct CreateServer {
    pub kind: String,
    pub name: String,
    pub properties: Option<HashMap<String, String>>,
}

#[instrument(skip(data))]
pub async fn create_server(data: Arc<AppData>, request: CreateServer) -> Result<impl Reply, Rejection> {
    let kind = match data.db.select_server_kind_object(&request.kind).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    let name = generate_server_name(&request.kind, &request.name);
    data.k8s.create_pod(&kind.name, &kind.image,
                        &name,
                        request.properties.unwrap_or_default(), HashMap::new()).await.map_err(ApiError::from)?;

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
async fn set_server_state(uuid: Uuid, state: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    data.db.update_server_state(&uuid, &state).await.map_err(ApiError::from)?;

    if state == "Idle" {
        autoscale::process_idling(data.clone(), uuid).await.map_err(ApiError::from)?;
    } else if state == "Waiting" {
        autoscale::process_waiting(data.clone(), uuid).await.map_err(ApiError::from)?;
    }
    Ok(reply::reply().into_response())
}


