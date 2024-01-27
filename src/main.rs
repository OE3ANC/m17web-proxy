#![feature(iter_collect_into)]
#![feature(ascii_char)]
#![feature(slice_pattern)]

use tokio::net::{ UdpSocket};
use std::{io, vec};
use std::str;
use async_trait::async_trait;

use ezsockets::{
    CloseFrame,
    Error,
    Request,
    Server,
    Socket,
};

use std::net::SocketAddr;
use lazy_static::lazy_static;
use tokio::sync::Mutex;
use serde::Serialize;

lazy_static! {
    static ref WS_SESSIONS: Mutex<Vec<Session>> = Mutex::new(vec![]);
}

#[derive(Serialize)]
struct WsPayload {
    src_call: String,
    dest_call: String,
    c2_stream: Vec<u8>,
    done: bool
}

type SessionID = u16;
type Session = ezsockets::Session<SessionID, ()>;

struct M17ClientServer {}

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
        let session = Session::create(|handle| M17ClientSession { id, handle }, id, socket);
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

// Basic websocket session struct
struct M17ClientSession {
    handle: Session,
    id: SessionID,
}

#[async_trait]
impl ezsockets::SessionExt for M17ClientSession {

    type ID = SessionID;
    type Call = ();

    fn id(&self) -> &Self::ID {
        &self.id
    }

    async fn on_text(&mut self, _text: String)      -> Result<(), Error> { unimplemented!() }
    async fn on_binary(&mut self, _bytes: Vec<u8>)  -> Result<(), Error> { unimplemented!() }
    async fn on_call(&mut self, _call: Self::Call)  -> Result<(), Error> { unimplemented!() }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // TODO -> Get from env vars
    let callsign = "<MYCALL>-WR";
    let ws_listener_address  = "0.0.0.0:3000";
    let reflector_address = "<reflector>:17000";
    let reflector_target_module = "A";

    // WS Server instance
    let (server, _) = Server::create(|_server| M17ClientServer {});
    tokio::spawn(async move {
        ezsockets::tungstenite::run(server, ws_listener_address).await.unwrap();
    });

    // UDP socket client - Fixed UDP Port for now
    let sock_udp = UdpSocket::bind("0.0.0.0:1177").await?;
    sock_udp.connect(reflector_address).await?;

    // RX Buffer -> 128 bytes should be more than enough -> Spec says 54 bytes
    let mut buf = [0; 128];
    let len = sock_udp.send(&create_conn_payload(callsign, reflector_target_module)).await?;

    println!("{:?} bytes sent", len);

    loop {
        let len = sock_udp.recv(&mut buf).await?;
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
            "ACKN" => println!("We are linked!"),
            "NACK" => println!("We got refused"), // Ignored for now -> mrefd sends ping anyway
            "PING" => {
                println!("We got a PING! Sending PONG...");
                sock_udp.send(create_pong_payload(callsign).as_slice()).await?;
            },
            // M17 frame!
            "M17 " => {
                println!("We received a payload: {:x?}", &buf[..len]);

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
                    let send_payload = WsPayload {
                        src_call: src_call.clone(), dest_call: dest_call.clone(), c2_stream: Vec::from(data), done: is_last
                    };
                    session.text(serde_json::to_string(&send_payload).unwrap()).unwrap();
                });

            }
            _ => {
                print!(" ");
                println!("{:x?}", &buf);
            }
        }
    }
    // TODO -> Disconnect when closed!
}

fn create_conn_payload(callsign: &str, module: &str) -> Vec<u8> {
    let mut payload = vec![];

    payload.extend_from_slice("CONN".as_bytes());
    payload.extend_from_slice(encode_callsign(callsign).as_slice());
    payload.extend_from_slice(module.as_bytes());

    payload
}

fn create_pong_payload(callsign: &str) -> Vec<u8> {
    let mut payload = vec![];
    payload.extend_from_slice("PONG".as_bytes());
    payload.extend_from_slice(encode_callsign(callsign).as_slice());
    payload
}

// Base40 charset
const CHARSET: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-/.";

// Base40 decode
fn decode_callsign(encoded: &[u8]) -> String {
    assert!(!encoded.is_empty());

    let callsign: &mut String = &mut "".to_string();

    if encoded[..6].iter().all(|&b| b == 0xFF) {
        *callsign = "ALL".to_owned()
    }

    let mut enc: u64 = (encoded[0] as u64) << 40
        | (encoded[1] as u64) << 32
        | (encoded[2] as u64) << 24
        | (encoded[3] as u64) << 16
        | (encoded[4] as u64) << 8
        | (encoded[5] as u64);

    if enc >= 262144000000000 { // 40^9
        *callsign = "".to_owned()
    }

    let digits = CHARSET.to_string().into_bytes();
    while enc > 0 {
        let digit = digits[(enc % 40) as usize];
        callsign.push(char::from(digit));
        enc /= 40;
    }
    callsign.to_owned()
}

// Base40 encode
fn encode_callsign(callsign: &str) -> Vec<u8> {
    let encoded: &mut [u8] = &mut [0;6];

    if callsign == "ALL" || callsign == " ALL      " {
        encoded[..].fill(0xFF)
    }

    let len = callsign.trim().len().min(9);

    let mut enc = 0;
    for ch in callsign[..len].chars().rev() {
        let pos = CHARSET
            .chars()
            .position(|c| c == ch)
            .unwrap_or(0) as u64;

        enc *= 40;
        enc += pos;
    }

    encoded[0] = ((enc >> 40) & 0xff) as u8;
    encoded[1] = ((enc >> 32) & 0xff) as u8;
    encoded[2] = ((enc >> 24) & 0xff) as u8;
    encoded[3] = ((enc >> 16) & 0xff) as u8;
    encoded[4] = ((enc >>  8) & 0xff) as u8;
    encoded[5] = ((enc >>  0) & 0xff) as u8;
    
    encoded.to_owned()
}