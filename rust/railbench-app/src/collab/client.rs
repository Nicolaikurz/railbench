use super::*;


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
    pub pending_chat_toasts: Vec<(String, String)>,
    pub last_remote_update_by: Option<String>,
    pub(super) server: EmbeddedCollabServer,
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
            pending_chat_toasts: Vec::new(),
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
                                "profile" => {
                                    let from_id =
                                        val.get("userId").and_then(|s| s.as_str()).unwrap_or("");
                                    if from_id != uid && !from_id.is_empty() {
                                        let name = val
                                            .get("name")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("Planner")
                                            .to_string();
                                        let color = val
                                            .get("color")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("#3b82f6")
                                            .to_string();
                                        let _ = in_tx.send(CollabEvent::ProfileUpdated {
                                            user_id: from_id.to_string(),
                                            name,
                                            color,
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

    pub fn send_profile_update(&self) {
        if !self.connected {
            return;
        }
        if let Some(tx) = &self.out_tx {
            let uname = if self.user_name.trim().is_empty() {
                "Planner"
            } else {
                self.user_name.trim()
            };
            let msg = serde_json::json!({
                "type": "profile",
                "name": uname,
                "color": self.user_color,
            })
            .to_string();
            let _ = tx.send(msg);
        }
    }

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
                CollabEvent::ProfileUpdated {
                    user_id,
                    name,
                    color,
                } => {
                    if let Some(u) = self.users.iter_mut().find(|u| u.id == user_id) {
                        u.name = name;
                        u.color = color;
                    }
                }
                CollabEvent::ChatReceived(chat) => {
                    if chat.user_id != self.user_id {
                        self.pending_chat_toasts
                            .push((chat.user_name.clone(), chat.text.clone()));
                    }
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
