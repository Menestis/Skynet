use std::collections::HashMap;
use std::env::var;
use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, Error};
use kube::api::ListParams;
use kube::runtime::Controller;
use kube::runtime::controller::Context;
use kube_leader_election::{LeaseLock, LeaseLockParams};
use tokio::select;
use tokio::sync::{oneshot};
use tokio::sync::oneshot::Sender;
use tokio::time::sleep;
use tracing::*;
use uuid::Uuid;

use crate::{AppData, Database, Messenger};

pub mod controller;
pub mod templates;
pub mod autoscale;
pub mod servers;

pub struct Kubernetes {
    pub client: Client,
    pod_api: Api<Pod>,
    leadership: LeaseLock,
    leader: AtomicBool,
    _namespace: String,
    sender: RwLock<Option<Sender<()>>>,
    database: Arc<Database>,
    messenger: Arc<Messenger>,
    online_player_count: Arc<tokio::sync::RwLock<HashMap<Uuid, i32>>>
}

#[instrument(name = "k8s_init", skip(database, messenger), fields(version = field::Empty))]
pub async fn init(id: &Uuid, database: Arc<Database>, messenger: Arc<Messenger>, online_player_count: Arc<tokio::sync::RwLock<HashMap<Uuid, i32>>>) -> Result<Kubernetes, Error> {
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

    let pod_api = Api::<Pod>::namespaced(client.clone(), &namespace);
    Ok(Kubernetes {
        client,
        pod_api,
        leadership,
        leader: AtomicBool::new(false),
        _namespace: namespace,
        sender: RwLock::new(None),
        database,
        messenger,
        online_player_count
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
    pub async fn run_task(&self, data: Arc<AppData>) {
        let mut r = data.shutdown_receiver.clone();

        select! {
            _ = self.run() => {}
            _ = r.changed() => {}
        }

        self.stop_control_loop().await;
    }

    pub async fn run(&self) {
        loop {
            tokio::task::yield_now().await;
            self.renew_lock().await;
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn start_control_loop(&self) {
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

        let controller = Controller::new(self.pod_api.clone(), ListParams::default()
            .labels("managed_by == skynet,skynet/kind")).graceful_shutdown_on(async move { if let Err(err) = receiver.await { error!("{}",err) } });

        tokio::spawn(controller.run(controller::reconcile, controller::on_error, Context::new((self.pod_api.clone(), self.database.clone(), self.messenger.clone(), self.online_player_count.clone()))).for_each(|res| async move {
            match res {
                Ok(o) => trace!("Reconciled {}", o.0.name),
                Err(e) => warn!("Reconcile failed: {:?}", e),
            }
        }).instrument(debug_span!("controller")));

        Ok(())
    }
}