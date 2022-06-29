use std::collections::HashMap;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_iter, select_one};
use scylla::FromRow;
use serde::Serialize;
use crate::database::bans::DbBan;
use crate::structures::players::PlayerInfo;
use crate::web::login::{ProxyLoginPlayerInfo, ServerLoginPlayerInfo};

#[derive(Serialize, Debug, Default, FromRow)]
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

    pub discord_id: Option<String>,

    pub locale: Option<String>,

    pub groups: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,

    pub currency: i32,
    pub premium_currency: i32,

    pub mute: Option<Uuid>,

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
    pub discord_id: Option<String>,
    pub mute: Option<Uuid>,
}

#[derive(Serialize, Debug, FromRow)]
pub struct DbPlayerInfo {
    pub uuid: Uuid,
    pub username: String,
    pub groups: Option<Vec<String>>,
    pub locale: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub currency: i32,
    pub premium_currency: i32,
    pub proxy: Option<Uuid>,
    pub server: Option<Uuid>,
    pub blocked: Option<Vec<Uuid>>,
    pub inventory: Option<HashMap<String, i32>>,
    pub properties: Option<HashMap<String, String>>,
    pub ban: Option<Uuid>,
    pub discord_id: Option<String>,
    pub mute: Option<Uuid>,
}

