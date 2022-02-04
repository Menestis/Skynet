use scylla::FromRow;
use tracing::{instrument, warn};
use uuid::Uuid;
use crate::Database;
use crate::database::{DatabaseError, select_one};

#[derive(Debug, FromRow)]
pub struct ApiGroup {
    pub name: String,
    pub permissions: Option<Vec<String>>,
}

impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn has_route_permission(&self, key: Uuid, permission: &str) -> Result<bool, DatabaseError> {
        //#[query(select_api_key = "SELECT key,group FROM api_keys WHERE key = ?;")]
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
                return Ok(true);
            }
            Some(group) => group
        };

        let group = match self.select_api_group(&group).await? {
            None => return Ok(false),
            Some(group) => group
        };

        Ok(group.permissions.as_ref().map(|perms| perms.contains(&permission.to_string())).unwrap_or(false))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn select_api_group(&self, name: &str) -> Result<Option<ApiGroup>, DatabaseError> {
        //#[query(select_api_group = "SELECT name, permissions FROM api_groups WHERE name = ?;")]
        select_one(&self.queries.select_api_group, &self.session, (name, )).await
    }
}