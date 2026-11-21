use super::*;
use crate::catalog::{builtin_catalogs, create_piece, find_entry};

fn make_test_shed(x: f64, y: f64, rotation: f64) -> Model3D {
    Model3D {
        id: "shed-1".into(),
        name: "Lokschuppen".into(),
        category: "railway".into(),
        transform: Transform { x, y, rotation },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: Some("layer-1".into()),
        width: 240.0,
        depth: 110.0,
        height: 95.0,
        model_type: "builtin".into(),
        builtin_id: Some("engine-shed".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    }
}

#[test]
fn engine_shed_snaps_to_open_track_connector() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24188").unwrap();
    let piece = create_piece("p1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    // Connector 1 is at (188.3, 0.0) facing 0 deg; shed door is at -120 -> center ~308.3
    let cand = Transform {
        x: 320.0,
        y: 12.0,
        rotation: 8.0,
    };
    let res = snap_engine_shed_to_track(cand, &[piece], &catalogs, 50.0, 75.0, 240.0);
    let (tr, pt) = res.expect("engine shed should snap to track connector");
    assert!((pt.x - 188.3).abs() < 0.2);
    assert!((tr.x - 308.3).abs() < 0.3);
    assert!((tr.y - 0.0).abs() < 0.3);
    assert!(tr.rotation.abs() < 0.5 || (tr.rotation - 360.0).abs() < 0.5);
}

#[test]
fn roundhouse_snaps_to_turntable_exit() {
    let catalogs = builtin_catalogs();
    let tt_entry = catalogs
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.kind == "turntable")
        .expect("catalog should contain a turntable");
    let tt_r = tt_entry
        .radius
        .or_else(|| tt_entry.length.map(|l| l * 0.5))
        .unwrap_or(189.0);
    let tt = create_piece("tt-1".into(), tt_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let target_dist = tt_r + 6.0 + 310.0 * 0.5;
    let cand = Transform {
        x: target_dist + 12.0,
        y: 8.0,
        rotation: 0.0,
    };
    let (tr, _edge) = snap_model_to_turntable(cand, &[tt], &catalogs, 75.0, 15.0, 310.0)
        .expect("roundhouse should snap radially to turntable");
    let d = tr.x.hypot(tr.y);
    assert!((d - target_dist).abs() < 3.0);
}

#[test]
fn track_piece_snaps_to_engine_shed_door() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24188").unwrap();
    let shed = make_test_shed(308.3, 0.0, 0.0);
    // Shed door is at (188.3, 0.0); place straight track near it
    let p = create_piece("p1".into(), entry, Transform { x: -10.0, y: 12.0, rotation: 4.0 }, None);
    let mut selected = HashSet::new();
    selected.insert("p1".to_string());
    let mut layout = Layout {
        name: "T".into(),
        scale: "H0".into(),
        pieces: vec![p],
        models: Some(vec![shed]),
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
    assert!(snap_selected_pieces_to_buildings(&mut layout, &catalogs, &selected, 45.0));
    let door_c = &layout.pieces[0].connectors[1];
    assert!((door_c.x - 188.3).abs() < 0.3);
    assert!(door_c.y.abs() < 0.3);
}

#[test]
fn roundhouse_door_connectors_align_radially_with_stalls() {
    let hall = Model3D {
        id: "rh-1".into(),
        name: "Ringlokschuppen".into(),
        category: "railway".into(),
        transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 320.0,
        depth: 310.0,
        height: 115.0,
        model_type: "builtin".into(),
        builtin_id: Some("turntable-trainhall".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let doors = get_building_door_connectors(&hall);
    assert_eq!(doors.len(), 3);
    // Stall 0 at (-40.54, -159.26) lies on -12 deg ray from turntable (0, -350): inward = 102 deg
    assert!((doors[0].x - (-40.5)).abs() < 0.2);
    assert!((doors[0].inward_angle - 102.0).abs() < 0.1);
    assert!((doors[0].outward_angle - 282.0).abs() < 0.1);
    // Stall 1 at (0, -155.0): inward = 90 deg
    assert!((doors[1].inward_angle - 90.0).abs() < 0.1);
    // Stall 2 at (+40.54, -159.26) lies on +12 deg ray from turntable (0, -350): inward = 78 deg
    assert!((doors[2].x - 40.5).abs() < 0.2);
    assert!((doors[2].inward_angle - 78.0).abs() < 0.1);
    assert!((doors[2].outward_angle - 258.0).abs() < 0.1);
}

#[test]
fn station_platform_coal_water_and_diesel_snap_parallel_to_track() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24188").unwrap();
    let track = create_piece(
        "t1".into(),
        entry,
        Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    // Track runs along +X from (0, 0) to (188.3, 0), midpoint (94.15, 0)
    let cases: &[(&str, f64, f64, f64, f64, f64)] = &[
        // (builtin_id, width, depth, height, expected_abs_y, expected_rot_above_track)
        ("station", 320.0, 140.0, 120.0, 70.0 + 18.0, 0.0),
        ("platform", 320.0, 54.0, 55.0, 27.0 + 17.5, 0.0),
        ("bahnsteig", 320.0, 54.0, 55.0, 27.0 + 17.5, 0.0),
        ("coaling-station", 180.0, 75.0, 125.0, 37.5 + 17.0, 0.0),
        ("diesel-fuel-station", 170.0, 65.0, 68.0, 32.5 + 16.5, 0.0),
        ("water-tower", 85.0, 85.0, 170.0, 42.5 + 17.5, 270.0),
    ];

    for &(bid, w, d, h, exp_y, exp_rot) in cases {
        let m = Model3D {
            id: format!("m-{bid}"),
            name: bid.into(),
            category: "railway".into(),
            transform: Transform {
                x: 90.0,
                y: -exp_y - 12.0,
                rotation: exp_rot + 7.0,
            },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: w,
            depth: d,
            height: h,
            model_type: "builtin".into(),
            builtin_id: Some(bid.into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let (snapped_tr, foot) =
            snap_trackside_model_to_track(&m, &[track.clone()], &[], &catalogs, 65.0)
                .unwrap_or_else(|| panic!("{bid} should snap alongside track"));
        assert!(
            (snapped_tr.y - (-exp_y)).abs() < 0.2,
            "{bid} y={} expected {}",
            snapped_tr.y,
            -exp_y
        );
        assert!(
            (snapped_tr.x - 94.15).abs() < 0.2,
            "{bid} x={} expected midpoint 94.15",
            snapped_tr.x
        );
        assert!(
            angle_diff_deg(snapped_tr.rotation, exp_rot) < 0.2,
            "{bid} rot={} expected {}",
            snapped_tr.rotation,
            exp_rot
        );
        assert!(foot.y.abs() < 0.1);
        assert!(!get_building_door_connectors(&m).is_empty());
    }
}

#[test]
fn building_and_track_snapping_requires_same_world_height() {
    use railbench_core::types::{
        Catalog, CatalogEntry, Elevation, Layer, Layout, LocalConnector, Model3D, Transform,
    };

    let mut tt_conns = Vec::new();
    for i in 0..8 {
        let deg = (i as f64) * 45.0;
        let rad = deg.to_radians();
        tt_conns.push(LocalConnector {
            x: 150.0 * rad.cos(),
            y: 150.0 * rad.sin(),
            angle: deg,
        });
    }
    let straight_entry = CatalogEntry {
        id: "test-straight-180".to_string(),
        name: "Test Straight 180".to_string(),
        name_en: None,
        kind: "straight".to_string(),
        category: "Track".to_string(),
        description: String::new(),
        length: Some(180.0),
        radius: None,
        angle: None,
        branch_radius: None,
        branch_angle: None,
        gauge: Some(16.5),
        has_roadbed: None,
        is_road: None,
        road_width: None,
        local_connectors: None,
    };
    let tt_entry = CatalogEntry {
        id: "test-turntable-150".to_string(),
        name: "Test Turntable 150".to_string(),
        name_en: None,
        kind: "turntable".to_string(),
        category: "Turntable".to_string(),
        description: String::new(),
        length: Some(300.0),
        radius: Some(150.0),
        angle: None,
        branch_radius: None,
        branch_angle: None,
        gauge: Some(16.5),
        has_roadbed: None,
        is_road: None,
        road_width: None,
        local_connectors: Some(tt_conns),
    };
    let cats = vec![Catalog {
        id: "test-cat".to_string(),
        name: "Test Catalog".to_string(),
        scale: "H0".to_string(),
        manufacturer: None,
        system: None,
        gauge: Some(16.5),
        is_road: None,
        conductors: None,
        entries: vec![straight_entry.clone(), tt_entry.clone()],
    }];

    let mk_model = |id: &str, bid: &str, w: f64, d: f64, h: f64, tr: Transform, elev: Option<f64>, layer_id: Option<&str>| {
        Model3D {
            id: id.to_string(),
            name: bid.to_string(),
            category: "Buildings".to_string(),
            transform: tr,
            elevation: elev,
            scale: None,
            layer_id: layer_id.map(|s| s.to_string()),
            width: w,
            depth: d,
            height: h,
            model_type: "builtin".to_string(),
            builtin_id: Some(bid.to_string()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        }
    };

    // 1. Trackside buildings & engine-shed & turntable-trainhall reject 80mm vs 0mm mismatch
    let specs = [
        ("station", 240.0, 120.0, 90.0),
        ("platform", 240.0, 50.0, 15.0),
        ("coaling-station", 120.0, 70.0, 80.0),
        ("water-tower", 60.0, 60.0, 110.0),
        ("diesel-fuel-station", 90.0, 55.0, 45.0),
        ("engine-shed", 220.0, 110.0, 85.0),
        ("turntable-trainhall", 220.0, 110.0, 85.0),
    ];

    for (bid, w, d, h) in specs {
        let track = crate::catalog::create_piece(
            "t1".to_string(),
            if bid == "turntable-trainhall" { &tt_entry } else { &straight_entry },
            Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            None,
        );
        let m_tr = match bid {
            "engine-shed" => Transform { x: 185.0 + w * 0.5, y: 0.0, rotation: 0.0 },
            "turntable-trainhall" => Transform { x: 150.0 + 12.0 + d * 0.5, y: 0.0, rotation: 270.0 },
            _ => Transform { x: 90.0, y: d * 0.5 + 22.0, rotation: 0.0 },
        };
        let mut layout = Layout {
            name: "test".to_string(),
            scale: "H0".to_string(),
            pieces: vec![track],
            models: Some(vec![mk_model("m1", bid, w, d, h, m_tr, Some(80.0), None)]),
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
        let sel: HashSet<String> = ["m1".to_string()].into_iter().collect();

        // Level mismatch: snap rejected, preview returns same_level == false
        assert!(
            !snap_selected_models(&mut layout, &cats, &sel),
            "{bid} must not snap across 80mm vs 0mm height mismatch"
        );
        let prev = preview_selected_model_snap(&layout, &cats, &sel);
        assert!(
            matches!(prev, Some((_, _, false))),
            "{bid} preview should report height mismatch (same_level=false), got {prev:?}"
        );

        // Bring track to 80mm via layer(50mm) + piece elevation(30mm) -> 80mm world height
        layout.layers = Some(vec![
            Layer {
                id: "l1".to_string(),
                name: "Layer 1".to_string(),
                color: None,
                visible: true,
                locked: false,
                height_mm: Some(50.0),
                wood_plate_mode: None,
                wood_plate_thickness_mm: None,
                wood_plate_style: None,
                plates: None,
            },
        ]);
        layout.pieces[0].layer_id = Some("l1".to_string());
        layout.pieces[0].elevation = Some(Elevation { start: 30.0, end: 30.0 });

        let prev_match = preview_selected_model_snap(&layout, &cats, &sel);
        assert!(
            matches!(prev_match, Some((_, _, true))),
            "{bid} preview should report same_level=true at 80mm==80mm, got {prev_match:?}"
        );
        assert!(
            snap_selected_models(&mut layout, &cats, &sel),
            "{bid} should snap when both world heights equal 80mm"
        );
    }

    // 2. Track-to-building snapping rejects 80mm track vs 0mm engine-shed/turntable-trainhall, succeeds when matched
    for (bid, w, d, h) in [
        ("engine-shed", 220.0, 110.0, 85.0),
        ("turntable-trainhall", 220.0, 110.0, 85.0),
    ] {
        let model = mk_model(
            "b1",
            bid,
            w,
            d,
            h,
            Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            None,
            None,
        );
        let mut layout = Layout {
            name: "test-t2b".to_string(),
            scale: "H0".to_string(),
            pieces: Vec::new(),
            models: Some(vec![model.clone()]),
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
        let mut doors = Vec::new();
        collect_model_door_positions(&layout, &model, &mut doors);
        let (dx, dy, _) = doors[0];
        let mut track = crate::catalog::create_piece(
            "t_sel".to_string(),
            &straight_entry,
            Transform { x: dx + 5.0, y: dy, rotation: 0.0 },
            None,
        );
        track.elevation = Some(Elevation { start: 80.0, end: 80.0 });
        layout.pieces.push(track);
        let sel: HashSet<String> = ["t_sel".to_string()].into_iter().collect();

        assert!(
            !snap_selected_pieces_to_buildings(&mut layout, &cats, &sel, 40.0),
            "track at 80mm must not snap to 0mm {bid}"
        );
        let any_cand = find_best_track_to_building_snap_any_level(&layout, &cats, &sel, 40.0);
        assert!(
            any_cand.as_ref().is_some_and(|c| !c.same_level),
            "any_level candidate for {bid} should exist with same_level=false, got {any_cand:?}"
        );

        layout.models.as_mut().unwrap()[0].elevation = Some(80.0);
        assert!(
            snap_selected_pieces_to_buildings(&mut layout, &cats, &sel, 40.0),
            "track at 80mm should snap to 80mm {bid}"
        );
    }
}