#[derive(Debug, FromRow)]
pub struct Group {
    pub name: String,
    pub power: i32,
    prefix: Option<String>,
    suffix: Option<String>,
    pub permissions: Option<Vec<String>>,
}

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn select_online_players_reduced_info(&self) -> Result<Vec<ReducedPlayerInfo>, DatabaseError> {
        //#[query(select_online_players_reduced_info = "SELECT uuid, username, session, proxy, server FROM players_by_session;")]
        select_iter(&self.queries.select_online_players_reduced_info, &self.session, ()).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_all_players_with_waiting_move_to(&self, kind: &str, limit: i32) -> Result<Vec<ReducedPlayerInfo>, DatabaseError> {
        //#[query(select_all_players_with_waiting_move_to = "SELECT uuid, username, session, proxy, server FROM players_by_waiting_move_to WHERE waiting_move_to = ? LIMIT ?;")]
        select_iter(&self.queries.select_all_players_with_waiting_move_to, &self.session, (kind, limit)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_online_player_proxy(&self, uuid: &Uuid) -> Result<Option<Uuid>, DatabaseError> {
        //#[query(select_player_proxy = "SELECT proxy FROM players WHERE uuid = ?")]
        Ok(select_one::<(Option<Uuid>, ), _>(&self.queries.select_player_proxy, &self.session, (uuid, )).await?.map(|t| t.0).flatten())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_online_player_proxy_and_discord(&self, uuid: &Uuid) -> Result<(Option<Uuid>, Option<String>), DatabaseError> {
        //#[query(select_player_proxy_and_discord = "SELECT proxy, discord_id FROM players WHERE uuid = ?")]
        Ok(select_one(&self.queries.select_player_proxy_and_discord, &self.session, (uuid, )).await?.unwrap_or_default())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_online_player_server(&self, uuid: &Uuid) -> Result<Option<Uuid>, DatabaseError> {
        //#[query(select_player_server = "SELECT server FROM players WHERE uuid = ?")]
        Ok(select_one::<(Option<Uuid>, ), _>(&self.queries.select_player_server, &self.session, (uuid, )).await?.map(|t| t.0).flatten())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_proxy_player_info(&self, uuid: &Uuid) -> Result<Option<DbProxyPlayerInfo>, DatabaseError> {
        //#[query(select_proxy_player_info = "SELECT locale, groups, permissions, properties, ban, ban_reason, TTL(ban) AS ban_ttl FROM players WHERE uuid = ?;")]
        select_one(&self.queries.select_proxy_player_info, &self.session, (uuid, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_player_info(&self, uuid: &Uuid) -> Result<Option<DbServerPlayerInfo>, DatabaseError> {
        //#[query(select_server_player_info = "SELECT prefix, suffix, proxy, session, discord_id, locale, groups, permissions, currency, premium_currency, blocked, inventory, properties FROM players WHERE uuid = ?;")]
        select_one(&self.queries.select_server_player_info, &self.session, (uuid, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_info(&self, uuid: &Uuid) -> Result<Option<DbPlayerInfo>, DatabaseError> {
        //#[query(select_player_info = "SELECT uuid, username, groups, locale, prefix, suffix, currency, premium_currency, proxy, server, blocked, inventory, properties, ban, discord_id, mute FROM players WHERE uuid = ?;")]
        select_one(&self.queries.select_player_info, &self.session, (uuid, )).await
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_group_object(&self, name: &str) -> Result<Option<Group>, DatabaseError> {
        //#[query(select_player_group_obj = "SELECT name,power,prefix,suffix,permissions FROM groups WHERE name = ?;")]
        select_one(&self.queries.select_player_group_obj, &self.session, (name, )).await
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_groups_objects(&self, names: &Vec<String>) -> Result<Vec<Group>, DatabaseError> {
        let mut groups = Vec::new();
        for name in names {
            match self.select_player_group_object(name).await? {
                None => warn!("Player group not found {}", name),
                Some(group) => groups.push(group)
            }
        }
        Ok(groups)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_info_by_name(&self, name: &str) -> Result<Option<DbPlayerInfo>, DatabaseError> {
        //#[query(select_player_info_by_name = "SELECT uuid, username, groups, locale, prefix, suffix, currency, premium_currency, proxy, server, blocked, inventory, properties, ban, discord_id, mute FROM players_by_username WHERE username = ?;")]
        select_one(&self.queries.select_player_info_by_name, &self.session, (name, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_full_player_info(&self, uuid: &Uuid) -> Result<Option<DbFullPlayerInfo>, DatabaseError> {
        //#[query(select_full_player_info = "SELECT uuid, ban, ban_reason, blocked, currency, premium_currency, friends, groups, inventory, locale, permissions, proxy, server, session, username, prefix, suffix, discord_id, mute FROM players WHERE uuid = ?")]
        select_one(&self.queries.select_full_player_info, &self.session, (uuid, )).await
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn insert_player(&self, uuid: &Uuid, username: &str, locale: Option<&str>) -> Result<(), DatabaseError> {
        //#[query(insert_player = "INSERT INTO players(uuid, username, currency, premium_currency, locale, groups) VALUES (?, ?, 0, 0, ?, ['Default']);")]
        execute(&self.queries.insert_player, &self.session, (uuid, username, locale)).await
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn update_player_online_proxy_info(&self, player: &Uuid, proxy: Uuid, session: &Uuid, username: &str) -> Result<(), DatabaseError> {
        //#[query(update_player_proxy_online_info = "UPDATE players SET proxy = ?, session = ?, username = ? WHERE uuid = ?;")]
        execute(&self.queries.update_player_proxy_online_info, &self.session, (proxy, session, username, player)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_player_server_and_null_waiting_move_to(&self, player: &Uuid, server: Uuid) -> Result<(), DatabaseError> {
        //#[query(update_player_server_and_null_waiting_move_to = "UPDATE players USING TTL 86400 SET server = ?, waiting_move_to = null WHERE uuid = ?;")]
        execute(&self.queries.update_player_server_and_null_waiting_move_to, &self.session, (server, player)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn set_player_waiting_move_to(&self, player: &Uuid, kind: &str) -> Result<(), DatabaseError> {
        //#[query(set_player_waiting_move_to = "UPDATE players SET waiting_move_to = ? WHERE uuid = ?;")]
        execute(&self.queries.set_player_waiting_move_to, &self.session, (kind, player)).await
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn close_player_session(&self, player: &Uuid) -> Result<(), DatabaseError> {
        //#[query(close_player_session = "UPDATE players SET proxy = null, server = null, session = null, waiting_move_to = null WHERE uuid = ?;")]
        execute(&self.queries.close_player_session, &self.session, (player, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_currencies(&self, player: &Uuid) -> Result<Option<(i32, i32)>, DatabaseError> {
        //#[query(select_player_currencies = "SELECT currency, premium_currency FROM players WHERE uuid = ?;")]
        select_one(&self.queries.select_player_currencies, &self.session, (player, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn set_player_currencies(&self, player: &Uuid, currency: i32, premium_currency: i32) -> Result<(), DatabaseError> {
        //#[query(set_player_currencies = "UPDATE players SET currency = ?, premium_currency = ? WHERE uuid = ?;")]
        execute(&self.queries.set_player_currencies, &self.session, (currency, premium_currency, player)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn set_player_discord_id(&self, player: &Uuid, discord_id: &str) -> Result<(), DatabaseError> {
        //#[query(set_player_discord_id = "UPDATE players SET discord_id = ? WHERE uuid = ?;")]
        execute(&self.queries.set_player_discord_id, &self.session, (discord_id, player)).await
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn add_player_group(&self, player: &Uuid, group: &str) -> Result<(), DatabaseError> {
        //#[query(add_player_group = "UPDATE players SET groups = groups + ? WHERE uuid = ?;")]
        execute(&self.queries.add_player_group, &self.session, (vec![group], player)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn add_player_group_ttl(&self, player: &Uuid, group: &str, ttl: i32) -> Result<(), DatabaseError> {
        //#[query(add_player_group_ttl = "UPDATE players USING TTL ? SET groups = groups + ? WHERE uuid = ?;")]
        execute(&self.queries.add_player_group_ttl, &self.session, (ttl, vec![group], player)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn remove_player_group(&self, player: &Uuid, group: &str) -> Result<(), DatabaseError> {
        //#[query(remove_player_group = "UPDATE players SET groups = groups - ? WHERE uuid = ?;")]
        execute(&self.queries.remove_player_group, &self.session, (vec![group], player)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_inventory(&self, player: &Uuid) -> Result<Option<HashMap<String, i32>>, DatabaseError> {
        //#[query(select_player_inventory = "SELECT inventory FROM players WHERE uuid = ?;")]
        Ok(select_one::<(Option<HashMap<String, i32>>, ), _>(&self.queries.select_player_inventory, &self.session, (player, )).await?.map(|t| t.0).flatten())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn set_player_inventory_item(&self, player: &Uuid, item: &str, count: i32) -> Result<(), DatabaseError> {
        //#[query(set_player_inventory_item = "UPDATE players SET inventory[?] = ? WHERE uuid = ?;")]
        execute(&self.queries.set_player_inventory_item, &self.session, (item, count, player)).await
    }
}

impl DbProxyPlayerInfo {
    pub async fn build_proxy_login_player_info(self, db: &Database) -> Result<ProxyLoginPlayerInfo, DatabaseError> {
        let raw_groups = self.groups.unwrap_or(vec!["Default".to_string()]);
        let groups: Vec<Group> = db.select_player_groups_objects(&raw_groups).await?;
        let power = groups.iter().map(|grp| grp.power).max().unwrap_or(0);
        let mut permissions: Vec<String> = groups.into_iter().filter_map(|grp| grp.permissions.clone()).flatten().collect();

        permissions.push(format!("power.{}", power));

        if let Some(user_permissions) = self.permissions {
            permissions.extend(user_permissions);
        }

        if let Some(kind_permissions) = &db.select_server_kind_object("proxy").await?.map(|kind| kind.permissions).flatten() {
            kind_permissions.iter().filter(|&(group, _kpermissions)| raw_groups.contains(group)).for_each(|(_group, kpermissions)| permissions.extend(kpermissions.clone()));
        }

        let permissions = permissions.into_iter().filter_map(|p| if !p.contains(":") {
            Some(p)
        } else if p.starts_with("proxy:") {
            Some(p.trim_start_matches("proxy:").to_string())
        } else {
            None
        }).collect();


        Ok(ProxyLoginPlayerInfo {
            power,
            permissions,
            locale: self.locale.unwrap_or("fr".to_string()),
            properties: self.properties.unwrap_or_default(),
        })
    }
}

impl DbServerPlayerInfo {
    pub async fn build_server_login_player_info(self, db: &Database, kind: &str) -> Result<ServerLoginPlayerInfo, DatabaseError> {
        let raw_groups = self.groups.unwrap_or(vec!["Default".to_string()]);
        let groups: Vec<Group> = db.select_player_groups_objects(&raw_groups).await?;
        let power = groups.iter().map(|grp| grp.power).max().unwrap_or(0);
        let mut permissions: Vec<String> = groups.iter().filter_map(|grp| grp.permissions.clone()).flatten().collect();

        permissions.push(format!("power.{}", power));

        if let Some(user_permissions) = self.permissions {
            permissions.extend(user_permissions);
        }

        if let Some(kind_permissions) = &db.select_server_kind_object(kind).await?.map(|kind| kind.permissions).flatten() {
            kind_permissions.iter().filter(|&(group, _kpermissions)| raw_groups.contains(group)).for_each(|(_group, kpermissions)| permissions.extend(kpermissions.clone()));
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
            groups.iter().filter(|&grp| grp.prefix.is_some()).max_by(|&g1, &g2| g1.power.cmp(&g2.power)).map(|grp| grp.prefix.clone()).flatten()
        };

        let suffix = if let Some(suffix) = self.suffix {
            Some(suffix)
        } else {
            groups.iter().filter(|&grp| grp.suffix.is_some()).max_by(|&g1, &g2| g1.power.cmp(&g2.power)).map(|grp| grp.suffix.clone()).flatten()
        };


        let mute = match self.mute {
            None => None,
            Some(mute_id) => db.select_mute(mute_id).await?
        };


        Ok(ServerLoginPlayerInfo {
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
            mute: mute.map(|t| t.into()),
        })
    }
}


impl DbPlayerInfo {
    pub async fn build_player_info(self, db: &Database) -> Result<PlayerInfo, DatabaseError> {
        let groups: Vec<Group> = db.select_player_groups_objects(&self.groups.unwrap_or(vec!["Default".to_string()])).await?;

        let power = groups.iter().map(|grp| grp.power).max().unwrap_or(0);

        let ban = match self.ban {
            None => None,
            Some(ban_id) => db.select_ban(ban_id).await?
        };

        let mute = match self.mute {
            None => None,
            Some(mute_id) => db.select_mute(mute_id).await?
        };

        Ok(PlayerInfo {
            uuid: self.uuid,
            username: self.username,
            power,
            locale: self.locale.unwrap_or("fr".to_string()),
            prefix: self.prefix,
            suffix: self.suffix,
            currency: self.currency,
            premium_currency: self.premium_currency,
            proxy: self.proxy,
            server: self.server,
            blocked: self.blocked.unwrap_or_default(),
            inventory: self.inventory.unwrap_or_default(),
            properties: self.properties.unwrap_or_default(),
            ban: ban.map(DbBan::into),
            discord_id: self.discord_id,
            mute: mute.map(|t| t.into())
        })
    }
}
