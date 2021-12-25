use std::collections::HashMap;
use futures::StreamExt;
use scylla::frame::value::Counter;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, Group};
use scylla::FromRow;
use crate::web::proxy::ProxyLoginPlayerInfo;
use serde::Serialize;
use crate::web::server::ServerLoginPlayerInfo;

#[derive(Serialize, Debug, FromRow)]
pub struct DbProxyPlayerInfo {
    pub locale: Option<String>, //Having one there means that we force his locale to this

    pub groups: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,

    pub properties: Option<HashMap<String, String>>,

    pub ban: Option<Uuid>,
    pub ban_reason: Option<String>,
    pub ban_ttl: Option<i32>,
}


#[derive(Serialize, Debug, FromRow)]
pub struct DbServerPlayerInfo {
    pub prefix: Option<String>,
    pub suffix: Option<String>,

    pub proxy: Uuid,
    pub session: Uuid,

    pub locale: Option<String>,

    pub groups: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,

    pub currency: i32,
    pub premium_currency: i32,

    pub blocked: Option<Vec<Uuid>>,
    pub inventory: Option<HashMap<String, i32>>,
    pub properties: Option<HashMap<String, String>>,
}

#[derive(Serialize, Debug, FromRow)]
pub struct ReducedPlayerInfo {
    pub uuid: Uuid,
    pub username: String,
    pub session: Uuid,
    pub proxy: Uuid,
    pub server: Option<Uuid>,
}

#[derive(Serialize, Debug, FromRow)]
pub struct DbFullPlayerInfo {
    pub uuid: Uuid,
    pub ban: Option<Uuid>,
    pub ban_reason: Option<String>,
    pub blocked: Option<Vec<Uuid>>,
    pub currency: i32,
    pub premium_currency: i32,
    pub friends: Option<Vec<Uuid>>,
    pub groups: Vec<String>,
    pub inventory: Option<HashMap<String, i32>>,
    pub locale: String,
    pub permissions: Option<Vec<String>>,
    pub proxy: Option<Uuid>,
    pub server: Option<Uuid>,
    pub session: Option<Uuid>,
    pub username: String,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}


impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn select_online_players_reduced_info(&self) -> Result<Vec<ReducedPlayerInfo>, DatabaseError> {
        let mut servers = Vec::new();
        let mut stream = self.session.execute_iter(self.queries.select_online_players_reduced_info.clone(), ()).await?.into_typed::<ReducedPlayerInfo>();

        while let Some(srv) = stream.next().await {
            servers.push(srv?);
        }

        Ok(servers)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_online_players_count(&self) -> Result<i64, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_online_players_count, ()).await?.rows;

        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<(i64, )>()).transpose()?.map(|(t, )| t).unwrap_or_default())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_online_player_proxy(&self, uuid: &Uuid) -> Result<Option<Uuid>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_player_proxy, (uuid, )).await?.rows;

        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<(Option<Uuid>, )>()).transpose()?.map(|(t, )| t).unwrap_or_default())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_proxy_player_info(&self, uuid: &Uuid) -> Result<Option<DbProxyPlayerInfo>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_proxy_player_info, (uuid, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<DbProxyPlayerInfo>()).transpose()?)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_player_info(&self, uuid: &Uuid) -> Result<Option<DbServerPlayerInfo>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_server_player_info, (uuid, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<DbServerPlayerInfo>()).transpose()?)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_full_player_info(&self, uuid: &Uuid) -> Result<Option<DbFullPlayerInfo>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_full_player_info, (uuid, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<DbFullPlayerInfo>()).transpose()?)
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn insert_player(&self, uuid: &Uuid, username: &str, locale: Option<&str>) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.insert_player, (uuid, username, locale)).await?;
        Ok(())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn update_player_online_proxy_info(&self, player: &Uuid, proxy: Uuid, session: &Uuid, username: &str) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.update_player_proxy_online_info, (proxy, session, username, player)).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_player_online_sever_info(&self, player: &Uuid, server: Uuid) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.update_player_server_online_info, (server, player)).await?;
        Ok(())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn close_player_session(&self, player: &Uuid) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.close_player_session, (player, )).await?;
        Ok(())
    }
}

