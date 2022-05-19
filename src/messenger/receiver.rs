use std::sync::Arc;
use lapin::message::Delivery;
use crate::{AppData, Messenger};
use tracing::*;
use crate::messenger::servers_events::ServerEvent;

impl Messenger {
    #[instrument(name = "messenger_task", skip(self, data, delivery), level = "trace")]
    pub async fn on_message(&self, delivery: Delivery, data: Arc<AppData>) -> anyhow::Result<()> {
        let msg: ServerEvent = match serde_json::from_str(&match String::from_utf8(delivery.data) {
            Ok(msg) => {
                trace!("Msg : {}", msg);
                msg
            }
            Err(err) => {
                warn!("Received non utf-8 message : {}", err);
                return Ok(());
            }
        }) {
            Ok(msg) => msg,
            Err(err) => {
                warn!("Unknown message : {}", err);
                return Ok(());
            }
        };

        match msg {
            ServerEvent::PlayerCountSync { proxy, count } => {
                super::online_count::process_online_count(data.clone(),proxy, count).await;
            }
            _ => {}
        }



        Ok(())
    }
}