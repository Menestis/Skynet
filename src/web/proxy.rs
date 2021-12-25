use std::collections::HashMap;
use std::net::{IpAddr};
use std::ops::Add;
use std::sync::Arc;
use chrono::{Duration, Local};
use kube::Error::Api;
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::*;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use warp::body::json;
use warp::hyper::StatusCode;
use crate::database::players::DbProxyPlayerInfo;
use crate::database::servers::Server;
use crate::utils::message::{Color, Message, MessageBuilder};
use crate::utils::proxycheck;
use crate::web::rejections::ApiError;


pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"procedure"/"proxy"/"login")).and(with_auth(data.clone(), "proxy-login")).and(with_data(data.clone())).and(json::<ProxyLoginRequest>()).and_then(login)
        .or(warp::post().and(path!("api"/"procedure"/"proxy"/"prelogin")).and(with_auth(data.clone(), "proxy-pre-login")).and(with_data(data.clone())).and(json::<ProxyPreLoginRequest>()).and_then(pre_login))
        .or(warp::post().and(path!("api"/"procedure"/"proxy"/"close")).and(with_auth(data.clone(), "proxy-close-session")).and(with_data(data.clone())).and(json::<ProxyCloseSessionRequest>()).and_then(close_session))
        .or(warp::get().and(path!("api"/"procedure"/"proxy"/"init")).and(with_auth(data.clone(), "proxy-init")).and(with_data(data.clone())).and_then(init))
        .or(warp::get().and(path!("api"/"procedure"/"proxy"/"ping")).and(with_auth(data.clone(), "proxy-ping")).and(with_data(data.clone())).and_then(ping))
}

////////////////////
//      Login     //
////////////////////

#[derive(Deserialize, Debug)]
struct ProxyLoginRequest {
    uuid: Uuid,
    username: String,
    proxy: Uuid,
    ip: IpAddr,
    version: MinecraftVersion,
    locale: Option<String>,
}

#[derive(Deserialize, Debug)]
struct MinecraftVersion {
    brand: String,
    version: String,
    mods: Option<Vec<ModInfo>>,
}

#[derive(Deserialize, Debug)]
struct ModInfo {
    id: String,
    version: String,
}


#[derive(Serialize, Debug)]
#[serde(tag = "result")]
enum ProxyLoginResponse {
    Allowed {
        session: Uuid,
        player_info: ProxyLoginPlayerInfo,
    },
    Denied {
        message: Message
    },
}

#[derive(Serialize, Debug)]
pub struct ProxyLoginPlayerInfo {
    pub power: i32,
    pub permissions: Vec<String>,
    pub locale: String,
    pub properties: HashMap<String, String>,
}


#[instrument(skip(data))]
async fn login(data: Arc<AppData>, request: ProxyLoginRequest) -> Result<impl Reply, Rejection> {
    let option = data.db.select_proxy_player_info(&request.uuid).await.map_err(ApiError::from)?;

    let info = match option {
        None => {
            data.db.insert_player(&request.uuid, &request.username, None).await.map_err(ApiError::from)?;
            DbProxyPlayerInfo {
                locale: None,
                groups: None,
                permissions: None,
                properties: None,
                ban: None,
                ban_reason: None,
                ban_ttl: None,
            }.build_proxy_login_player_info(&data.db)
        }
        Some(info) if info.ban.is_some() => {
            let builder = MessageBuilder::new()
                .component("SkyNet ".to_string()).with_color(Some(Color::DarkPurple)).close()
                .component("> ".to_string()).with_color(Some(Color::DarkGray)).close()
                .component("Connection impossible...".to_string()).with_color(Some(Color::Red)).close()
                .line_break()
                .component(format!("Vous avez été bannis pour : {}", info.ban_reason.unwrap_or("non spécifié".to_string()))).close()
                .line_break()
                .component(format!("Référence : {}", info.ban.unwrap())).close()
                .line_break()
                .component(match info.ban_ttl {
                    None => {
                        "Votre sanction ne prendra jamais fin".to_string()
                    }
                    Some(time) => {
                        format!("Votre sanction prendra fin le {}", Local::now().add(Duration::seconds(time as i64)).format("%c"))
                    }
                }).close();
            return Ok(reply::json(&ProxyLoginResponse::Denied { message: builder.close() }));
        }
        Some(info) => {
            info.build_proxy_login_player_info(&data.db)
        }
    };


    let session = Uuid::new_v4();

    let mods = {
        let mut map = HashMap::new();
        if let Some(mods) = request.version.mods {
            for m in mods {
                map.insert(m.id, m.version);
            }
        }
        map
    };

    data.db.insert_session(&session, &request.version.brand, &request.version.version, &request.uuid, &request.ip, &mods).await.map_err(ApiError::from)?;

    data.db.update_player_online_proxy_info(&request.uuid, request.proxy, &session, &request.username).await.map_err(ApiError::from)?;

    Ok(reply::json(&ProxyLoginResponse::Allowed { session, player_info: info }))
}


