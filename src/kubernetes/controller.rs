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
use crate::{Database, Messenger};
use crate::log::info;
use uuid::Uuid;
use crate::database::DatabaseError;
use crate::database::servers::Server;

#[derive(Debug, thiserror::Error)]
pub enum K8sWorkerError {
    #[error(transparent)]
    K8S(#[from] Error),
    #[error(transparent)]
    Parsing(#[from] uuid::Error),
    #[error(transparent)]
    AddrParsing(#[from] AddrParseError),
    #[error(transparent)]
    DatabaseError(#[from] DatabaseError)
}

const FINALIZER: &str = "skynet/finalizer";

pub async fn reconcile(pod: Pod, ctx: Context<(Client, Arc<Database>, Arc<Messenger>)>) -> Result<ReconcilerAction, K8sWorkerError> {
    let (client, db, msgr) = ctx.get_ref();

    let ip = pod.status.as_ref().map(|t| t.pod_ip.as_ref()).flatten();
    if !pod.finalizers().contains(&FINALIZER.to_string()) && !pod.labels().contains_key("skynet_id") && ip.is_some() {
        let id = Uuid::new_v4();

        let kind = pod.labels().get("skynet/kind").unwrap().clone();

        db.insert_server(&Server{
            id,
            description: "".to_string(),
            ip: IpAddr::from_str(ip.unwrap())?,
            kind,
            label: "".to_string(),
            state: "Starting".to_string(),
            properties: None
        }).await?;

        //TODO add in database, broadcast route creation
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
        Api::<Pod>::namespaced(client.clone(), &pod.namespace().unwrap_or("default".to_string())).patch(&pod.name(), &PatchParams::default(), &Patch::Merge(&patch)).await?;
        info!("New server {} (@{})", pod.name(), ip.unwrap());
    }


    if let Some(deletion) = pod.metadata.deletion_timestamp.as_ref() {
        if pod.finalizers().contains(&FINALIZER.to_string()) {
            if let Some(d) = pod.labels().get("skynet_id") {
                let uuid = Uuid::parse_str(d.as_str().trim_start_matches("sky-").trim_end_matches("-net"))?;
                info!("Removing server : {}",uuid);

                db.delete_server(&uuid).await?;

                //TODO delete in database, broadcast route deletion

            }
            let mut finalizers: Vec<String> = pod.finalizers().iter().filter(|&x| x != FINALIZER).map(|x| x.clone()).collect();
            let patch = json!({
                "metadata": {
                    "finalizers": finalizers
                }
            });
            Api::<Pod>::namespaced(client.clone(), &pod.namespace().unwrap_or("default".to_string())).patch(&pod.name(), &PatchParams::default(), &Patch::Merge(patch)).await?;
        }
    }

    Ok(ReconcilerAction {
        requeue_after: None,
    })
}

pub fn on_error(error: &K8sWorkerError, ctx: Context<(Client, Arc<Database>, Arc<Messenger>)>) -> ReconcilerAction {
    ReconcilerAction {
        requeue_after: Some(Duration::from_secs(60)),
    }
}

