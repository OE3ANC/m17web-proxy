use std::net::SocketAddr;
use async_trait::async_trait;
use ezsockets::{
    CloseFrame,
    Error,
    Request,
    Socket,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use crate::{REFLECTOR_CONNECTIONS};

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
    pub(crate) done: bool,
}

#[derive(Serialize)]
pub(crate) struct ModuleInfo {
    pub(crate) reflector: String,
    pub(crate) module: String,
    pub(crate) last_heard: u64,
    pub(crate) active: bool
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

        let mut is_info = false;

        match request.uri().path() {
            "/info" => {
                let mut mod_info = vec![];

                for info in REFLECTOR_CONNECTIONS.lock().await.iter() {
                    mod_info.push(
                        ModuleInfo {
                            reflector: info.reflector.clone(),
                            module: info.module.clone(),
                            last_heard: info.last_heard.clone(),
                            active: info.active.clone()
                        }
                    );
                }

                session.text(serde_json::to_string(&mod_info).unwrap()).unwrap();
                is_info = true;
            },
            _ => {
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
                    reflector: "".to_string(),
                    module: "".to_string()
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

    async fn on_text(&mut self, text: String) -> Result<(), Error> {
       let payload: ClientSubscription = serde_json::from_str(&text).unwrap();

       for session in WS_SESSIONS.lock().await.iter_mut() {
           session.subscription.reflector = payload.reflector.clone();
           session.subscription.module = payload.module.clone();
           println!("Subscribing to {}'s Module {}", session.subscription.reflector, session.subscription.module);
       }
        Ok(())
    }
    async fn on_binary(&mut self, _bytes: Vec<u8>) -> Result<(), Error> { unimplemented!() }
    async fn on_call(&mut self, _call: Self::Call) -> Result<(), Error> { unimplemented!() }
}

