use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use railbench_core::io::{load_layout_json, save_layout_json};
use railbench_core::types::Layout;
use tungstenite::Message;

pub enum CollabEvent {
    Connected(String),
    Disconnected(String),
    LayoutReceived(Layout),
}

pub struct CollabClient {
    out_tx: Option<Sender<String>>,
    in_rx: Option<Receiver<CollabEvent>>,
    pub connected: bool,
    pub status: String,
    pub server_url: String,
    pub room_id: String,
    pub user_name: String,
}

impl Default for CollabClient {
    fn default() -> Self {
        Self {
            out_tx: None,
            in_rx: None,
            connected: false,
            status: "Offline".into(),
            server_url: "ws://localhost:8765".into(),
            room_id: "railbench-demo".into(),
            user_name: "Builder".into(),
        }
    }
}

impl CollabClient {
    pub fn connect(&mut self) {
        self.disconnect();
        let url = self.server_url.trim().to_string();
        let room = self.room_id.trim().to_string();
        let user = self.user_name.trim().to_string();

        let (out_tx, out_rx) = mpsc::channel::<String>();
        let (in_tx, in_rx) = mpsc::channel::<CollabEvent>();
        self.out_tx = Some(out_tx);
        self.in_rx = Some(in_rx);
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

            let join_msg = serde_json::json!({
                "type": "join",
                "room": room,
                "user": user,
            })
            .to_string();
            let _ = socket.send(Message::Text(join_msg.into()));
            let _ = in_tx.send(CollabEvent::Connected(format!("Connected ({room})")));

            // Switch underlying TCP stream to non-blocking so we can both send & receive cleanly
            if let tungstenite::stream::MaybeTlsStream::Plain(tcp) = socket.get_mut() {
                let _ = tcp.set_nonblocking(true);
            }

            loop {
                // Flush outgoing messages
                while let Ok(payload) = out_rx.try_recv() {
                    if payload == "__CLOSE__" {
                        let _ = socket.close(None);
                        let _ = in_tx.send(CollabEvent::Disconnected("Disconnected".into()));
                        return;
                    }
                    if socket.send(Message::Text(payload.into())).is_err() {
                        let _ = in_tx.send(CollabEvent::Disconnected("Connection lost".into()));
                        return;
                    }
                }

                // Poll incoming messages
                match socket.read() {
                    Ok(Message::Text(txt)) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(txt.as_str()) {
                            if let Some(layout_val) = val.get("layout") {
                                if let Ok(layout_str) = serde_json::to_string(layout_val) {
                                    if let Ok(layout) = load_layout_json(&layout_str) {
                                        let _ = in_tx.send(CollabEvent::LayoutReceived(layout));
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        let _ = in_tx.send(CollabEvent::Disconnected("Server closed".into()));
                        return;
                    }
                    Err(tungstenite::Error::Io(ref e))
                        if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        let _ = in_tx.send(CollabEvent::Disconnected(format!("Error: {e}")));
                        return;
                    }
                    _ => {}
                }

                thread::sleep(std::time::Duration::from_millis(25));
            }
        });
    }

    pub fn disconnect(&mut self) {
        if let Some(tx) = self.out_tx.take() {
            let _ = tx.send("__CLOSE__".into());
        }
        self.connected = false;
        self.status = "Offline".into();
    }

    pub fn send_layout(&self, layout: &Layout) {
        if !self.connected {
            return;
        }
        if let (Some(tx), Ok(json)) = (&self.out_tx, save_layout_json(layout)) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                let msg = serde_json::json!({
                    "type": "layout",
                    "room": self.room_id,
                    "layout": val,
                })
                .to_string();
                let _ = tx.send(msg);
            }
        }
    }

    pub fn poll(&mut self) -> Option<Layout> {
        let rx = self.in_rx.as_ref()?;
        let mut latest_layout = None;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                CollabEvent::Connected(msg) => {
                    self.connected = true;
                    self.status = msg;
                }
                CollabEvent::Disconnected(msg) => {
                    self.connected = false;
                    self.status = msg;
                }
                CollabEvent::LayoutReceived(layout) => {
                    latest_layout = Some(layout);
                }
            }
        }
        latest_layout
    }
}
