use std::num::ParseIntError;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::error;
use warp::http::StatusCode;
use warp::reject::Reject;
use warp::{Rejection, Reply};
use warp::reply::{Response, with_status};
use crate::database::DatabaseError;
use crate::messenger::MessengerError;

pub async fn handle_rejection(err: Rejection) -> Result<Response, Rejection> {
    if err.is_not_found() {
        Ok(StatusCode::NOT_FOUND.into_response())
    } else if let Some(e) = err.find::<ApiError>() {
        e.log_if_needed();
        Ok(e.as_response())
    } else {
        Err(err)
    }
}


#[derive(Error, Debug)]
pub enum ApiError {
    #[error("An internal server error occurred : {0}")]
    Failure(String),
    #[error("You are not authorized to use this endpoint")]
    Authorization,
    #[error("An internal server error occurred : {0}")]
    Database(#[from] DatabaseError),
    #[error("Could not send signal on channel : {0}")]
    Channel(#[from] mpsc::error::SendError<()>),
    #[error("Could not parse uuid : {0}")]
    UUID(#[from] uuid::Error),
    #[error("Could not send message : {0}")]
    MessengerError(#[from] MessengerError),
    #[error("Could not parse int : {0}")]
    ParsingInt(#[from] ParseIntError),
    #[error("Kubernetes error : {0}")]
    Kubernetes(#[from] kube::Error)
}


impl Reject for ApiError {}

impl ApiError {
    pub fn as_response(&self) -> Response {
        match self {
            ApiError::Authorization => with_status(self.to_string(), StatusCode::UNAUTHORIZED).into_response(),
            _ => with_status("An internal server error occurred", StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        }
    }
    pub fn log_if_needed(&self) {
        match self {
            ApiError::Authorization => {},
            _ => error!("{}", self)
        }
    }
}