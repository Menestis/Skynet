use std::sync::Arc;
use warp::{Filter, path, Rejection, Reply};
use crate::AppData;
use tracing::*;
use crate::web::{with_auth, with_data};
use uuid::Uuid;
use warp::hyper::StatusCode;
use crate::web::echo::ECHO_URL;
use crate::web::rejections::ApiError;


pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::delete().and(path!("api"/"players"/Uuid/"session")).and(with_auth(data.clone(), "proxy-close-session")).and(with_data(data.clone())).and_then(close_session)
}


#[instrument(skip(data))]
async fn close_session(uuid: Uuid, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    if let Some(session) = data.db.select_player_session(&uuid).await.map_err(ApiError::from)? {
        data.db.close_session(&session).await.map_err(ApiError::from)?;
        data.db.close_player_session(&uuid).await.map_err(ApiError::from)?;

        if data.db.select_player_echo_enabled(&uuid).await.map_err(ApiError::from)? {
            let client = &data.client;
            info!("Disabling alpha feature echo for player {}", uuid);
            client.delete(format!("{}/players/{}", ECHO_URL, uuid)).header("Authorization", data.echo_key.to_string()).send().await.map_err(ApiError::from)?;
            data.db.set_player_echo_enabled(&uuid, false).await.map_err(ApiError::from)?;
            info!("Disabled alpha feature echo for player {}", uuid);
        }
    }

    Ok(StatusCode::OK)
}