use std::env::{var, VarError};
use std::str::FromStr;
use std::sync::Arc;

use futures::StreamExt;
use lapin::{Channel, Connection, ConnectionProperties, Consumer, Error, ExchangeKind, Queue};
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicRecoverOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::uri::AMQPUri;
use thiserror::Error;
use tokio::select;
use tracing::*;
use uuid::Uuid;

use crate::AppData;

pub mod receiver;
pub mod sender;
pub mod servers_events;
pub mod online_count;

pub struct Messenger {
    con: Connection,
    queue: Queue,
    channel: Channel,
}

#[derive(Error, Debug)]
pub enum MessengerError {
    #[error("A required environment variable could not be found : {0}")]
    Var(#[from] VarError),
    #[error("Could not parse uri : {0}")]
    Parsing(String),
    #[error(transparent)]
    Lapin(#[from] Error),
    #[error("Could not deserialise/serialize data : {0}")]
    Serde(#[from] serde_json::Error),
}


#[instrument(name = "messenger_init", fields(vhost = field::Empty))]
pub async fn init(id: &Uuid) -> Result<Messenger, MessengerError> {
    let uri = AMQPUri::from_str(var("AMQP_ADDRESS")?.as_str()).map_err(|err| MessengerError::Parsing(err))?;

    Span::current().record("vhost", &uri.vhost.as_str());

    let id_str = id.to_string();

    let con = Connection::connect_uri(uri, ConnectionProperties::default().with_connection_name(id_str.clone().into())).await?;

    let channel = con.create_channel().await?;

    let queue = channel.queue_declare(&id_str, QueueDeclareOptions {
        passive: false,
        durable: true,
        exclusive: true,
        auto_delete: true,
        nowait: false,
    }, FieldTable::default()).await?;


    channel.exchange_declare("direct", ExchangeKind::Direct, ExchangeDeclareOptions {
        passive: false,
        durable: true,
        auto_delete: false,
        internal: false,
        nowait: false,
    }, FieldTable::default()).await?;

    channel.exchange_declare("events", ExchangeKind::Topic, ExchangeDeclareOptions {
        passive: false,
        durable: true,
        auto_delete: false,
        internal: false,
        nowait: false,
    }, FieldTable::default()).await?;

    channel.queue_bind(&id_str, "direct", &id_str, QueueBindOptions::default(), FieldTable::default()).await?;
    channel.queue_bind(&id_str, "events", "skynet.#", QueueBindOptions::default(), FieldTable::default()).await?;

    let msgr = Messenger {
        con,
        queue,
        channel,
    };

    Ok(msgr)
}

impl Messenger {
    #[instrument(name = "messenger_task", skip(self, data))]
    pub async fn run_task(&self, data: Arc<AppData>) {
        let mut r = data.shutdown_receiver.clone();

        let consumer = match self.channel.basic_consume(self.queue.name().as_str(), "skynet", BasicConsumeOptions {
            no_local: false,
            no_ack: false,
            exclusive: true,
            nowait: false,
        }, FieldTable::default()).await {
            Ok(consumer) => consumer,
            Err(err) => {
                error!("{}", err);
                data.shutdown().await;
                return;
            }
        };

        select! {
            _ = self.run(consumer, data)  => {},
            _ = r.changed() => {}
        }
        if let Err(err) = self.close().await{
            error!("{}", err)

        }
    }

    async fn close(&self) -> Result<(), lapin::Error>{
        self.channel.close(200, "OK").await?;
        self.con.close(200, "OK").await?;
        Ok(())
    }

    async fn run(&self, mut consumer: Consumer, data: Arc<AppData>) {
        while let Some(delivery_r) = consumer.next().await {
            let delivery = match delivery_r {
                Ok(s) => s,
                Err(err) => {
                    warn!("Rabbitmq error : {}. Attempting recovery !", err);
                    if let Err(err) = self.channel.basic_recover(BasicRecoverOptions::default()).await {
                        error!("Recovery failed : {}. Shutdown initiated ", err);
                        data.shutdown().await;
                        return;
                    } else {
                        info!("Recovered successfully");
                    }
                    continue;
                }
            };
            if let Err(e) = match delivery.ack(BasicAckOptions::default()).await {
                Ok(()) => self.on_message(delivery, data.clone()).await,
                Err(err) => {
                    warn!("Rabbitmq error : {}. Attempting recovery !", err);
                    if let Err(err) = self.channel.basic_recover(BasicRecoverOptions::default()).await {
                        error!("Recovery failed : {}. Shutdown initiated ", err);
                        data.shutdown().await;
                        return;
                    } else {
                        info!("Recovered successfully");
                    }
                    Ok(())
                }
            } {
                error!("Could not process message : {}", e)
            }
        }
    }
}