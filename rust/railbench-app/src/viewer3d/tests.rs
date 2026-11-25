use super::*;
use crate::catalog::{builtin_catalogs, create_piece, find_entry};
use railbench_core::types::Transform;

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
fn turntable_3d_generates_pit_bridge_rails_catenary_and_tight_hole() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "74862").expect("74862 turntable in catalog");
    let mut tt = create_piece("tt-1".into(), entry, Transform { x: 400.0, y: 400.0, rotation: 30.0 }, None);
    let bb = BaseboardConfig {
        id: Some("b1".into()),
        name: Some("B1".into()),
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1200.0,
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
        corner_radius: Some(0.0),
        show_dimensions: None,
    };
    let outer = board_tools::get_baseboard_polygon(&bb);
    let layer_info = std::collections::HashMap::new();
    let mk_layout = |piece: TrackPiece| Layout {
        name: "TurntableTest".into(),
        scale: "H0".into(),
        pieces: vec![piece],
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
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb.clone()]),
        terrain_field: None,
    };

    let plain_layout = mk_layout(tt.clone());
    let plain = build_scene_mesh_3d(&plain_layout, &catalogs, &Scene3DOptions::default());
    assert!(plain.vertices.len() > 4000, "turntable mesh too small: {}", plain.vertices.len());
    assert!(plain.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
    assert!(plain.vertices.iter().any(|v| v.pos[1] < -14.0), "missing deep pit floor");
    assert!(plain.vertices.iter().any(|v| (v.pos[1] - 9.6).abs() < 0.05), "missing flush 9.6mm rail top");

    let holes = collect_turntable_holes(&plain_layout, &catalogs, &outer, 0.0, &layer_info);
    assert_eq!(holes.len(), 1);
    assert_eq!(holes[0].len(), 36);
    for pt in &holes[0] {
        let dist = (pt.x - 400.0).hypot(pt.y - 400.0);
        assert!((dist - 142.0).abs() < 0.1 && dist < 189.0, "hole radius {dist} must stay inside outer_r");
    }

    tt.has_catenary = Some(true);
    let cat_mesh = build_scene_mesh_3d(&mk_layout(tt), &catalogs, &Scene3DOptions::default());
    assert!(cat_mesh.vertices.len() > plain.vertices.len() + 1000);
    assert!(cat_mesh.vertices.iter().any(|v| v.pos[1] > 78.0), "missing Drehscheiben-Spinne catenary");
}

#[test]
fn transfer_table_3d_generates_pit_bridge_centered_ties_and_rect_hole() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "72941").expect("72941 transfer-table in catalog");
    let mut tf = create_piece("tf-1".into(), entry, Transform { x: 0.0, y: -154.8, rotation: 90.0 }, None);
    tf.has_catenary = Some(true);
    let bb = BaseboardConfig {
        id: Some("b1".into()),
        name: Some("B1".into()),
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1400.0,
        depth: 1400.0,
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
    let outer = board_tools::get_baseboard_polygon(&bb);
    let layer_info = std::collections::HashMap::new();
    let layout = Layout {
        name: "TransferTest".into(),
        scale: "H0".into(),
        pieces: vec![tf],
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
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb]),
        terrain_field: None,
    };

    let mesh = build_scene_mesh_3d(&layout, &catalogs, &Scene3DOptions::default());
    assert!(mesh.vertices.len() > 4000, "transfer-table mesh too small: {}", mesh.vertices.len());
    assert!(mesh.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
    assert!(mesh.vertices.iter().any(|v| v.pos[1] < -11.5), "missing recessed transfer pit floor");
    assert!(mesh.vertices.iter().any(|v| (v.pos[1] - 9.6).abs() < 0.05), "missing flush 9.6mm rails");
    assert!(mesh.vertices.iter().any(|v| v.pos[1] > 80.0), "missing overhead power portal arch / catenary");

    let holes = collect_turntable_holes(&layout, &catalogs, &outer, 0.0, &layer_info);
    assert_eq!(holes.len(), 1);
    assert_eq!(holes[0].len(), 4);
}

