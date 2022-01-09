use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::{fs, thread};

use mlua::{Function, Lua, LuaSerdeExt, Table};
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

    let state = Arc::new(RwLock::new(State::default()));

    let lua = Lua::new();
    setup_lua_mod(&lua, config_dir, state.clone()).unwrap();
    let lua = Arc::new(Mutex::new(lua));

    let server = TcpListener::bind(address).unwrap();
    for tcp_stream in server.incoming() {
        let state = state.clone();
        let lua = lua.clone();
        thread::spawn(move || {
            let lua = lua.lock().unwrap();
            let config = lua
                .load("require('init')")
                .eval::<Table>()
                .expect("Couldn't create new instance of configuration");
            let callbacks = Callbacks {
                update: config.get::<&str, Function>("on_update").ok(),
                close: config.get::<&str, Function>("close").ok(),
            };

            match accept(tcp_stream.unwrap()) {
                Ok(mut ws) => handle_ws(&mut ws, state, &callbacks).unwrap(),
                // just a ping from lib.checkSocket
                Err(HandshakeError::Failure(Protocol(ProtocolError::HandshakeIncomplete))) => {
                    return
                },
                Err(e) => {
                    panic!("{:#?}", e);
                },
            };
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

fn setup_lua_mod<'lua>(
    lua: &Lua,
    config_dir: PathBuf,
    state: Arc<RwLock<State>>,
) -> mlua::Result<()> {
    let globals = lua.globals();
    let package: Table = globals.get("package")?;

    let unread_mod = lua.create_table()?;
    unread_mod.set(
        "get_notifications",
        lua.create_function(move |lua, ()| {
            let state = state.read().unwrap();
            let notifications = lua.to_value(&state.notifications)?;
            Ok(notifications)
        })?,
    )?;

    package.set("path", format!("{}/?.lua", config_dir.to_str().unwrap()))?;
    package.get::<&str, Table>("loaded")?.set("unread-bell", unread_mod)?;
    Ok(())
}

fn handle_ws(
    ws: &mut WebSocket<TcpStream>,
    state: Arc<RwLock<State>>,
    callbacks: &Callbacks,
) -> mlua::Result<()> {
    let local_address = format!("{}", ws.get_ref().local_addr().unwrap());
    let remote_address = format!("{}", ws.get_ref().peer_addr().unwrap());
    println!("Accepted websocket connection from {} on {}.", remote_address, local_address);
    loop {
        match ws.read_message() {
            Ok(Message::Text(message)) => {
                if let Ok(packet) = serde_json::from_slice(&base64::decode(message).unwrap()[..]) {
                    match packet {
                        Packet::Update { payload, revive } => {
                            {
                                let mut state = state.write().unwrap();
                                state.notifications = payload;
                            }

                            if let Some(on_update) = &callbacks.update {
                                if let Err(err) = on_update.call::<_, ()>(revive) {
                                    eprintln!("Error calling on_update, {:?}", err);
                                }
                            }
                        },
                    }
                }
            },
            Ok(_) => {},
            Err(tungstenite::Error::ConnectionClosed) => {
                if let Some(on_close) = &callbacks.close {
                    on_close.call::<_, ()>(()).unwrap();
                }
                break;
            },
            Err(err) => {
                if !ws.can_read() {
                    panic!("{:#?}", err);
                } else {
                    eprintln!("{:#?}", err);
                }
            },
        }
    }
    println!("Closed websocket connection to {} ({}).", remote_address, local_address);
    Ok(())
}
