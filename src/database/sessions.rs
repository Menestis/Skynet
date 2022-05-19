use std::collections::HashMap;
use std::net::IpAddr;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_iter, select_one};
use itertools::Itertools;

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_session(&self, id: &Uuid, version: &str, player: &Uuid, ip: &IpAddr) -> Result<(), DatabaseError> {
        //#[query(insert_session = "INSERT INTO sessions(id, ip, player, version, start) VALUES (?, ?, ?, ?, dateOf(now()));")]
        execute(&self.queries.insert_session, &self.session, (id, ip, player, version)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn set_session_mods_info(&self, id: &Uuid, mods: &HashMap<String, String>) -> Result<(), DatabaseError> {
        //#[query(set_session_mods_info = "UPDATE sessions SET mods = ? WHERE id = ?;")]
        execute(&self.queries.set_session_mods_info, &self.session, (mods, id)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn set_session_brand(&self, id: &Uuid, brand: &str) -> Result<(), DatabaseError> {
        //#[query(set_session_brand = "UPDATE sessions SET brand = ? WHERE id = ?;")]
        execute(&self.queries.set_session_brand, &self.session, (brand, id)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn close_session(&self, id: &Uuid) -> Result<(), DatabaseError> {
        //#[query(close_session = "UPDATE sessions SET end = toTimestamp(now()) WHERE id = ?;")]
        execute(&self.queries.close_session, &self.session, (id, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_session(&self, id: &Uuid) -> Result<Option<Uuid>, DatabaseError> {
        //#[query(select_player_session = "SELECT session FROM players WHERE uuid = ?")]
        Ok(select_one::<(Option<Uuid>, ), _>(&self.queries.select_player_session, &self.session, (id, )).await?.map(|t| t.0).flatten())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_username(&self, id: &Uuid) -> Result<Option<String>, DatabaseError> {
        //#[query(select_player_username = "SELECT username FROM players WHERE uuid = ?")]
        Ok(select_one::<(Option<String>, ), _>(&self.queries.select_player_username, &self.session, (id, )).await?.map(|t| t.0).flatten())
    }




    #[instrument(skip(self), level = "debug")]
    pub async fn select_all_player_sessions_ips(&self, player: &Uuid) -> Result<Vec<IpAddr>, DatabaseError> {
        //#[query(select_all_player_sessions_ips = "SELECT ip FROM sessions_by_player WHERE player = ?")]
        Ok(select_iter::<(Option<IpAddr>, ), _>(&self.queries.select_all_player_sessions_ips, &self.session, (player, )).await?.into_iter().filter_map(|x| x.0).dedup().collect())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_players_with_session_ip(&self, ip: &IpAddr) -> Result<Vec<Uuid>, DatabaseError> {
        //#[query(select_players_with_session_ip = "SELECT player FROM sessions_by_ip WHERE ip = ?")]
        Ok(select_iter::<(Option<Uuid>, ), _>(&self.queries.select_players_with_session_ip, &self.session, (ip, )).await?.into_iter().filter_map(|x| x.0).dedup().collect())
    }





}
