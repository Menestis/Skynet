use std::net::SocketAddr;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::{instrument, warn};
use uuid::Uuid;
use crate::messenger::servers_events::ServerEvent;
use crate::web::rejections::ApiError;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::get().and(path!("api"/"procedure"/"register"/String)).and(super::with_data(data)).and_then(register)
}


#[instrument(skip(data), level = "debug")]
async fn register(hostname: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let mut srv = match data.db.select_server_by_label(&hostname).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(srv) => srv
    };

    let key = Uuid::new_v4();
    srv.key = Some(key);

    data.db.update_server_key(&srv.id, &key).await.map_err(ApiError::from)?;
    data.db.update_server_state(&srv.id, "Started").await.map_err(ApiError::from)?;


    crate::utils::property_handler::handle_property_actions(&data, &srv).await?;


    data.msgr.send_event(&ServerEvent::ServerStarted {
        addr: srv.ip,
        id: srv.id.clone(),
        description: srv.description,
        name: srv.label.clone(),
        kind: srv.kind.clone(),
        properties: srv.properties.clone().unwrap_or_default(),
    }).await.map_err(ApiError::from)?;


    Ok(reply::json(&srv).into_response())
}
