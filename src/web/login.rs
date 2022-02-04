use std::collections::HashMap;
use std::net::IpAddr;
use std::ops::Add;
use std::sync::Arc;
use chrono::{Duration, Local};
use warp::{Filter, path, Rejection, Reply, reply};
use crate::AppData;
use tracing::{error, instrument};
use uuid::Uuid;
use warp::body::json;
use crate::web::rejections::ApiError;
use crate::web::{with_auth, with_data};
use serde::{Serialize, Deserialize};
use warp::hyper::StatusCode;
use crate::database::players::DbProxyPlayerInfo;
use crate::log::debug;
use crate::utils::message::{Color, Message, MessageBuilder};
use crate::utils::proxycheck;

pub fn filter(data: Arc<AppData>) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post().and(path!("api"/"players"/Uuid/"proxy"/"login")).and(with_auth(data.clone(), "proxy-login")).and(with_data(data.clone())).and(json::<ProxyLoginRequest>()).and_then(proxy_login)
        .or(warp::post().and(path!("api"/"sessions"/Uuid/"modsinfo")).and(with_auth(data.clone(), "proxy-login")).and(with_data(data.clone())).and(json::<Vec<ModInfo>>()).and_then(add_modinfo))
        .or(warp::post().and(path!("api"/"sessions"/Uuid/"clientbrand")).and(with_auth(data.clone(), "proxy-login")).and(with_data(data.clone())).and(json::<String>()).and_then(add_client_brand))
        .or(warp::get().and(path!("api"/"players"/IpAddr/"proxy"/"prelogin")).and(with_auth(data.clone(), "proxy-pre-login")).and(with_data(data.clone())).and_then(pre_login))
        .or(warp::post().and(path!("api"/"players"/Uuid/"login")).and(with_auth(data.clone(), "server-login")).and(with_data(data.clone())).and(json::<Uuid>()).and_then(login))
}


////////////////////
//      Login     //
////////////////////

#[derive(Deserialize, Debug)]
struct ProxyLoginRequest {
    username: String,
    proxy: Uuid,
    ip: IpAddr,
    version: String,
    locale: Option<String>,
}

// #[derive(Deserialize, Debug)]
// struct MinecraftBrand {
//     brand: String,
//     mods: Option<Vec<ModInfo>>,
// }

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
async fn proxy_login(uuid: Uuid, data: Arc<AppData>, request: ProxyLoginRequest) -> Result<impl Reply, Rejection> {
    let option = data.db.select_proxy_player_info(&uuid).await.map_err(ApiError::from)?;

    let info = match option {
        None => {
            data.db.insert_player(&uuid, &request.username, None).await.map_err(ApiError::from)?;
            DbProxyPlayerInfo::default().build_proxy_login_player_info(&data.db).await.map_err(ApiError::from)?
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
            info.build_proxy_login_player_info(&data.db).await.map_err(ApiError::from)?
        }
    };


    let session = Uuid::new_v4();


    data.db.insert_session(&session, &request.version, &uuid, &request.ip).await.map_err(ApiError::from)?;

    data.db.update_player_online_proxy_info(&uuid, request.proxy, &session, &request.username).await.map_err(ApiError::from)?;

    Ok(reply::json(&ProxyLoginResponse::Allowed { session, player_info: info }))
}

#[instrument(skip(data))]
async fn add_modinfo(uuid: Uuid, data: Arc<AppData>, request: Vec<ModInfo>) -> Result<impl Reply, Rejection> {
    let mods = {
        let mut map = HashMap::new();
        for m in request {
            map.insert(m.id, m.version);
        }
        map
    };

    data.db.set_session_mods_info(&uuid, &mods).await.map_err(ApiError::from)?;

    Ok(reply().into_response())
}

#[instrument(skip(data))]
async fn add_client_brand(uuid: Uuid, data: Arc<AppData>, request: String) -> Result<impl Reply, Rejection> {
    data.db.set_session_brand(&uuid, &request).await.map_err(ApiError::from)?;

    Ok(reply().into_response())
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

#[instrument(skip(data))]
async fn pre_login(ip: IpAddr, data: Arc<AppData>) -> Result<impl Reply, Rejection> {
    let builder = MessageBuilder::new()
        .component("SkyNet ".to_string()).with_color(Some(Color::DarkPurple)).close()
        .component("> ".to_string()).with_color(Some(Color::DarkGray)).close()
        .component("Connection impossible...".to_string()).with_color(Some(Color::Red)).close()
        .line_break()
        .component("Votre adresse ip n'est pas autorisée a se connecter".to_string()).close().line_break();


    //Check if ip is banned in database
    if let Some(ip_ban) = data.db.select_ip_ban(&ip).await.map_err(ApiError::from)? {
        let msg = builder
            .component(format!("Référence : {}", ip_ban.ban.map(|t| t.to_string()).unwrap_or("Aucune".to_string()))).close()
            .close();


        return Ok(reply::json(&ProxyPreLoginResponse::Denied(msg)));
    }

    if ip.is_loopback() {
        return Ok(reply::json(&ProxyPreLoginResponse::Allowed));
    }

    //Then check if ip is a vpn
    let proxycheck = match proxycheck::check_ip(&data.client, &data.proxy_check_api_key, &ip).await {
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
    let ban_id = data.db.insert_ip_ban(&ip, Some(&proxycheck.to_string()), None, Some(&Duration::days(7)), true).await.map_err(ApiError::from)?;

    //Then return denied
    Ok(reply::json(&ProxyPreLoginResponse::Denied(builder.component(format!("Référence : {}", ban_id)).close().close())))
}

////////////////////
//    Login       //
////////////////////
#[derive(Serialize, Debug)]
pub struct ServerLoginPlayerInfo {
    pub session: Uuid,
    pub proxy: Uuid,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub locale: String,
    pub permissions: Vec<String>,
    pub power: i32,
    pub currency: i32,
    pub premium_currency: i32,
    pub blocked: Vec<Uuid>,
    pub inventory: HashMap<String, i32>,
    pub properties: HashMap<String, String>,
}


#[instrument(skip(data))]
async fn login(uuid: Uuid, data: Arc<AppData>, server: Uuid) -> Result<impl Reply, Rejection> {
    let (kind, props) = match data.db.select_server_kind_and_properties(&server).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    let server_kind = match data.db.select_server_kind_object(&kind).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };


    let mut player = match data.db.select_server_player_info(&uuid).await.map_err(ApiError::from)? {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(kind) => kind
    };

    if (props.get("host").map(|host| {
        if let Ok(host) = Uuid::parse_str(host) {
            uuid == host
        } else {
            false
        }
    })).unwrap_or_default() {
        debug!("Player is server host, adding group !");
        match player.groups {
            None => {
                player.groups = Some(vec!["Host".to_string()]);
            }
            Some(ref mut groups) => {
                groups.push("Host".to_string());
            }
        }
    }


    let info = player.build_server_login_player_info(&data.db, &server_kind.name).await.map_err(ApiError::from)?;


    data.db.update_player_server_and_null_waiting_move_to(&uuid, server).await.map_err(ApiError::from)?;

    Ok(reply::json(&info).into_response())
}

