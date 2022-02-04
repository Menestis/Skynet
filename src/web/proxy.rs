use std::sync::Arc;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::{AppData, join};
use tracing::*;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use crate::web::rejections::ApiError;


pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
        warp::get().and(path!("api"/"proxy"/"ping")).and(with_auth(data.clone(), "proxy-ping")).and(with_data(data.clone())).and_then(ping)
}

#[derive(Deserialize, Serialize, Debug)]
struct ProxyPingResponse {
    slots: i32,
    motd: String,
}

#[instrument(skip(data))]
async fn ping(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let (slots, motd) = join!(data.db.select_setting("slots"), data.db.select_setting("motd"));


    Ok(reply::json(&ProxyPingResponse {
        slots: slots.map_err(ApiError::from)?.map(|s| s.parse::<i32>()).transpose().map_err(ApiError::from)?.unwrap_or_default(),
        motd: motd.map_err(ApiError::from)?.map(|t| t.clone()).unwrap_or("".to_string()),
    }))
}