use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use chrono::{DateTime, Duration, NaiveDateTime};
use warp::http::StatusCode;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::*;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use warp::body::json;
use crate::utils::proxycheck;
use crate::web::rejections::ApiError;


pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"login")).and(with_auth(data.clone(), "login")).and(with_data(data.clone())).and(json::<LoginRequest>()).and_then(login)
        .or(warp::post().and(path!("api"/"prelogin")).and(with_auth(data.clone(), "pre-login")).and(with_data(data.clone())).and(json::<PreLoginRequest>()).and_then(pre_login))
}

#[derive(Deserialize, Debug)]
struct LoginRequest {}

#[derive(Serialize, Debug)]
struct LoginResponse {}


#[instrument(skip(data))]
async fn login(data: Arc<AppData>, request: LoginRequest) -> Result<impl Reply, Rejection> {
    Ok(StatusCode::OK)
}


#[derive(Deserialize, Serialize, Debug)]
struct PreLoginRequest {
    address: IpAddr,
    username: String,
}

#[derive(Serialize, Debug)]
#[serde(tag = "result")]
enum PreLoginResponse {
    Allowed,
    Denied {
        #[serde(skip_serializing_if = "Option::is_none")]
        reference: Option<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        until: Option<NaiveDateTime>,
        automated: bool,
    },
}

#[instrument(skip(data, request), level = "debug", fields(username = field::Empty))]
async fn pre_login(data: Arc<AppData>, request: PreLoginRequest) -> Result<impl Reply, Rejection> {
    Span::current().record("username", &request.username.as_str());

    //Check if ip is banned in database
    if let Some(ip_ban) = data.db.select_ip_ban(&request.address).await.map_err(|err| ApiError::Database(err))? {
        return Ok(reply::json(&PreLoginResponse::Denied {
            reference: ip_ban.ban,
            reason: if ip_ban.automated { None } else { ip_ban.reason },
            until: if ip_ban.automated { None } else { ip_ban.end.map(|dur| NaiveDateTime::from_timestamp(dur.num_seconds(), 0)) },
            automated: ip_ban.automated,
        }));
    }

    if request.address.is_loopback() {
        return Ok(reply::json(&PreLoginResponse::Allowed));
    }

    //Then check if ip is a vpn
    let proxycheck = match proxycheck::check_ip(&data.client, &data.proxy_check_api_key, &request.address).await {
        Ok(resp) => resp,
        Err(err) => {
            error!("{}", err);
            return Ok(reply::json(&PreLoginResponse::Allowed));
        }
    };

    //Not a risk, allow
    if !proxycheck.is_risk() {
        return Ok(reply::json(&PreLoginResponse::Allowed));
    }

    //Risk, insert ban in database (for a week, so everything is properly cached)
    let ban_id = data.db.insert_ip_ban(&request.address, Some(&proxycheck.to_string()), None, Some(&Duration::days(7)), true).await.map_err(|err| ApiError::Database(err))?;

    //Then return denied
    Ok(reply::json(&PreLoginResponse::Denied {
        reference: Some(ban_id),
        reason: None,
        until: None,
        automated: true,
    }))
}