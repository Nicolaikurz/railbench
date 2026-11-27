use super::*;

#[test]
fn snap_selected_flex_track_bridges_both_ends_and_shortens() {
    let cats = crate::catalog::builtin_catalogs();
    let straight = crate::catalog::find_entry(&cats, "24188").unwrap();
    let flex_entry = cats
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.kind == "flex")
        .unwrap();

    // Track A ends at (188.3, 0) facing 0°
    let track_a = crate::catalog::create_piece(
        "a".into(),
        straight,
        Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        Some("layer-1".into()),
    );
    // Track B starts at (1068.3, 18.0) facing 172° (slightly angled and 20 mm shorter than 900 mm flex)
    let track_b = crate::catalog::create_piece(
        "b".into(),
        straight,
        Transform { x: 1068.3, y: 18.0, rotation: -8.0 },
        Some("layer-1".into()),
    );
    // Flex placed near Track A's open connector (188.3, 0)
    let flex = crate::catalog::create_piece(
        "flex".into(),
        flex_entry,
        Transform { x: 195.0, y: 4.0, rotation: 2.0 },
        Some("layer-1".into()),
    );

    let mut layout = Layout {
        name: "FlexBridge".into(),
        scale: "H0".into(),
        pieces: vec![track_a, track_b, flex],
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
        active_layer_id: Some("layer-1".into()),
        baseboard: None,
        baseboards: None,
        terrain_field: None,
    };
    let mut sel = HashSet::new();
    sel.insert("flex".to_string());

    assert!(snap_selected_pieces(&mut layout, &cats, &sel, 24.0));
    let f = &layout.pieces[2];
    // Both ends snapped with 0° kink!
    assert!((f.connectors[0].x - layout.pieces[0].connectors[1].x).abs() < 1e-4);
    assert!((f.connectors[0].y - layout.pieces[0].connectors[1].y).abs() < 1e-4);
    assert!((f.connectors[1].x - layout.pieces[1].connectors[0].x).abs() < 1e-4);
    assert!((f.connectors[1].y - layout.pieces[1].connectors[0].y).abs() < 1e-4);
    let expected_a1 = norm_angle_deg(layout.pieces[1].connectors[0].angle + 180.0);
    assert!((f.connectors[1].angle - expected_a1).abs() < 1e-4);
    assert!(f.custom_length.unwrap() < flex_entry.length.unwrap_or(900.0));
}

