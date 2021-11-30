use std::net::IpAddr;
use std::ops::Add;
use chrono::{Duration, Local, NaiveDateTime};
use scylla::frame::value::Timestamp;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::DatabaseError;
use scylla::FromRow;

#[derive(Debug, FromRow)]
pub struct IpBan {
    pub ip: IpAddr,
    pub reason: Option<String>,
    pub date: Duration,
    pub end: Option<Duration>,
    pub ban: Option<Uuid>,
    pub automated: bool,
}


impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_ban_log(&self, duration: Option<&Duration>, target_uuid: Option<&Uuid>, target_ip: Option<&IpAddr>, issuer: Option<&Uuid>, reason: Option<&String>) -> Result<Uuid, DatabaseError> {
        let uuid = Uuid::new_v4();
        let end = duration.map(|d| Timestamp(d.add(Duration::seconds(Local::now().timestamp()))));
        self.session.execute(&self.queries.insert_ban_log, (uuid, end, target_uuid, target_ip, issuer, reason)).await?;
        Ok(uuid)
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_ip_ban(&self, ip: &IpAddr) -> Result<Option<IpBan>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_ip_ban, (ip, )).await?.rows;

        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<IpBan>()).transpose()?)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn insert_ip_ban(&self, ip: &IpAddr, reason: Option<&String>, issuer: Option<&Uuid>, duration: Option<&Duration>, automated: bool) -> Result<Uuid, DatabaseError> {
        let ban = self.insert_ban_log(duration, None, Some(ip), issuer, reason).await?;
        match duration {
             None => {
                 self.session.execute(&self.queries.insert_ip_ban, (ip, reason, ban, automated)).await?;
             }
             Some(duration) => {
                 let end = duration.add(Duration::seconds(Local::now().timestamp()));
                 self.session.execute(&self.queries.insert_ip_ban_ttl, (ip, reason, Timestamp(end), ban, automated, duration.num_seconds() as i32)).await?;
             }
         }
        Ok(ban)
    }
}