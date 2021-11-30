use std::convert::Infallible;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::{Filter, path, Rejection, Reply};
use crate::AppData;
use tracing::instrument;

struct Processor{
    name: &'static str,
    usage: f32,
    frequency: u64,
}

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone{
    warp::get().and(path!("status")).and_then(status)
}


#[instrument(level = "debug")]
pub async fn status() -> Result<impl Reply, Infallible> {
    Ok(StatusCode::OK)
}
