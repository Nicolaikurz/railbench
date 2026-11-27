use super::physics::{obb_overlaps, Obb2D};
use super::*;
use crate::catalog::{builtin_catalogs, create_piece, find_entry};
use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{Elevation, TrackPiece, Transform};

fn make_test_layout(pieces: Vec<TrackPiece>) -> Layout {
    Layout {
        name: "DriveTest".into(),
        scale: "H0".into(),
        pieces,
        models: None,
        feed_voltage: Some(16.0),
        catenary_feed_voltage: None,
        catenary_circuit_mode: None,
        loco_current_ma: None,
        loco_count: None,
        train_current_amps: None,
        turnout_lantern_current_ma: None,
        turnout_decoder_current_ma: None,
        layers: None,
        active_layer_id: Some("layer-1".into()),
        baseboard: None,
        baseboards: None,
        terrain_field: None,
    }
}

#[test]
fn spawn_drive_and_transition_across_connected_tracks() {
    let cats = builtin_catalogs();
    let straight = find_entry(&cats, "24188").unwrap();
    let p0 = create_piece("t0".into(), straight, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let p1 = create_piece("t1".into(), straight, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let mut layout = make_test_layout(vec![p0, p1]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim = VehicleSimState::default();
    let vid = sim.spawn_vehicle(&layout, &cats, VehicleKind::Br243, Some("t0")).unwrap();
    sim.add_wagon(&vid, VehicleKind::PassengerCoach, &layout, &cats);
    sim.running = true;

    for _ in 0..80 {
        sim.step(0.05, &layout, &cats);
    }
    assert_eq!(sim.vehicles[0].piece_id, "t1");
    assert!(sim.vehicles[0].current_speed_mm_s > 20.0);
    assert_eq!(sim.all_render_poses("H0").len(), 2);
}

#[test]
fn heavier_consist_has_more_inertia_and_wagons_follow_curves_and_ramps() {
    let cats = builtin_catalogs();
    let curve = find_entry(&cats, "24130").unwrap();
    let mut p0 = create_piece("c0".into(), curve, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    p0.elevation = Some(Elevation { start: 0.0, end: 15.0 });
    let c1 = p0.connectors[1].clone();
    let mut p1 = create_piece("c1".into(), curve, Transform { x: c1.x, y: c1.y, rotation: c1.angle }, Some("layer-1".into()));
    p1.elevation = Some(Elevation { start: 15.0, end: 30.0 });
    let mut layout = make_test_layout(vec![p0, p1]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim_light = VehicleSimState::default();
    let l_id = sim_light.spawn_vehicle(&layout, &cats, VehicleKind::V60Diesel, Some("c1")).unwrap();
    sim_light.vehicles[0].throttle = 0.8;
    sim_light.running = true;

    let mut sim_heavy = VehicleSimState::default();
    let h_id = sim_heavy.spawn_vehicle(&layout, &cats, VehicleKind::V60Diesel, Some("c1")).unwrap();
    for _ in 0..4 {
        sim_heavy.add_wagon(&h_id, VehicleKind::PassengerCoach, &layout, &cats);
    }
    sim_heavy.vehicles[0].throttle = 0.8;
    sim_heavy.running = true;

    assert!(sim_heavy.vehicles[0].total_mass_tons() > sim_light.vehicles[0].total_mass_tons() * 2.5);
    for _ in 0..12 {
        sim_light.step(0.05, &layout, &cats);
        sim_heavy.step(0.05, &layout, &cats);
    }
    assert!(
        sim_light.vehicles[0].current_speed_mm_s > sim_heavy.vehicles[0].current_speed_mm_s,
        "solo loco should accelerate faster than heavy 4-coach train"
    );
    let poses = sim_heavy.all_render_poses("H0");
    assert_eq!(poses.len(), 5);
    assert!(poses[0].pitch_rad.abs() > 0.02, "loco should pitch on ramp");
    assert!(
        (poses[0].heading_deg - poses[1].heading_deg).abs() > 3.0,
        "coupled coach must follow curve heading instead of sticking straight behind loco"
    );
    let _ = l_id;
}

#[test]
fn reversing_brakes_smoothly_through_zero_without_flipping_body() {
    let cats = builtin_catalogs();
    let straight = find_entry(&cats, "24188").unwrap();
    let p0 = create_piece("t0".into(), straight, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let p1 = create_piece("t1".into(), straight, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let p2 = create_piece("t2".into(), straight, Transform { x: 376.6, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let mut layout = make_test_layout(vec![p0, p1, p2]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim = VehicleSimState::default();
    sim.spawn_vehicle(&layout, &cats, VehicleKind::Br96Mallet, Some("t0")).unwrap();
    sim.vehicles[0].throttle = 0.7;
    sim.running = true;
    for _ in 0..40 {
        sim.step(0.05, &layout, &cats);
    }
    let hdg_fwd = sim.vehicles[0].heading_deg;
    assert!(sim.vehicles[0].velocity_mm_s > 30.0);

    sim.vehicles[0].reverse = true;
    sim.step(0.05, &layout, &cats);
    assert!(sim.vehicles[0].velocity_mm_s > 0.0, "must brake smoothly instead of instant reversal");
    for _ in 0..60 {
        sim.step(0.05, &layout, &cats);
    }
    assert!(sim.vehicles[0].velocity_mm_s < -10.0, "should accelerate smoothly in reverse");
    assert!((sim.vehicles[0].heading_deg - hdg_fwd).abs() < 1.0, "loco body must not teleport-flip 180 deg");
    assert!(sim.all_render_poses("H0")[0].driving_backward, "headlights must switch to rear when reversing");
}

#[test]
fn street_car_spawns_on_road_and_all_vehicle_models_build() {
    let cats = builtin_catalogs();
    let road = cats
        .iter()
        .find(|c| c.is_road == Some(true))
        .and_then(|c| c.entries.first())
        .expect("road catalog entry");
    let r0 = create_piece("r0".into(), road, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let layout = make_test_layout(vec![r0]);

    let mut sim = VehicleSimState::default();
    assert!(sim.spawn_vehicle(&layout, &cats, VehicleKind::Sedan, None).is_ok());
    assert!(sim.spawn_vehicle(&layout, &cats, VehicleKind::Br96Mallet, None).is_err());

    for def in VEHICLE_CATALOG {
        let mut v = Vec::new();
        let mut i = Vec::new();
        crate::models3d_mesh::push_vehicle_3d(
            &mut v,
            &mut i,
            def.kind.id(),
            [0.0, 0.0, 0.0],
            0.0,
            0.05,
            [def.length_h0_mm as f32, def.height_h0_mm as f32, def.width_h0_mm as f32],
            16.5,
            def.default_rgb,
            true,
            false,
        );
        assert!(!v.is_empty() && !i.is_empty(), "empty mesh for {}", def.kind.id());
    }
}

#[test]
fn train_wheels_match_exact_track_gauge_and_railhead_height() {
    let cats = builtin_catalogs();
    let h0_track = find_entry(&cats, "24188").expect("H0 track");
    let n_track = cats
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.gauge == Some(9.0) && e.kind == "straight")
        .expect("N 9.0mm track");

    let rail_kinds = [
        VehicleKind::Br96Mallet,
        VehicleKind::Br243,
        VehicleKind::V60Diesel,
        VehicleKind::PassengerCoach,
        VehicleKind::FreightCar,
    ];

    for (entry, expected_gauge) in [(h0_track, 16.5_f32), (n_track, 9.0_f32)] {
        let mut piece = create_piece(
            "trk".into(),
            entry,
            Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            Some("layer-1".into()),
        );
        piece.elevation = None;
        let layout = make_test_layout(vec![piece]);

        for kind in rail_kinds {
            let mut sim = VehicleSimState::default();
            sim.spawn_vehicle(&layout, &cats, kind, Some("trk")).unwrap();
            let (verts, idxs) = sim.build_vehicles_mesh_3d("H0");
            assert!(!verts.is_empty() && !idxs.is_empty());

            // Lowest vertices come from inner wheel flanges dipping ~0.72 mm*sf below rail top (9.6 mm)
            let min_y = verts.iter().map(|v| v.pos[1]).fold(f32::INFINITY, f32::min);
            let sf = (expected_gauge / 16.5).clamp(0.35, 3.0);
            assert!(
                (min_y - (RAIL_TOP_Y_MM as f32 - 0.72 * sf)).abs() < 0.35,
                "{kind:?} on gauge {expected_gauge}: lowest wheel flange min_y={min_y:.2} expected near {:.2}",
                RAIL_TOP_Y_MM as f32 - 0.72 * sf
            );

            // Flange & tread vertices near railhead height must cluster symmetrically around ±(gauge * 0.5)
            let half_g = expected_gauge * 0.5;
            let low_wheel_zs: Vec<f32> = verts
                .iter()
                .filter(|v| v.pos[1] < RAIL_TOP_Y_MM as f32 + 0.6 * sf)
                .map(|v| v.pos[2].abs())
                .collect();
            assert!(!low_wheel_zs.is_empty(), "{kind:?} should have wheel vertices near railhead");
            let max_abs_z = low_wheel_zs.iter().copied().fold(0.0_f32, f32::max);
            assert!(
                (max_abs_z - half_g).abs() < 1.3 * sf,
                "{kind:?} on gauge {expected_gauge}: wheel outer z={max_abs_z:.2} should align with half-gauge {half_g:.2}"
            );
        }
    }
}

#[test]
fn turnout_lantern_follows_active_switch_direction() {
    let mut sim = VehicleSimState::default();
    assert_eq!(sim.turnout_version, 0);
    sim.toggle_turnout("sw-1", 2);
    assert_eq!(sim.turnout_states.get("sw-1").copied(), Some(1));
    assert_eq!(sim.turnout_version, 1);

    let (mut v_straight, mut i_straight) = (Vec::new(), Vec::new());
    crate::models3d_mesh::build_turnout_accessories_3d(
        &mut v_straight,
        &mut i_straight,
        [26.0, 0.0, -22.0],
        [26.0, 0.0, 0.0],
        0.0,
        "turnout-left",
        true,
        true,
        0,
        false,
    );
    let (mut v_div, mut i_div) = (Vec::new(), Vec::new());
    crate::models3d_mesh::build_turnout_accessories_3d(
        &mut v_div,
        &mut i_div,
        [26.0, 0.0, -22.0],
        [26.0, 0.0, 0.0],
        0.0,
        "turnout-left",
        true,
        true,
        1,
        false,
    );
    assert!(!v_straight.is_empty() && !v_div.is_empty());
    let pos_straight: Vec<[f32; 3]> = v_straight.iter().map(|v| v.pos).collect();
    let pos_div: Vec<[f32; 3]> = v_div.iter().map(|v| v.pos).collect();
    assert_ne!(pos_straight, pos_div, "lantern geometry must reflect turnout route direction");
}

#[test]
fn turntable_rotates_loco_and_transfer_table_slides_loco() {
    let cats = builtin_catalogs();
    let tt_entry = find_entry(&cats, "74862").expect("turntable 74862");
    let tt = create_piece("tt-1".into(), tt_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let tr_entry = cats
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.kind == "transfer-table")
        .expect("transfer-table entry");
    let tr = create_piece("tr-1".into(), tr_entry, Transform { x: 800.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let layout = make_test_layout(vec![tt, tr]);

    let mut sim = VehicleSimState::default();
    sim.spawn_vehicle(&layout, &cats, VehicleKind::Br96Mallet, Some("tt-1")).unwrap();
    sim.spawn_vehicle(&layout, &cats, VehicleKind::V60Diesel, Some("tr-1")).unwrap();
    let hdg_before = sim.vehicles[0].heading_deg;
    let y_before = sim.vehicles[1].world_y;

    sim.step_turntable("tt-1", Some(tt_entry), 1);
    sim.step_transfer_table("tr-1", Some(tr_entry), 1);
    assert!(sim.has_moving_bridges());
    for _ in 0..40 {
        sim.step_bridges(0.05, &layout, &cats);
    }
    assert!((sim.vehicles[0].heading_deg - hdg_before).abs() > 5.0, "loco must rotate with turntable bridge");
    assert!((sim.vehicles[1].world_y - y_before).abs() > 10.0, "loco must slide laterally with transfer table");
    assert!(sim.bridge_mesh_version > 0);
}

#[test]
fn train_stops_before_rear_wagon_of_train_ahead() {
    let cats = builtin_catalogs();
    let straight = find_entry(&cats, "24188").unwrap();
    let pieces: Vec<TrackPiece> = (0..5)
        .map(|i| {
            create_piece(
                format!("t{i}"),
                straight,
                Transform { x: i as f64 * 188.3, y: 0.0, rotation: 0.0 },
                Some("layer-1".into()),
            )
        })
        .collect();
    let mut layout = make_test_layout(pieces);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim = VehicleSimState::default();
    let lead_id = sim.spawn_vehicle(&layout, &cats, VehicleKind::Br243, Some("t3")).unwrap();
    sim.add_wagon(&lead_id, VehicleKind::PassengerCoach, &layout, &cats);
    sim.vehicles[0].throttle = 0.0;

    sim.spawn_vehicle(&layout, &cats, VehicleKind::V60Diesel, Some("t0")).unwrap();
    sim.vehicles[1].throttle = 0.9;
    sim.running = true;

    for _ in 0..140 {
        sim.step(0.05, &layout, &cats);
    }
    let lead_rear_pose = sim.vehicles[0].cached_poses.last().unwrap();
    let follow_front_pose = &sim.vehicles[1].cached_poses[0];
    let obb_a = Obb2D::from_pose(lead_rear_pose, 0, 0.0, 0.0);
    let obb_b = Obb2D::from_pose(follow_front_pose, 1, 0.0, 0.0);
    assert!(!obb_overlaps(&obb_a, &obb_b, 26.0), "following train must never drive through rear wagon");
    assert!(sim.vehicles[1].blocked_ahead, "following train should report blocked_ahead");
    assert!(sim.vehicles[1].current_speed_mm_s < 1.0, "following train should come to a full stop");
}

#[test]
fn cars_do_not_drive_through_each_other_on_street() {
    let cats = builtin_catalogs();
    let road = cats
        .iter()
        .find(|c| c.is_road == Some(true))
        .and_then(|c| c.entries.iter().find(|e| e.kind == "straight"))
        .expect("straight road entry");
    let len = road.length.unwrap_or(200.0);
    let r0 = create_piece("r0".into(), road, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let r1 = create_piece("r1".into(), road, Transform { x: len, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let mut layout = make_test_layout(vec![r0, r1]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim = VehicleSimState::default();
    sim.spawn_vehicle(&layout, &cats, VehicleKind::Bus, Some("r1")).unwrap();
    sim.vehicles[0].throttle = 0.0;
    sim.spawn_vehicle(&layout, &cats, VehicleKind::Sedan, Some("r0")).unwrap();
    sim.vehicles[1].throttle = 0.9;
    sim.running = true;

    for _ in 0..100 {
        sim.step(0.05, &layout, &cats);
    }
    let obb_bus = Obb2D::from_pose(&sim.vehicles[0].cached_poses[0], 0, 0.0, 0.0);
    let obb_car = Obb2D::from_pose(&sim.vehicles[1].cached_poses[0], 1, 0.0, 0.0);
    assert!(!obb_overlaps(&obb_bus, &obb_car, 26.0), "car must not drive through bus ahead");
    assert!(sim.vehicles[1].blocked_ahead);
    assert!(sim.vehicles[1].current_speed_mm_s < 1.0);
}

#[test]
fn three_way_turnout_drives_straight_left_and_right_correctly() {
    let cats = builtin_catalogs();
    let straight = find_entry(&cats, "24188").unwrap();
    let three_way = cats
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.kind == "three-way")
        .expect("three-way catalog entry");

    let entry_p = create_piece("app".into(), straight, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let sw = create_piece("sw3".into(), three_way, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let mut pieces = vec![entry_p, sw.clone()];
    for ci in 1..=3 {
        let cn = &sw.connectors[ci];
        pieces.push(create_piece(
            format!("exit-{ci}"),
            straight,
            Transform { x: cn.x, y: cn.y, rotation: cn.angle },
            Some("layer-1".into()),
        ));
    }
    let mut layout = make_test_layout(pieces);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    for (state, expected_exit) in [(0usize, "exit-1"), (1usize, "exit-2"), (2usize, "exit-3")] {
        let mut sim = VehicleSimState::default();
        sim.turnout_states.insert("sw3".into(), state);
        sim.spawn_vehicle(&layout, &cats, VehicleKind::Br243, Some("app")).unwrap();
        sim.vehicles[0].throttle = 0.7;
        sim.running = true;
        for _ in 0..85 {
            sim.step(0.05, &layout, &cats);
            if sim.vehicles[0].piece_id == expected_exit {
                break;
            }
        }
        assert_eq!(
            sim.vehicles[0].piece_id, expected_exit,
            "three-way state {state} must route train into {expected_exit}"
        );
    }
}

#[test]
fn turntable_centers_incoming_loco_and_only_bridge_rotates() {
    let cats = builtin_catalogs();
    let straight = find_entry(&cats, "24188").unwrap();
    let tt_entry = find_entry(&cats, "74862").expect("turntable 74862");

    let mut tt = create_piece("tt-1".into(), tt_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    tt.has_catenary = Some(true);
    let c180 = &tt.connectors[15];
    let app = create_piece(
        "app-1".into(),
        straight,
        Transform { x: c180.x - 188.3, y: c180.y, rotation: 0.0 },
        Some("layer-1".into()),
    );
    let mut layout = make_test_layout(vec![app, tt.clone()]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim = VehicleSimState::default();
    sim.spawn_vehicle(&layout, &cats, VehicleKind::Br96Mallet, Some("app-1")).unwrap();
    sim.vehicles[0].throttle = 0.7;
    sim.running = true;

    for _ in 0..160 {
        sim.step(0.05, &layout, &cats);
    }
    assert_eq!(sim.vehicles[0].piece_id, "tt-1");
    assert!(
        sim.vehicles[0].world_x.abs() < 4.0 && sim.vehicles[0].world_y.abs() < 4.0,
        "incoming loco should stop centered on turntable bridge, got ({}, {})",
        sim.vehicles[0].world_x,
        sim.vehicles[0].world_y
    );
    assert!(sim.vehicles[0].current_speed_mm_s < 1.0);

    let hdg_before = sim.vehicles[0].heading_deg;
    sim.step_turntable("tt-1", Some(tt_entry), 3);
    assert!(sim.is_piece_bridge_moving("tt-1"));
    sim.vehicles[0].throttle = 0.8;
    for _ in 0..30 {
        sim.step_bridges(0.05, &layout, &cats);
        sim.step(0.05, &layout, &cats);
    }
    assert!(
        sim.vehicles[0].world_x.abs() < 4.0 && sim.vehicles[0].world_y.abs() < 4.0,
        "loco must rotate in place around turntable center"
    );
    assert!((sim.vehicles[0].heading_deg - hdg_before).abs() > 25.0);

    let (mut v0, mut i0) = (Vec::new(), Vec::new());
    crate::turntables_3d::push_turntable_3d(&mut v0, &mut i0, &tt, Some(tt_entry), 0.0, 0.0);
    let (mut v1, mut i1) = (Vec::new(), Vec::new());
    crate::turntables_3d::push_turntable_3d(&mut v1, &mut i1, &tt, Some(tt_entry), 0.0, 36.0);
    let outer_cat_0: Vec<[f32; 3]> = v0
        .iter()
        .filter(|v| v.pos[1] > 25.0 && (v.pos[0].hypot(v.pos[2])) > 160.0)
        .map(|v| v.pos)
        .collect();
    let outer_cat_1: Vec<[f32; 3]> = v1
        .iter()
        .filter(|v| v.pos[1] > 25.0 && (v.pos[0].hypot(v.pos[2])) > 160.0)
        .map(|v| v.pos)
        .collect();
    assert!(!outer_cat_0.is_empty());
    assert_eq!(outer_cat_0, outer_cat_1, "outer catenary spider must remain stationary while only bridge turns");
}

#[test]
fn double_slip_24624_switches_all_four_entries_straight_and_curved() {
    let cats = builtin_catalogs();
    let straight = find_entry(&cats, "24188").expect("24188");
    let dkw_entry = find_entry(&cats, "24624").expect("24624");
    let crs_entry = find_entry(&cats, "24640").expect("24640");

    assert_eq!(crate::turnout_state_count(&dkw_entry.kind, 4), 2);
    assert_eq!(crate::turnout_state_count("three-way", 4), 3);

    let dkw = create_piece(
        "dkw".into(),
        dkw_entry,
        Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        Some("layer-1".into()),
    );
    let ang_rad = 24.3_f64.to_radians();
    let leg_len = straight.length.unwrap_or(188.3);
    let leg0 = create_piece(
        "leg-0".into(),
        straight,
        Transform { x: dkw.connectors[0].x - leg_len, y: dkw.connectors[0].y, rotation: 0.0 },
        Some("layer-1".into()),
    );
    let leg1 = create_piece(
        "leg-1".into(),
        straight,
        Transform { x: dkw.connectors[1].x, y: dkw.connectors[1].y, rotation: 0.0 },
        Some("layer-1".into()),
    );
    let leg2 = create_piece(
        "leg-2".into(),
        straight,
        Transform {
            x: dkw.connectors[2].x - leg_len * ang_rad.cos(),
            y: dkw.connectors[2].y - leg_len * ang_rad.sin(),
            rotation: 24.3,
        },
        Some("layer-1".into()),
    );
    let leg3 = create_piece(
        "leg-3".into(),
        straight,
        Transform { x: dkw.connectors[3].x, y: dkw.connectors[3].y, rotation: 24.3 },
        Some("layer-1".into()),
    );

    let mut layout = make_test_layout(vec![dkw, leg0, leg1, leg2, leg3]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
    let dkw_connected = layout
        .pieces
        .iter()
        .find(|p| p.id == "dkw")
        .unwrap()
        .connectors
        .iter()
        .all(|c| c.connected_to.is_some());
    assert!(dkw_connected, "all 4 connectors of 24624 must connect cleanly");

    let routes = [
        // State 0: Straight Crossing (0 <-> 1, 2 <-> 3)
        (0usize, "leg-0", true, "leg-1"),
        (0usize, "leg-1", false, "leg-0"),
        (0usize, "leg-2", true, "leg-3"),
        (0usize, "leg-3", false, "leg-2"),
        // State 1: Curved Slip (0 <-> 3, 2 <-> 1)
        (1usize, "leg-0", true, "leg-3"),
        (1usize, "leg-3", false, "leg-0"),
        (1usize, "leg-2", true, "leg-1"),
        (1usize, "leg-1", false, "leg-2"),
    ];

    for (state, start_leg, fwd, expected_exit) in routes {
        let mut sim = VehicleSimState::default();
        sim.turnout_states.insert("dkw".into(), state);
        sim.spawn_vehicle(&layout, &cats, VehicleKind::Br243, Some(start_leg)).unwrap();
        sim.vehicles[0].forward = fwd;
        sim.vehicles[0].throttle = 0.7;
        sim.running = true;
        for _ in 0..90 {
            sim.step(0.05, &layout, &cats);
            if sim.vehicles[0].piece_id == expected_exit {
                break;
            }
        }
        assert_eq!(
            sim.vehicles[0].piece_id, expected_exit,
            "DKW state {state} from {start_leg} (fwd={fwd}) must route to {expected_exit}"
        );
        assert_eq!(
            sim.turnout_states.get("dkw").copied(),
            Some(state),
            "passing train must not overwrite DKW switch state"
        );
    }

    let mut toggle_sim = VehicleSimState::default();
    let cnt = crate::turnout_state_count(&dkw_entry.kind, 4);
    toggle_sim.toggle_turnout("dkw", cnt);
    assert_eq!(toggle_sim.turnout_states.get("dkw").copied(), Some(1));
    toggle_sim.toggle_turnout("dkw", cnt);
    assert_eq!(toggle_sim.turnout_states.get("dkw").copied(), Some(0));

    // Also verify plain crossing 24640 routes 0->1 and 2->3 independently without teleporting
    let crs = create_piece(
        "crs".into(),
        crs_entry,
        Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        Some("layer-1".into()),
    );
    let c_leg0 = create_piece(
        "cleg-0".into(),
        straight,
        Transform { x: crs.connectors[0].x - leg_len, y: crs.connectors[0].y, rotation: 0.0 },
        Some("layer-1".into()),
    );
    let c_leg1 = create_piece(
        "cleg-1".into(),
        straight,
        Transform { x: crs.connectors[1].x, y: crs.connectors[1].y, rotation: 0.0 },
        Some("layer-1".into()),
    );
    let c_leg2 = create_piece(
        "cleg-2".into(),
        straight,
        Transform {
            x: crs.connectors[2].x - leg_len * ang_rad.cos(),
            y: crs.connectors[2].y - leg_len * ang_rad.sin(),
            rotation: 24.3,
        },
        Some("layer-1".into()),
    );
    let c_leg3 = create_piece(
        "cleg-3".into(),
        straight,
        Transform { x: crs.connectors[3].x, y: crs.connectors[3].y, rotation: 24.3 },
        Some("layer-1".into()),
    );
    let mut crs_layout = make_test_layout(vec![crs, c_leg0, c_leg1, c_leg2, c_leg3]);
    update_layout_connections(&mut crs_layout, 4.0, 35.0, 1.0);
    for (start_leg, expected_exit) in [("cleg-0", "cleg-1"), ("cleg-2", "cleg-3")] {
        let mut sim = VehicleSimState::default();
        sim.spawn_vehicle(&crs_layout, &cats, VehicleKind::Br243, Some(start_leg)).unwrap();
        sim.vehicles[0].forward = true;
        sim.vehicles[0].throttle = 0.7;
        sim.running = true;
        for _ in 0..90 {
            sim.step(0.05, &crs_layout, &cats);
            if sim.vehicles[0].piece_id == expected_exit {
                break;
            }
        }
        assert_eq!(sim.vehicles[0].piece_id, expected_exit, "crossing from {start_leg} must route to {expected_exit}");
    }
}

#[test]
fn maerklin_24997_is_recognized_as_uncoupler_and_disconnects_train_wagons() {
    let cats = builtin_catalogs();
    let e_std = find_entry(&cats, "24188").unwrap();
    let e_unc = find_entry(&cats, "24997").expect("Maerklin C-Gleis 24997 in catalog");
    assert!(!crate::catalog::is_uncoupler_entry(e_std));
    assert!(crate::catalog::is_uncoupler_entry(e_unc));

    // Layout: [24188] -> [24997 uncoupler] -> [24188] -> [24188] -> [24188]
    let p0 = create_piece("t0".into(), e_std, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let c0_end = p0.connectors[1].clone();
    let p1 = create_piece("u1".into(), e_unc, Transform { x: c0_end.x, y: c0_end.y, rotation: c0_end.angle }, Some("layer-1".into()));
    let c1_end = p1.connectors[1].clone();
    let p2 = create_piece("t2".into(), e_std, Transform { x: c1_end.x, y: c1_end.y, rotation: c1_end.angle }, Some("layer-1".into()));
    let c2_end = p2.connectors[1].clone();
    let p3 = create_piece("t3".into(), e_std, Transform { x: c2_end.x, y: c2_end.y, rotation: c2_end.angle }, Some("layer-1".into()));
    let c3_end = p3.connectors[1].clone();
    let p4 = create_piece("t4".into(), e_std, Transform { x: c3_end.x, y: c3_end.y, rotation: c3_end.angle }, Some("layer-1".into()));

    let mut layout = make_test_layout(vec![p0, p1, p2, p3, p4]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim = VehicleSimState::default();
    let vid = sim.spawn_vehicle(&layout, &cats, VehicleKind::Br243, Some("t2")).unwrap();
    sim.add_wagon(&vid, VehicleKind::PassengerCoach, &layout, &cats);
    sim.add_wagon(&vid, VehicleKind::FreightCar, &layout, &cats);

    // Position locomotive at dist_mm=50 on t2 (world_x = 332.5) so coupler #0 (~world_x = 235.1)
    // rests directly over the center of Maerklin 24997 u1 (world_x = 188.3..282.5).
    sim.vehicles[0].dist_mm = 50.0;
    recompute_vehicle_poses(
        &mut sim.vehicles[0],
        &layout,
        &cats,
        &sim.turnout_states,
        &sim.turntable_angles_deg,
        &sim.transfer_offsets_mm,
    );
    assert!(
        sim.find_coupler_over_piece("u1", &layout, &cats).is_some(),
        "coupler should align above Maerklin 24997 uncoupler"
    );

    // 1. Trigger Maerklin 24997 uncoupler -> raises ramp and disconnects both wagons as a parked cut
    let cut_id = sim
        .trigger_uncoupler("u1", &layout, &cats)
        .expect("24997 uncoupler should detach wagon cut");
    assert!(sim.is_uncoupler_raised("u1"));
    assert_eq!(sim.vehicles.len(), 2);
    assert_eq!(sim.vehicles[0].coupled_cars.len(), 0);
    let cut = sim.vehicles.iter().find(|v| v.id == cut_id).unwrap();
    assert_eq!(cut.kind, VehicleKind::PassengerCoach);
    assert_eq!(cut.coupled_cars, vec![VehicleKind::FreightCar]);
    assert_eq!(cut.velocity_mm_s, 0.0);
    let cut_x = cut.world_x;

    // 2. Locomotive pulls away cleanly while detached wagons stay parked behind
    sim.vehicles[0].throttle = 0.8;
    sim.running = true;
    for _ in 0..25 {
        sim.step(0.05, &layout, &cats);
        sim.step_uncouplers(0.05, &layout, &cats);
    }
    assert!(sim.vehicles[0].world_x > cut_x + 150.0, "loco must pull cleanly away from uncoupled cut");
    assert!(!sim.vehicles[0].blocked_ahead);

    // 3. Verify timer lowers ramp after >= 2.0 s, and direct train uncouple + re-couple work
    for _ in 0..25 {
        sim.step_uncouplers(0.05, &layout, &cats);
    }
    assert!(!sim.is_uncoupler_raised("u1"));

    // Spawn second train directly on t4 with 2 wagons and test direct uncouple_last + couple_nearby
    let v2 = sim.spawn_vehicle(&layout, &cats, VehicleKind::V60Diesel, Some("t4")).unwrap();
    sim.add_wagon(&v2, VehicleKind::FreightCar, &layout, &cats);
    sim.add_wagon(&v2, VehicleKind::PassengerCoach, &layout, &cats);
    let det2 = sim.uncouple_last_wagon(&v2, &layout, &cats).expect("direct uncouple_last");
    assert_eq!(sim.vehicles.iter().find(|v| v.id == v2).unwrap().coupled_cars.len(), 1);
    assert!(sim.vehicles.iter().any(|v| v.id == det2));
    assert!(sim.couple_nearby_wagons(&v2, &layout, &cats) >= 1);
    assert_eq!(sim.vehicles.iter().find(|v| v.id == v2).unwrap().coupled_cars.len(), 2);

    // 4. Verify armed Auto-Uncouple mode decouples cleanly when passing over 24997
    assert!(sim.toggle_uncoupler_armed("u1"));
    assert!(sim.is_uncoupler_raised("u1"));
    let v1_idx = sim.vehicles.iter().position(|v| v.id == vid).unwrap();
    sim.vehicles[v1_idx].uncouple_cooldown_s = 0.0;
    sim.vehicles[v1_idx].piece_id = "t2".to_string();
    sim.vehicles[v1_idx].dist_mm = 50.0;
    sim.add_wagon(&vid, VehicleKind::FreightCar, &layout, &cats);
    let before_cnt = sim.vehicles.len();
    sim.step_uncouplers(0.05, &layout, &cats);
    assert_eq!(sim.vehicles.len(), before_cnt + 1);
}

#[test]
fn vehicles_mesh_cache_reuses_arc_and_invalidates_on_visual_change() {
    let cats = builtin_catalogs();
    let e24188 = find_entry(&cats, "24188").unwrap();
    let p1 = create_piece("t1".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let p2 = create_piece("t2".into(), e24188, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
    let mut layout = make_test_layout(vec![p1, p2]);
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);

    let mut sim = VehicleSimState::default();
    assert!(sim.get_or_build_vehicles_mesh_3d("H0").is_none());

    sim.spawn_vehicle(&layout, &cats, VehicleKind::Br243, Some("t1")).unwrap();
    let (v1, m1) = sim.get_or_build_vehicles_mesh_3d("H0").expect("mesh built");
    let (v2, m2) = sim.get_or_build_vehicles_mesh_3d("H0").expect("mesh reused");
    assert_eq!(v1, v2);
    assert!(std::sync::Arc::ptr_eq(&m1, &m2), "unchanged state must reuse same Arc<SceneMesh3D>");

    // Toggling headlights invalidates fingerprint and bumps version
    sim.vehicles[0].headlights = !sim.vehicles[0].headlights;
    let (v3, m3) = sim.get_or_build_vehicles_mesh_3d("H0").expect("rebuilt after headlight toggle");
    assert_ne!(v3, v2);
    assert!(!std::sync::Arc::ptr_eq(&m2, &m3));

    // Stepping simulation invalidates and rebuilds mesh
    sim.vehicles[0].throttle = 0.8;
    sim.running = true;
    sim.step(0.05, &layout, &cats);
    let (v4, _m4) = sim.get_or_build_vehicles_mesh_3d("H0").expect("rebuilt after sim step");
    assert_ne!(v4, v3);

    sim.clear_all();
    assert!(sim.get_or_build_vehicles_mesh_3d("H0").is_none());
}

#[test]
fn cars_only_drive_on_roads_and_trains_only_drive_on_tracks() {
    let cats = builtin_catalogs();
    let e24188 = find_entry(&cats, "24188").unwrap();
    let st200 = find_entry(&cats, "ST-S200").unwrap();

    // Put rail piece (0..188.3) and road piece (188.3..388.3) right next to each other
    let mut rail_p = create_piece("r1".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let mut road_p = create_piece("s1".into(), st200, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
    // Deliberately inject invalid cross-type connector links to verify physics & snapping reject them
    rail_p.connectors[1].connected_to = Some(railbench_core::types::ConnectorRef {
        piece_id: "s1".into(),
        connector_index: 0,
    });
    road_p.connectors[0].connected_to = Some(railbench_core::types::ConnectorRef {
        piece_id: "r1".into(),
        connector_index: 1,
    });
    let mut layout = make_test_layout(vec![rail_p, road_p]);

    let mut sim = VehicleSimState::default();
    // Even if preferred_piece_id points to the wrong piece type, vehicles must only spawn on their matching network
    let train_id = sim
        .spawn_vehicle(&layout, &cats, VehicleKind::Br243, Some("s1"))
        .expect("train falls back to rail track");
    let car_id = sim
        .spawn_vehicle(&layout, &cats, VehicleKind::Sedan, Some("r1"))
        .expect("car falls back to street piece");

    sim.vehicles.iter_mut().for_each(|v| {
        v.throttle = 1.0;
        if v.id == car_id {
            v.reverse = true; // Drive toward the r1/s1 seam at x=188.3
        }
    });
    sim.running = true;

    for _ in 0..80 {
        sim.step(0.05, &layout, &cats);
        let tr = sim.vehicles.iter().find(|v| v.id == train_id).unwrap();
        let cr = sim.vehicles.iter().find(|v| v.id == car_id).unwrap();
        assert_eq!(tr.piece_id, "r1", "train must stay on rail track r1, never cross onto road s1");
        assert_eq!(cr.piece_id, "s1", "car must stay on street s1, never cross onto track r1");
    }

    // Also verify sanitize_for_layout removes vehicles whose piece disappeared or changed domain
    layout.pieces[1].catalog_id = "24188".into();
    sim.step(0.05, &layout, &cats);
    assert!(sim.vehicles.iter().any(|v| v.id == train_id));
    assert!(!sim.vehicles.iter().any(|v| v.id == car_id), "car must be removed if piece becomes a track");
}

#[test]
fn imported_is_road_system_catalog_restricts_trains_and_allows_cars() {
    let raw = r#"{
        "id": "custom-city-streets",
        "name": "Custom City Streets",
        "scale": "H0",
        "isRoadSystem": true,
        "entries": [
            { "id": "MY-AVE-200", "name": "Avenue 200", "kind": "straight", "length": 200 }
        ]
    }"#;
    let cat = crate::catalog_store::parse_catalog_json(raw).expect("valid road catalog");
    assert_eq!(cat.is_road, Some(true));
    assert_eq!(cat.entries[0].is_road, Some(true));

    let cats = vec![cat];
    assert!(crate::catalog::is_road_id(&cats, "MY-AVE-200"));
    let ave = find_entry(&cats, "MY-AVE-200").unwrap();
    let street = create_piece("st1".into(), ave, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let layout = make_test_layout(vec![street]);

    let mut sim = VehicleSimState::default();
    assert!(sim.spawn_vehicle(&layout, &cats, VehicleKind::Br243, None).is_err());
    let cid = sim.spawn_vehicle(&layout, &cats, VehicleKind::Sedan, None).unwrap();
    assert_eq!(sim.vehicles[0].id, cid);
    assert_eq!(sim.vehicles[0].piece_id, "st1");
}
