use std::net::IpAddr;
use std::ops::Add;
use std::time::SystemTime;
use chrono::{DateTime, Duration, Local, NaiveDateTime};
use itertools::Itertools;
use scylla::frame::value::Timestamp;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_iter, select_one};
use scylla::FromRow;
use crate::structures::players::Mute;

#[derive(Debug, FromRow)]
pub struct DbMute {
    pub id: Uuid,
    pub start: Duration,
    pub end: Option<Duration>,
    pub issuer: Option<Uuid>,
    pub reason: Option<String>,
    pub target: Option<Uuid>,
}


impl Database {
    #[instrument(skip(self), level = "debug")]
    pub(crate) async fn insert_mute_log(&self, duration: Option<&Duration>, target_uuid: Option<&Uuid>, target_ip: Option<&IpAddr>, issuer: Option<&Uuid>, reason: Option<&String>) -> Result<Uuid, DatabaseError> {
        //#[query(insert_mute_log = "INSERT INTO mutes_logs(id, start, end, target, issuer, reason) VALUES (?, toTimestamp(now()), ?, ?, ?, ?);")]
        let uuid = Uuid::new_v4();
        let end = duration.map(|d| Timestamp(d.add(Duration::seconds(Local::now().timestamp()))));
        execute(&self.queries.insert_mute_log, &self.session, (uuid, end, target_uuid, target_ip, issuer, reason)).await?;
        Ok(uuid)
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_players_from_mute(&self, mute: &Uuid) -> Result<Vec<Uuid>, DatabaseError> {
        //#[query(select_players_from_mute = "SELECT uuid FROM players WHERE mute = ? ALLOW FILTERING")]
        Ok(select_iter::<(Option<Uuid>, ), _>(&self.queries.select_players_from_mute, &self.session, (mute, )).await?.into_iter().filter_map(|x| x.0).dedup().collect())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn insert_mute(&self, uuid: &Uuid, reason: Option<&String>, issuer: Option<&Uuid>, duration: Option<&Duration>) -> Result<Uuid, DatabaseError> {
        let mute = self.insert_mute_log(duration, Some(uuid), None, issuer, reason).await?;
        match duration {
            None => {
                //#[query(insert_mute = "UPDATE players SET mute = ?, mute_reason = ? WHERE uuid = ?;")]
                execute(&self.queries.insert_mute, &self.session, (mute, reason, uuid)).await?;
            }
            Some(duration) => {
                //#[query(insert_mute_ttl = "UPDATE players USING TTL ? SET mute = ?, mute_reason = ? WHERE uuid = ?;")]
                execute(&self.queries.insert_mute_ttl, &self.session, (duration.num_seconds() as i32, mute, reason, uuid)).await?;
            }
        }
        Ok(mute)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn insert_mute_with_log(&self, uuid: &Uuid, reason: Option<&String>, issuer: Option<&Uuid>, duration: Option<&Duration>, mute: &Uuid) -> Result<(), DatabaseError> {
        match duration {
            None => {
                execute(&self.queries.insert_mute, &self.session, (mute, reason, uuid)).await?;
            }
            Some(duration) => {
                execute(&self.queries.insert_mute_ttl, &self.session, (duration.num_seconds() as i32, mute, reason, uuid)).await?;
            }
        }
        Ok(())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn remove_player_mute(&self, uuid: &Uuid) -> Result<(), DatabaseError> {
        //#[query(remove_mute = "UPDATE players SET mute = null, mute_reason = null WHERE uuid = ?;")]
        execute(&self.queries.remove_mute, &self.session, (uuid, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_mute(&self, mute_id: Uuid) -> Result<Option<DbMute>, DatabaseError> {
        //#[query(select_mute_log = "SELECT id, start, end, issuer, reason, target FROM mutes_logs WHERE id = ?;")]
        select_one(&self.queries.select_mute_log, &self.session, (mute_id, )).await
    }
}

impl Into<Mute> for DbMute {
    fn into(self) -> Mute {
        let time = DateTime::from(SystemTime::now());
        Mute {
            id: self.id,
            start: NaiveDateTime::from_timestamp(self.start.num_seconds(), 0).to_string(),
            end: self.end.map(|t| NaiveDateTime::from_timestamp(t.num_seconds(), 0).to_string()),
            issuer: self.issuer,
            reason: self.reason,
            target: self.target,
            remaining: self.end.map(|t| t.num_seconds() - time.timestamp()),
        }
    }
}