#[test]
fn scene_3d_cache_reuses_unchanged_submeshes_and_parallel_terrain() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24188").expect("24188 in catalog");
    let p1 = create_piece("p1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let m1 = Model3D {
        id: "mtn-1".into(),
        name: "Alps 1".into(),
        category: "nature".into(),
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        file_name: None,
        file_data: None,
        transform: Transform { x: -300.0, y: 0.0, rotation: 0.0 },
        width: 300.0,
        depth: 260.0,
        height: 150.0,
        mountain_style: Some("alpine".into()),
        mountain_path: None,
        ground_terrain_type: None,
        scale: Some(1.0),
        elevation: Some(0.0),
        layer_id: None,
    };
    let m2 = Model3D {
        id: "mtn-2".into(),
        name: "Alps 2".into(),
        category: "nature".into(),
        model_type: "builtin".into(),
        builtin_id: Some("ground-terrain".into()),
        file_name: None,
        file_data: None,
        transform: Transform { x: 300.0, y: 0.0, rotation: 0.0 },
        width: 280.0,
        depth: 260.0,
        height: 30.0,
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: Some("hill".into()),
        scale: Some(1.0),
        elevation: Some(0.0),
        layer_id: None,
    };
    let mut layout = Layout {
        name: "CacheTest".into(),
        scale: "H0".into(),
        pieces: vec![p1],
        models: Some(vec![m1, m2]),
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

    let mut cache = Scene3DCache::default();
    let mut opts = Scene3DOptions::default();
    let mesh1 = build_scene_mesh_3d_cached(&layout, &catalogs, &opts, &mut cache);
    assert!(mesh1.vertex_count() > 2000);
    assert!(cache.last_build_threads >= 1);
    assert_eq!(cache.env_rebuilds, 1);
    assert_eq!(cache.boards_rebuilds, 1);
    assert_eq!(cache.terrain_rebuilds, 1);
    assert_eq!(cache.tracks_rebuilds, 1);

    // Changing only time_of_day or color_mode should NOT rebuild env, boards, or terrain
    opts.time_of_day = 18.0;
    let _mesh2 = build_scene_mesh_3d_cached(&layout, &catalogs, &opts, &mut cache);
    assert_eq!(cache.env_rebuilds, 1);
    assert_eq!(cache.boards_rebuilds, 1);
    assert_eq!(cache.terrain_rebuilds, 1);
    assert_eq!(cache.tracks_rebuilds, 1);

    opts.color_mode = ColorMode::Types;
    let _mesh3 = build_scene_mesh_3d_cached(&layout, &catalogs, &opts, &mut cache);
    assert_eq!(cache.env_rebuilds, 1);
    assert_eq!(cache.boards_rebuilds, 1);
    assert_eq!(cache.terrain_rebuilds, 1);
    assert_eq!(cache.tracks_rebuilds, 2);

    // Toggling catenary on a track piece rebuilds tracks_and_models, leaving env & boards cached
    layout.pieces[0].has_catenary = Some(true);
    let mesh4 = build_scene_mesh_3d_cached(&layout, &catalogs, &opts, &mut cache);
    assert!(mesh4.vertex_count() > mesh1.vertex_count());
    assert_eq!(cache.env_rebuilds, 1);
    assert_eq!(cache.boards_rebuilds, 1);
    assert_eq!(cache.terrain_rebuilds, 1);
    assert_eq!(cache.tracks_rebuilds, 3);
}

