
mod config;
mod websocket;
mod utils;
mod payloads;

use tokio::net::{ UdpSocket};
use std::io;
use std::io::Write;
use std::str;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ezsockets::Server;

use crate::config::Config;
use crate::payloads::{create_conn_payload, create_pong_payload};
use crate::utils::decode_callsign;
use crate::websocket::{M17ClientServer, WS_SESSIONS, WsPayload, ModuleInfo};
use tokio::sync::Mutex;

use envconfig::{Envconfig};
use lazy_static::lazy_static;
use reqwest::Response;
use serde::{Deserialize, Serialize};

static APP_USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.159 Safari/537.36",
);

lazy_static! {
    pub static ref REFLECTOR_CONNECTIONS: Mutex<Vec<ReflectorConnection>> = Mutex::new(vec![]);
    pub static ref CFG: Config = Config::init_from_env().unwrap();
    pub static ref ACTIVE_MOULES: Mutex<ActiveModules> = Mutex::new(ActiveModules {modules: vec![]});
    pub static ref REF_LIST: Mutex<ReflectorList> = Mutex::new(ReflectorList {status: None,generated_at: None,reflectors: vec![],});
}

#[derive(Deserialize, Clone)]
pub struct ReflectorList {
    status : Option<String>,
    generated_at: Option<String>,
    reflectors: Vec<Reflector>
}

#[derive(Deserialize, Clone)]
pub struct Reflector {
    designator: Option<String>,
    url: Option<String>,
    ipv4: Option<String>,
    ipv6: Option<String>,
    port: Option<u64>,
    sponsor: Option<String>,
    country: Option<String>
}

#[derive(Serialize)]
pub struct ActiveModules {
    pub modules: Vec<ModuleInfo>,
}

#[derive(Serialize)]
pub struct ReflectorConnection {
    reflector: String,
    module: String,
    address: String,
    last_heard: u64,
    active_qso: bool,
    active_qso_meta: QsoMeta,
    #[serde(skip_serializing)]
    socket: UdpSocket,
}

#[derive(Serialize)]
pub struct QsoMeta {
    callsign: String,
    timestamp: u64
}

