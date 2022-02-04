use std::net::IpAddr;
use std::ops::Add;
use chrono::{Duration, Local, NaiveDateTime};
use scylla::frame::value::Timestamp;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_one};
use scylla::FromRow;
use crate::structures::players::Ban;

#[derive(Debug, FromRow)]
pub struct IpBan {
    pub ip: IpAddr,
    pub reason: Option<String>,
    pub date: Duration,
    pub end: Option<Duration>,
    pub ban: Option<Uuid>,
    pub automated: bool,
}

#[derive(Debug, FromRow)]
pub struct DbBan {
    pub id: Uuid,
    pub start: Duration,
    pub end: Option<Duration>,
    pub issuer: Option<Uuid>,
    pub reason: Option<String>,
    pub ip: Option<IpAddr>,
    pub target: Option<Uuid>,
}


impl Database {
    #[instrument(skip(self), level = "debug")]
    async fn insert_ban_log(&self, duration: Option<&Duration>, target_uuid: Option<&Uuid>, target_ip: Option<&IpAddr>, issuer: Option<&Uuid>, reason: Option<&String>) -> Result<Uuid, DatabaseError> {
        //#[query(insert_ban_log = "INSERT INTO bans_logs(id, start, end, target, ip, issuer, reason) VALUES (?, toTimestamp(now()), ?, ?, ?, ?, ?);")]
        let uuid = Uuid::new_v4();
        let end = duration.map(|d| Timestamp(d.add(Duration::seconds(Local::now().timestamp()))));
        execute(&self.queries.insert_ban_log, &self.session, (uuid, end, target_uuid, target_ip, issuer, reason)).await?;
        Ok(uuid)
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_ip_ban(&self, ip: &IpAddr) -> Result<Option<IpBan>, DatabaseError> {
        //#[query(select_ip_ban = "SELECT ip, reason, date, end, ban, automated FROM ip_bans WHERE ip = ?;")]
        select_one(&self.queries.select_ip_ban, &self.session, (ip, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn insert_ip_ban(&self, ip: &IpAddr, reason: Option<&String>, issuer: Option<&Uuid>, duration: Option<&Duration>, automated: bool) -> Result<Uuid, DatabaseError> {
        let ban = self.insert_ban_log(duration, None, Some(ip), issuer, reason).await?;
        match duration {
            None => {
                //#[query(insert_ip_ban = "INSERT INTO ip_bans(ip, reason, date, end, ban, automated) VALUES (?, ?, toTimestamp(now()), null, ?, ?);")]
                execute(&self.queries.insert_ip_ban, &self.session, (ip, reason, ban, automated)).await?;
            }
            Some(duration) => {
                let end = duration.add(Duration::seconds(Local::now().timestamp()));
                //#[query(insert_ip_ban_ttl = "INSERT INTO ip_bans(ip, reason, date, end, ban, automated) VALUES (?, ?, toTimestamp(now()), ?, ?, ?) USING TTL ?;")]
                execute(&self.queries.insert_ip_ban_ttl, &self.session, (ip, reason, Timestamp(end), ban, automated, duration.num_seconds() as i32)).await?;
            }
        }
        Ok(ban)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn insert_ban(&self, uuid: &Uuid, reason: Option<&String>, issuer: Option<&Uuid>, duration: Option<&Duration>) -> Result<Uuid, DatabaseError> {
        let ban = self.insert_ban_log(duration, Some(uuid), None, issuer, reason).await?;
        match duration {
            None => {
                //#[query(insert_ban = "UPDATE players SET ban = ?, ban_reason = ? WHERE uuid = ?;")]
                execute(&self.queries.insert_ban, &self.session, (ban, reason, uuid)).await?;
            }
            Some(duration) => {
                //#[query(insert_ban_ttl = "UPDATE players USING TTL ? SET ban = ?, ban_reason = ? WHERE uuid = ?;")]
                execute(&self.queries.insert_ban_ttl, &self.session, (duration.num_seconds() as i32, ban, reason, uuid)).await?;
            }
        }
        Ok(ban)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn remove_player_ban(&self, uuid: &Uuid) -> Result<(), DatabaseError> {
        //#[query(remove_ban = "UPDATE players SET ban = null, ban_reason = null WHERE uuid = ?;")]
        execute(&self.queries.remove_ban, &self.session, (uuid, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_ban(&self, ban_id: Uuid) -> Result<Option<DbBan>, DatabaseError>{
        //#[query(select_ban_log = "SELECT id, start, end, issuer, reason, ip, target FROM bans_logs WHERE id = ?;")]
        select_one(&self.queries.select_ban_log, &self.session, (ban_id, )).await
    }
}

impl Into<Ban> for DbBan {
    fn into(self) -> Ban {
        Ban{
            id: self.id,
            start: NaiveDateTime::from_timestamp(self.start.num_seconds(),0).to_string(),
            end: self.end.map(|t| NaiveDateTime::from_timestamp(t.num_seconds(),0).to_string()),
            issuer: self.issuer,
            reason: self.reason,
            ip: self.ip,
            target: self.target
        }
    }
}