#[test]
fn gpu_track_instancing_shares_one_prototype_per_track_type() {
    let catalogs = builtin_catalogs();
    let straight = find_entry(&catalogs, "24188").expect("24188 in catalog");
    let curve = find_entry(&catalogs, "24130").expect("24130 in catalog");

    let mut pieces = Vec::new();
    for i in 0..50 {
        pieces.push(create_piece(
            format!("s-{i}"),
            straight,
            Transform {
                x: (i as f64) * 200.0,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        ));
        pieces.push(create_piece(
            format!("c-{i}"),
            curve,
            Transform {
                x: (i as f64) * 200.0,
                y: 120.0,
                rotation: (i as f64) * 15.0,
            },
            None,
        ));
    }

    let layout = Layout {
        name: "InstancingTest".into(),
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

    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: false,
        ..Default::default()
    };
    let mut cache = Scene3DCache::default();
    let mesh = build_scene_mesh_3d_cached(&layout, &catalogs, &opts, &mut cache);

    // 100 track pieces of 2 catalog types must produce exactly 2 shared prototype meshes and 100 instances!
    assert_eq!(mesh.track_prototype_count(), 2);
    assert_eq!(mesh.track_instance_count(), 100);
    assert_eq!(cache.track_prototypes.len(), 2);
    for proto in &mesh.instanced_tracks {
        assert_eq!(proto.instances.len(), 50);
        assert!(!proto.vertices.is_empty());
    }
}

#[test]
fn vertex3d_is_compact_24_bytes_and_preserves_material_thresholds() {
    assert_eq!(std::mem::size_of::<Vertex3D>(), 24);

    let ballast = Vertex3D::new([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [0.5, 0.45, 0.4], MAT_BALLAST);
    let terrain = Vertex3D::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.2, 0.5, 0.1], MAT_TERRAIN);
    let night_win = Vertex3D::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.85, 0.4], [0.14, 0.18, -0.85]);
    let sun = Vertex3D::new([0.0, 100.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.95, 0.68], [0.10, 0.0, 1.35]);

    // WGSL gravel threshold > 0.935 must separate MAT_BALLAST from MAT_TERRAIN after 8-bit quantization
    assert!(ballast.mat_f32()[0] > 0.935);
    assert!(terrain.mat_f32()[0] <= 0.935);
    // Signed emissive range [-2.0, +2.0] preserved for night windows & celestial HDR sun
    assert!((night_win.mat_f32()[2] - -0.85).abs() < 0.02);
    assert!((sun.mat_f32()[2] - 1.35).abs() < 0.02);
    assert!((ballast.normal_f32()[1] - 1.0).abs() < 1e-4);
    assert!((sun.color_f32()[0] - 1.0).abs() < 1e-4);
}

