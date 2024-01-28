#![feature(iter_collect_into)]
#![feature(ascii_char)]
#![feature(slice_pattern)]

mod config;
mod websocket;
mod utils;
mod payloads;

use tokio::net::{ UdpSocket};
use std::{io, vec};
use std::str;

use ezsockets::Server;

use serde::Serialize;

use crate::config::Config;
use crate::payloads::{create_conn_payload, create_pong_payload};
use crate::utils::decode_callsign;
use crate::websocket::{M17ClientServer, WS_SESSIONS, WsPayload};

use envconfig::{Envconfig, Error};

#[tokio::main]
async fn main() -> io::Result<()> {
    let cfg = Config::init_from_env().unwrap();
    println!("Startup with config: {:?}", cfg);
    // WS Server instance
    let (server, _) = Server::create(|_server| M17ClientServer {});

    let listener = cfg.ws_listener_address.clone();

    tokio::spawn(async move {
        ezsockets::tungstenite::run(server, listener).await.unwrap();
    });

    let sock_udp = UdpSocket::bind("0.0.0.0:0").await?;
    sock_udp.connect(cfg.reflector_address).await?;

    // RX Buffer -> 128 bytes should be more than enough -> Spec says 54 bytes
    let mut buf = [0; 128];

    let conn_payload = create_conn_payload(cfg.callsign.clone(), cfg.reflector_target_module);

    let len = sock_udp.send(&conn_payload).await?;

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
                sock_udp.send(create_pong_payload(cfg.callsign.clone()).as_slice()).await?;
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
