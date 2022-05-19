use std::collections::HashMap;
use std::sync::Arc;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::{error, info, instrument};
use uuid::Uuid;
use warp::body::json;
use crate::database::DatabaseError;
use crate::web::{with_auth, with_data};

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"stats")).and(with_auth(data.clone(), "generate-stats")).and(json::<StatsBuildingRules>()).and(with_data(data.clone())).and_then(generate_stats)
}

#[derive(Deserialize, Debug)]
pub struct StatsBuildingRules {
    start: i64,
    end: i64,
    key: String,
    server_kind: String,
}

#[derive(Serialize, Debug)]
pub struct Stat{
    player: Uuid,
    username: String,
    value: i32
}

#[instrument(skip(data))]
async fn generate_stats(rules: StatsBuildingRules, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let mut rows_stream = match data.db.session
        .query_iter("SELECT player, sum(value) FROM statistics_processable WHERE timestamp > ? AND timestamp < ? AND server_kind = ?  AND key = ? GROUP BY player ALLOW FILTERING;", (rules.start, rules.end, rules.server_kind, rules.key)).await {
        Ok(row) => { row }
        Err(err) => {
            error!("{}", err);
            return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }.into_typed::<(Uuid, i32)>();

    let mut result = Vec::new();

    while let Some(row) = rows_stream.next().await {
        let (player, value) = match row {
            Ok(row) => { row }
            Err(err) => {
                error!("{}", err);
                continue;
            }
        };

        let username = match data.db.select_player_username(&player).await {
            Ok(username) => username,
            Err(err) => {
                error!("{}", err);
                continue;
            }
        }.unwrap_or("Inconnu".to_string());

        result.push(Stat{
            player,
            username,
            value
        });
    }

    result.sort_by(|x, y|x.value.cmp(&y.value).reverse());


    Ok(warp::reply::json(&result).into_response())
}