#[test]
fn scene_3d_bundle_shares_arcs_and_prunes_unused_prototypes() {
    let catalogs = builtin_catalogs();
    let straight = find_entry(&catalogs, "24188").expect("24188 in catalog");
    let curve = find_entry(&catalogs, "24130").expect("24130 in catalog");
    let mut layout = Layout {
        name: "BundleTest".into(),
        scale: "H0".into(),
        pieces: vec![
            create_piece("s1".into(), straight, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None),
            create_piece("c1".into(), curve, Transform { x: 300.0, y: 0.0, rotation: 0.0 }, None),
        ],
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

    let mut cache = Scene3DCache::default();
    let opts = Scene3DOptions {
        show_wood_plates: false,
        ..Default::default()
    };
    let bundle1 = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    assert_eq!(cache.track_prototypes.len(), 2);

    // Bundle shares exact Arcs with Scene3DCache and InstancedTrackPrototype3D shares Arc with track_prototypes
    assert!(Arc::ptr_eq(&bundle1.env.as_ref().unwrap().1, &cache.env.as_ref().unwrap().1));
    assert!(Arc::ptr_eq(&bundle1.boards.as_ref().unwrap().1, &cache.boards.as_ref().unwrap().1));
    assert!(Arc::ptr_eq(&bundle1.terrain.as_ref().unwrap().1, &cache.terrain.as_ref().unwrap().1));
    let tracks_mesh = &bundle1.tracks.as_ref().unwrap().1;
    for proto in &tracks_mesh.instanced_tracks {
        let (cached_v, cached_i) = cache.track_prototypes.get(&proto.catalog_id).unwrap();
        assert!(Arc::ptr_eq(&proto.vertices, cached_v));
        assert!(Arc::ptr_eq(&proto.indices, cached_i));
    }

    // Remove the curve piece: unused track_prototype must be pruned, while env, boards & terrain keep exact Arcs
    layout.pieces.pop();
    let bundle2 = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    assert_eq!(cache.track_prototypes.len(), 1);
    assert!(cache.track_prototypes.contains_key("24188"));
    assert!(Arc::ptr_eq(&bundle1.env.as_ref().unwrap().1, &bundle2.env.as_ref().unwrap().1));
    assert!(Arc::ptr_eq(&bundle1.boards.as_ref().unwrap().1, &bundle2.boards.as_ref().unwrap().1));
    assert!(Arc::ptr_eq(&bundle1.terrain.as_ref().unwrap().1, &bundle2.terrain.as_ref().unwrap().1));

    cache.clear();
    assert!(cache.env.is_none() && cache.tracks_and_models.is_none() && cache.track_prototypes.is_empty());
}

#[test]
fn window_msaa_samples_is_enabled_for_2d_antialiasing() {
    assert_eq!(WINDOW_MSAA_SAMPLES, 4);
    assert!(WINDOW_MSAA_SAMPLES.is_power_of_two());
}

#[test]
fn terrain_3d_mountain_and_water_never_exceed_table_or_upper_plate() {
    use railbench_core::types::{Layer, LayerPlate};

    let catalogs = builtin_catalogs();
    let bb = BaseboardConfig {
        id: Some("b1".into()),
        name: Some("Table".into()),
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
        corner_radius: Some(0.0),
        show_dimensions: None,
    };
    let upper_plate = LayerPlate {
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
        width: 500.0,
        depth: 400.0,
        x: Some(950.0),
        y: Some(400.0),
        thickness_mm: Some(12.0),
        position_mode: Some("custom".into()),
        style: None,
    };
    let mtn_table = Model3D {
        id: "mtn-edge".into(),
        name: "EdgeMtn".into(),
        category: "terrain".into(),
        transform: Transform { x: 1180.0, y: 0.0, rotation: 0.0 },
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
    let water_edge = Model3D {
        id: "water-edge".into(),
        name: "EdgeLake".into(),
        category: "terrain".into(),
        transform: Transform { x: -1180.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: Some("layer-1".into()),
        width: 500.0,
        depth: 400.0,
        height: 8.0,
        model_type: "builtin".into(),
        builtin_id: Some("ground-terrain".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: Some("water".into()),
        file_data: None,
        file_name: None,
    };
    let mtn_plate = Model3D {
        id: "mtn-plate".into(),
        name: "PlateMtn".into(),
        category: "terrain".into(),
        transform: Transform { x: 240.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: Some("layer-2".into()),
        width: 450.0,
        depth: 400.0,
        height: 150.0,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: Some("alpine".into()),
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };

    let layout = Layout {
        name: "Clip3D".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
        models: Some(vec![mtn_table, water_edge, mtn_plate]),
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
                height_mm: Some(300.0),
                wood_plate_mode: Some("table".into()),
                wood_plate_thickness_mm: Some(12.0),
                wood_plate_style: None,
                plates: Some(vec![upper_plate]),
                terrain_field: None,
            },
        ]),
        active_layer_id: Some("layer-1".into()),
        baseboard: Some(bb),
        baseboards: None,
        terrain_field: None,
    };

    let mut cache = Scene3DCache::default();
    let bundle = update_scene_3d_cache(&layout, &catalogs, &Scene3DOptions::default(), &mut cache);
    let terrain = &bundle.terrain.expect("terrain mesh").1;
    assert!(!terrain.indices.is_empty());

    for &vi in &terrain.indices {
        let p = terrain.vertices[vi as usize].pos;
        if p[1] >= 295.0 {
            // Elevated layer-2 terrain must stay strictly within its 500x400 plate ([-250..250, -200..200])
            assert!(
                p[0].abs() <= 250.5 && p[2].abs() <= 200.5,
                "elevated terrain vertex ({:.1}, {:.1}, {:.1}) escaped plate!",
                p[0],
                p[1],
                p[2]
            );
        } else {
            // Table-level terrain must stay strictly within the 2400x1200 table ([-1200..1200, -600..600])
            assert!(
                p[0].abs() <= 1200.5 && p[2].abs() <= 600.5,
                "table terrain vertex ({:.1}, {:.1}, {:.1}) escaped table!",
                p[0],
                p[1],
                p[2]
            );
        }
    }
}

#[test]
fn double_slip_24624_and_crossing_24640_generate_correct_3d_geometry() {
    let catalogs = builtin_catalogs();
    let e_dkw = find_entry(&catalogs, "24624").expect("24624 entry");
    let e_crs = find_entry(&catalogs, "24640").expect("24640 entry");
    let e_curv_sw = find_entry(&catalogs, "24671").expect("24671 entry");

    let c_dkw = crate::catalog::local_connectors(e_dkw);
    assert_eq!(c_dkw.len(), 4, "24624 must have 4 local connectors");
    assert!((c_dkw[0].x - 0.0).abs() < 1e-3 && (c_dkw[0].y - 0.0).abs() < 1e-3 && (c_dkw[0].angle - 180.0).abs() < 1e-3);
    assert!((c_dkw[1].x - 188.3).abs() < 1e-3 && (c_dkw[1].y - 0.0).abs() < 1e-3 && (c_dkw[1].angle - 0.0).abs() < 1e-3);
    assert!((c_dkw[2].x - 8.3414).abs() < 1e-2 && (c_dkw[2].y + 38.7441).abs() < 1e-2 && (c_dkw[2].angle - 204.3).abs() < 1e-2);
    assert!((c_dkw[3].x - 179.9586).abs() < 1e-2 && (c_dkw[3].y - 38.7441).abs() < 1e-2 && (c_dkw[3].angle - 24.3).abs() < 1e-2);

    let c_crs = crate::catalog::local_connectors(e_crs);
    assert_eq!(c_crs.len(), 4, "24640 must have 4 centered connectors");
    assert!((c_crs[0].x + 94.15).abs() < 1e-3 && (c_crs[1].x - 94.15).abs() < 1e-3);

    let c_curv = crate::catalog::local_connectors(e_curv_sw);
    assert_eq!(c_curv.len(), 3, "24671 curved turnout must have 3 connectors");

    let dkw_piece = create_piece("dkw-1".into(), e_dkw, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let branches_dkw = super::tracks::sample_track_branches_3d(&dkw_piece, Some(e_dkw), 0.0, 0.0);
    assert_eq!(branches_dkw.len(), 4, "24624 must emit 2 crossing straights + 2 curved slip branches");
    let mid_br2_z = branches_dkw[2][branches_dkw[2].len() / 2][2];
    let mid_br3_z = branches_dkw[3][branches_dkw[3].len() / 2][2];
    assert!(
        mid_br2_z.abs() > 8.5 && mid_br3_z.abs() > 8.5,
        "24624 curved slip branches must bow at midpoint (got z={mid_br2_z}, {mid_br3_z})"
    );

    let crs_piece = create_piece("crs-1".into(), e_crs, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let branches_crs = super::tracks::sample_track_branches_3d(&crs_piece, Some(e_crs), 0.0, 0.0);
    assert_eq!(branches_crs.len(), 2, "24640 crossing must emit 2 straight branches");

    let layout = Layout {
        name: "DKW24624".into(),
        scale: "H0".into(),
        pieces: vec![dkw_piece],
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

    let scene = build_scene_mesh_3d(&layout, &catalogs, &Scene3DOptions::default());
    let dkw_proto = scene
        .instanced_tracks
        .iter()
        .find(|p| p.catalog_id == "24624")
        .expect("24624 should produce an instanced 3D prototype");
    assert!(!dkw_proto.vertices.is_empty() && !dkw_proto.indices.is_empty());
    let min_z = dkw_proto.vertices.iter().map(|v| v.pos[2]).fold(f32::INFINITY, f32::min);
    let max_z = dkw_proto.vertices.iter().map(|v| v.pos[2]).fold(f32::NEG_INFINITY, f32::max);
    assert!(
        min_z < -40.0 && max_z > 40.0,
        "24624 3D prototype must span both crossing arms & slips (got z range [{min_z}, {max_z}])"
    );
    assert!(dkw_proto.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
}

#[test]
fn default_turnout_lantern_is_unlit_and_reflects_branch_position() {
    let catalogs = builtin_catalogs();
    let turnout_entry = find_entry(&catalogs, "24611").expect("24611 in catalog");
    let plain_turnout = create_piece("sw1".into(), turnout_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let mut lit_turnout = plain_turnout.clone();
    lit_turnout.has_lantern = Some(true);

    let make_layout = |p: TrackPiece| Layout {
        name: "Lantern3D".into(),
        scale: "H0".into(),
        pieces: vec![p],
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

    let all_verts = |mesh: &SceneMesh3D| -> Vec<Vertex3D> {
        mesh.vertices
            .iter()
            .copied()
            .chain(mesh.instanced_tracks.iter().flat_map(|p| p.vertices.iter().copied()))
            .collect()
    };

    let minimal = |turnout_states| Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: false,
        turnout_states,
        ..Default::default()
    };

    // 1. Default turnout: lantern housing is built into geometry, all vertices finite & UNLIT (emissive == 0)
    let m_default_b0 = build_scene_mesh_3d(&make_layout(plain_turnout.clone()), &catalogs, &minimal(Default::default()));
    let v_b0 = all_verts(&m_default_b0);
    assert!(v_b0.len() > 150, "expected turnout + mechanical lantern vertices");
    assert!(
        v_b0.iter().all(|v| v.mat_f32()[2].abs() < 1e-3),
        "default unequipped turnout lantern must have zero emissive vertices"
    );

    // 2. Switching turnout branch to diverging (1) changes generated 3D geometry while remaining UNLIT
    let mut branches_div = std::collections::HashMap::new();
    branches_div.insert("sw1".to_string(), 1usize);
    let m_default_b1 = build_scene_mesh_3d(&make_layout(plain_turnout), &catalogs, &minimal(branches_div));
    assert!(
        !m_default_b1.vertices.is_empty(),
        "switched turnout branch should de-instance and emit active-branch geometry"
    );
    let v_b1 = all_verts(&m_default_b1);
    assert!(
        v_b1.iter().all(|v| v.mat_f32()[2].abs() < 1e-3),
        "switched unequipped turnout lantern must stay unlit"
    );
    let b0_flat: Vec<[f32; 3]> = v_b0.iter().map(|v| v.pos).collect();
    let b1_flat: Vec<[f32; 3]> = v_b1.iter().map(|v| v.pos).collect();
    assert_ne!(b0_flat, b1_flat, "diverging turnout lantern aspect should differ from straight");

    // 3. Equipping has_lantern = Some(true) lights up the lantern with emissive vertices
    let m_lit = build_scene_mesh_3d(&make_layout(lit_turnout), &catalogs, &minimal(Default::default()));
    let v_lit = all_verts(&m_lit);
    assert!(
        v_lit.iter().any(|v| v.mat_f32()[2] > 0.5),
        "equipped turnout lantern should emit warm milk-glass emissive vertices"
    );
}

#[test]
fn unified_heightfield_generates_3d_mesh_with_skirts_water_and_track_carving() {
    let catalogs = builtin_catalogs();
    let e24188 = find_entry(&catalogs, "24188").expect("24188 in catalog");
    let piece = create_piece(
        "hf-t1".into(),
        e24188,
        Transform {
            x: -94.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let bb = BaseboardConfig {
        id: Some("hf-board".into()),
        name: None,
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1000.0,
        depth: 600.0,
        rotation: None,
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: Some(0.0),
        custom_y: Some(0.0),
        style: "wood".into(),
        corner_radius: None,
        show_dimensions: None,
    };
    let mut layout = Layout {
        name: "UnifiedHF3D".into(),
        scale: "H0".into(),
        pieces: vec![piece],
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
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb]),
        terrain_field: None,
    };

    // Stamp corner mountain and paint a pond
    assert!(crate::terrain_tools::stamp_preset_into_heightfield(&mut layout, "corner", 190.0));
    assert!(crate::terrain_tools::apply_heightfield_brush(
        &mut layout,
        crate::terrain_tools::SculptOp::PaintBiome(railbench_core::terrain::BIOME_WATER),
        Vec2 { x: 240.0, y: 120.0 },
        140.0,
        0.9,
        240.0,
        railbench_core::terrain::BIOME_WATER,
    ));

    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: true,
        ..Default::default()
    };
    let mut cache = Scene3DCache::default();
    let b1 = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    let terr1 = &b1.terrain.as_ref().expect("heightfield emits terrain submesh").1;
    assert!(!terr1.vertices.is_empty());
    assert!(terr1.vertices.iter().all(|v| {
        v.pos[0].is_finite()
            && v.pos[1].is_finite()
            && v.pos[2].is_finite()
            && v.pos[0].abs() <= 500.5
            && v.pos[2].abs() <= 300.5
    }));
    // Water surface MAT_WATER should be present
    assert!(
        terr1
            .vertices
            .iter()
            .any(|v| (v.mat_f32()[0] - MAT_WATER[0]).abs() < 0.04 && v.color_f32()[2] > 0.5),
        "expected glossy water surface vertices from BIOME_WATER"
    );

    // Mutate only heightfield revision via sculpt and confirm Scene3DCache reuses env/boards/tracks!
    assert!(crate::terrain_tools::apply_heightfield_brush(
        &mut layout,
        crate::terrain_tools::SculptOp::Raise,
        Vec2 { x: -180.0, y: 140.0 },
        130.0,
        0.8,
        220.0,
        0,
    ));
    let b2 = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    assert!(std::sync::Arc::ptr_eq(&b1.env.as_ref().unwrap().1, &b2.env.as_ref().unwrap().1));
    assert!(std::sync::Arc::ptr_eq(&b1.boards.as_ref().unwrap().1, &b2.boards.as_ref().unwrap().1));
    assert!(std::sync::Arc::ptr_eq(&b1.tracks.as_ref().unwrap().1, &b2.tracks.as_ref().unwrap().1));
    assert!(!std::sync::Arc::ptr_eq(&b1.terrain.as_ref().unwrap().1, &b2.terrain.as_ref().unwrap().1));
}

#[test]
fn per_layer_heightfield_3d_renders_at_layer_elevation_and_clips_to_plate() {
    let catalogs = builtin_catalogs();
    let bb = BaseboardConfig {
        id: Some("hf-board".into()),
        name: None,
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1000.0,
        depth: 600.0,
        rotation: None,
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: Some(0.0),
        custom_y: Some(0.0),
        style: "wood".into(),
        corner_radius: None,
        show_dimensions: None,
    };
    let mut layout = Layout {
        name: "LayerHF3D".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
        models: None,
        feed_voltage: None,
        catenary_feed_voltage: None,
        catenary_circuit_mode: None,
        loco_current_ma: None,
        loco_count: None,
        train_current_amps: None,
        turnout_lantern_current_ma: None,
        turnout_decoder_current_ma: None,
        layers: Some(vec![
            railbench_core::types::Layer {
                id: "layer-1".into(),
                name: "Base".into(),
                color: None,
                visible: true,
                locked: false,
                height_mm: Some(0.0),
                wood_plate_mode: None,
                wood_plate_thickness_mm: None,
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            },
            railbench_core::types::Layer {
                id: "layer-2".into(),
                name: "Upper".into(),
                color: None,
                visible: true,
                locked: false,
                height_mm: Some(80.0),
                wood_plate_mode: Some("custom".into()),
                wood_plate_thickness_mm: Some(10.0),
                wood_plate_style: Some("wood".into()),
                plates: Some(vec![railbench_core::types::LayerPlate {
                    id: "plate-l2".into(),
                    name: None,
                    baseboard_id: Some("hf-board".into()),
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
                    x: Some(300.0),
                    y: Some(150.0),
                    thickness_mm: Some(10.0),
                    position_mode: Some("custom".into()),
                    style: Some("wood".into()),
                }]),
                terrain_field: None,
            },
        ]),
        active_layer_id: Some("layer-2".into()),
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb]),
        terrain_field: None,
    };

    assert!(crate::terrain_tools::apply_heightfield_brush(
        &mut layout,
        crate::terrain_tools::SculptOp::Raise,
        Vec2 { x: 0.0, y: 0.0 },
        180.0,
        1.0,
        240.0,
        0,
    ));

    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: true,
        ..Default::default()
    };
    let mut cache = Scene3DCache::default();
    let batch = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    let terr = &batch.terrain.as_ref().expect("layer-2 heightfield emits 3D submesh").1;
    assert!(!terr.vertices.is_empty());
    assert!(
        terr.vertices.iter().all(|v| v.pos[0].abs() <= 200.5 && v.pos[2].abs() <= 150.5),
        "layer-2 3D terrain should clip to its 400x300 plate"
    );
    assert!(
        terr.vertices.iter().any(|v| v.pos[1] > 85.0),
        "layer-2 3D terrain vertices should rise above its 80mm layer height"
    );
}
