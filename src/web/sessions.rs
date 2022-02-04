use std::sync::Arc;
use warp::{Filter, path, Rejection, Reply};
use crate::AppData;
use tracing::*;
use crate::web::{with_auth, with_data};
use uuid::Uuid;
use warp::hyper::StatusCode;
use crate::web::rejections::ApiError;


pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::delete().and(path!("api"/"players"/Uuid/"session")).and(with_auth(data.clone(), "proxy-close-session")).and(with_data(data.clone())).and_then(close_session)
}


#[instrument(skip(data))]
async fn close_session(uuid: Uuid, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    if let Some(session) = data.db.select_player_session(&uuid).await.map_err(ApiError::from)? {
        data.db.close_session(&session).await.map_err(ApiError::from)?;
        data.db.close_player_session(&uuid).await.map_err(ApiError::from)?;
    }

    Ok(StatusCode::OK)
}