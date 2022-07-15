use std::sync::Arc;
use itertools::Itertools;
use serde::Serialize;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::{debug, info, instrument};
use uuid::Uuid;
use warp::http::StatusCode;
use crate::messenger::servers_events::ServerEvent;
use crate::web::{with_auth, with_data};
use crate::web::rejections::ApiError;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"leaderboards")).and(with_auth(data.clone(), "generate-stats")).and(with_data(data.clone())).and_then(generate_stats)
        .or(warp::get().and(path!("api"/"leaderboards"/String)).and(with_auth(data.clone(), "get-stats")).and(with_data(data.clone())).and_then(get_stats))
}

// #[derive(Serialize, Debug)]
// pub struct Stat {
//     player: Uuid,
//     username: String,
//     value: i32,
// }

#[instrument(skip(data))]
async fn generate_stats(data: Arc<AppData>) -> Result<impl Reply, Rejection> {

    //.query_iter("SELECT player, sum(value) FROM statistics_processable WHERE timestamp > ? AND timestamp < ? AND server_kind = ?  AND key = ? GROUP BY player ALLOW FILTERING;", (rules.start, rules.end, rules.server_kind, rules.key)).await {
    let rules = data.db.select_leaderboards_rules().await.map_err(ApiError::from)?;

    info!("Processing leaderboards, found {} leaderboards to generate", rules.len());

    for (name, label, rule) in rules {
        info!("Processing {} leaderboard", name);

        let stats = if let Some(kind) = rule.game_kind {
            data.db.select_stats_kind(&rule.stat_key, &kind, rule.period.timestamp()).await
        } else {
            data.db.select_stats(&rule.stat_key, rule.period.timestamp()).await
        }.map_err(ApiError::from)?;

        let raw_leaderboard: Vec<(Uuid, i64)> = stats.into_iter().sorted_by(|&x1, &x2| x1.1.cmp(&x2.1).reverse()).take(rule.size as usize).collect();


        let mut leaderboard = Vec::new();
        for (player, value) in raw_leaderboard {
            let username = data.db.select_player_username(&player).await.map_err(ApiError::from)?;
            leaderboard.push(format!("{}:{}", username.unwrap_or("?".to_string()), value));
        }


        data.db.update_leaderboard(&name, &leaderboard).await.map_err(ApiError::from)?;
        data.msgr.send_event(&ServerEvent::InvalidateLeaderBoard { name, label, leaderboard }).await.map_err(ApiError::from)?;
    }


    Ok(warp::reply().into_response())
}

#[derive(Serialize, Debug)]
pub struct Leaderboard {
    label: String,
    leaderboard: Vec<String>,
}


#[instrument(skip(data))]
async fn get_stats(name: String, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    match data.db.select_leaderboard(&name).await.map_err(ApiError::from)? {
        None => Ok(StatusCode::NOT_FOUND.into_response()),
        Some((label, leaderboard)) => Ok(reply::json(&Leaderboard { label, leaderboard }).into_response())
    }
}
