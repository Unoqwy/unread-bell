use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{fs, thread};

use mlua::LuaSerdeExt;
use mlua::{Function, Lua, Table};
use serde::Deserialize;
use tungstenite::error::Error::Protocol;
use tungstenite::error::ProtocolError;
use tungstenite::{accept, HandshakeError, Message, WebSocket};

use crate::model::Notifications;

#[macro_use]
extern crate clap;

mod model;

type UpdatePayload = Notifications;

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
        (@arg CONFIG: --config +takes_value "Path to Lua configuration directory (default to $XDG_CONFIG_HOME/unread-bell)")
        (@arg BIND_HOST: --host +takes_value "Bind host (default to 127.0.0.1)")
        (@arg BIND_PORT: --port +takes_value "Bind port (default to 3631)")
    )
    .get_matches();

    let config_dir = matches.value_of("CONFIG").unwrap_or("$XDG_CONFIG_HOME/unread-bell");
    let config_dir: String = shellexpand::env(config_dir).unwrap().into();
    let config_dir = match fs::canonicalize(&PathBuf::from(config_dir)) {
        Ok(path) => path,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => panic!("Configuration directory does not exist"),
            _ => panic!("{}", e),
        },
    };

    let address = format!(
        "{}:{}",
        matches.value_of("BIND_HOST").unwrap_or("127.0.0.1"),
        matches
            .value_of("BIND_PORT")
            .unwrap_or("3631")
            .parse::<u16>()
            .expect("Port must a be an integer between 0 and 65535")
    );

    let server = TcpListener::bind(address).unwrap();
    for tcp_stream in server.incoming() {
        let config_dir = config_dir.clone();
        thread::spawn(move || match accept(tcp_stream.unwrap()) {
            // TODO: handle parallel websockets better / where are they coming from,etc
            Ok(mut ws) => handle_ws(&mut ws, config_dir).unwrap(),
            // just a ping from lib.checkSocket
            Err(HandshakeError::Failure(Protocol(ProtocolError::HandshakeIncomplete))) => return,
            Err(e) => {
                panic!("{:#?}", e);
            }
        });
    }
}

#[derive(Debug, Clone, Default)]
struct State {
    notifications: Notifications,
}

struct Callbacks<'lua> {
    pub update: Option<Function<'lua>>,
    pub close: Option<Function<'lua>>,
}

fn handle_ws(ws: &mut WebSocket<TcpStream>, config_dir: PathBuf) -> mlua::Result<()> {
    let state = Arc::new(RwLock::new(State::default()));

    // TODO: shared lua instance
    let lua = Lua::new();
    let globals = lua.globals();
    let package: Table = globals.get("package")?;

    let unread_mod = lua.create_table()?;

    let cloned_state = state.clone();
    unread_mod.set(
        "get_notifications",
        lua.create_function(move |lua, ()| {
            let state = cloned_state.read().unwrap();
            let notifications = lua.to_value(&state.notifications)?;
            Ok(notifications)
        })?,
    )?;

    package.set("path", format!("{}/?.lua", config_dir.to_str().unwrap()))?;
    package.get::<&str, Table>("loaded")?.set("unread-bell", unread_mod)?;

    let config = lua
        .load("require('init')")
        .eval::<Table>()
        // TODO: differnet error messages
        .expect("Unable to load module 'init', make sure the file exists and return a table");

    let callbacks = Callbacks {
        update: config.get::<&str, Function>("on_update").ok(),
        close: config.get::<&str, Function>("close").ok(),
    };

    loop {
        match ws.read_message() {
            Ok(Message::Text(message)) => {
                let packet: Packet =
                    serde_json::from_slice(&base64::decode(message).unwrap()[..]).unwrap();
                handle_packet(packet, &callbacks, &state)
                    .expect("Could not handle incoming packet");
            }
            Ok(_) => {}
            Err(tungstenite::Error::ConnectionClosed) => {
                if let Some(on_close) = &callbacks.close {
                    on_close.call::<_, ()>(()).unwrap();
                }
                break;
            }
            Err(e) => {
                panic!("{:#?}", e);
            }
        }
    }
    Ok(())
}

fn handle_packet(
    packet: Packet,
    callbacks: &Callbacks,
    state: &Arc<RwLock<State>>,
) -> mlua::Result<()> {
    match packet {
        Packet::Update { payload, revive } => {
            {
                let mut state = state.write().unwrap();
                state.notifications = payload;
            }

            if let Some(on_update) = &callbacks.update {
                on_update.call::<_, ()>(revive)?;
            }
        }
    }
    Ok(())
}
