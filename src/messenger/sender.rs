use lapin::BasicProperties;
use lapin::options::BasicPublishOptions;
use tracing::*;
use crate::Messenger;
use crate::messenger::servers_events::ServerEvent;
use crate::messenger::MessengerError;


impl Messenger {
    pub async fn send_event(&self, event: &ServerEvent) -> Result<(), MessengerError> {
        let data = serde_json::to_vec(event)?;

        let _confirm = self.channel.basic_publish(if event.direct() {
            "direct"
        } else {
            "events"
        }, &event.route(), BasicPublishOptions::default(), &data, BasicProperties::default()).await?.await?;
        trace!("Got confirmation for event {} !", event.route());
        Ok(())
    }
}