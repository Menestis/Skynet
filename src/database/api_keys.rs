use tracing::{instrument, warn};
use uuid::Uuid;
use crate::Database;
use crate::database::DatabaseError;

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn has_route_permission(&self, key: Uuid, permission: String) -> Result<bool, DatabaseError> {
        let rows = self.session.execute(&self.queries.select_api_key, (key, )).await?.rows;

        let rows = match rows {
            None => return Ok(false),
            Some(rows) => rows
        };


        let row = match rows.into_iter().last() {
            None => return Ok(false),
            Some(row) => row
        };

        let group = row.into_typed::<(Uuid, Option<String>)>()?.1;


        let group = match group {
            None => {
                warn!("Unrestricted api key usage ({}), usage of this type of key should be avoided", key);
                return Ok(true)
            },
            Some(group) => group
        };

        let group = match self.cache.api_groups.get(&group) {
            None => return Ok(false),
            Some(group) => group
        };

        Ok(group.permissions.as_ref().map(|perms| perms.contains(&permission)).unwrap_or(false))
    }
}