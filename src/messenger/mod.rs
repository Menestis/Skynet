use std::any::Any;
use lapin::{Channel, Connection, ConnectionProperties, Consumer, Error, ExchangeKind, PromiseChain, Queue};
use tracing::*;
use std::env::{var, VarError};
use std::str::FromStr;
use std::string::FromUtf8Error;
use std::sync::Arc;
use futures::StreamExt;
use lapin::message::Delivery;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicRecoverAsyncOptions, BasicRecoverOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::uri::AMQPUri;
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio::sync::watch::Receiver;
use uuid::Uuid;
use crate::AppData;

pub mod receiver;

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

        let mut consumer = match self.channel.basic_consume(self.queue.name().as_str(), "skynet", BasicConsumeOptions {
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
            opt = self.run(consumer, data)  => {},
            _ = r.changed() => {}
        }
    }

    async fn run(&self, mut consumer: Consumer, data: Arc<AppData>) {
        while let Some(delivery) = consumer.next().await {
            let (_channel, delivery) = delivery.expect("error in consumer");
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