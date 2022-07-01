use std::collections::HashMap;
use std::net::IpAddr;
use scylla::FromRow;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_iter, select_one};
use serde::{Serialize, Deserialize};

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Server {
    pub id: Uuid,
    pub description: String,
    pub ip: IpAddr,
    pub key: Option<Uuid>,
    pub kind: String,
    pub label: String,
    pub properties: Option<HashMap<String, String>>,
    pub state: String,
}

#[derive(Debug, FromRow)]
pub struct ServerKind {
    pub name: String,
    pub image: String,
    pub permissions: Option<HashMap<String, Vec<String>>>,
    pub autoscale: Option<String>,
}

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_server(&self, server: &Server) -> Result<(), DatabaseError> {
        //#[query(insert_server = "INSERT INTO servers(id, description, ip, key, kind, label, properties, state) VALUES (?, ?, ?, ?, ?, ?, ?, ?);")]
        execute(&self.queries.insert_server, &self.session, (&server.id, &server.description, &server.ip, &server.key, &server.kind, &server.label, &server.properties, &server.state)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_all_servers(&self) -> Result<Vec<Server>, DatabaseError> {
        //#[query(select_all_servers = "SELECT id, description, ip, key, kind, label, properties, state FROM servers;")]
        select_iter(&self.queries.select_all_servers, &self.session, ()).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_all_servers_by_kind(&self, kind: &str) -> Result<Vec<Server>, DatabaseError> {
        //#[query(select_all_servers_by_kind = "SELECT id, description, ip, key, kind, label, properties, state FROM servers_by_kinds WHERE kind = ?;")]
        select_iter(&self.queries.select_all_servers_by_kind, &self.session, (kind, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_player_count_by_server(&self, id: &Uuid) -> Result<i64, DatabaseError> {
        //#[query(select_player_count_by_server = "SELECT count(*) FROM players_by_server WHERE server = ?;")]
        Ok(select_one::<(i64, ), _>(&self.queries.select_player_count_by_server, &self.session, (id, )).await?.map(|t| t.0).unwrap_or(0))
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_label_and_properties(&self, id: &Uuid) -> Result<Option<(String, HashMap<String, String>)>, DatabaseError> {
        //#[query(select_server_label_and_properties = "SELECT label, properties FROM servers WHERE id = ?;")]
        Ok(select_one::<(String, Option<HashMap<String, String>>), _>(&self.queries.select_server_label_and_properties, &self.session, (id, )).await?.map(|(s, m)| (s, m.unwrap_or_default())))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_label(&self, uuid: &Uuid) -> Result<Option<String>, DatabaseError> {
        //#[query(select_server_label = "SELECT label FROM servers WHERE id = ?;")]
        Ok(select_one::<(String, ), _>(&self.queries.select_server_label, &self.session, (uuid, )).await?.map(|t| t.0))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server(&self, id: &Uuid) -> Result<Option<Server>, DatabaseError> {
        //#[query(select_server = "SELECT id, description, ip, key, kind, label, properties, state FROM servers WHERE id = ?;")]
        select_one::<Server, _>(&self.queries.select_server, &self.session, (id, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn delete_server(&self, uuid: &Uuid) -> Result<(), DatabaseError> {
        //#[query(delete_server = "DELETE FROM servers WHERE id = ?;")]
        self.session.execute(&self.queries.delete_server, (uuid, )).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn insert_server_log(&self, id: &Uuid, description: &str, kind: &str, label: &str, properties: &HashMap<String, String>) -> Result<(), DatabaseError> {
        //#[query(insert_server_log = "INSERT INTO servers_logs(id, description, kind, label, properties) VALUES (?, ?, ?, ?, ?);")]
        self.session.execute(&self.queries.insert_server_log, (id, description, kind, label, properties)).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_kind_by_key(&self, key: &Uuid) -> Result<Option<String>, DatabaseError> {
        //#[query(select_server_kind_by_key = "SELECT kind FROM servers_by_key WHERE key = ?;")]
        Ok(select_one::<(Option<String>, ), _>(&self.queries.select_server_kind_by_key, &self.session, (key, )).await?.map(|t| t.0).flatten())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_by_label(&self, label: &str) -> Result<Option<Server>, DatabaseError> {
        //#[query(select_server_by_label = "SELECT id, description, ip, key, kind, label, properties, state FROM servers_by_label WHERE label = ?;")]
        select_one(&self.queries.select_server_by_label, &self.session, (label, )).await
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_kind(&self, id: &Uuid) -> Result<Option<String>, DatabaseError> {
        //#[query(select_server_kind = "SELECT kind FROM servers WHERE id = ?;")]
        Ok(select_one::<(String, ), _>(&self.queries.select_server_kind, &self.session, (id, )).await?.map(|t| t.0))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_kind_and_properties(&self, id: &Uuid) -> Result<Option<(String, HashMap<String, String>)>, DatabaseError> {
        //#[query(select_server_kind_and_properties = "SELECT kind, properties FROM servers WHERE id = ?;")]
        Ok(select_one::<(String, Option<HashMap<String, String>>), _>(&self.queries.select_server_kind_and_properties, &self.session, (id, )).await?.map(|(t, tt)| (t, tt.unwrap_or_default())))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_kind_object(&self, kind: &str) -> Result<Option<ServerKind>, DatabaseError> {
        //#[query(select_server_kind_object = "SELECT name, image, permissions, autoscale FROM servers_kinds WHERE name = ?;")]
        select_one(&self.queries.select_server_kind_object, &self.session, (kind, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_server_key(&self, id: &Uuid, key: &Uuid) -> Result<(), DatabaseError> {
        //#[query(update_server_key = "UPDATE servers SET key = ? WHERE id = ?;")]
        execute(&self.queries.update_server_key, &self.session, (key, id)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_server_state(&self, id: &Uuid, state: &str) -> Result<(), DatabaseError> {
        //#[query(update_server_state = "UPDATE servers SET state = ? WHERE id = ?;")]
        execute(&self.queries.update_server_state, &self.session, (state, id)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_server_description(&self, id: &Uuid, description: &str) -> Result<(), DatabaseError> {
        //#[query(update_server_description = "UPDATE servers SET description = ? WHERE id = ?;")]
        execute(&self.queries.update_server_description, &self.session, (description, id)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_server_playercount(&self, id: &Uuid, count: i32) -> Result<(), DatabaseError> {
        //#[query(update_server_playercount = "UPDATE servers SET online = ? WHERE id = ?;")]
        execute(&self.queries.update_server_playercount, &self.session, (count, id)).await
    }



}