#[test]
fn terrain_2d_mountain_and_ground_never_exceed_table_or_plate() {
    use railbench_core::types::{Layer, LayerPlate, Model3D};

    let cats = builtin_catalogs();
    let bb = BaseboardConfig {
        id: Some("b1".into()),
        name: Some("Table".into()),
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 2000.0,
        depth: 1000.0,
        rotation: Some(0.0),
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: None,
        custom_y: None,
        style: "wood".into(),
        corner_radius: Some(0.0),
        show_dimensions: None,
    };
    let plate = LayerPlate {
        id: "pl-1".into(),
        name: Some("Upper".into()),
        baseboard_id: Some("b1".into()),
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        rotation: Some(0.0),
        width: 400.0,
        depth: 300.0,
        x: Some(800.0),
        y: Some(350.0),
        thickness_mm: Some(12.0),
        position_mode: Some("custom".into()),
        style: None,
    };
    let mtn = Model3D {
        id: "m1".into(),
        name: "M".into(),
        category: "terrain".into(),
        transform: Transform { x: 960.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: Some("layer-1".into()),
        width: 600.0,
        depth: 500.0,
        height: 220.0,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: Some("alpine".into()),
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let water = Model3D {
        id: "w1".into(),
        name: "W".into(),
        category: "terrain".into(),
        transform: Transform { x: 180.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: Some("layer-2".into()),
        width: 400.0,
        depth: 320.0,
        height: 8.0,
        model_type: "builtin".into(),
        builtin_id: Some("ground-terrain".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: Some("water".into()),
        file_data: None,
        file_name: None,
    };
    let layout = Layout {
        name: "Clip2D".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
        models: Some(vec![mtn, water]),
        feed_voltage: None,
        catenary_feed_voltage: None,
        catenary_circuit_mode: None,
        loco_current_ma: None,
        loco_count: None,
        train_current_amps: None,
        turnout_lantern_current_ma: None,
        turnout_decoder_current_ma: None,
        layers: Some(vec![
            Layer {
                id: "layer-1".into(),
                name: "Base".into(),
                color: None,
                visible: true,
                locked: false,
                height_mm: Some(0.0),
                wood_plate_mode: Some("table".into()),
                wood_plate_thickness_mm: None,
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            },
            Layer {
                id: "layer-2".into(),
                name: "Upper".into(),
                color: None,
                visible: true,
                locked: false,
                height_mm: Some(80.0),
                wood_plate_mode: Some("table".into()),
                wood_plate_thickness_mm: Some(12.0),
                wood_plate_style: None,
                plates: Some(vec![plate]),
                terrain_field: None,
            },
        ]),
        active_layer_id: Some("layer-1".into()),
        baseboard: Some(bb),
        baseboards: None,
        terrain_field: None,
    };

    let mut cache = Canvas2DCache::default();
    cache.ensure_fresh(&layout, &cats, 1);

    let m_map = cache.model_mtn_maps[0].as_ref().expect("2D mountain map");
    assert!(!m_map.tint_quads.is_empty());
    for (pts, _) in &m_map.tint_quads {
        for pt in pts {
            assert!(pt.x.abs() <= 1000.5 && pt.y.abs() <= 500.5, "2D mountain quad pt escaped table: {pt:?}");
        }
    }
    for &(a, b, _) in &m_map.contour_segs {
        assert!(a.x.abs() <= 1000.5 && b.x.abs() <= 1000.5, "2D contour escaped table");
    }

    let g_map = cache.model_ground_maps[1].as_ref().expect("2D ground water map");
    assert!(!g_map.quads.is_empty());
    for (pts, _) in &g_map.quads {
        for pt in pts {
            assert!(pt.x.abs() <= 200.5 && pt.y.abs() <= 150.5, "2D water quad pt escaped upper plate: {pt:?}");
        }
    }
    for &(a, b) in &g_map.contour_segs {
        assert!(a.x.abs() <= 200.5 && b.x.abs() <= 200.5 && a.y.abs() <= 150.5 && b.y.abs() <= 150.5);
    }
}

#[test]
fn track_rotation_handle_places_point_perpendicular_to_piece_and_rotates_rigidly() {
    let cats = builtin_catalogs();
    let e24188 = find_entry(&cats, "24188").unwrap();
    let p1 = create_piece(
        "r1".into(),
        e24188,
        Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let p2 = create_piece(
        "r2".into(),
        e24188,
        Transform {
            x: 188.3,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let mut layout = Layout {
        name: "Rot2DTest".into(),
        scale: "H0".into(),
        pieces: vec![p1, p2],
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

    let mut selected = HashSet::new();
    selected.insert("r1".to_string());
    let (pivot, handle, base_ang, single_id, init_rot) =
        compute_track_selection_rotation_handle(&layout, &selected, true, 1.0, None)
            .expect("single track emits rotation handle");
    assert_eq!(single_id.as_deref(), Some("r1"));
    assert!((init_rot - 0.0).abs() < 1e-6);
    assert!((base_ang - 270.0).abs() < 1e-4);
    assert!((pivot.x - 94.15).abs() < 0.2 && pivot.y.abs() < 1e-4);
    assert!((handle.x - pivot.x).abs() < 1e-4 && handle.y < pivot.y - 30.0);

    let orig_len = (layout.pieces[0].connectors[1].x - layout.pieces[0].connectors[0].x)
        .hypot(layout.pieces[0].connectors[1].y - layout.pieces[0].connectors[0].y);
    assert!(rotate_selection_around_pivot(&mut layout, &selected, true, pivot, 90.0));
    let r1 = &layout.pieces[0];
    assert!((r1.transform.rotation - 90.0).abs() < 1e-4);
    let new_len = (r1.connectors[1].x - r1.connectors[0].x).hypot(r1.connectors[1].y - r1.connectors[0].y);
    assert!((new_len - orig_len).abs() < 1e-3);
    assert!((r1.connectors[0].angle - 270.0).abs() < 1e-4 || (r1.connectors[0].angle - 90.0).abs() < 1e-4);
    assert!((r1.connectors[1].angle - 90.0).abs() < 1e-4);

    // Multi-piece rigid rotation preserves inter-piece spacing
    selected.insert("r2".to_string());
    let before_dist = (layout.pieces[1].transform.x - layout.pieces[0].transform.x)
        .hypot(layout.pieces[1].transform.y - layout.pieces[0].transform.y);
    let (multi_piv, _multi_hdl, _, multi_single, _) =
        compute_track_selection_rotation_handle(&layout, &selected, true, 1.0, None)
            .expect("multi track emits rotation handle");
    assert!(multi_single.is_none());
    assert!(rotate_selection_around_pivot(&mut layout, &selected, true, multi_piv, 45.0));
    let after_dist = (layout.pieces[1].transform.x - layout.pieces[0].transform.x)
        .hypot(layout.pieces[1].transform.y - layout.pieces[0].transform.y);
    assert!((after_dist - before_dist).abs() < 1e-3);
}

#[test]
fn track_rotation_handle_snaps_to_toolbar_rotation_step() {
    assert!((snap_drag_rotation_delta(26.0, 30.0, false) - 30.0).abs() < 1e-6);
    assert!((snap_drag_rotation_delta(13.0, 15.0, false) - 15.0).abs() < 1e-6);
    assert!((snap_drag_rotation_delta(11.0, 7.5, false) - 7.5).abs() < 1e-6);
    assert!((snap_drag_rotation_delta(25.0, 24.3, false) - 24.3).abs() < 1e-6);
    assert!((snap_drag_rotation_delta(-11.0, 5.7, false) + 11.4).abs() < 1e-6);
    // Holding Shift disables snapping for fine free-hand rotation
    assert!((snap_drag_rotation_delta(26.25, 30.0, true) - 26.25).abs() < 1e-6);

    // Unwrapping across +/-180 deg preserves continuous drag direction
    let unwrapped = unwrap_drag_angle_delta(-170.0, 0.0, 175.0);
    assert!((unwrapped - 190.0).abs() < 1e-6);

    // Readout keeps decimals for fractional toolbar steps (24.3 deg, +5.7 deg) and clean ints otherwise
    assert_eq!(format_rotation_readout(30.0, false, false), "30°");
    assert_eq!(format_rotation_readout(24.3, false, false), "24.3°");
    assert_eq!(format_rotation_readout(5.7, true, false), "+5.7°");
    assert_eq!(format_rotation_readout(-0.001, false, false), "0°");
    assert_eq!(format_rotation_readout(-0.001, true, false), "+0°");
}

#[test]
fn board_drag_hit_is_locked_outside_board_tab() {
    let bb = BaseboardConfig {
        id: Some("board-1".into()),
        name: Some("Table 1".into()),
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1000.0,
        depth: 600.0,
        rotation: Some(0.0),
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: None,
        custom_y: None,
        style: "wood".into(),
        corner_radius: Some(0.0),
        show_dimensions: None,
    };
    let mut layout = make_layout_with_pieces(Vec::new());
    layout.baseboard = Some(bb.clone());
    layout.baseboards = Some(vec![bb]);

    let bounds = board_tools::get_baseboard_bounds(layout.baseboards.as_ref().unwrap().first().unwrap());
    let camera = Camera2D {
        offset: EguiVec2::ZERO,
        zoom: 1.0,
    };
    let canvas_rect = Rect::from_min_size(Pos2::ZERO, EguiVec2::new(1400.0, 900.0));
    let header_world = Vec2 {
        x: bounds.x + 12.0,
        y: bounds.y + 12.0,
    };
    let header_sp = camera.world_to_screen(canvas_rect, header_world.x, header_world.y);
    let header_rects = vec![(
        "board-1".to_string(),
        Rect::from_min_size(
            camera.world_to_screen(canvas_rect, bounds.x, bounds.y) + EguiVec2::new(6.0, 6.0),
            EguiVec2::new(80.0, 18.0),
        ),
    )];

    let edge_world = Vec2 {
        x: bounds.x,
        y: bounds.y + bounds.depth * 0.5,
    };
    let edge_sp = camera.world_to_screen(canvas_rect, edge_world.x, edge_world.y);
    let interior_world = Vec2 {
        x: bounds.x + bounds.width * 0.5,
        y: bounds.y + bounds.depth * 0.5,
    };
    let interior_sp = camera.world_to_screen(canvas_rect, interior_world.x, interior_world.y);

    for (wp, sp) in [
        (header_world, header_sp),
        (edge_world, edge_sp),
        (interior_world, interior_sp),
    ] {
        assert!(
            find_board_drag_hit(false, &layout, wp, sp, camera.zoom, &header_rects, true).is_none(),
            "board must not be draggable when Board tab is not active"
        );
        let hit = find_board_drag_hit(true, &layout, wp, sp, camera.zoom, &header_rects, true)
            .expect("board should be draggable when Board tab is active");
        assert_eq!(hit.0, "board-1");
    }
}

#[test]
fn direct_feeder_and_isolator_canvas_button_hit_and_toggle() {
    let cats = builtin_catalogs();
    let feeder_entry = find_entry(&cats, "24088").unwrap();
    let iso_entry = find_entry(&cats, "24994").unwrap();
    let mut p_feed = create_piece(
        "f1".into(),
        feeder_entry,
        Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        None,
    );
    p_feed.power_feed = Some(false);
    let mut p_iso = create_piece(
        "i1".into(),
        iso_entry,
        Transform { x: 200.0, y: 0.0, rotation: 0.0 },
        None,
    );
    p_iso.is_isolator = Some(false);
    p_iso.isolated_connector_index = None;
    let mut layout = make_layout_with_pieces(vec![p_feed, p_iso]);

    let buttons = vec![
        TrackAccessoryButton {
            piece_id: "f1".into(),
            kind: TrackAccessoryButtonKind::Feeder,
            screen_center: Pos2::new(120.0, 200.0),
            radius_px: 11.0,
            active: false,
        },
        TrackAccessoryButton {
            piece_id: "i1".into(),
            kind: TrackAccessoryButtonKind::Isolator,
            screen_center: Pos2::new(320.0, 200.0),
            radius_px: 11.0,
            active: false,
        },
    ];

    assert!(hit_test_accessory_button(&buttons, Pos2::new(120.0, 200.0)).is_some());
    assert!(hit_test_accessory_button(&buttons, Pos2::new(500.0, 500.0)).is_none());

    assert!(toggle_track_accessory_on_piece(&mut layout, "f1", TrackAccessoryButtonKind::Feeder));
    assert_eq!(layout.pieces[0].power_feed, Some(true));
    assert!(toggle_track_accessory_on_piece(&mut layout, "f1", TrackAccessoryButtonKind::Feeder));
    assert_eq!(layout.pieces[0].power_feed, Some(false));

    assert!(toggle_track_accessory_on_piece(&mut layout, "i1", TrackAccessoryButtonKind::Isolator));
    assert_eq!(layout.pieces[1].is_isolator, Some(true));
    assert_eq!(layout.pieces[1].isolated_connector_index, Some(0));
    assert!(toggle_track_accessory_on_piece(&mut layout, "i1", TrackAccessoryButtonKind::Isolator));
    assert_eq!(layout.pieces[1].is_isolator, Some(false));
    assert_eq!(layout.pieces[1].isolated_connector_index, None);
}

#[test]
fn selection_scope_filters_tracks_roads_buildings_and_terrain_in_2d() {
    let cats = builtin_catalogs();
    let e24188 = find_entry(&cats, "24188").expect("24188");
    let road_entry = cats
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.is_road == Some(true))
        .expect("builtin road entry");
    let t1 = create_piece(
        "t1".into(),
        e24188,
        Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let t_under = create_piece(
        "t-under".into(),
        e24188,
        Transform {
            x: 310.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let r1 = create_piece(
        "r1".into(),
        road_entry,
        Transform {
            x: 0.0,
            y: 250.0,
            rotation: 0.0,
        },
        None,
    );
    let road_pt = r1
        .connectors
        .first()
        .map(|c| Vec2 { x: c.x, y: c.y })
        .unwrap_or(Vec2 { x: 0.0, y: 250.0 });
    let mut layout = make_layout_with_pieces(vec![t1, t_under, r1]);
    layout.models = Some(vec![
        railbench_core::types::Model3D {
            id: "b1".into(),
            name: "Station".into(),
            category: "Buildings".into(),
            transform: Transform {
                x: 400.0,
                y: 0.0,
                rotation: 0.0,
            },
            elevation: None,
            scale: None,
            layer_id: None,
            width: 120.0,
            depth: 80.0,
            height: 60.0,
            model_type: "builtin".into(),
            builtin_id: Some("station-platform".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        },
        railbench_core::types::Model3D {
            id: "m1".into(),
            name: "Alpine Mountain".into(),
            category: "Terrain".into(),
            transform: Transform {
                x: 800.0,
                y: 0.0,
                rotation: 0.0,
            },
            elevation: None,
            scale: None,
            layer_id: None,
            width: 240.0,
            depth: 240.0,
            height: 150.0,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: Some("alpine".into()),
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        },
    ]);

    let mut cache = Canvas2DCache::default();
    cache.ensure_fresh(&layout, &cats, 1);

    // Bom / All scope picks tracks, roads, buildings, and terrain
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            Vec2 { x: 90.0, y: 0.0 },
            24.0,
            false,
            crate::SelectionScope::All,
        )
        .as_deref(),
        Some("t1")
    );
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            road_pt,
            24.0,
            false,
            crate::SelectionScope::All,
        )
        .as_deref(),
        Some("r1")
    );
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            Vec2 { x: 400.0, y: 0.0 },
            24.0,
            false,
            crate::SelectionScope::All,
        )
        .as_deref(),
        Some("b1")
    );
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            Vec2 { x: 800.0, y: 0.0 },
            24.0,
            false,
            crate::SelectionScope::All,
        )
        .as_deref(),
        Some("m1")
    );

    // Tracks scope picks rail tracks (including underneath buildings) and ignores roads/buildings/terrain
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            Vec2 { x: 90.0, y: 0.0 },
            24.0,
            false,
            crate::SelectionScope::Tracks,
        )
        .as_deref(),
        Some("t1")
    );
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            Vec2 { x: 400.0, y: 0.0 },
            24.0,
            false,
            crate::SelectionScope::Tracks,
        )
        .as_deref(),
        Some("t-under")
    );
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        road_pt,
        24.0,
        false,
        crate::SelectionScope::Tracks,
    )
    .is_none());
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 800.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Tracks,
    )
    .is_none());

    // Roads scope picks roads only
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            road_pt,
            24.0,
            false,
            crate::SelectionScope::Roads,
        )
        .as_deref(),
        Some("r1")
    );
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 90.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Roads,
    )
    .is_none());
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 400.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Roads,
    )
    .is_none());

    // Buildings scope picks non-terrain buildings only
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            Vec2 { x: 400.0, y: 0.0 },
            24.0,
            false,
            crate::SelectionScope::Buildings,
        )
        .as_deref(),
        Some("b1")
    );
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 90.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Buildings,
    )
    .is_none());
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 800.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Buildings,
    )
    .is_none());

    // Terrain scope picks terrain only
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 90.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Terrain,
    )
    .is_none());
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 400.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Terrain,
    )
    .is_none());
    assert_eq!(
        snapping::find_canvas_object_at(
            &layout,
            &cache,
            &cats,
            Vec2 { x: 800.0, y: 0.0 },
            24.0,
            false,
            crate::SelectionScope::Terrain,
        )
        .as_deref(),
        Some("m1")
    );

    // Boards scope ignores tracks/roads/buildings/terrain
    assert!(snapping::find_canvas_object_at(
        &layout,
        &cache,
        &cats,
        Vec2 { x: 90.0, y: 0.0 },
        24.0,
        false,
        crate::SelectionScope::Boards,
    )
    .is_none());
}
