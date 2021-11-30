use std::collections::HashMap;
use std::env::var;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LockResult, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use tokio::sync::oneshot::{Receiver, Sender};
use k8s_openapi::api::core::v1::{Pod, PodStatus};
use tracing::*;
use kube::{Api, Client, Error, ResourceExt};
use kube::api::{ListParams, Patch, PatchParams};
use kube::runtime::Controller;
use kube::runtime::controller::{Context, ReconcilerAction};
use kube_leader_election::{LeaseLock, LeaseLockParams};
use tokio::time::sleep;
use uuid::Uuid;
use futures::{channel, StreamExt};
use serde_json::json;
use tokio::select;
use tokio::sync::{oneshot, watch};
use crate::{AppData, Database, Messenger};

mod controller;

type Servers = Arc<RwLock<HashMap<Uuid, ()>>>;

pub struct Kubernetes {
    pub client: Client,
    leadership: LeaseLock,
    leader: AtomicBool,
    namespace: String,
    sender: RwLock<Option<Sender<()>>>,
    database: Arc<Database>,
    messenger: Arc<Messenger>,
}

#[instrument(name = "k8s_init", skip(database, messenger), fields(version = field::Empty))]
pub async fn init(id: &Uuid, database: Arc<Database>, messenger: Arc<Messenger>) -> Result<Kubernetes, Error> {
    let client = Client::try_default().await?;
    let info = client.apiserver_version().await?;
    Span::current().record("version", &format!("{}.{}", info.major, info.minor).as_str());

    let namespace = var("KUBERNETES_NAMESPACE").unwrap_or("skynet".to_string());

    let leadership = LeaseLock::new(
        client.clone(),
        &namespace,
        LeaseLockParams {
            holder_id: id.to_string(),
            lease_name: "skynet-lease".into(),
            lease_ttl: Duration::from_secs(15),
        },
    );

    Ok(Kubernetes {
        client,
        leadership,
        leader: AtomicBool::new(false),
        namespace,
        sender: RwLock::new(None),
        database,
        messenger,
    })
}

impl Kubernetes {
    async fn renew_lock(&self) {
        match self.leadership.try_acquire_or_renew().await {
            Ok(ll) => {
                let was_leader = self.leader.load(Ordering::Relaxed);
                self.leader.store(ll.acquired_lease, Ordering::Relaxed);

                if !was_leader && ll.acquired_lease {
                    info!("Got lease, scheduling control loop !");
                    self.start_control_loop().await;
                } else if was_leader && !ll.acquired_lease {
                    //Lost lease
                    info!("Lost lease, disabling control loop !");
                    self.stop_control_loop().await;
                }
            }
            Err(err) => error!("{:?}", err),
        }
    }

    pub fn is_leader(&self) -> bool {
        self.leader.load(Ordering::Relaxed)
    }

    #[instrument(name = "k8s_task", skip(self, data))]
    pub async fn run_task(&self, mut data: Arc<AppData>) {
        let mut r = data.shutdown_receiver.clone();

        select! {
            _ = self.run() => {}
            _ = r.changed() => {}
        }

        self.stop_control_loop().await;
    }
    pub async fn run(&self) {
        loop {
            self.renew_lock().await;
            sleep(Duration::from_secs(5)).await;
        }
    }


    async fn start_control_loop(&self) {
        //Got lease
        if let Err(err) = self.start_controller().await {
            error!("{}", err);
            self.leader.store(false, Ordering::Relaxed);
        }
        debug!("Control loop started !");
    }

    async fn stop_control_loop(&self) {
        match self.sender.write() {
            Ok(mut guard) => {
                match guard.take() {
                    None => {}
                    Some(sender) => { let _ = sender.send(()); }
                }
            }
            Err(e) => {
                error!("{}",e)
            }
        };
    }


    async fn start_controller(&self) -> Result<(), PoisonError<RwLockWriteGuard<'_, Option<Sender<()>>>>> {
        let (sender, receiver) = oneshot::channel::<()>();
        {
            let mut guard = self.sender.write()?;
            if guard.is_some() {
                return Ok(());
            }
            *guard = Some(sender);
        }

        let controller = Controller::new(Api::namespaced(self.client.clone(), &self.namespace), ListParams::default()
            .labels("managed_by == skynet,skynet/kind")).graceful_shutdown_on(async move { if let Err(err) = receiver.await { error!("{}",err) } });

        tokio::spawn(controller.run(controller::reconcile, controller::on_error, Context::new((self.client.clone(), self.database.clone(), self.messenger.clone()))).for_each(|res| async move {
            match res {
                Ok(o) => debug!("reconciled {}", o.0.name),
                Err(e) => warn!("reconcile failed: {:?}", e),
            }
        }).instrument(debug_span!("controller")));

        Ok(())
    }
}