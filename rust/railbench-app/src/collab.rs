use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use railbench_core::io::{load_layout_json, save_layout_json_compact};
use railbench_core::types::Layout;
use serde_json::Value;
use tungstenite::Message;

pub const COLLAB_COLORS: &[&str] = &[
    "#3b82f6",
    "#10b981",
    "#f59e0b",
    "#ec4899",
    "#8b5cf6",
    "#06b6d4",
    "#f97316",
    "#14b8a6",
];

static NEXT_CLIENT_SEQ: AtomicU64 = AtomicU64::new(1);
static NEXT_SERVER_CONN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq)]
pub struct CollabUser {
    pub id: String,
    pub name: String,
    pub color: String,
    pub cursor: Option<[f64; 2]>,
    pub selected_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollabChatMessage {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_color: String,
    pub text: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabDialogTab {
    Host,
    Join,
}

pub enum CollabEvent {
    Connected {
        is_host: bool,
        users: Vec<CollabUser>,
        initial_layout: Option<Layout>,
        status: String,
    },
    Disconnected(String),
    Error(String),
    UserJoined(CollabUser),
    UserLeft {
        user_id: String,
    },
    LayoutReceived {
        layout: Layout,
        from_user_name: String,
    },
    CursorReceived {
        user_id: String,
        x: f64,
        y: f64,
    },
    SelectionReceived {
        user_id: String,
        selected_ids: Vec<String>,
    },
    ChatReceived(CollabChatMessage),
}

/// Normalizes user-entered server URLs so bare hostnames, `http(s)://`, or root paths map cleanly to `/collab`.
pub fn normalize_ws_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "ws://localhost:3000/collab".to_string();
    }
    let with_scheme = if trimmed.len() >= 7 && trimmed[..7].eq_ignore_ascii_case("http://") {
        format!("ws://{}", &trimmed[7..])
    } else if trimmed.len() >= 8 && trimmed[..8].eq_ignore_ascii_case("https://") {
        format!("wss://{}", &trimmed[8..])
    } else if (trimmed.len() >= 5 && trimmed[..5].eq_ignore_ascii_case("ws://"))
        || (trimmed.len() >= 6 && trimmed[..6].eq_ignore_ascii_case("wss://"))
    {
        trimmed.to_string()
    } else {
        format!("ws://{trimmed}")
    };

    let scheme_len = if with_scheme.len() >= 6 && with_scheme[..6].eq_ignore_ascii_case("wss://") {
        6
    } else {
        5
    };
    let rest = &with_scheme[scheme_len..];
    match rest.find('/') {
        None => format!("{with_scheme}/collab"),
        Some(slash_idx) if slash_idx + 1 == rest.len() => format!("{with_scheme}collab"),
        _ => with_scheme,
    }
}

fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn sanitize_room_name(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "default".to_string()
    } else {
        cleaned
    }
}

fn detect_lan_ipv4() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    let addr = sock.local_addr().ok()?;
    match addr.ip() {
        std::net::IpAddr::V4(v4) if !v4.is_loopback() => Some(v4.to_string()),
        _ => None,
    }
}

fn is_transient_io_err(err: &tungstenite::Error) -> bool {
    matches!(
        err,
        tungstenite::Error::Io(io_err)
            if matches!(io_err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
    )
}

fn parse_collab_user(v: &Value) -> Option<CollabUser> {
    let id = v.get("id")?.as_str()?.to_string();
    let name = v
        .get("name")
        .and_then(|s| s.as_str())
        .unwrap_or("Planner")
        .to_string();
    let color = v
        .get("color")
        .and_then(|s| s.as_str())
        .unwrap_or("#3b82f6")
        .to_string();
    let cursor = v.get("cursor").and_then(|c| {
        let x = c.get("x")?.as_f64()?;
        let y = c.get("y")?.as_f64()?;
        Some([x, y])
    });
    let selected_ids = v
        .get("selectedIds")
        .and_then(|arr| arr.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();
    Some(CollabUser {
        id,
        name,
        color,
        cursor,
        selected_ids,
    })
}

fn configure_tcp_stream(tcp: &TcpStream) {
    let _ = tcp.set_nonblocking(false);
    let _ = tcp.set_read_timeout(Some(Duration::from_millis(25)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(5)));
    let _ = tcp.set_nodelay(true);
}

// ---------------------------------------------------------------------------
// Embedded multi-user WebSocket server (wire-compatible with server.mjs & TS)
// ---------------------------------------------------------------------------

struct RoomClient {
    user: CollabUser,
    tx: Sender<String>,
}

struct ServerRoom {
    password: Option<String>,
    layout: Option<Value>,
    clients: HashMap<u64, RoomClient>,
}

struct ServerRoomsState {
    default_room: String,
    default_password: Option<String>,
    rooms: HashMap<String, ServerRoom>,
}

impl ServerRoom {
    fn users_json(&self) -> Vec<Value> {
        self.clients
            .values()
            .map(|c| {
                let mut obj = serde_json::json!({
                    "id": c.user.id,
                    "name": c.user.name,
                    "color": c.user.color,
                    "selectedIds": c.user.selected_ids,
                });
                if let Some([x, y]) = c.user.cursor {
                    obj["cursor"] = serde_json::json!({ "x": x, "y": y });
                } else {
                    obj["cursor"] = Value::Null;
                }
                obj
            })
            .collect()
    }

    fn broadcast(&self, msg: &Value, exclude_conn_id: Option<u64>) {
        let Ok(raw) = serde_json::to_string(msg) else {
            return;
        };
        for (&cid, client) in &self.clients {
            if Some(cid) != exclude_conn_id {
                let _ = client.tx.send(raw.clone());
            }
        }
    }
}

pub struct EmbeddedCollabServer {
    stop_flag: Arc<AtomicBool>,
    state: Arc<Mutex<ServerRoomsState>>,
    accept_thread: Option<thread::JoinHandle<()>>,
    pub actual_port: u16,
    pub lan_ip: Option<String>,
}

impl Default for EmbeddedCollabServer {
    fn default() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            state: Arc::new(Mutex::new(ServerRoomsState {
                default_room: "default".to_string(),
                default_password: None,
                rooms: HashMap::new(),
            })),
            accept_thread: None,
            actual_port: 0,
            lan_ip: None,
        }
    }
}

