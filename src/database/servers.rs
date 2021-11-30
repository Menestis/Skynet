use std::collections::HashMap;
use std::net::IpAddr;
use scylla::FromRow;
use tracing::{instrument, warn};
use uuid::Uuid;
use crate::Database;
use crate::database::DatabaseError;

#[derive(Debug, FromRow)]
pub struct Server {
    pub id: Uuid,
    pub description: String,
    pub ip: IpAddr,
    pub kind: String,
    pub label: String,
    pub state: String,
    pub properties: Option<HashMap<String, String>>,
}

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_server(&self, server: &Server) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.insert_server, (&server.id, &server.description, &server.ip, &server.kind, &server.label, &server.properties, &server.state)).await?;
        Ok(())
    }


    #[instrument(skip(self), level = "debug")]
    pub async fn delete_server(&self, uuid: &Uuid) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.delete_server, (uuid, )).await?;
        Ok(())
    }
}