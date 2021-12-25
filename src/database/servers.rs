use std::collections::HashMap;
use std::net::IpAddr;
use futures::StreamExt;
use scylla::FromRow;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::DatabaseError;
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

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_server(&self, server: &Server) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.insert_server, (&server.id, &server.description, &server.ip, &server.key, &server.kind, &server.label, &server.properties, &server.state)).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_all_servers(&self) -> Result<Vec<Server>, DatabaseError> {
        let mut servers = Vec::new();
        let mut stream = self.session.execute_iter(self.queries.select_all_servers.clone(), ()).await?.into_typed::<Server>();

        while let Some(srv) = stream.next().await {
            servers.push(srv?);
        }

        Ok(servers)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_label_and_properties(&self, id: &Uuid) -> Result<Option<(String, HashMap<String, String>)>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_server_label_and_properties, (id, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<(String, Option<HashMap<String, String>>)>()).transpose()?.map(|(label, properties)| (label, properties.unwrap_or_default())))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_label(&self, id: &Uuid) -> Result<Option<String>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_server_label, (id, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<(String, )>()).transpose()?.map(|t| t.0))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn delete_server(&self, uuid: &Uuid) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.delete_server, (uuid, )).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_kind_by_key(&self, key: &Uuid) -> Result<Option<String>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_server_kind_by_key, (key, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<(Option<String>, )>()).transpose()?.map(|(x, )| x).flatten())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_by_label(&self, label: &str) -> Result<Option<Server>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_server_by_label, (label, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<Server>()).transpose()?)
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn select_server_kind(&self, id: &Uuid) -> Result<Option<String>, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_server_kind, (id, )).await?.rows;
        Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<(String, )>()).transpose()?.map(|(x, )| x))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_server_key(&self, id: &Uuid, key: &Uuid) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.update_server_key, (key, id)).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn update_server_state(&self, id: &Uuid, state: &str) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.update_server_state, (state, id)).await?;
        Ok(())
    }
}