////////////////////
//    PreLogin    //
////////////////////
#[derive(Deserialize, Serialize, Debug)]
struct ProxyPreLoginRequest {
    address: IpAddr,
    username: String,
}

#[derive(Serialize, Debug)]
#[serde(tag = "result", content = "message")]
enum ProxyPreLoginResponse {
    Allowed,
    Denied(Message),
}

#[instrument(skip(data), level = "debug")]
async fn pre_login(data: Arc<AppData>, request: ProxyPreLoginRequest) -> Result<impl Reply, Rejection> {
    let builder = MessageBuilder::new()
        .component("SkyNet ".to_string()).with_color(Some(Color::DarkPurple)).close()
        .component("> ".to_string()).with_color(Some(Color::DarkGray)).close()
        .component("Connection impossible...".to_string()).with_color(Some(Color::Red)).close()
        .line_break()
        .component("Votre adresse ip n'est pas autorisée a se connecter".to_string()).close().line_break();


    //Check if ip is banned in database
    if let Some(ip_ban) = data.db.select_ip_ban(&request.address).await.map_err(ApiError::from)? {
        let msg = builder
            .component(format!("Référence : {}", ip_ban.ban.map(|t| t.to_string()).unwrap_or("Aucune".to_string()))).close()
            .close();


        return Ok(reply::json(&ProxyPreLoginResponse::Denied(msg)));
    }

    if request.address.is_loopback() {
        return Ok(reply::json(&ProxyPreLoginResponse::Allowed));
    }

    //Then check if ip is a vpn
    let proxycheck = match proxycheck::check_ip(&data.client, &data.proxy_check_api_key, &request.address).await {
        Ok(resp) => resp,
        Err(err) => {
            error!("{}", err);
            return Ok(reply::json(&ProxyPreLoginResponse::Allowed));
        }
    };

    //Not a risk, allow
    if !proxycheck.is_risk() {
        return Ok(reply::json(&ProxyPreLoginResponse::Allowed));
    }

    //Risk, insert ban in database (for a week, so everything is properly cached)
    let ban_id = data.db.insert_ip_ban(&request.address, Some(&proxycheck.to_string()), None, Some(&Duration::days(7)), true).await.map_err(ApiError::from)?;

    //Then return denied
    Ok(reply::json(&ProxyPreLoginResponse::Denied(builder.component(format!("Référence : {}", ban_id)).close().close())))
}

////////////////////
//     Close      //
////////////////////

#[derive(Deserialize, Serialize, Debug)]
struct ProxyCloseSessionRequest {
    uuid: Uuid,
    username: String,
    session: Uuid,
}

#[instrument(skip(data), level = "debug")]
async fn close_session(data: Arc<AppData>, request: ProxyCloseSessionRequest) -> Result<impl Reply, Rejection> {
    data.db.close_session(&request.session).await.map_err(ApiError::from)?;
    data.db.close_player_session(&request.uuid).await.map_err(ApiError::from)?;

    Ok(StatusCode::OK)
}


#[instrument(skip(data), level = "debug")]
async fn init(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    Ok(reply::json(&data.db.select_all_servers().await.map_err(ApiError::from)?.into_iter().filter(|srv| srv.kind != "proxy" && srv.state == "Started").collect::<Vec<Server>>()))
    //let settings = data.db.get_cached_settings();

    /*    Ok(reply::json(&ProxyInitResponse {
            servers: data.db.select_all_servers().await.map_err(ApiError::from)?,
            slots: settings.get("slots").map(|s| s.parse::<i32>()).transpose().map_err(ApiError::from)?.unwrap_or_default(),
            motd: settings.get("motd").map(|t| t.clone()).unwrap_or("".to_string()),
        }))*/
}

#[derive(Deserialize, Serialize, Debug)]
struct ProxyPingResponse {
    slots: i32,
    motd: String,
}

#[instrument(skip(data), level = "debug")]
async fn ping(data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let settings = data.db.get_cached_settings();

    Ok(reply::json(&ProxyPingResponse {
        slots: settings.get("slots").map(|s| s.parse::<i32>()).transpose().map_err(ApiError::from)?.unwrap_or_default(),
        motd: settings.get("motd").map(|t| t.clone()).unwrap_or("".to_string()),
    }))
}