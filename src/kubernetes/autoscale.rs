use std::collections::HashMap;
use std::num::ParseIntError;
use std::sync::Arc;
use kube::Error::Api;
use thiserror::Error;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};
use uuid::Uuid;
use crate::AppData;
use crate::database::DatabaseError;
use crate::database::servers::{Server, ServerKind};
use crate::messenger::MessengerError;
use crate::messenger::servers_events::ServerEvent;
use crate::web::rejections::ApiError;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Autoscale {
    Simple {
        slots: i64,
        #[serde(default)]
        properties: HashMap<String, String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default)]
        min: i32,
    },
}

#[derive(Error, Debug)]
pub enum ScalingError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Kubernetes(#[from] kube::Error),
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Messenger(#[from] MessengerError)
}

#[instrument(skip(data))]
pub async fn process_idling(data: Arc<AppData>, uuid: Uuid) -> Result<(), ScalingError> {
    let server = match data.db.select_server(&uuid).await.map_err(ScalingError::from)? {
        None => return Ok(()),
        Some(kind) => kind
    };

    if server.properties.map(|props| props.get("canidle").map(|b| b == "false")).flatten().unwrap_or(false) {
        info!("Server {} cannot idle, deleting...", server.id);
        data.k8s.delete_pod(&server.label).await.map_err(ScalingError::from)?;
        return Ok(());
    }

    let autoscale = match data.db.select_server_kind_object(&server.kind).await.map_err(ScalingError::from)?.map(|k| k.autoscale).flatten() {
        None => return Ok(()),
        Some(autoscale) => { serde_json::from_str::<Autoscale>(&autoscale).map_err(ScalingError::from) }
    }?;

    let min = match autoscale { Autoscale::Simple { min, .. } => min };
    if min == 0 {
        info!("Autoscaling deletion for kind {} : {}", server.kind, server.label);
        data.k8s.delete_pod(&server.label).await.map_err(ScalingError::from)?;
    } else {
        let servers: Vec<Server> = data.db.select_all_servers_by_kind(&server.kind).await.map_err(ScalingError::from)?.into_iter().filter(|x| x.id != uuid && (x.state == "Waiting" || x.state == "Idle")).collect();
        if servers.len() >= min as usize {
            info!("Autoscaling deletion for kind {} : {}", server.kind, server.label);
            data.k8s.delete_pod(&server.label).await.map_err(ScalingError::from)?;
        }
    }

    Ok(())
}

#[instrument(skip(data))]
pub async fn process_waiting(data: Arc<AppData>, server: Uuid) -> Result<(), ScalingError> {
    let (kind, properties) = match data.db.select_server_kind_and_properties(&server).await.map_err(ScalingError::from)? {
        None => return Ok(()),
        Some(kind) => kind
    };

    let kind_obj = match data.db.select_server_kind_object(&kind).await.map_err(ScalingError::from)? {
        None => return Ok(()),
        Some(kind) => kind
    };
    let autoscale_opt = kind_obj.autoscale.as_ref().map(|t| serde_json::from_str::<Autoscale>(t)).transpose().map_err(ScalingError::from)?;


    let slots = if let Some(slots) = properties.get("slots").map(|t| t.parse::<i64>()).transpose().map_err(ScalingError::from)? {
        slots
    } else if let Some(autoscale) = autoscale_opt.as_ref() {
        *match autoscale { Autoscale::Simple { slots, .. } => slots }
    } else {
        100
    };

    let mut players = data.db.select_all_players_with_waiting_move_to(&kind, (slots + 1) as i32).await.map_err(ScalingError::from)?;
    if players.len() == slots as usize + 1 && autoscale_opt.is_some() {
        players.remove(slots as usize);

        debug!("Autoscaling new burst server for kind : {}", kind);

        create_autoscale_server(data.clone(), &kind_obj, &autoscale_opt.unwrap()).await.map_err(ApiError::from)?;
    }
    for pl in players {
        data.msgr.send_event(&ServerEvent::MovePlayer {
            proxy: pl.proxy,
            server,
            player: pl.uuid,
        }).await.map_err(ScalingError::from)?;
    }

    Ok(())
}

#[instrument(skip(data))]
pub async fn create_autoscale_server(data: Arc<AppData>, kind: &ServerKind, autoscale: &Autoscale) -> Result<String, ScalingError> {
    let name = format!("{}-{}", kind.name, rand::thread_rng().gen_range(10000..99999));

    info!("Autoscaling new server {} for {}", name, kind.name);

    let (env, mut properties) = match autoscale { Autoscale::Simple { env, properties, .. } => (env.clone(), properties.clone()) };
    properties.insert("autoscale".to_string(), "true".to_string());


    data.k8s.create_pod(&kind.name, &kind.image,
                        &name,
                        properties, env)
        .await.map_err(ScalingError::from)?;

    Ok(name)
}