use std::env;
use std::env::var;
use std::future::Future;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use futures::join;
use reqwest::Client;
use tokio::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::{mpsc, watch};
use tokio::sync::mpsc::Receiver;
use tokio::sync::watch::Sender;
use tracing::*;
use tracing_subscriber::fmt::format::FmtSpan;
use uuid::Uuid;

use crate::database::Database;
use crate::kubernetes::Kubernetes;
use crate::messenger::Messenger;

mod database;
mod messenger;
mod kubernetes;
mod web;
mod utils;
mod structures;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logs();
    let uuid = Uuid::new_v4();
    info!("Starting skynet : ServerId({})", uuid);


    let db = Arc::new(database::init().await?);
    let msgr = Arc::new(messenger::init(&uuid).await?);
    let k8s = Arc::new(kubernetes::init(&uuid, db.clone(), msgr.clone()).await?);

    let (shutdown_task, s, r) = shutdown();

    let data = Arc::new(AppData {
        uuid,
        db,
        msgr,
        k8s,
        client: reqwest::Client::new(),
        proxy_check_api_key: var("PROXYCHECK_API_KEY")?,
        shutdown_sender: s,
        shutdown_receiver: r,
    });

    let addr = SocketAddr::from_str(&var("SKYNET_ADDRESS").unwrap_or("127.0.0.1:8888".to_string()))?;
    let web_task = web::create_task(addr, data.clone()).await;
    let k8s_task = data.k8s.run_task(data.clone());
    let messenger_task = data.msgr.run_task(data.clone());

    join!(shutdown_task, web_task, k8s_task, messenger_task);

    Ok(())
}

pub struct AppData {
    pub uuid: Uuid,
    pub db: Arc<Database>,
    pub msgr: Arc<Messenger>,
    pub k8s: Arc<Kubernetes>,
    pub client: Client,
    pub proxy_check_api_key: String,
    pub shutdown_sender: mpsc::Sender<()>,
    pub shutdown_receiver: watch::Receiver<bool>,
}

impl AppData {
    pub async fn shutdown(&self) {
        if let Err(err) = self.shutdown_sender.send(()).await {
            error!("Could not shutdown app : {}", err)
        }
    }
}

fn init_logs() {
    tracing_subscriber::fmt()
        .with_env_filter(env::var("LOG_LEVEL").unwrap_or("mio=info,lapin=info,pinky_swear=info,scylla=info,tower=info,hyper=info,want=info,warp=info,kube_leader_election=warn,renew_lock=warn,kube=warn,tokio=info,info".to_string()))
        .with_span_events(FmtSpan::CLOSE).init();
}

fn shutdown() -> (impl Future, mpsc::Sender<()>, watch::Receiver<bool>) {
    let (s, r) = tokio::sync::mpsc::channel(1);
    let (s2, r2) = tokio::sync::watch::channel(false);
    (shutdown_task(r, s2), s, r2)
}

async fn shutdown_task(mut r: Receiver<()>, s: Sender<bool>) {
    let mut signal = signal::unix::signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = signal.recv() => {
            info!("Shutdown signal received");
        }
        _ = signal::ctrl_c() => {
            info!("Shutdown requested");
        },
        _ = r.recv() => {
            info!("Shutdown requested by inner component");
        },
    }

    s.send_replace(true);
}
