use std::collections::HashMap;
use std::net::IpAddr;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::DatabaseError;

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_session(&self, id: &Uuid, brand: &str, version: &str, player: &Uuid, ip: &IpAddr, mods: &HashMap<String, String>) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.insert_session, (id, brand, ip, mods, player, version)).await?;
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn close_session(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.session.execute(&self.queries.close_session, (id, )).await?;
        Ok(())
    }
}
