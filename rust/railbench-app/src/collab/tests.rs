use super::*;
use std::thread;
use std::time::Duration;
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
            terrain_field: None,
        };

        let mut host = CollabClient::default();
        host.server.persist_dir = None;
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
        host.server.persist_dir = None;
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

    #[test]
    fn room_persistence_port_fallback_and_live_profile_sync() {
        let dir = std::env::temp_dir().join(format!("railbench-collab-test-{}", epoch_millis()));
        let _ = std::fs::create_dir_all(&dir);

        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").expect("24188");
        let p0 = create_piece("p0".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        let init_layout = Layout {
            name: "RoomPersist".into(),
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
            terrain_field: None,
        };

        // Occupy a port to exercise free-port fallback on host start
        let occupied = TcpListener::bind(("0.0.0.0", 0)).expect("bind dummy");
        let base_port = occupied.local_addr().unwrap().port();

        let mut host = CollabClient::default();
        host.server.persist_dir = Some(dir.clone());
        host.host_port = base_port;
        host.room_id = "clubroom".into();
        host.start_host_and_connect(Some(&init_layout));
        assert!(host.server.actual_port > base_port && host.server.actual_port <= base_port + 20);
        drop(occupied);

        assert!(wait_until(3000, || {
            host.poll();
            host.connected
        }));

        let mut guest = CollabClient::default();
        guest.server_url = host.server_urls().unwrap().0;
        guest.room_id = "clubroom".into();
        guest.user_name = "Alice".into();
        guest.connect();
        assert!(wait_until(3000, || {
            host.poll();
            guest.poll();
            guest.connected && !host.users.is_empty()
        }));

        // Live profile update propagates
        guest.user_name = "AliceUpdated".into();
        guest.send_profile_update();
        assert!(wait_until(3000, || {
            host.poll();
            host.users.iter().any(|u| u.name == "AliceUpdated")
        }));

        // Trigger layout update so server writes clubroom.railbench
        host.send_layout(&init_layout);
        let plan_file = dir.join("clubroom.railbench");
        assert!(wait_until(3000, || plan_file.exists()));

        guest.disconnect();
        host.stop_host_and_disconnect();
        let _ = std::fs::remove_dir_all(&dir);
    }