impl EmbeddedCollabServer {
    pub fn is_running(&self) -> bool {
        self.accept_thread.is_some()
    }

    pub fn ws_urls(&self) -> Option<(String, Option<String>)> {
        if !self.is_running() {
            return None;
        }
        let local = format!("ws://localhost:{}/collab", self.actual_port);
        let lan = self
            .lan_ip
            .as_ref()
            .map(|ip| format!("ws://{ip}:{}/collab", self.actual_port));
        Some((local, lan))
    }

    pub fn start(
        &mut self,
        port: u16,
        password: Option<String>,
        default_room: &str,
    ) -> Result<u16, String> {
        self.stop();

        let listener = TcpListener::bind(("0.0.0.0", port))
            .map_err(|e| format!("Failed to bind port {port}: {e}"))?;
        let bound_port = listener
            .local_addr()
            .map_err(|e| format!("Failed to read bound port: {e}"))?
            .port();
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to configure listener: {e}"))?;

        let clean_default_room = sanitize_room_name(default_room);
        let clean_pwd = password
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty());

        let stop_flag = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(ServerRoomsState {
            default_room: clean_default_room,
            default_password: clean_pwd,
            rooms: HashMap::new(),
        }));

        let flag_clone = Arc::clone(&stop_flag);
        let state_clone = Arc::clone(&state);

        let handle = thread::spawn(move || {
            while !flag_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        let c_flag = Arc::clone(&flag_clone);
                        let c_state = Arc::clone(&state_clone);
                        thread::spawn(move || handle_server_client(stream, c_flag, c_state));
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        });

        self.stop_flag = stop_flag;
        self.state = state;
        self.accept_thread = Some(handle);
        self.actual_port = bound_port;
        self.lan_ip = detect_lan_ipv4();
        Ok(bound_port)
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Ok(mut guard) = self.state.lock() {
            guard.rooms.clear();
        }
        if let Some(handle) = self.accept_thread.take() {
            let _ = handle.join();
        }
        self.actual_port = 0;
        self.lan_ip = None;
    }
}

