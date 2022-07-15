use std::sync::Arc;
use scylla::load_balancing::{RoundRobinPolicy, TokenAwarePolicy};
use scylla::{FromRow, Session, SessionBuilder};
use std::env::{var, VarError};
use std::num::ParseIntError;
use futures::{StreamExt};
use scylla::batch::Consistency;
use scylla::transport::errors::{NewSessionError, QueryError};
use scylla::transport::iterator::NextRowError;
use thiserror::Error;
use tracing::*;
use scylla::cql_to_rust::FromRowError;
use scylla::frame::value::ValueList;
use scylla::prepared_statement::PreparedStatement;
use crate::database::queries::Queries;

mod queries;
pub mod api_keys;
pub mod bans;
pub mod servers;
pub mod players;
pub mod sessions;
pub mod stats;
pub mod discord;
pub mod mutes;

pub struct Database {
    pub session: Session,
    queries: Queries,
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("A required environment variable could not be found : {0}")]
    Var(#[from] VarError),
    #[error(transparent)]
    Session(#[from] NewSessionError),
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    NextRow(#[from] NextRowError),
    #[error(transparent)]
    FromRow(#[from] FromRowError),
    #[error(transparent)]
    ParsingInt(#[from] ParseIntError),
}

#[instrument(name = "database_init", fields(k = field::Empty, addr = field::Empty))]
pub async fn init() -> Result<Database, DatabaseError> {
    let robin = Box::new(RoundRobinPolicy::new());
    let policy = Arc::new(TokenAwarePolicy::new(robin));

    let addr = var("DB_ADDRESS")?;
    let keyspace = var("DB_KEYSPACE")?;
    let user = var("DB_USER")?;
    let passwd = var("DB_PASSWORD")?;

    Span::current().record("keyspace", &keyspace.as_str());

    let session = SessionBuilder::new().known_node(addr).user(user, passwd).load_balancing(policy).use_keyspace(&keyspace, true).default_consistency(Consistency::Quorum).build().await?;
    let queries = Queries::new(&session).await?;
    let database = Database { session, queries };
    Ok(database)
}


impl Database {
    #[instrument(skip(self), level = "debug")]
    pub async fn select_setting(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        //#[query(select_setting = "SELECT value FROM settings WHERE key = ?;")]
        Ok(select_one::<(String, ), _>(&self.queries.select_setting, &self.session, (key, )).await?.map(|t| t.0))
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn insert_setting(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        //#[query(insert_setting = "INSERT INTO settings (key, value) VALUES (?, ?);")]
        execute(&self.queries.insert_setting, &self.session, (key, value)).await
    }
}

async fn select_one<U: FromRow, V: ValueList>(statement: &PreparedStatement, session: &Session, values: V) -> Result<Option<U>, DatabaseError> {
    let rows = session.execute(statement, values).await?.rows;
    Ok(rows.map(|rows| rows.into_iter().last()).flatten().map(|row| row.into_typed::<U>()).transpose()?)
}

async fn execute<V: ValueList>(statement: &PreparedStatement, session: &Session, values: V) -> Result<(), DatabaseError> {
    session.execute(statement, values).await?;
    Ok(())
}

async fn select_iter<U: FromRow, V: ValueList>(statement: &PreparedStatement, session: &Session, values: V) -> Result<Vec<U>, DatabaseError> {
    let mut vector = Vec::new();
    let mut stream = session.execute_iter(statement.clone(), values).await?.into_typed::<U>();

    while let Some(srv) = stream.next().await {
        vector.push(srv?);
    }

    Ok(vector)
}