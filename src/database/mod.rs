use std::collections::HashMap;
use std::sync::Arc;
use scylla::load_balancing::{RoundRobinPolicy, TokenAwarePolicy};
use scylla::{FromRow, Session, SessionBuilder};
use std::env::{var, VarError};
use futures::{StreamExt};
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

pub struct Database {
    session: Session,
    cache: Cache,
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

    let session = SessionBuilder::new().known_node(addr).user(user, passwd).load_balancing(policy).use_keyspace(&keyspace, true).build().await?;
    let cache = prepare_cache(&session).await?;
    let queries = Queries::new(&session).await?;
    let database = Database { session, cache, queries };
    Ok(database)
}

#[derive(Debug)]
struct Cache {
    settings: HashMap<String, String>,
    groups: HashMap<String, Group>,
    api_groups: HashMap<String, ApiGroup>,
    servers_kinds: HashMap<String, ServerKind>,
}

#[derive(Debug, FromRow)]
pub struct Group {
    pub name: String,
    pub power: i32,
    prefix: Option<String>,
    suffix: Option<String>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, FromRow)]
pub struct ApiGroup {
    pub name: String,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, FromRow)]
pub struct ServerKind {
    pub name: String,
    pub image: String,
    pub permissions: Option<HashMap<String, Vec<String>>>,
    pub autoscale: Option<String>,
}


#[instrument(skip(s))]
async fn prepare_cache(s: &Session) -> Result<Cache, DatabaseError> {
    let settings: Result<_, DatabaseError> = async {
        let mut settings = HashMap::new();
        let mut it = s.query_iter("SELECT key,value FROM settings", ()).await?
            .into_typed::<(String, String)>();
        while let Some(res) = it.next().await {
            let (k, v) = res?;
            settings.insert(k, v);
        }
        Ok(settings)
    }.instrument(debug_span!("settings")).await;

    let groups: Result<_, DatabaseError> = async {
        let mut groups = HashMap::new();
        let mut it = s.query_iter("SELECT name,power,prefix,suffix,permissions FROM groups", ()).await?
            .into_typed::<Group>();
        while let Some(res) = it.next().await {
            let group = res?;
            trace!("Loaded {:?}", group);
            groups.insert(group.name.clone(), group);
        }
        Ok(groups)
    }.instrument(debug_span!("groups")).await;

    let api_groups: Result<_, DatabaseError> = async {
        let mut api_groups = HashMap::new();
        let mut it = s.query_iter("SELECT name, permissions FROM api_groups", ()).await?
            .into_typed::<ApiGroup>();
        while let Some(res) = it.next().await {
            let api_group = res?;
            trace!("Loaded {:?}", api_group);
            api_groups.insert(api_group.name.clone(), api_group);
        }
        Ok(api_groups)
    }.instrument(debug_span!("api_groups")).await;

    let servers_kinds: Result<_, DatabaseError> = async {
        let mut servers_kinds = HashMap::new();
        let mut it = s.query_iter("SELECT name, image, permissions, autoscale FROM servers_kinds", ()).await?
            .into_typed::<ServerKind>();
        while let Some(res) = it.next().await {
            let servers_kind = res?;
            trace!("Loaded {:?}", servers_kind);
            servers_kinds.insert(servers_kind.name.clone(), servers_kind);
        }
        Ok(servers_kinds)
    }.instrument(debug_span!("servers_kinds")).await;

    Ok(Cache { settings: settings?, groups: groups?, api_groups: api_groups?, servers_kinds: servers_kinds? })
}

impl Database {
    pub fn _get_cached_group(&self, group: &str) -> Option<&Group> {
        self.cache.groups.get(group)
    }
    pub fn get_cached_kind(&self, kind: &str) -> Option<&ServerKind> {
        self.cache.servers_kinds.get(kind)
    }
    pub fn get_cached_api_group(&self, group: &str) -> Option<&ApiGroup> {
        self.cache.api_groups.get(group)
    }

    pub fn get_cached_settings(&self) -> &HashMap<String, String> {
        &self.cache.settings
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