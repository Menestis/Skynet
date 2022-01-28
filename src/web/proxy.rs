use std::sync::Arc;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
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
    let settings = data.db.get_cached_settings();

    Ok(reply::json(&ProxyPingResponse {
        slots: settings.get("slots").map(|s| s.parse::<i32>()).transpose().map_err(ApiError::from)?.unwrap_or_default(),
        motd: settings.get("motd").map(|t| t.clone()).unwrap_or("".to_string()),
    }))
}