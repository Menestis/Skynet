use std::collections::HashMap;
use std::sync::Arc;
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
    warp::post().and(path!("api"/"servers")).and(with_auth(data.clone(), "create-server")).and(with_data(data.clone())).and(json::<CreateServer>()).and_then(create_server)
        .or(warp::delete().and(path!("api"/"servers"/String)).and(with_auth(data.clone(), "delete-server")).and(with_data(data.clone())).and_then(delete_server))
        .or(warp::get().and(path!("api"/"servers")).and(with_auth(data.clone(), "get-all-servers")).and(with_data(data.clone())).and_then(get_all_servers))
    //TODO get server
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




