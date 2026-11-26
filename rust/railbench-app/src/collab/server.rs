use super::*;

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
    persist_dir: Option<PathBuf>,
    rooms: HashMap<String, ServerRoom>,
}

fn default_saved_plans_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".railbench").join("saved_plans"))
}

fn room_plan_path(persist_dir: Option<&Path>, room_name: &str) -> Option<PathBuf> {
    let dir = persist_dir?;
    Some(dir.join(format!("{}.railbench", sanitize_room_name(room_name))))
}

fn load_room_layout_from_disk(persist_dir: Option<&Path>, room_name: &str) -> Option<Value> {
    let path = room_plan_path(persist_dir, room_name)?;
    let raw = std::fs::read_to_string(path).ok()?;
    let val: Value = serde_json::from_str(&raw).ok()?;
    if val.is_object() {
        Some(val)
    } else {
        None
    }
}

fn save_room_layout_to_disk(persist_dir: Option<&Path>, room_name: &str, layout_val: &Value) {
    let Some(path) = room_plan_path(persist_dir, room_name) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(layout_val) {
        let _ = std::fs::write(path, json);
    }
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
    pub persist_dir: Option<PathBuf>,
}

impl Default for EmbeddedCollabServer {
    fn default() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            state: Arc::new(Mutex::new(ServerRoomsState {
                default_room: "default".to_string(),
                default_password: None,
                persist_dir: default_saved_plans_dir(),
                rooms: HashMap::new(),
            })),
            accept_thread: None,
            actual_port: 0,
            lan_ip: None,
            persist_dir: default_saved_plans_dir(),
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

        let listener = if port == 0 {
            TcpListener::bind(("0.0.0.0", 0))
                .map_err(|e| format!("Failed to bind ephemeral port: {e}"))?
        } else {
            let mut bound = None;
            let mut last_err = None;
            for candidate in port..=port.saturating_add(20) {
                match TcpListener::bind(("0.0.0.0", candidate)) {
                    Ok(l) => {
                        bound = Some(l);
                        break;
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            bound.ok_or_else(|| {
                let err = last_err
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "unknown error".into());
                format!("Failed to bind ports {port}..={}: {err}", port.saturating_add(20))
            })?
        };
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
            persist_dir: self.persist_dir.clone(),
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
                            let persist_dir = guard.persist_dir.clone();
                            let room = guard.rooms.entry(room_name.clone()).or_insert_with(|| {
                                let initial_pass = req_pass.clone().or_else(|| {
                                    if room_name == default_room_name {
                                        default_pwd.clone()
                                    } else {
                                        None
                                    }
                                });
                                let persisted_layout =
                                    load_room_layout_from_disk(persist_dir.as_deref(), &room_name);
                                ServerRoom {
                                    password: initial_pass,
                                    layout: persisted_layout,
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
                            let persist_dir = guard.persist_dir.clone();
                            if let Some(room) = guard.rooms.get_mut(rname) {
                                let (uid, uname) = room
                                    .clients
                                    .get(&conn_id)
                                    .map(|c| (c.user.id.clone(), c.user.name.clone()))
                                    .unwrap_or_default();
                                room.layout = Some(layout_val.clone());
                                save_room_layout_to_disk(persist_dir.as_deref(), rname, layout_val);
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
                        "profile" => {
                            let Some(ref rname) = joined_room else {
                                continue;
                            };
                            let new_name = msg
                                .get("name")
                                .and_then(|s| s.as_str())
                                .map(|s| s.trim().chars().take(32).collect::<String>())
                                .filter(|s| !s.is_empty());
                            let new_color = msg
                                .get("color")
                                .and_then(|s| s.as_str())
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty());
                            let mut guard = state.lock().unwrap();
                            if let Some(room) = guard.rooms.get_mut(rname) {
                                if let Some(client) = room.clients.get_mut(&conn_id) {
                                    if let Some(n) = new_name {
                                        client.user.name = n;
                                    }
                                    if let Some(c) = new_color {
                                        client.user.color = c;
                                    }
                                    let uid = client.user.id.clone();
                                    let uname = client.user.name.clone();
                                    let ucol = client.user.color.clone();
                                    room.broadcast(
                                        &serde_json::json!({
                                            "type": "profile",
                                            "userId": uid,
                                            "name": uname,
                                            "color": ucol,
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
