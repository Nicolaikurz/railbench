use super::*;

#[test]
fn mountain_with_intersecting_track_generates_tunnel_geometry() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap(); // 360 mm straight
    let p0 = create_piece("t0".into(), entry, Transform { x: -360.0, y: 0.0, rotation: 0.0 }, None);
    let p1 = create_piece("t1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let mtn = Model3D {
        id: "m1".into(),
        name: "Alpine".into(),
        category: "terrain".into(),
        transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 600.0,
        depth: 500.0,
        height: 260.0,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };

    let layout_only_mtn = Layout {
        name: "T".into(),
        scale: "H0".into(),
        pieces: vec![],
        models: Some(vec![mtn.clone()]),
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
    let mut layout_with_tunnel = layout_only_mtn.clone();
    layout_with_tunnel.pieces = vec![p0, p1];

    let mesh_base = build_scene_mesh_3d(&layout_only_mtn, &catalogs, &Scene3DOptions::default());
    let mesh_tunnel = build_scene_mesh_3d(&layout_with_tunnel, &catalogs, &Scene3DOptions::default());
    assert!(mesh_tunnel.vertices.len() > mesh_base.vertices.len() + 100);
    assert!(mesh_tunnel.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
}

#[test]
fn accessories_and_engine_shed_generate_3d_geometry() {
    let catalogs = builtin_catalogs();
    let turnout_entry = find_entry(&catalogs, "24611").unwrap();
    let mut turnout = create_piece("sw1".into(), turnout_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    turnout.has_lantern = Some(true);
    turnout.has_decoder = Some(true);
    turnout.has_signal = Some(true);
    turnout.signal_side = Some("both".into());
    turnout.signal_direction = Some("both".into());

    let base_turnout = create_piece("sw0".into(), turnout_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let make_layout = |pieces, models| Layout {
        name: "Acc3D".into(),
        scale: "H0".into(),
        pieces,
        models,
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

    let opts = Scene3DOptions::default();
    let m_plain = build_scene_mesh_3d(&make_layout(vec![base_turnout], None), &catalogs, &opts);
    let shed = Model3D {
        id: "shed-1".into(),
        name: "Lokschuppen".into(),
        category: "railway".into(),
        transform: Transform { x: 300.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
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
    };
    let hall = Model3D {
        id: "hall-1".into(),
        name: "Ringlokschuppen".into(),
        category: "railway".into(),
        transform: Transform { x: 600.0, y: 0.0, rotation: 0.0 },
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
    let m_acc = build_scene_mesh_3d(&make_layout(vec![turnout], Some(vec![shed, hall])), &catalogs, &opts);
    assert!(m_acc.vertices.len() > m_plain.vertices.len() + 200);
    assert!(m_acc.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
}

#[test]
fn l_shape_table_with_cutout_and_upper_layer_plate_generates_3d_mesh() {
    let catalogs = builtin_catalogs();
    let straight = find_entry(&catalogs, "24360").unwrap();
    let mut ramp_piece = create_piece("r1".into(), straight, Transform { x: -400.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    ramp_piece.elevation = Some(railbench_core::types::Elevation { start: -45.0, end: 25.0 });

    let l_board = BaseboardConfig {
        id: Some("board-l".into()),
        name: Some("L-Tisch".into()),
        enabled: true,
        shape: Some("l-shape".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 2400.0,
        depth: 1600.0,
        rotation: Some(0.0),
        l_wing_width: Some(800.0),
        l_wing_depth: Some(800.0),
        l_side: Some("left".into()),
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: None,
        custom_y: None,
        style: "plywood".into(),
        corner_radius: Some(10.0),
        show_dimensions: Some(true),
    };
    let layer1 = railbench_core::types::Layer {
        id: "layer-1".into(),
        name: "Ebene 0".into(),
        color: Some("#38bdf8".into()),
        visible: true,
        locked: false,
        height_mm: Some(0.0),
        wood_plate_mode: Some("table".into()),
        wood_plate_thickness_mm: Some(20.0),
        wood_plate_style: None,
        plates: None,
        terrain_field: None,
    };
    let layer2 = railbench_core::types::Layer {
        id: "layer-2".into(),
        name: "Ebene 1".into(),
        color: Some("#f59e0b".into()),
        visible: true,
        locked: false,
        height_mm: Some(90.0),
        wood_plate_mode: Some("table".into()),
        wood_plate_thickness_mm: Some(12.0),
        wood_plate_style: Some("wood".into()),
        plates: Some(vec![railbench_core::types::LayerPlate {
            id: "plate-1".into(),
            name: Some("Oberplatte".into()),
            baseboard_id: Some("board-l".into()),
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
            width: 900.0,
            depth: 500.0,
            x: Some(200.0),
            y: Some(200.0),
            thickness_mm: Some(12.0),
            position_mode: Some("custom".into()),
            style: Some("wood".into()),
        }]),
        terrain_field: None,
    };

    let layout = Layout {
        name: "LPlate3D".into(),
        scale: "H0".into(),
        pieces: vec![ramp_piece],
        models: None,
        feed_voltage: None,
        catenary_feed_voltage: None,
        catenary_circuit_mode: None,
        loco_current_ma: None,
        loco_count: None,
        train_current_amps: None,
        turnout_lantern_current_ma: None,
        turnout_decoder_current_ma: None,
        layers: Some(vec![layer1, layer2]),
        active_layer_id: Some("layer-2".into()),
        baseboard: Some(l_board.clone()),
        baseboards: Some(vec![l_board]),
        terrain_field: None,
    };

    let mesh_no_plates = build_scene_mesh_3d(
        &layout,
        &catalogs,
        &Scene3DOptions { show_wood_plates: false, env_mode: Viewer3DEnv::Minimal, ..Default::default() },
    );
    let mesh_with_plates = build_scene_mesh_3d(
        &layout,
        &catalogs,
        &Scene3DOptions { show_wood_plates: true, env_mode: Viewer3DEnv::Minimal, ..Default::default() },
    );
    assert!(mesh_with_plates.vertices.len() > mesh_no_plates.vertices.len());
    assert!(mesh_with_plates.vertices.iter().any(|v| (v.pos[1] - 90.0).abs() < 0.5));
    assert!(mesh_with_plates.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
}

#[test]
fn new_builtin_models_custom_glb_and_hall_env_generate_3d_mesh() {
    let catalogs = builtin_catalogs();
    // Build minimal GLB custom mesh
    let mut bin = Vec::new();
    for c in [0.0_f32, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 10.0] {
        bin.extend_from_slice(&c.to_le_bytes());
    }
    let json = serde_json::json!({
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": bin.len() }],
        "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": bin.len() }],
        "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
        "nodes": [{ "mesh": 0 }],
        "scenes": [{ "nodes": [0] }],
        "scene": 0
    }).to_string();
    let mut jb = json.into_bytes();
    while jb.len() % 4 != 0 { jb.push(b' '); }
    let total = (12 + 8 + jb.len() + 8 + bin.len()) as u32;
    let mut glb = Vec::new();
    glb.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&total.to_le_bytes());
    glb.extend_from_slice(&(jb.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
    glb.extend_from_slice(&jb);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
    glb.extend_from_slice(&bin);

    let mk = |id: &str, bid: &str, x: f64| Model3D {
        id: id.into(),
        name: bid.into(),
        category: "railway".into(),
        transform: Transform { x, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 120.0,
        depth: 80.0,
        height: 110.0,
        model_type: "builtin".into(),
        builtin_id: Some(bid.into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let custom = Model3D {
        id: "c1".into(),
        name: "CustomGLB".into(),
        category: "custom".into(),
        transform: Transform { x: 500.0, y: 100.0, rotation: 45.0 },
        elevation: Some(0.0),
        scale: Some(2.0),
        layer_id: None,
        width: 20.0,
        depth: 20.0,
        height: 20.0,
        model_type: "glb".into(),
        builtin_id: None,
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: Some(Arc::from(format!(
            "data:model/gltf-binary;base64,{}",
            crate::model_tools::base64_encode(&glb)
        ))),
        file_name: Some("custom.glb".into()),
    };

    let models = vec![
        mk("m1", "coaling-station", -400.0),
        mk("m2", "diesel-fuel-station", -200.0),
        mk("m3", "buffer-stop", -50.0),
        mk("m4", "platform", 100.0),
        mk("m5", "skyscraper", 260.0),
        mk("m6", "industry-building", 420.0),
        custom,
    ];
    let layout = Layout {
        name: "Group4Test".into(),
        scale: "H0".into(),
        pieces: vec![],
        models: Some(models),
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

    let minimal = build_scene_mesh_3d(
        &layout,
        &catalogs,
        &Scene3DOptions {
            env_mode: Viewer3DEnv::Minimal,
            ..Default::default()
        },
    );
    let hall_land = build_scene_mesh_3d(
        &layout,
        &catalogs,
        &Scene3DOptions {
            env_mode: Viewer3DEnv::HallLandscape,
            short_circuit_points: vec![[10.0, 80.0, 10.0]],
            ..Default::default()
        },
    );
    assert!(minimal.vertices.len() > 200);
    assert!(hall_land.vertices.len() > minimal.vertices.len());
    // Hall & Landscape emits horizon-spanning terrain (r > 65000mm) and Alpine peaks
    assert!(
        hall_land
            .vertices
            .iter()
            .any(|v| v.pos[0].hypot(v.pos[2]) > 65_000.0 && v.pos[1] > 4000.0),
        "hall-landscape env missing horizon-spanning alpine terrain"
    );
    // Short-circuit beacon raised by 80mm sits above Y = 140
    assert!(
        hall_land
            .vertices
            .iter()
            .any(|v| (v.pos[0] - 10.0).abs() < 15.0 && (v.pos[2] - 10.0).abs() < 15.0 && v.pos[1] > 140.0),
        "elevated short-circuit beacon missing"
    );
    assert!(
        hall_land
            .vertices
            .iter()
            .any(|v| v.pos[1] > 2000.0 && v.pos[0].abs() < 5500.0),
        "combined env missing industrial hall steel arches"
    );
    assert!(
        hall_land
            .vertices
            .iter()
            .any(|v| v.pos[0].abs() > 6000.0 || v.pos[1] > 4500.0),
        "combined env missing surrounding alpine landscape peaks/clouds"
    );

    // Hall brick walls (above the 1 m wainscot) and glazed windows must exist on the side walls (x = +-4500)
    let floor_y = -755.0_f32;
    let side_wall = |lo: f32, hi: f32| {
        hall_land
            .vertices
            .iter()
            .filter(|v| (v.pos[0].abs() - 4_500.0).abs() < 70.0 && (v.pos[2]).abs() < 4_800.0 && (floor_y + lo..floor_y + hi).contains(&v.pos[1]))
            .count()
    };
    assert!(side_wall(1_700.0, 1_800.0) > 8, "hall brick wall band under windows missing");
    assert!(side_wall(2_500.0, 2_540.0) > 8, "hall window glazing missing");
    assert!(side_wall(3_300.0, 3_400.0) > 8, "hall upper brick wall missing");
    // Translucent glass roof + windows emit dedicated alpha-blended indices & barrel-vault glass vertices
    assert!(
        hall_land.glass_indices.len() >= 360,
        "expected translucent glass roof & window indices, got {}",
        hall_land.glass_indices.len()
    );
    assert!(
        hall_land.vertices.iter().any(|v| {
            v.pos[0].abs() <= 4_550.0
                && v.pos[2].abs() <= 4_950.0
                && v.pos[1] > floor_y + 4_500.0
                && v.color[3] < 200
        }),
        "hall translucent barrel-vault glass roof missing"
    );
    assert!(
        hall_land.vertices.iter().any(|v| {
            (v.pos[0]).abs() < 120.0 && (v.pos[2]).abs() >= 4_900.0 && v.pos[1] > floor_y + 5_100.0
        }),
        "hall longitudinal ridge steel beam missing"
    );
    // Street-facing entrance double door & canopy on west wall (x = -4500, z = 0)
    assert!(
        hall_land.vertices.iter().any(|v| {
            v.pos[0] < -5_100.0
                && v.pos[0] > -5_800.0
                && v.pos[2].abs() < 1_200.0
                && v.pos[1] > floor_y + 2_500.0
        }),
        "street-facing west entrance canopy missing"
    );
    assert!(
        hall_land.vertices.iter().any(|v| {
            (v.pos[0] + 4_500.0).abs() < 30.0
                && v.pos[2].abs() < 800.0
                && v.pos[1] > floor_y + 500.0
                && v.pos[1] < floor_y + 2_150.0
                && v.color[3] < 200
        }),
        "street-facing west entrance door glazing missing"
    );

    // 1:1 real-world sizes (1 unit = 1 mm). Default table => hall 9 x 9.8 m, floor_y = -755, ground g = -779.
    let g = -779.0_f32;
    let road_x = -20_500.0_f32;
    assert!(
        hall_land.vertices.iter().all(|v| v.pos.iter().all(|c| c.is_finite())
            && v.pos[0].hypot(v.pos[2]) <= 340_000.0),
        "landscape must stay finite and inside the 340 m horizon"
    );
    // Country road: asphalt carriageway ~7 m wide at z ~ 0 (top surface at g + 200 mm)
    let road_xs: Vec<f32> = hall_land
        .vertices
        .iter()
        .filter(|v| v.pos[2].abs() < 1_600.0 && (v.pos[0] - road_x).abs() < 4_200.0 && (v.pos[1] - (g + 200.0)).abs() < 12.0)
        .map(|v| v.pos[0])
        .collect();
    assert!(!road_xs.is_empty(), "country road surface missing");
    let road_w = road_xs.iter().cloned().fold(f32::MIN, f32::max) - road_xs.iter().cloned().fold(f32::MAX, f32::min);
    assert!(road_w >= 6_800.0, "road should be ~7 m wide, got {road_w} mm");
    // Alpine chalets (~9.3 m) and chapel steeple (~28 m) east of the hall
    assert!(
        hall_land
            .vertices
            .iter()
            .any(|v| v.pos[0] > 18_000.0 && v.pos[0] < 60_000.0 && v.pos[2].abs() < 30_000.0 && v.pos[1] - g >= 9_000.0),
        "village chalets should reach realistic 9+ m height"
    );
    assert!(
        hall_land
            .vertices
            .iter()
            .any(|v| (v.pos[0] - 34_500.0).abs() < 3_000.0 && (v.pos[2] - 38_000.0).abs() < 12_000.0 && v.pos[1] - g >= 25_000.0),
        "chapel steeple should reach ~28 m"
    );
    // Forest trees 10-23 m tall
    assert!(
        hall_land
            .vertices
            .iter()
            .any(|v| v.pos[0] < -30_000.0 && v.pos[0].hypot(v.pos[2]) < 110_000.0 && (10_000.0..23_000.0).contains(&(v.pos[1] - g))),
        "forest trees should be realistic 10-23 m tall"
    );
}

#[test]
fn minimal_env_supports_dynamic_day_night_lighting() {
    let noon_dark = eval_sky_state(12.0, Viewer3DEnv::Minimal, true);
    let sunset_dark = eval_sky_state(18.0, Viewer3DEnv::Minimal, true);
    let night_dark = eval_sky_state(23.0, Viewer3DEnv::Minimal, true);
    assert!(noon_dark.hemi_ground_pad[3] < 0.05);
    assert!(night_dark.hemi_ground_pad[3] > 0.90);
    assert!(noon_dark.sun_color_int[3] > night_dark.sun_color_int[3]);
    assert!(
        (noon_dark.sun_dir[0] - sunset_dark.sun_dir[0]).abs() > 0.1
            || (noon_dark.sun_dir[1] - sunset_dark.sun_dir[1]).abs() > 0.1
    );

    let noon_light = eval_sky_state(12.0, Viewer3DEnv::Minimal, false);
    let night_light = eval_sky_state(23.0, Viewer3DEnv::Minimal, false);
    assert!(noon_light.clear_color[0] > 0.6 && noon_light.clear_color[1] > 0.6);
    assert!(night_light.clear_color[0] < 0.1 && night_light.clear_color[1] < 0.1);
}

#[test]
fn minimal_cad_background_has_cad_blue_slate_gradient() {
    for dark_mode in [false, true] {
        let sky = eval_sky_state(12.0, Viewer3DEnv::Minimal, dark_mode);
        assert!(
            sky.sky_zenith[2] > sky.sky_zenith[0],
            "CAD top color should have a steel/slate blue tint (dark_mode={dark_mode})"
        );
        assert!(
            sky.fog_color[0] > sky.sky_zenith[0],
            "CAD bottom horizon should be brighter than top steel-blue (dark_mode={dark_mode})"
        );
    }
}

#[test]
fn standalone_tunneled_piece_generates_3d_tube_cover_and_portals() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let mut t0 = create_piece("t0".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    t0.has_tunnel = Some(true);
    let t_plain = create_piece("t1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);

    let mk_layout = |pieces| Layout {
        name: "StandaloneTunnel".into(),
        scale: "H0".into(),
        pieces,
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

    let plain_mesh = build_scene_mesh_3d(&mk_layout(vec![t_plain]), &catalogs, &Scene3DOptions::default());
    let tunnel_mesh = build_scene_mesh_3d(&mk_layout(vec![t0]), &catalogs, &Scene3DOptions::default());
    assert!(tunnel_mesh.vertices.len() > plain_mesh.vertices.len() + 200);
    assert!(tunnel_mesh.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
}

#[test]
fn ground_terrain_brushed_water_and_sand_generate_surface_and_clip() {
    let catalogs = builtin_catalogs();
    let water = Model3D {
        id: "gt-water".into(),
        name: "River".into(),
        category: "terrain".into(),
        transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 500.0,
        depth: 260.0,
        height: 6.0,
        model_type: "builtin".into(),
        builtin_id: Some("ground-terrain".into()),
        mountain_style: None,
        mountain_path: Some(vec![
            railbench_core::types::MountainBrushNode { x: -120.0, y: 0.0, radius: 110.0, height: 6.0 },
            railbench_core::types::MountainBrushNode { x: 120.0, y: 0.0, radius: 110.0, height: 6.0 },
        ]),
        ground_terrain_type: Some("water".into()),
        file_data: None,
        file_name: None,
    };
    let sand = Model3D {
        id: "gt-sand".into(),
        name: "Sand".into(),
        category: "terrain".into(),
        transform: Transform { x: 400.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 300.0,
        depth: 240.0,
        height: 12.0,
        model_type: "builtin".into(),
        builtin_id: Some("ground-terrain".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: Some("sand".into()),
        file_data: None,
        file_name: None,
    };
    let bb = BaseboardConfig {
        id: Some("b1".into()),
        name: Some("B1".into()),
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 2400.0,
        depth: 1200.0,
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
        corner_radius: Some(12.0),
        show_dimensions: Some(true),
    };
    let layout = Layout {
        name: "GroundTest".into(),
        scale: "H0".into(),
        pieces: vec![],
        models: Some(vec![water, sand]),
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
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb]),
        terrain_field: None,
    };

    let mesh = build_scene_mesh_3d(&layout, &catalogs, &Scene3DOptions::default());
    assert!(mesh.vertices.len() > 2500);
    // Must contain reflective blue water surface vertices at ~1.85 mm height
    assert!(mesh.vertices.iter().any(|v| (v.pos[1] - 1.85).abs() < 0.1 && v.color_f32()[2] > 0.7));
    // Must contain warm sand vertices
    assert!(mesh.vertices.iter().any(|v| {
        let c = v.color_f32();
        c[0] > 0.78 && c[1] > 0.65 && c[2] < 0.60
    }));
}

#[test]
fn engine_shed_and_roundhouse_transparent_roofs_reach_scene_cache_single_and_parallel() {
    let catalogs = crate::catalog::builtin_catalogs();
    let mk = |id: &str, bid: &str, x: f64, y: f64, w: f64, d: f64, h: f64| Model3D {
        id: id.into(),
        name: bid.into(),
        category: "railway".into(),
        transform: Transform { x, y, rotation: 0.0 },
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

    // 1. Single engine-shed (single-threaded model path)
    let mut layout1 = crate::make_empty_layout();
    layout1.models = Some(vec![mk("shed-1", "engine-shed", 0.0, 0.0, 240.0, 72.0, 105.0)]);
    let mut cache1 = Scene3DCache::default();
    let bundle1 = update_scene_3d_cache(&layout1, &catalogs, &Scene3DOptions::default(), &mut cache1);
    let tracks1 = bundle1.tracks.expect("tracks bundle").1;
    assert!(!tracks1.glass_indices.is_empty(), "single engine-shed should populate tracks.glass_indices");
    assert!(tracks1.glass_indices.iter().all(|&gi| tracks1.vertices[gi as usize].color[3] < 128));

    // 2. Multiple models (parallel model chunk path)
    let mut layout2 = crate::make_empty_layout();
    layout2.models = Some(vec![
        mk("shed-1", "engine-shed", -200.0, 0.0, 240.0, 72.0, 105.0),
        mk("hall-1", "turntable-trainhall", 300.0, 0.0, 320.0, 310.0, 115.0),
    ]);
    let mut cache2 = Scene3DCache::default();
    let bundle2 = update_scene_3d_cache(&layout2, &catalogs, &Scene3DOptions::default(), &mut cache2);
    let tracks2 = bundle2.tracks.expect("tracks bundle").1;
    assert!(
        tracks2.glass_indices.len() > tracks1.glass_indices.len(),
        "parallel model chunks must merge glass_indices from all sheds"
    );
    assert!(tracks2.glass_indices.iter().all(|&gi| tracks2.vertices[gi as usize].color[3] < 128));
}

