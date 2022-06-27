use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, execute, select_one};

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_discord_link(&self, code: &str, player: &Uuid) -> Result<(), DatabaseError> {
        //#[query(insert_discord_link = "INSERT INTO discords_link (code, uuid) VALUES (?, ?) USING TTL 600")]
        execute(&self.queries.insert_discord_link, &self.session, (code, player)).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn delete_discord_link(&self, code: &str) -> Result<(), DatabaseError> {
        //#[query(delete_discord_link = "DELETE FROM discords_link WHERE code = ?;")]
        execute(&self.queries.delete_discord_link, &self.session, (code, )).await
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_discord_link(&self, code: &str) -> Result<Option<Uuid>, DatabaseError> {
        //#[query(select_discord_link = "SELECT uuid FROM discords_link WHERE code = ?;")]
        Ok(select_one::<(Uuid, ), _>(&self.queries.select_discord_link, &self.session, (code, )).await?.map(|t| t.0))
    }

}