impl Drop for EmbeddedCollabServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_server_client(
    stream: TcpStream,
    stop_flag: Arc<AtomicBool>,
    state: Arc<Mutex<ServerRoomsState>>,
) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_nodelay(true);
    let Ok(mut ws) = tungstenite::accept(stream) else {
        return;
    };
    configure_tcp_stream(ws.get_ref());

    let conn_id = NEXT_SERVER_CONN_ID.fetch_add(1, Ordering::Relaxed);
    let (out_tx, out_rx) = mpsc::channel::<String>();
    let mut joined_room: Option<String> = None;

    while !stop_flag.load(Ordering::Relaxed) {
        // 1. Flush outgoing messages queued by room broadcasts
        while let Ok(payload) = out_rx.try_recv() {
            if ws.send(Message::Text(payload.into())).is_err() {
                break;
            }
        }

        // 2. Poll incoming WebSocket frames
        match ws.read() {
            Ok(Message::Text(txt)) => {
                if let Ok(msg) = serde_json::from_str::<Value>(txt.as_str()) {
                    let msg_type = msg.get("type").and_then(|s| s.as_str()).unwrap_or("");
                    match msg_type {
                        "join" => {
                            let req_pass = msg
                                .get("password")
                                .and_then(|s| s.as_str())
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty());

                            let mut guard = state.lock().unwrap();
                            let room_name = sanitize_room_name(
                                msg.get("room")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or(&guard.default_room),
                            );

                            // Remove from previous room if switching
                            if let Some(prev) = joined_room.take() {
                                remove_client_from_room(&mut guard, &prev, conn_id);
                            }

                            let default_room_name = guard.default_room.clone();
                            let default_pwd = guard.default_password.clone();
                            let room = guard.rooms.entry(room_name.clone()).or_insert_with(|| {
                                let initial_pass = req_pass.clone().or_else(|| {
                                    if room_name == default_room_name {
                                        default_pwd.clone()
                                    } else {
                                        None
                                    }
                                });
                                ServerRoom {
                                    password: initial_pass,
                                    layout: None,
                                    clients: HashMap::new(),
                                }
                            });
                            if room.password.is_none() && req_pass.is_some() {
                                room.password = req_pass.clone();
                            }

                            if let Some(ref expected) = room.password {
                                if req_pass.as_ref() != Some(expected) {
                                    let err_msg = serde_json::json!({
                                        "type": "error",
                                        "code": "INVALID_PASSWORD",
                                        "message": "Invalid room password.",
                                    })
                                    .to_string();
                                    let _ = out_tx.send(err_msg);
                                    continue;
                                }
                            }

                            let user_val = msg.get("user");
                            let uid = user_val
                                .and_then(|u| u.get("id"))
                                .and_then(|s| s.as_str())
                                .filter(|s| !s.is_empty())
                                .map(ToString::to_string)
                                .unwrap_or_else(|| format!("u-{conn_id}"));
                            let uname = user_val
                                .and_then(|u| u.get("name"))
                                .and_then(|s| s.as_str())
                                .or_else(|| user_val.and_then(|u| u.as_str()))
                                .unwrap_or("Planner")
                                .trim()
                                .chars()
                                .take(32)
                                .collect::<String>();
                            let ucolor = user_val
                                .and_then(|u| u.get("color"))
                                .and_then(|s| s.as_str())
                                .unwrap_or("#3b82f6")
                                .to_string();

                            let user = CollabUser {
                                id: uid,
                                name: if uname.is_empty() {
                                    "Planner".to_string()
                                } else {
                                    uname
                                },
                                color: ucolor,
                                cursor: None,
                                selected_ids: Vec::new(),
                            };

                            if room.layout.is_none() {
                                if let Some(init_layout) = msg.get("initialLayout") {
                                    if !init_layout.is_null() {
                                        room.layout = Some(init_layout.clone());
                                    }
                                }
                            }

                            room.clients.insert(
                                conn_id,
                                RoomClient {
                                    user: user.clone(),
                                    tx: out_tx.clone(),
                                },
                            );
                            joined_room = Some(room_name.clone());

                            let init_packet = serde_json::json!({
                                "type": "init",
                                "room": room_name,
                                "user": {
                                    "id": user.id,
                                    "name": user.name,
                                    "color": user.color,
                                },
                                "users": room.users_json(),
                                "layout": room.layout,
                                "hasPassword": room.password.is_some(),
                                "isHost": room.clients.len() == 1,
                            })
                            .to_string();
                            let _ = out_tx.send(init_packet);

                            room.broadcast(
                                &serde_json::json!({
                                    "type": "user_joined",
                                    "user": {
                                        "id": user.id,
                                        "name": user.name,
                                        "color": user.color,
                                        "selectedIds": [],
                                    }
                                }),
                                Some(conn_id),
                            );
                        }
                        "layout_update" | "layout" => {
                            let Some(ref rname) = joined_room else {
                                continue;
                            };
                            let Some(layout_val) = msg.get("layout") else {
                                continue;
                            };
                            let mut guard = state.lock().unwrap();
                            if let Some(room) = guard.rooms.get_mut(rname) {
                                let (uid, uname) = room
                                    .clients
                                    .get(&conn_id)
                                    .map(|c| (c.user.id.clone(), c.user.name.clone()))
                                    .unwrap_or_default();
                                room.layout = Some(layout_val.clone());
                                room.broadcast(
                                    &serde_json::json!({
                                        "type": "layout_update",
                                        "layout": layout_val,
                                        "fromUserId": uid,
                                        "fromUserName": uname,
                                    }),
                                    Some(conn_id),
                                );
                            }
                        }
                        "cursor" => {
                            let Some(ref rname) = joined_room else {
                                continue;
                            };
                            let (Some(x), Some(y)) = (
                                msg.get("x").and_then(|v| v.as_f64()),
                                msg.get("y").and_then(|v| v.as_f64()),
                            ) else {
                                continue;
                            };
                            let mut guard = state.lock().unwrap();
                            if let Some(room) = guard.rooms.get_mut(rname) {
                                if let Some(client) = room.clients.get_mut(&conn_id) {
                                    client.user.cursor = Some([x, y]);
                                    let uid = client.user.id.clone();
                                    room.broadcast(
                                        &serde_json::json!({
                                            "type": "cursor",
                                            "userId": uid,
                                            "x": x,
                                            "y": y,
                                        }),
                                        Some(conn_id),
                                    );
                                }
                            }
                        }
                        "selection" => {
                            let Some(ref rname) = joined_room else {
                                continue;
                            };
                            let ids: Vec<String> = msg
                                .get("selectedIds")
                                .and_then(|a| a.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|s| s.as_str().map(ToString::to_string))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let mut guard = state.lock().unwrap();
                            if let Some(room) = guard.rooms.get_mut(rname) {
                                if let Some(client) = room.clients.get_mut(&conn_id) {
                                    client.user.selected_ids = ids.clone();
                                    let uid = client.user.id.clone();
                                    room.broadcast(
                                        &serde_json::json!({
                                            "type": "selection",
                                            "userId": uid,
                                            "selectedIds": ids,
                                        }),
                                        Some(conn_id),
                                    );
                                }
                            }
                        }
                        "chat" => {
                            let Some(ref rname) = joined_room else {
                                continue;
                            };
                            let text: String = msg
                                .get("text")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .trim()
                                .chars()
                                .take(500)
                                .collect();
                            if text.is_empty() {
                                continue;
                            }
                            let guard = state.lock().unwrap();
                            if let Some(room) = guard.rooms.get(rname) {
                                if let Some(client) = room.clients.get(&conn_id) {
                                    let ts = epoch_millis();
                                    room.broadcast(
                                        &serde_json::json!({
                                            "type": "chat",
                                            "id": format!("msg-{ts}-{conn_id}"),
                                            "userId": client.user.id,
                                            "userName": client.user.name,
                                            "userColor": client.user.color,
                                            "text": text,
                                            "timestamp": ts,
                                        }),
                                        None,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = ws.send(Message::Pong(data));
            }
            Ok(Message::Close(_)) => break,
            Err(ref e) if is_transient_io_err(e) => {}
            Err(_) => break,
            _ => {}
        }
    }

    if let Some(rname) = joined_room {
        let mut guard = state.lock().unwrap();
        remove_client_from_room(&mut guard, &rname, conn_id);
    }
    let _ = ws.close(None);
}

fn remove_client_from_room(state: &mut ServerRoomsState, room_name: &str, conn_id: u64) {
    if let Some(room) = state.rooms.get_mut(room_name) {
        if let Some(removed) = room.clients.remove(&conn_id) {
            room.broadcast(
                &serde_json::json!({
                    "type": "user_left",
                    "userId": removed.user.id,
                    "userName": removed.user.name,
                }),
                None,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Collaboration Client (connects to EmbeddedCollabServer, server.mjs, or Electron)
// ---------------------------------------------------------------------------

pub struct CollabClient {
    out_tx: Option<Sender<String>>,
    in_rx: Option<Receiver<CollabEvent>>,
    pub connected: bool,
    pub connecting: bool,
    pub status: String,
    pub error_message: Option<String>,
    pub server_url: String,
    pub room_id: String,
    pub password: String,
    pub user_id: String,
    pub user_name: String,
    pub user_color: String,
    pub dialog_tab: CollabDialogTab,
    pub host_port: u16,
    pub show_password: bool,
    pub is_host: bool,
    pub users: Vec<CollabUser>,
    pub chat_messages: Vec<CollabChatMessage>,
    pub chat_draft: String,
    pub last_remote_update_by: Option<String>,
    server: EmbeddedCollabServer,
}

impl Default for CollabClient {
    fn default() -> Self {
        let seq = NEXT_CLIENT_SEQ.fetch_add(1, Ordering::Relaxed);
        let color = COLLAB_COLORS[(seq as usize) % COLLAB_COLORS.len()].to_string();
        Self {
            out_tx: None,
            in_rx: None,
            connected: false,
            connecting: false,
            status: "Offline".into(),
            error_message: None,
            server_url: "ws://localhost:3000/collab".into(),
            room_id: "default".into(),
            password: String::new(),
            user_id: format!("rb-{}-{seq}", epoch_millis()),
            user_name: format!("Planer-{seq}"),
            user_color: color,
            dialog_tab: CollabDialogTab::Host,
            host_port: 3000,
            show_password: false,
            is_host: false,
            users: Vec::new(),
            chat_messages: Vec::new(),
            chat_draft: String::new(),
            last_remote_update_by: None,
            server: EmbeddedCollabServer::default(),
        }
    }
}

impl Drop for CollabClient {
    fn drop(&mut self) {
        self.disconnect();
        self.server.stop();
    }
}

impl CollabClient {
    pub fn is_active(&self) -> bool {
        self.connected || self.connecting || self.server.is_running()
    }

    pub fn is_server_running(&self) -> bool {
        self.server.is_running()
    }

    pub fn server_urls(&self) -> Option<(String, Option<String>)> {
        self.server.ws_urls()
    }

    pub fn start_host_and_connect(&mut self, initial_layout: Option<&Layout>) {
        self.disconnect();
        self.error_message = None;
        let pwd = if self.password.trim().is_empty() {
            None
        } else {
            Some(self.password.trim().to_string())
        };
        let room = sanitize_room_name(&self.room_id);
        self.room_id = room.clone();

        match self.server.start(self.host_port, pwd, &room) {
            Ok(actual_port) => {
                self.server_url = format!("ws://localhost:{actual_port}/collab");
                self.connect_with_layout(initial_layout);
            }
            Err(err) => {
                self.error_message = Some(err.clone());
                self.status = err;
            }
        }
    }

    pub fn stop_host_and_disconnect(&mut self) {
        self.disconnect();
        self.server.stop();
    }

    #[allow(dead_code)]
    pub fn connect(&mut self) {
        self.connect_with_layout(None);
    }

    pub fn connect_with_layout(&mut self, initial_layout: Option<&Layout>) {
        self.disconnect();
        self.error_message = None;

        let url = normalize_ws_url(&self.server_url);
        self.server_url = url.clone();
        let room = sanitize_room_name(&self.room_id);
        self.room_id = room.clone();
        let uid = self.user_id.clone();
        let uname = if self.user_name.trim().is_empty() {
            "Planner".to_string()
        } else {
            self.user_name.trim().to_string()
        };
        let ucolor = self.user_color.clone();
        let pwd = self.password.trim().to_string();
        let init_layout_val = initial_layout
            .and_then(|l| save_layout_json_compact(l).ok())
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());

        let (out_tx, out_rx) = mpsc::channel::<String>();
        let (in_tx, in_rx) = mpsc::channel::<CollabEvent>();
        self.out_tx = Some(out_tx);
        self.in_rx = Some(in_rx);
        self.connecting = true;
        self.status = format!("Connecting to {url}...");

        thread::spawn(move || {
            let conn = tungstenite::connect(url.as_str());
            let (mut socket, _) = match conn {
                Ok(pair) => pair,
                Err(e) => {
                    let _ = in_tx.send(CollabEvent::Disconnected(format!("Connect error: {e}")));
                    return;
                }
            };

            if let tungstenite::stream::MaybeTlsStream::Plain(tcp) = socket.get_mut() {
                configure_tcp_stream(tcp);
            }

            let mut join_obj = serde_json::json!({
                "type": "join",
                "room": room,
                "user": {
                    "id": uid,
                    "name": uname,
                    "color": ucolor,
                },
            });
            if !pwd.is_empty() {
                join_obj["password"] = Value::String(pwd);
            }
            if let Some(layout_v) = init_layout_val {
                join_obj["initialLayout"] = layout_v;
            }
            if socket.send(Message::Text(join_obj.to_string().into())).is_err() {
                let _ = in_tx.send(CollabEvent::Disconnected("Failed to send join".into()));
                return;
            }

            loop {
                // Flush outgoing messages, coalescing bursts of layout_update packets
                let mut pending = Vec::new();
                while let Ok(payload) = out_rx.try_recv() {
                    if payload == "__CLOSE__" {
                        let _ = socket.close(None);
                        let _ = in_tx.send(CollabEvent::Disconnected("Disconnected".into()));
                        return;
                    }
                    pending.push(payload);
                }

                let mut last_layout_pos = None;
                for (i, p) in pending.iter().enumerate() {
                    if p.starts_with(r#"{"type":"layout_update""#) {
                        last_layout_pos = Some(i);
                    }
                }

                for (i, payload) in pending.into_iter().enumerate() {
                    if payload.starts_with(r#"{"type":"layout_update""#)
                        && Some(i) != last_layout_pos
                    {
                        continue;
                    }
                    if socket.send(Message::Text(payload.into())).is_err() {
                        let _ = in_tx.send(CollabEvent::Disconnected("Connection lost".into()));
                        return;
                    }
                }

                // Poll incoming messages (25ms read timeout on plain TCP keeps CPU ~0%)
                match socket.read() {
                    Ok(Message::Text(txt)) => {
                        if let Ok(val) = serde_json::from_str::<Value>(txt.as_str()) {
                            let msg_type = val.get("type").and_then(|s| s.as_str()).unwrap_or("");
                            match msg_type {
                                "init" => {
                                    let is_host = val
                                        .get("isHost")
                                        .and_then(|b| b.as_bool())
                                        .unwrap_or(false);
                                    let users = val
                                        .get("users")
                                        .and_then(|u| u.as_array())
                                        .map(|arr| {
                                            arr.iter()
                                                .filter_map(parse_collab_user)
                                                .filter(|u| u.id != uid)
                                                .collect()
                                        })
                                        .unwrap_or_default();
                                    let initial_layout = val
                                        .get("layout")
                                        .filter(|v| !v.is_null())
                                        .and_then(|v| serde_json::to_string(v).ok())
                                        .and_then(|s| load_layout_json(&s).ok());
                                    let _ = in_tx.send(CollabEvent::Connected {
                                        is_host,
                                        users,
                                        initial_layout,
                                        status: format!("Connected ({room})"),
                                    });
                                }
                                "user_joined" => {
                                    if let Some(u) = val.get("user").and_then(parse_collab_user) {
                                        if u.id != uid {
                                            let _ = in_tx.send(CollabEvent::UserJoined(u));
                                        }
                                    }
                                }
                                "user_left" => {
                                    if let Some(user_id) =
                                        val.get("userId").and_then(|s| s.as_str())
                                    {
                                        let _ = in_tx.send(CollabEvent::UserLeft {
                                            user_id: user_id.to_string(),
                                        });
                                    }
                                }
                                "layout_update" => {
                                    let from_id = val
                                        .get("fromUserId")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("");
                                    if from_id != uid {
                                        let from_name = val
                                            .get("fromUserName")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("Collaborator")
                                            .to_string();
                                        if let Some(layout_val) =
                                            val.get("layout").filter(|v| !v.is_null())
                                        {
                                            if let Ok(layout_str) =
                                                serde_json::to_string(layout_val)
                                            {
                                                if let Ok(layout) = load_layout_json(&layout_str) {
                                                    let _ = in_tx.send(
                                                        CollabEvent::LayoutReceived {
                                                            layout,
                                                            from_user_name: from_name,
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                                "cursor" => {
                                    let from_id =
                                        val.get("userId").and_then(|s| s.as_str()).unwrap_or("");
                                    if from_id != uid {
                                        if let (Some(x), Some(y)) = (
                                            val.get("x").and_then(|v| v.as_f64()),
                                            val.get("y").and_then(|v| v.as_f64()),
                                        ) {
                                            let _ = in_tx.send(CollabEvent::CursorReceived {
                                                user_id: from_id.to_string(),
                                                x,
                                                y,
                                            });
                                        }
                                    }
                                }
                                "selection" => {
                                    let from_id =
                                        val.get("userId").and_then(|s| s.as_str()).unwrap_or("");
                                    if from_id != uid {
                                        let selected_ids = val
                                            .get("selectedIds")
                                            .and_then(|a| a.as_array())
                                            .map(|a| {
                                                a.iter()
                                                    .filter_map(|s| s.as_str().map(ToString::to_string))
                                                    .collect()
                                            })
                                            .unwrap_or_default();
                                        let _ = in_tx.send(CollabEvent::SelectionReceived {
                                            user_id: from_id.to_string(),
                                            selected_ids,
                                        });
                                    }
                                }
                                "chat" => {
                                    let chat = CollabChatMessage {
                                        id: val
                                            .get("id")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        user_id: val
                                            .get("userId")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        user_name: val
                                            .get("userName")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("Planner")
                                            .to_string(),
                                        user_color: val
                                            .get("userColor")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("#3b82f6")
                                            .to_string(),
                                        text: val
                                            .get("text")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        timestamp: val
                                            .get("timestamp")
                                            .and_then(|t| t.as_u64())
                                            .unwrap_or_else(epoch_millis),
                                    };
                                    let _ = in_tx.send(CollabEvent::ChatReceived(chat));
                                }
                                "error" => {
                                    let msg = val
                                        .get("message")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("Collaboration error")
                                        .to_string();
                                    let _ = in_tx.send(CollabEvent::Error(msg));
                                    let _ = socket.close(None);
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        let _ = socket.send(Message::Pong(data));
                    }
                    Ok(Message::Close(_)) => {
                        let _ = in_tx.send(CollabEvent::Disconnected("Server closed".into()));
                        return;
                    }
                    Err(ref e) if is_transient_io_err(e) => {}
                    Err(e) => {
                        let _ = in_tx.send(CollabEvent::Disconnected(format!("Error: {e}")));
                        return;
                    }
                    _ => {}
                }
            }
        });
    }

    pub fn disconnect(&mut self) {
        if let Some(tx) = self.out_tx.take() {
            let _ = tx.send("__CLOSE__".into());
        }
        self.in_rx = None;
        self.connected = false;
        self.connecting = false;
        self.is_host = false;
        self.users.clear();
        self.status = "Offline".into();
    }

    pub fn send_layout(&self, layout: &Layout) {
        if !self.connected {
            return;
        }
        if let (Some(tx), Ok(layout_json)) = (&self.out_tx, save_layout_json_compact(layout)) {
            let mut msg = String::with_capacity(34 + layout_json.len());
            msg.push_str(r#"{"type":"layout_update","layout":"#);
            msg.push_str(&layout_json);
            msg.push('}');
            let _ = tx.send(msg);
        }
    }

    pub fn send_chat(&self, text: &str) {
        if !self.connected {
            return;
        }
        let clean = text.trim();
        if clean.is_empty() {
            return;
        }
        if let Some(tx) = &self.out_tx {
            let msg = serde_json::json!({
                "type": "chat",
                "text": clean,
            })
            .to_string();
            let _ = tx.send(msg);
        }
    }

    #[allow(dead_code)]
    pub fn send_selection(&self, selected_ids: &[String]) {
        if !self.connected {
            return;
        }
        if let Some(tx) = &self.out_tx {
            let msg = serde_json::json!({
                "type": "selection",
                "selectedIds": selected_ids,
            })
            .to_string();
            let _ = tx.send(msg);
        }
    }

    #[allow(dead_code)]
    pub fn send_cursor(&self, x: f64, y: f64) {
        if !self.connected {
            return;
        }
        if let Some(tx) = &self.out_tx {
            let msg = serde_json::json!({
                "type": "cursor",
                "x": x.round(),
                "y": y.round(),
            })
            .to_string();
            let _ = tx.send(msg);
        }
    }

    pub fn poll(&mut self) -> Option<Layout> {
        let rx = self.in_rx.as_ref()?;
        let mut latest_layout = None;
        let mut disconnect_now = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                CollabEvent::Connected {
                    is_host,
                    users,
                    initial_layout,
                    status,
                } => {
                    self.connected = true;
                    self.connecting = false;
                    self.is_host = is_host;
                    self.users = users;
                    self.error_message = None;
                    self.status = status;
                    if let Some(l) = initial_layout {
                        latest_layout = Some(l);
                        self.last_remote_update_by = Some("Server".to_string());
                    }
                }
                CollabEvent::Disconnected(msg) => {
                    self.connected = false;
                    self.connecting = false;
                    self.is_host = false;
                    self.users.clear();
                    self.status = msg;
                    disconnect_now = true;
                }
                CollabEvent::Error(msg) => {
                    self.connected = false;
                    self.connecting = false;
                    self.is_host = false;
                    self.users.clear();
                    self.error_message = Some(msg.clone());
                    self.status = msg;
                    disconnect_now = true;
                }
                CollabEvent::UserJoined(user) => {
                    self.users.retain(|u| u.id != user.id);
                    self.users.push(user);
                }
                CollabEvent::UserLeft { user_id } => {
                    self.users.retain(|u| u.id != user_id);
                }
                CollabEvent::LayoutReceived {
                    layout,
                    from_user_name,
                } => {
                    latest_layout = Some(layout);
                    self.last_remote_update_by = Some(from_user_name);
                }
                CollabEvent::CursorReceived { user_id, x, y } => {
                    if let Some(u) = self.users.iter_mut().find(|u| u.id == user_id) {
                        u.cursor = Some([x, y]);
                    }
                }
                CollabEvent::SelectionReceived {
                    user_id,
                    selected_ids,
                } => {
                    if let Some(u) = self.users.iter_mut().find(|u| u.id == user_id) {
                        u.selected_ids = selected_ids;
                    }
                }
                CollabEvent::ChatReceived(chat) => {
                    self.chat_messages.push(chat);
                    if self.chat_messages.len() > 100 {
                        self.chat_messages.remove(0);
                    }
                }
            }
        }
        if disconnect_now {
            self.out_tx = None;
            self.in_rx = None;
        }
        latest_layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};
    use railbench_core::types::Transform;

    fn wait_until<F: FnMut() -> bool>(timeout_ms: u64, mut pred: F) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if pred() {
                return true;
            }
            thread::sleep(Duration::from_millis(15));
        }
        false
    }

    #[test]
    fn normalize_ws_url_handles_schemes_and_collab_path() {
        assert_eq!(normalize_ws_url("localhost:3000"), "ws://localhost:3000/collab");
        assert_eq!(
            normalize_ws_url("http://192.168.1.10:3000/"),
            "ws://192.168.1.10:3000/collab"
        );
        assert_eq!(
            normalize_ws_url("https://example.org"),
            "wss://example.org/collab"
        );
        assert_eq!(
            normalize_ws_url("ws://localhost:3000/collab"),
            "ws://localhost:3000/collab"
        );
    }

    #[test]
    fn embedded_server_two_clients_sync_layout_chat_users_and_selection() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").expect("24188 exists");
        let p0 = create_piece(
            "p0".into(),
            e24188,
            Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        );

        let mut layout_a = Layout {
            name: "Collab Plan".into(),
            scale: "H0".into(),
            pieces: vec![p0],
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
        };

        let mut host = CollabClient::default();
        host.host_port = 0;
        host.room_id = "test-room".into();
        host.user_name = "Alice".into();
        host.start_host_and_connect(Some(&layout_a));

        assert!(
            wait_until(3000, || {
                host.poll();
                host.connected
            }),
            "host failed to connect: {}",
            host.status
        );
        assert!(host.is_host);

        let mut guest = CollabClient::default();
        guest.server_url = host.server_url.clone();
        guest.room_id = "test-room".into();
        guest.user_name = "Bob".into();
        guest.connect();

        let mut guest_init_layout = None;
        assert!(
            wait_until(3000, || {
                if let Some(l) = guest.poll() {
                    guest_init_layout = Some(l);
                }
                host.poll();
                guest.connected && guest.users.len() == 1 && host.users.len() == 1
            }),
            "guest/host sync failed: guest={} host={}",
            guest.status,
            host.status
        );
        assert_eq!(
            guest_init_layout.as_ref().map(|l| l.pieces.len()),
            Some(1),
            "guest should receive host initial layout on init"
        );
        assert_eq!(host.users[0].name, "Bob");
        assert_eq!(guest.users[0].name, "Alice");

        // Host updates layout -> guest receives it, host does not get echoed back
        let p1 = create_piece(
            "p1".into(),
            e24188,
            Transform {
                x: 188.3,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        );
        layout_a.pieces.push(p1);
        host.send_layout(&layout_a);

        let mut guest_updated = None;
        let mut host_echoed = false;
        assert!(
            wait_until(3000, || {
                if let Some(l) = guest.poll() {
                    guest_updated = Some(l);
                }
                if host.poll().is_some() {
                    host_echoed = true;
                }
                guest_updated.as_ref().map_or(false, |l| l.pieces.len() == 2)
            }),
            "guest did not receive layout_update"
        );
        assert!(!host_echoed, "host should not receive its own layout echo");

        // Guest sends chat, cursor & selection -> host receives all three
        guest.send_chat("Gleis 1 fertig!");
        guest.send_selection(&["p0".to_string()]);
        guest.send_cursor(125.0, 250.0);
        assert!(
            wait_until(3000, || {
                host.poll();
                guest.poll();
                !host.chat_messages.is_empty()
                    && !guest.chat_messages.is_empty()
                    && host.users.first().map_or(false, |u| {
                        u.selected_ids == vec!["p0".to_string()] && u.cursor == Some([125.0, 250.0])
                    })
            }),
            "chat, cursor, or selection failed to propagate"
        );
        assert_eq!(host.chat_messages[0].text, "Gleis 1 fertig!");

        // Guest disconnects -> host removes guest from users list
        guest.disconnect();
        assert!(
            wait_until(3000, || {
                host.poll();
                host.users.is_empty()
            }),
            "host did not process user_left"
        );
        host.stop_host_and_disconnect();
    }

    #[test]
    fn embedded_server_enforces_room_password() {
        let mut host = CollabClient::default();
        host.host_port = 0;
        host.room_id = "locked-room".into();
        host.password = "bahn123".into();
        host.user_name = "Host".into();
        host.start_host_and_connect(None);
        assert!(wait_until(3000, || {
            host.poll();
            host.connected
        }));

        // Wrong password fails cleanly
        let mut guest = CollabClient::default();
        guest.server_url = host.server_url.clone();
        guest.room_id = "locked-room".into();
        guest.password = "wrong".into();
        guest.user_name = "Intruder".into();
        guest.connect();
        assert!(
            wait_until(3000, || {
                guest.poll();
                !guest.connecting && guest.error_message.is_some()
            }),
            "expected password error"
        );
        assert!(!guest.connected);

        // Correct password succeeds
        guest.password = "bahn123".into();
        guest.user_name = "Friend".into();
        guest.connect();
        assert!(
            wait_until(3000, || {
                guest.poll();
                host.poll();
                guest.connected
            }),
            "expected connection with valid password, got {:?}",
            guest.error_message
        );
        host.stop_host_and_disconnect();
    }
}
