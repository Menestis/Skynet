use std::ops::Add;
use std::time::SystemTime;
use chrono::{DateTime, Duration, Local, NaiveDateTime};
use itertools::Itertools;
use scylla::frame::value::Timestamp;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_one};

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn select_sanction_board(&self, category: &str) -> Result<Option<(String, Vec<String>)>, DatabaseError> {
        //#[query(select_sanction_board = "SELECT label,sanctions FROM sanctions_board WHERE category = ?")]
        Ok(select_one::<(String, Option<Vec<String>>), _>(&self.queries.select_sanction_board, &self.session, (category, )).await?.map(|(x, y)| (x, y.unwrap_or_default())))
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_sanction_state(&self, uuid: &Uuid, category: &str) -> Result<i32, DatabaseError> {
        //#[query(select_sanction_state = "SELECT value FROM sanctions_states WHERE player = ? AND category = ?")]
        Ok(select_one::<(Option<i32>, ), _>(&self.queries.select_sanction_state, &self.session, (uuid, category)).await?.map(|x| x.0).flatten().unwrap_or_default())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn insert_player_sanction_state(&self, uuid: &Uuid, category: &str, value: i32) -> Result<(), DatabaseError> {
        //#[query(insert_player_sanction_state = "INSERT INTO sanctions_states(player, category, value) VALUES (?, ?, ?)")]
        Ok(execute(&self.queries.insert_player_sanction_state, &self.session, (uuid, category, value)).await?)
    }
}