impl DbProxyPlayerInfo {
    pub fn build_proxy_login_player_info(self, db: &Database) -> ProxyLoginPlayerInfo {
        let raw_groups = self.groups.unwrap_or(vec!["Default".to_string()]);
        let groups: Vec<&Group> = raw_groups.iter().filter_map(|x| db.cache.groups.get(x)).collect();
        let power = groups.iter().map(|grp| grp.power).max().unwrap_or(0);
        let mut permissions: Vec<String> = groups.into_iter().filter_map(|grp| grp.permissions.clone()).flatten().collect();

        permissions.push(format!("power.{}", power));

        if let Some(user_permissions) = self.permissions {
            permissions.extend(user_permissions);
        }

        if let Some(kind_permissions) = &db.get_cached_kind("proxy").map(|kind| kind.permissions.as_ref()).flatten() {
            kind_permissions.iter().filter(|&(group, kpermissions)| raw_groups.contains(group)).for_each(|(group, kpermissions)| permissions.extend(kpermissions.clone()));
        }

        let permissions = permissions.into_iter().filter_map(|p| if !p.contains(":") {
            Some(p)
        } else if p.starts_with("proxy:") {
            Some(p.trim_start_matches("proxy:").to_string())
        } else {
            None
        }).collect();


        ProxyLoginPlayerInfo {
            power,
            permissions,
            locale: self.locale.unwrap_or("fr".to_string()),
            properties: self.properties.unwrap_or_default(),
        }
    }
}

impl DbServerPlayerInfo {
    pub fn build_server_login_player_info(self, db: &Database, kind: &str) -> ServerLoginPlayerInfo {
        let raw_groups = self.groups.unwrap_or(vec!["Default".to_string()]);
        let groups: Vec<&Group> = raw_groups.iter().filter_map(|x| db.cache.groups.get(x)).collect();
        let power = groups.iter().map(|grp| grp.power).max().unwrap_or(0);
        let mut permissions: Vec<String> = groups.iter().filter_map(|grp| grp.permissions.clone()).flatten().collect();

        permissions.push(format!("power.{}", power));

        if let Some(user_permissions) = self.permissions {
            permissions.extend(user_permissions);
        }

        if let Some(kind_permissions) = &db.get_cached_kind(kind).map(|kind| kind.permissions.as_ref()).flatten() {
            kind_permissions.iter().filter(|&(group, kpermissions)| raw_groups.contains(group)).for_each(|(group, kpermissions)| permissions.extend(kpermissions.clone()));
        }

        let permissions: Vec<String> = permissions.into_iter().filter_map(|p| if !p.contains(":") {
            Some(p)
        } else if p.starts_with("proxy:") {
            Some(p.trim_start_matches("proxy:").to_string())
        } else {
            None
        }).collect();

        let prefix = if let Some(prefix) = self.prefix {
            Some(prefix)
        } else {
            groups.iter().filter(|&&grp| grp.prefix.is_some()).max_by(|&&g1, &&g2| g1.power.cmp(&g2.power)).map(|&grp| grp.prefix.clone()).flatten()
        };

        let suffix = if let Some(suffix) = self.suffix {
            Some(suffix)
        } else {
            groups.iter().filter(|&&grp| grp.suffix.is_some()).max_by(|&&g1, &&g2| g1.power.cmp(&g2.power)).map(|&grp| grp.suffix.clone()).flatten()
        };


        ServerLoginPlayerInfo {
            session: self.session,
            proxy: self.proxy,
            prefix,
            suffix,
            locale: self.locale.unwrap_or("fr".to_string()),
            permissions,
            power,
            currency: self.currency,
            premium_currency: self.premium_currency,
            blocked: self.blocked.unwrap_or_default(),
            inventory: self.inventory.unwrap_or_default(),
            properties: self.properties.unwrap_or_default(),
        }
    }
}