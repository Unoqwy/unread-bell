use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::thread;

use serde::Deserialize;
use tungstenite::error::Error::Protocol;
use tungstenite::error::ProtocolError;
use tungstenite::{accept, HandshakeError, Message, WebSocket};

#[macro_use]
extern crate clap;

type DiscordId = String;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectMessageNotifications {
    channel_id: String,
    unread_count: u32,
    last_message_id: String,
    username: String,
    discriminator: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupNotifications {
    unread_count: u32,
    last_message_id: String,
    name: String,
    users: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuildNotifications {
    unread_count: u32,
    mention_count: u32,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdatePayload {
    dms: HashMap<DiscordId, DirectMessageNotifications>,
    groups: HashMap<DiscordId, GroupNotifications>,
    guilds: HashMap<DiscordId, GuildNotifications>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum Packet {
    Update { payload: UpdatePayload, revive: bool },
}

fn main() {
    let matches = clap_app!(app =>
        (name: crate_name!())
        (version: crate_version!())
        (about: crate_description!())
        (@arg CONFIG: --rc +takes_value "Path to Lua configuration file (default to $XDG_CONFIG_HOME/unread-bell/init.lua)")
        (@arg BIND_HOST: --host +takes_value "Bind host (default to 127.0.0.1)")
        (@arg BIND_PORT: --port +takes_value "Bind port (default to 3631)")
        (@arg OUTPUT: +required "Target FIFO to write to")
    )
    .get_matches();

    let address = format!(
        "{}:{}",
        matches.value_of("BIND_HOTS").unwrap_or("127.0.0.1"),
        matches
            .value_of("BIND_PORT")
            .unwrap_or("3631")
            .parse::<u16>()
            .expect("Port must a be an integer between 0 and 65535")
    );

    let server = TcpListener::bind(address).unwrap();
    for tcp_stream in server.incoming() {
        thread::spawn(move || match accept(tcp_stream.unwrap()) {
            Ok(mut ws) => handle_ws(&mut ws),
            // just a ping from lib.checkSocket
            Err(HandshakeError::Failure(Protocol(ProtocolError::HandshakeIncomplete))) => return,
            Err(e) => {
                panic!("{:#?}", e);
            },
        });
    }
}

fn handle_ws(ws: &mut WebSocket<TcpStream>) {
    loop {
        match ws.read_message() {
            Ok(message) => {
                if let Message::Text(b64_packet) = message {
                    handle_packet(
                        serde_json::from_slice(&base64::decode(b64_packet).unwrap()[..]).unwrap(),
                    )
                }
            },
            Err(tungstenite::Error::ConnectionClosed) => {
                // TODO: clean up status bar
                break;
            },
            Err(e) => {
                panic!("{:#?}", e);
            },
        }
    }
}

fn handle_packet(packet: Packet) {
    dbg!(packet);
}
