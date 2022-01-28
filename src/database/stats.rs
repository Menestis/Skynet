use std::collections::HashMap;
use chrono::{Duration, Local};
use scylla::batch::Batch;
use scylla::frame::value::Timestamp;
use tracing::*;
use uuid::Uuid;
use crate::Database;
use crate::database::DatabaseError;

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn insert_stats(&self, player: &Uuid, session: &Uuid, server_id: &Uuid, server_kind: &str, stats: &HashMap<String, i32>) -> Result<(), DatabaseError> {
        let mut batch = Batch::default();
        let mut values = vec![];
        let timestamp = Timestamp(Duration::seconds(Local::now().timestamp()));
        for (k, v) in stats {
            //#[query(insert_stats = "INSERT INTO statistics(player, session, timestamp, server_id, server_kind, key, value) VALUES (?, ?, ? ,?, ?, ?, ?);")]
            batch.append_statement(self.queries.insert_stats.clone());
            values.push((player, session, &timestamp, server_id, server_kind, k, v));
        }

        self.session.batch(&batch, values).await?;
        Ok(())
    }
}
