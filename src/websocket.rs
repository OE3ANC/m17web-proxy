use std::net::SocketAddr;
use async_trait::async_trait;
use ezsockets::{CloseFrame, Error, Request, Socket, Utf8Bytes};
use lazy_static::lazy_static;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use crate::{get_module_infos, MsgData};

lazy_static! {
    pub static ref WS_SESSIONS: Mutex<Vec<M17ClientSession>> = Mutex::new(vec![]);
}

type SessionID = u16;
type Session = ezsockets::Session<SessionID, ()>;

pub struct M17ClientServer {}

pub struct WebSocketClientSession {
    pub(crate) handle: Session,
    id: SessionID,
}

pub struct M17ClientSession {
    pub(crate) ws_session: WebSocketClientSession,
    pub(crate) subscription: ClientSubscription,
    pub(crate) info_connection: bool,
}

#[derive(Serialize)]
pub(crate) struct WsPayload {
    pub(crate) reflector: String,
    pub(crate) module: String,
    pub(crate) src_call: String,
    pub(crate) dest_call: String,
    pub(crate) c2_stream: Vec<u8>,
    pub(crate) pm_stream: Vec<u8>,
    pub(crate) done: bool,
}

#[derive(Serialize)]
pub struct ModuleInfo {
    pub reflector: String,
    pub module: String,
    pub last_heard: u64,
    pub last_qso_call: String,
    pub last_qso_time: u64,
    pub active_qso: bool,
    pub messages: Vec<MsgData>,
}


#[derive(Deserialize)]
pub(crate) struct ClientSubscription {
    pub(crate) reflector: String,
    pub(crate) module: String
}

#[async_trait]
impl ezsockets::ServerExt for M17ClientServer {
    type Session = WebSocketClientSession;
    type Call = ();


    async fn on_connect(
        &mut self,
        socket: Socket,
        request: Request,
        address: SocketAddr,
    ) -> Result<Session, Option<CloseFrame>> {
        let id = address.port();
        let session = Session::create(|handle| WebSocketClientSession { id, handle }, id, socket);

        let is_info: bool;

        let mut sub_ref = "";
        let mut sub_mod = "";

        match request.uri().path() {
            "/" => {
                info!("WS_CONNECTION {} connected as info client from {}", id, address);
                // Send init module info
                session.text(serde_json::to_string(&get_module_infos().await).unwrap()).unwrap();
                is_info = true;
            },
            _ => {
                let mut path = request.uri().path().split("/");
                // TODO-> Check if path and module are ok
                _ = path.next().unwrap();
                sub_ref = path.next().unwrap();
                sub_mod = path.next().unwrap();

                info!("WS_CONNECTION {} connected as stream client from {} subscribing Reflector {} Module {}", id, address, sub_ref, sub_mod);
                is_info = false;
            }
        }

        WS_SESSIONS.lock().await.push(
            M17ClientSession {
                ws_session: WebSocketClientSession {
                    handle: session.clone(),
                    id,
                }
                ,
                subscription: ClientSubscription {
                    reflector: sub_ref.to_string(),
                    module: sub_mod.to_string()
                },
                info_connection: is_info
            }
        );
        Ok(session)
    }

    async fn on_disconnect(
        &mut self,
        id: <Self::Session as ezsockets::SessionExt>::ID,
        _reason: Result<Option<CloseFrame>, Error>,
    ) -> Result<(), Error> {
        let index = WS_SESSIONS.lock().await.iter().position(|x| x.ws_session.id == id).unwrap();
        info!("WS_SESSION {} disconnected", index);
        WS_SESSIONS.lock().await.remove(index);
        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), Error> {
        let () = call;
        Ok(())
    }
}

#[async_trait]
impl ezsockets::SessionExt for WebSocketClientSession {
    type ID = SessionID;
    type Call = ();

    fn id(&self) -> &Self::ID {
        &self.id
    }

    async fn on_text(&mut self, text: Utf8Bytes) -> Result<(), Error> {
        let payload: ClientSubscription = serde_json::from_str(&text).unwrap();
        info!("New subscription to stream from WS_CONNECTION {}: Reflector {} Module {}", self.id, payload.reflector.clone(), payload.module.clone());

        let mut ws_sessions = WS_SESSIONS.lock().await;
        for session in ws_sessions.iter_mut() {
            if session.ws_session.id == self.id {
                if session.info_connection {
                    warn!("Stream subscription with info client failed!")
                } else {
                    session.subscription.reflector = payload.reflector.clone();
                    session.subscription.module = payload.module.clone();
                }
            }

        }
        Ok(())
    }
    async fn on_binary(&mut self, _bytes: ezsockets::Bytes) -> Result<(), Error> { unimplemented!() }
    async fn on_call(&mut self, _call: Self::Call) -> Result<(), Error> { unimplemented!() }
}
