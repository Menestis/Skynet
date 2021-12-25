use std::collections::HashMap;
use chrono::{Duration, Local};
use scylla::batch::{Batch, BatchType};
use scylla::frame::value::Timestamp;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::DatabaseError;

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_stats(&self, player: &Uuid, session: &Uuid, server_kind: &str, stats: &HashMap<String, i32>) -> Result<(), DatabaseError> {
        let mut batch = Batch::default();
        let mut values = vec![];
        let timestamp = Timestamp(Duration::seconds(Local::now().timestamp()));
        for (k, v) in stats {
            batch.append_statement(self.queries.insert_stats.clone());
            values.push((player, session, &timestamp, server_kind, k, v));
        }

        self.session.batch(&batch, values).await?;
        Ok(())
    }
}
