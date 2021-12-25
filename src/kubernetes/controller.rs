use std::collections::HashMap;
use std::net::{AddrParseError, IpAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, Error, ResourceExt};
use kube::api::{Patch, PatchParams};
use kube::runtime::controller::{Context, ReconcilerAction};
use serde_json::json;
use tracing::error;
use uuid::Uuid;

use crate::{Database, Messenger};
use crate::database::DatabaseError;
use crate::database::servers::Server;
use crate::log::info;
use crate::messenger::MessengerError;
use crate::messenger::servers_events::ServerEvent;

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
}

const FINALIZER: &str = "skynet/finalizer";

pub async fn reconcile(pod: Pod, ctx: Context<(Api<Pod>, Arc<Database>, Arc<Messenger>)>) -> Result<ReconcilerAction, K8sWorkerError> {
    let (pod_api, db, msgr) = ctx.get_ref();

    let ip = pod.status.as_ref().map(|t| t.pod_ip.as_ref()).flatten();
    if !pod.finalizers().contains(&FINALIZER.to_string()) && !pod.labels().contains_key("skynet_id") && ip.is_some() {
        let id = Uuid::new_v4();

        let kind = pod.labels().get("skynet/kind").unwrap().clone();

        let mut properties = HashMap::new();
        pod.labels().iter().filter(|&(k, v)| k.starts_with("skynet-prop/")).for_each(|(k, v)| {
            properties.insert(k.trim_start_matches("skynet-prop/").to_string(), v.to_string());
        });


        let addr = IpAddr::from_str(ip.unwrap())?;
        db.insert_server(&Server {
            id,
            description: "".to_string(),
            ip: addr,
            key: None,
            kind: kind.clone(),
            label: pod.name(),
            state: "Starting".to_string(),
            properties: Some(properties.clone()),
        }).await?;

        if kind != "proxy" {
            msgr.send_event(&ServerEvent::NewRoute { addr, id, description: "".to_string(), name: pod.name(), kind, properties }).await?;
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
        pod_api.patch(&pod.name(), &PatchParams::default(), &Patch::Merge(&patch)).await?;
        info!("New server {} (@{})", pod.name(), ip.unwrap());
    }


    if let Some(deletion) = pod.metadata.deletion_timestamp.as_ref() {
        if pod.finalizers().contains(&FINALIZER.to_string()) {
            if let Some(d) = pod.labels().get("skynet_id") {
                let id = Uuid::parse_str(d.as_str().trim_start_matches("sky-").trim_end_matches("-net"))?;
                info!("Removing server : {}",id);


                msgr.send_event(&ServerEvent::DeleteRoute { id, name: pod.name() }).await?;

                db.delete_server(&id).await?;
            }
            let finalizers: Vec<String> = pod.finalizers().iter().filter(|&x| x != FINALIZER).map(|x| x.clone()).collect();
            let patch = json!({
                "metadata": {
                    "finalizers": finalizers
                }
            });
            pod_api.patch(&pod.name(), &PatchParams::default(), &Patch::Merge(patch)).await?;
        }
    }

    Ok(ReconcilerAction {
        requeue_after: None,
    })
}

pub fn on_error(error: &K8sWorkerError, ctx: Context<(Api<Pod>, Arc<Database>, Arc<Messenger>)>) -> ReconcilerAction {
    ReconcilerAction {
        requeue_after: Some(Duration::from_secs(60)),
    }
}

