use std::sync::Arc;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::{AppData, join};
use tracing::*;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use warp::body::json;
use crate::messenger::servers_events::ServerEvent;
use crate::web::rejections::ApiError;


pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::get().and(path!("api"/"proxy"/"ping")).and(with_auth(data.clone(), "proxy-ping")).and(with_data(data.clone())).and_then(ping)
        .or(warp::post().and(path!("api"/"proxy"/Uuid/"playercount")).and(with_auth(data.clone(), "proxy-update-playercount")).and(json::<i32>()).and(with_data(data.clone())).and_then(update_playercount))
}

#[derive(Deserialize, Serialize, Debug)]
struct ProxyPingResponse {
    online: i32,
    slots: i32,
    motd: String,
}

#[instrument(skip(data))]
async fn ping(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let (slots, motd, online) = join!(data.db.select_setting("slots"), data.db.select_setting("motd"), data.db.select_setting("online_count"));

    //TODO if maintenance montrer mode maintenance

    Ok(reply::json(&ProxyPingResponse {
        online: online.map_err(ApiError::from)?.map(|x| x.parse::<i32>()).transpose().map_err(ApiError::from)?.unwrap_or_default(),
        slots: slots.map_err(ApiError::from)?.map(|s| s.parse::<i32>()).transpose().map_err(ApiError::from)?.unwrap_or_default(),
        motd: motd.map_err(ApiError::from)?.map(|t| t.clone()).unwrap_or("".to_string()),
    }))
}

#[instrument(skip(data))]
async fn update_playercount(proxy: Uuid, count: i32, data: Arc<AppData>) -> Result<impl Reply, Rejection> {

    //if !data.k8s.is_leader() {
    if let Err(e) = data.msgr.send_event(&ServerEvent::PlayerCountSync { proxy, count}).await {
        error!("{}", e);
    }
    // } else {
    //     data.player_count.write().await.insert(request.proxy, request.online_count);
    //
    //     let online: i32 = data.player_count.read().await.values().sum();
    //
    //     if let Err(e) = data.db.insert_setting("online_count", &online.to_string()).await {
    //         error!("{}", e);
    //     }
    // }


    Ok(reply())
}