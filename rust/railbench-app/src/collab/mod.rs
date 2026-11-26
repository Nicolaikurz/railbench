use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
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

pub(super) static NEXT_CLIENT_SEQ: AtomicU64 = AtomicU64::new(1);
pub(super) static NEXT_SERVER_CONN_ID: AtomicU64 = AtomicU64::new(1);

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
    ProfileUpdated {
        user_id: String,
        name: String,
        color: String,
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

pub(super) fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(super) fn sanitize_room_name(raw: &str) -> String {
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

pub(super) fn detect_lan_ipv4() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    let addr = sock.local_addr().ok()?;
    match addr.ip() {
        std::net::IpAddr::V4(v4) if !v4.is_loopback() => Some(v4.to_string()),
        _ => None,
    }
}

pub(super) fn is_transient_io_err(err: &tungstenite::Error) -> bool {
    matches!(
        err,
        tungstenite::Error::Io(io_err)
            if matches!(io_err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
    )
}

pub(super) fn parse_collab_user(v: &Value) -> Option<CollabUser> {
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

pub(super) fn configure_tcp_stream(tcp: &TcpStream) {
    let _ = tcp.set_nonblocking(false);
    let _ = tcp.set_read_timeout(Some(Duration::from_millis(25)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(5)));
    let _ = tcp.set_nodelay(true);
}


mod server;
mod client;
#[cfg(test)]
mod tests;

pub use server::EmbeddedCollabServer;
pub use client::CollabClient;
