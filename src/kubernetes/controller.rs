use std::collections::HashMap;
use std::net::{AddrParseError, IpAddr};
use std::str::FromStr;
use std::sync::{Arc};
use std::time::Duration;

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Error, ResourceExt};
use kube::api::{Patch, PatchParams};
use kube::runtime::controller::Action;
use serde_json::json;
use tokio::sync::RwLock;
use tracing::error;
use uuid::Uuid;

use crate::{Database, Messenger};
use crate::database::DatabaseError;
use crate::database::servers::Server;
use crate::log::info;
use crate::messenger::MessengerError;
use crate::messenger::servers_events::ServerEvent;
use crate::web::echo::ECHO_URL;

#[derive(Debug, thiserror::Error)]
pub enum K8sWorkerError {
    #[error(transparent)]
    K8S(#[from] Error),
    #[error(transparent)]
    Parsing(#[from] uuid::Error),
    #[error(transparent)]
    AddrParsing(#[from] AddrParseError),
    #[error(transparent)]
    DatabaseError(#[from] DatabaseError),
    #[error(transparent)]
    MessengerError(#[from] MessengerError),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

const FINALIZER: &str = "skynet/finalizer";

pub async fn reconcile(pod: Arc<Pod>, ctx: Arc<(Api<Pod>, Arc<Database>, Arc<Messenger>, Arc<RwLock<HashMap<Uuid, i32>>>, Arc<reqwest::Client>, Uuid)>) -> Result<Action, K8sWorkerError> {
    let (pod_api, db, msgr, online_player_count, client, echo_key) = ctx.as_ref();

    let ip = pod.status.as_ref().map(|t| t.pod_ip.as_ref()).flatten();
    if !pod.finalizers().contains(&FINALIZER.to_string()) && !pod.labels().contains_key("skynet_id") && ip.is_some() {
        let id = Uuid::new_v4();

        let kind = pod.labels().get("skynet/kind").unwrap().clone();

        let mut properties = HashMap::new();
        pod.labels().iter().filter(|&(k, _v)| k.starts_with("skynet-prop/")).for_each(|(k, v)| {
            properties.insert(k.trim_start_matches("skynet-prop/").to_string(), v.to_string());
        });


        let addr = IpAddr::from_str(ip.unwrap())?;
        db.insert_server(&Server {
            id,
            description: "".to_string(),
            ip: addr,
            key: None,
            kind: kind.clone(),
            label: pod.name_any(),
            state: "Starting".to_string(),
            properties: Some(properties.clone()),
            online: 0,
        }).await?;

        if kind != "proxy" {
            msgr.send_event(&ServerEvent::NewRoute { addr, id, description: "".to_string(), name: pod.name_any(), kind, properties }).await?;
        }

        let mut finalizers = Vec::from(pod.finalizers());
        finalizers.push(FINALIZER.to_string());

        let patch = json!({
            "metadata": {
                "finalizers": finalizers,
                "labels": {
                    "skynet_id": format!("sky-{}-net",id)
                }
            }
        });
        pod_api.patch(&pod.name_any(), &PatchParams::default(), &Patch::Merge(&patch)).await?;
        info!("New server {} (@{})", pod.name_any(), ip.unwrap());
    }


    if let Some(_deletion) = pod.metadata.deletion_timestamp.as_ref() {
        if pod.finalizers().contains(&FINALIZER.to_string()) {
            if let Some(d) = pod.labels().get("skynet_id") {
                let id = Uuid::parse_str(d.as_str().trim_start_matches("sky-").trim_end_matches("-net"))?;
                info!("Removing server : {}",id);


                msgr.send_event(&ServerEvent::DeleteRoute { id, name: pod.name_any() }).await?;

                let server = db.select_server(&id).await?;
                if let Some(server) = server {
                    online_player_count.write().await.remove(&id);
                    let online: i32 = online_player_count.read().await.values().sum();
                    if let Err(e) = db.insert_setting("online_count", &online.to_string()).await {
                        error!("{}", e);
                    }
                    if server.kind == "proxy" {
                        for pl in db.select_online_players_reduced_info().await? {
                            if pl.proxy == server.id {
                                info!("Force closing {} session (due to {} shutdown)",pl.username, pl.proxy);
                                db.close_player_session(&pl.uuid).await?;
                            }
                        }
                    }

                    if let Some(key) = db.select_server_echo_key(&server.id).await? {
                        info!("Removing echo server : {}", key);
                        client.delete(format!("{}/servers/{}", ECHO_URL, key)).header("Authorization",echo_key.to_string()).send().await.map_err(K8sWorkerError::from)?;
                    }

                    db.delete_server(&id).await?;
                    db.insert_server_log(&server.id, &server.description, &server.kind, &server.label, &server.properties.unwrap_or_default()).await?;
                }
            }
            let finalizers: Vec<String> = pod.finalizers().iter().filter(|&x| x != FINALIZER).map(|x| x.clone()).collect();
            let patch = json!({
                "metadata": {
                    "finalizers": finalizers
                }
            });
            pod_api.patch(&pod.name_any(), &PatchParams::default(), &Patch::Merge(patch)).await?;
        }
    }

    Ok(Action::await_change())
}

pub fn on_error(_error: &K8sWorkerError, _ctx: Arc<(Api<Pod>, Arc<Database>, Arc<Messenger>, Arc<RwLock<HashMap<Uuid, i32>>>,Arc<reqwest::Client>, Uuid)>) -> Action {
    Action::requeue(Duration::from_secs(60))
}

