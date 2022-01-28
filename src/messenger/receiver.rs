use std::sync::Arc;
use lapin::message::Delivery;
use crate::{AppData, Messenger};
use tracing::*;

impl Messenger {
    #[instrument(name = "messenger_task", skip(self, _data, delivery), level = "trace")]
    pub async fn on_message(&self, delivery: Delivery, _data: Arc<AppData>) -> anyhow::Result<()> {
        let _msg = match serde_json::from_str(&match String::from_utf8(delivery.data) {
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

        Ok(())
    }
}