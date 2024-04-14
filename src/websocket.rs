use std::net::SocketAddr;
use async_trait::async_trait;
use ezsockets::{
    CloseFrame,
    Error,
    Request,
    Socket,
};
use lazy_static::lazy_static;
use serde::Serialize;
use tokio::sync::Mutex;

lazy_static! {
    pub static ref WS_SESSIONS: Mutex<Vec<Session>> = Mutex::new(vec![]);
}

type SessionID = u16;
type Session = ezsockets::Session<SessionID, ()>;

pub struct M17ClientServer {}

pub struct M17ClientSession {
    //handle: Session,
    id: SessionID,
}

#[derive(Serialize)]
pub(crate) struct WsPayload {
    pub(crate) src_call: String,
    pub(crate) dest_call: String,
    pub(crate) c2_stream: Vec<u8>,
    pub(crate) done: bool,
}


#[async_trait]
impl ezsockets::ServerExt for M17ClientServer {
    type Session = M17ClientSession;
    type Call = ();

    async fn on_connect(
        &mut self,
        socket: Socket,
        _request: Request,
        address: SocketAddr,
    ) -> Result<Session, Option<CloseFrame>> {
        let id = address.port();
        let session = Session::create(|_| M17ClientSession { id }, id, socket);
        WS_SESSIONS.lock().await.push(session.clone());
        Ok(session)
    }

    async fn on_disconnect(
        &mut self,
        id: <Self::Session as ezsockets::SessionExt>::ID,
        _reason: Result<Option<CloseFrame>, Error>,
    ) -> Result<(), Error> {
        let index = WS_SESSIONS.lock().await.iter().position(|x| x.id == id).unwrap();
        WS_SESSIONS.lock().await.remove(index);
        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), Error> {
        let () = call;
        Ok(())
    }
}


#[async_trait]
impl ezsockets::SessionExt for M17ClientSession {
    type ID = SessionID;
    type Call = ();

    fn id(&self) -> &Self::ID {
        &self.id
    }

    async fn on_text(&mut self, _text: String) -> Result<(), Error> { unimplemented!() }
    async fn on_binary(&mut self, _bytes: Vec<u8>) -> Result<(), Error> { unimplemented!() }
    async fn on_call(&mut self, _call: Self::Call) -> Result<(), Error> { unimplemented!() }
}

