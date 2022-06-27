use std::collections::HashMap;
use std::time::SystemTime;
use chrono::{Datelike, DateTime, Duration, Local, Timelike};
use scylla::batch::Batch;
use scylla::frame::value::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_iter, select_one};

#[derive(Debug, Deserialize, Serialize)]
pub struct LeaderboardRule {
    pub stat_key: String,
    pub period: LeaderboardPeriod,
    pub game_kind: Option<String>,
    pub size: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum LeaderboardPeriod {
    Monthly,
    AllTime,
}

impl LeaderboardPeriod {
    pub fn timestamp(&self) -> i64 {
        match self {
            LeaderboardPeriod::Monthly => {
                let now: DateTime<Local> = DateTime::from(SystemTime::now());
                now
                    .with_day0(0).expect("Setting Day 0")
                    .with_hour(0).expect("Setting hour 0")
                    .with_minute(0).expect("Setting minute 0")
                    .with_second(0).expect("Setting second to 0")
                    .with_nanosecond(0).expect("Setting nanos to 0")
                    .timestamp_millis()
            }
            LeaderboardPeriod::AllTime => 0
        }
    }
}

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_stats(&self, player: &Uuid, session: &Uuid, server_id: &Uuid, server_kind: &str, stats: &HashMap<String, i32>) -> Result<(), DatabaseError> {
        let mut batch = Batch::default();
        let mut values = vec![];
        let timestamp = Timestamp(Duration::seconds(Local::now().timestamp()));
        for (k, v) in stats {
            //#[query(insert_stats = "INSERT INTO statistics(player, session, timestamp, server_id, server_kind, key, value) VALUES (?, ?, ? ,?, ?, ?, ?);")]
            batch.append_statement(self.queries.insert_stats.clone());
            values.push((player, session, &timestamp, server_id, server_kind, k, v));
        }

        self.session.batch(&batch, values).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_leaderboards_rules(&self) -> Result<Vec<(String, String, LeaderboardRule)>, DatabaseError> {
        //#[query(select_leaderboards_rules = "SELECT name, label, rules FROM leaderboards")]
        Ok(select_iter::<(String, String, String), _>(&self.queries.select_leaderboards_rules, &self.session, ()).await?.into_iter().filter_map(|x| match serde_json::from_str(&x.2) {
            Ok(data) => Some((x.0, x.1, data)),
            Err(e) => {
                warn!("Invalid leaderboard rule {} : {}", x.0, e);
                None
            }
        }).collect())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_stats(&self, key: &str, timestamp: i64) -> Result<HashMap<Uuid, i64>, DatabaseError> {
        //#[query(select_stats = "SELECT player, value FROM statistics_processable WHERE key = ? AND timestamp > ?;")]
        let stats: Vec<(Uuid, i32)> = select_iter(&self.queries.select_stats, &self.session, (key, timestamp)).await?;

        let mut ret: HashMap<Uuid, i64> = HashMap::new();

        for (id, value) in stats {
            *ret.entry(id).or_insert(0) += value as i64;
        }

        Ok(ret)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_stats_kind(&self, key: &str, game_kind: &str, timestamp: i64) -> Result<HashMap<Uuid, i64>, DatabaseError> {
        //#[query(select_stats_kind = "SELECT player, value FROM statistics_processable_ig WHERE key = ? AND game_kind = ? AND timestamp > ?;")]
        let stats: Vec<(Uuid, i32)> = select_iter(&self.queries.select_stats_kind, &self.session, (key, game_kind, timestamp)).await?;

        let mut ret: HashMap<Uuid, i64> = HashMap::new();

        for (id, value) in stats {
            *ret.entry(id).or_insert(0) += value as i64;
        }

        Ok(ret)
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn update_leaderboard(&self, name: &str, leaderboard: &Vec<String>) -> Result<(), DatabaseError> {
        //#[query(update_leaderboard = "UPDATE leaderboards SET leaderboard = ? WHERE name = ?;")]
        execute(&self.queries.update_leaderboard, &self.session, (leaderboard, name)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_leaderboard(&self, name: &str) -> Result<Option<(String, Vec<String>)>, DatabaseError> {
        //#[query(select_leaderboard = "SELECT label, leaderboard FROM leaderboards")]
        Ok(select_one::<(String, Option<Vec<String>>), _>(&self.queries.select_leaderboard, &self.session, (name, )).await?.map(|(name, leaderboard)| (name, leaderboard.unwrap_or_default())))

    }
}