#[tokio::main]
async fn main() -> io::Result<()> {

    // WS Server instance
    let (server, _) = Server::create(|_server| M17ClientServer {});
    let listener = CFG.ws_listener_address.clone();

    tokio::spawn(async move {
        ezsockets::tungstenite::run(server, listener).await.unwrap();
    });

    map_reflector_list().await;

    for reflector in CFG.subscription.split(",") {
        for module in reflector.split("_").last().unwrap().chars() {
            println!("Subscribed to {} Module {}", reflector, module);
            REFLECTOR_CONNECTIONS.lock().await.push(
                ReflectorConnection {
                    reflector: reflector.split("_").next().unwrap().to_string(),
                    module: module.to_string(),
                    address: get_ref_address(reflector.split("_").next().unwrap().to_string()).await,
                    last_heard: 0,
                    active_qso: false,
                    active_qso_meta: QsoMeta {
                        callsign: "".to_string(),
                        timestamp: 0
                    },
                    socket: UdpSocket::bind("0.0.0.0:0").await?
                }
            );

        }
    }


    let mut buf = [0; 128];

    for reflector_connection in REFLECTOR_CONNECTIONS.lock().await.iter() {
        reflector_connection.socket.connect(&reflector_connection.address).await?;
    }

    let mut info_to_send = false;

    loop {
        handle_reconnects().await;
        refresh_module_info().await;
        if info_to_send {
            send_module_info().await;
            info_to_send = false;
        }

        for reflector_connection in REFLECTOR_CONNECTIONS.lock().await.iter_mut() {

            // Try to recv data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match reflector_connection.socket.try_recv(&mut buf) {

                Ok(_n) => {

                    let cmd = str::from_utf8(&buf[..4]).unwrap();

                    /*
                        • CONN - Connect to a reflector
                        • ACKN - acknowledge connection
                        • NACK - deny connection
                        • PING - keepalive for the connection from the reflector to the client
                        • PONG - keepalive response from the client to the reflector
                        • DISC - Disconnect (client->reflector or reflector->client)
                    */

                    match cmd {
                        "DISC" => {
                            println!("We got disconnected!");
                            reflector_connection.last_heard = 0;
                        }
                        "ACKN" => {
                            println!("We are linked!");
                        }
                        "NACK" => {
                            println!("We got denied! Waiting a minute before reconnecting...");
                            reflector_connection.last_heard = get_epoch().as_secs();
                        }, // Ignored for now -> mrefd sends ping anyway
                        "PING" => {
                            let now = get_epoch().as_secs();

                            if reflector_connection.active_qso && now - reflector_connection.active_qso_meta.timestamp > 1 {
                                reflector_connection.active_qso = false;
                                info_to_send = true;
                            }
                            reflector_connection.last_heard = now;
                            reflector_connection.socket.send(create_pong_payload(CFG.callsign.clone()).as_slice()).await?;
                        },
                        // M17 frame!
                        "M17 " => {
                            //println!("We received a payload: {:x?}", &buf[..n]);

                            // Decoded source callsign
                            let src_call =  decode_callsign(&buf[12..18]);

                            // Decoded destination callsign
                            let dest_call = decode_callsign(&buf[6..12]);

                            // Codec 2 stream
                            let data: &[u8] = &buf[36..52];

                            // Last frame 1st byte of last stream is always > 0x80
                            let mut is_last = false;
                            if buf[34] >= 0x80 {
                                println!("Received last frame!");
                                is_last = true;
                            }

                            // Serialize as json and send to all connected websocket clients
                            WS_SESSIONS.lock().await.iter().for_each(|session|{
                                if session.subscription.reflector == reflector_connection.reflector && session.subscription.module == reflector_connection.module && session.info_connection == false {
                                    let send_payload = WsPayload {
                                        reflector: reflector_connection.reflector.to_string(),
                                        module: reflector_connection.module.to_string(),
                                        src_call: src_call.clone(),
                                        dest_call: dest_call.clone(),
                                        c2_stream: Vec::from(data),
                                        done: is_last
                                    };
                                    session.ws_session.handle.text(serde_json::to_string(&send_payload).unwrap()).unwrap();
                                }
                            });

                            if reflector_connection.active_qso == false {
                                reflector_connection.active_qso = true;
                                info_to_send = true;
                            }


                            reflector_connection.active_qso_meta.callsign = src_call.clone();
                            reflector_connection.active_qso_meta.timestamp = get_epoch().as_secs();
                        }
                        _ => {
                            print!(" ");
                            println!("{:x?}", &buf);
                        }
                    }
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn send_module_info() {
    for session in WS_SESSIONS.lock().await.iter() {
        if session.info_connection {
            session.ws_session.handle.text(serde_json::to_string(&get_module_infos().await).unwrap()).unwrap();
        }
    }
}

async fn get_module_infos() -> Vec<ModuleInfo> {
    let mut mod_info = vec![];

    for info in REFLECTOR_CONNECTIONS.lock().await.iter() {
        mod_info.push(
            ModuleInfo {
                reflector: info.reflector.clone(),
                module: info.module.clone(),
                last_heard: info.last_heard.clone(),
                last_qso_call: info.active_qso_meta.callsign.clone(),
                last_qso_time: info.active_qso_meta.timestamp.clone(),
                active_qso: info.active_qso.clone()
            }
        );
    }
    mod_info
}

async fn refresh_module_info() {
    ACTIVE_MOULES.lock().await.modules = vec![];
    for info in REFLECTOR_CONNECTIONS.lock().await.iter() {
        ACTIVE_MOULES.lock().await.modules.push(
            ModuleInfo {
                reflector: info.reflector.clone(),
                module: info.module.clone(),
                last_heard: info.last_heard.clone(),
                last_qso_call: "".to_string(),
                last_qso_time: info.active_qso_meta.timestamp.clone(),
                active_qso: false,
            }
        );
    }
}

async fn handle_reconnects() {
    for reflector_connection in REFLECTOR_CONNECTIONS.lock().await.iter_mut() {
        let now = get_epoch().as_secs();
        if now - reflector_connection.last_heard > 60 {
            let module = reflector_connection.module.clone();
            let conn_payload = create_conn_payload("LSTN".to_string(), CFG.callsign.clone(), module);
            let _len = reflector_connection.socket.send(&conn_payload).await;
            reflector_connection.last_heard = get_epoch().as_secs();
        }
    }
}

fn get_epoch() -> Duration {
    let start = SystemTime::now();
    start.duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
}
 async fn load_reflector_list() -> ReflectorList {
    let result: String = download_reflector_list().await;
    serde_json::from_str(result.as_str()).unwrap()
}

// TODO -> There must be a better way?
async fn map_reflector_list() {
    let tmp_ref = load_reflector_list().await;

    REF_LIST.lock().await.reflectors = tmp_ref.reflectors;
    REF_LIST.lock().await.status = tmp_ref.status;
    REF_LIST.lock().await.generated_at = tmp_ref.generated_at;
}

async fn http_client(url: String) -> Response {
    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build().unwrap();
    client.get(url).send().await.unwrap()
}

/*
    TODO:
    - If local file exists and is older, overwrite after testing if new can be deserialized
    - Fallback to local file!
    - cfg
 */
async fn download_reflector_list() -> String {
    http_client(String::from("https://dvref.com/mrefd/json/?format=json")).await.text().await.unwrap()
}


async fn get_ref_address(designator: String) -> String {
    let mut result = String::new();
    let tmp_list = REF_LIST.lock().await.clone();
    for reflector in tmp_list.reflectors.clone().iter() {
        let tmp_des = reflector.designator.clone();
        if tmp_des.unwrap() == designator.split("-").last().unwrap() {
            result = format!("{}:{}", reflector.clone().ipv4.unwrap(), reflector.port.unwrap());
        }
    }
    result
}