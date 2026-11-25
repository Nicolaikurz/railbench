use super::*;
use crate::catalog::{builtin_catalogs, create_piece};
use railbench_core::snapping::update_layout_connections;

fn empty_test_layout() -> Layout {
    Layout {
        name: "TerrainTest".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
        models: Some(Vec::new()),
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
    }
}

#[test]
fn brush_paint_extend_smooth_and_erase_nodes() {
    let mut layout = empty_test_layout();
    let mut seq = 1u64;
    let mut brush = TerrainBrushState::default();
    brush.set_active(true, Some(TerrainBrushMode::Paint), None);
    brush.radius = 150.0;
    brush.height = 200.0;
    brush.ridge_profile = TerrainRidgeProfile::PeakCenter;

    brush.start_stroke(Vec2 { x: -200.0, y: 0.0 }, &mut layout);
    brush.move_stroke(Vec2 { x: 0.0, y: 80.0 }, &mut layout);
    brush.move_stroke(Vec2 { x: 200.0, y: 0.0 }, &mut layout);
    let id = brush.finish_stroke(&mut layout, &mut seq, None, false).unwrap();

    let m = layout.models.as_ref().unwrap().iter().find(|m| m.id == id).unwrap();
    assert_eq!(m.mountain_path.as_ref().unwrap().len(), 3);
    let mut smoothed = m.clone();
    assert!(smooth_mountain_path(&mut smoothed));

    // Extend existing mountain
    brush.set_active(true, Some(TerrainBrushMode::Extend), Some(id.clone()));
    brush.start_stroke(Vec2 { x: 350.0, y: 0.0 }, &mut layout);
    brush.move_stroke(Vec2 { x: 480.0, y: 0.0 }, &mut layout);
    let ext_id = brush.finish_stroke(&mut layout, &mut seq, None, false).unwrap();
    assert_eq!(ext_id, id);
    assert_eq!(
        layout.models.as_ref().unwrap()[0]
            .mountain_path
            .as_ref()
            .unwrap()
            .len(),
        5
    );

    // Node-accurate erase at x=480 reduces path to 4 nodes
    assert!(erase_terrain_at(Vec2 { x: 480.0, y: 0.0 }, 120.0, &mut layout));
    assert_eq!(
        layout.models.as_ref().unwrap()[0]
            .mountain_path
            .as_ref()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn brush_ground_mode_creates_brushed_ground_terrain() {
    let mut layout = empty_test_layout();
    let mut seq = 1u64;
    let mut brush = TerrainBrushState::default();
    brush.set_active(true, Some(TerrainBrushMode::Ground), None);
    brush.set_ground_type("water");
    brush.radius = 120.0;

    brush.start_stroke(Vec2 { x: -120.0, y: 0.0 }, &mut layout);
    brush.move_stroke(Vec2 { x: 120.0, y: 40.0 }, &mut layout);
    let id = brush.finish_stroke(&mut layout, &mut seq, None, false).unwrap();

    let m = layout.models.as_ref().unwrap().iter().find(|m| m.id == id).unwrap();
    assert_eq!(m.builtin_id.as_deref(), Some("ground-terrain"));
    assert_eq!(m.ground_terrain_type.as_deref(), Some("water"));
    assert_eq!(m.mountain_path.as_ref().unwrap().len(), 2);
}

#[test]
fn auto_tunnel_marks_intersecting_tracks_and_clears_bridge() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let mut p = create_piece("t1".into(), entry, Transform { x: -180.0, y: 0.0, rotation: 0.0 }, None);
    p.has_bridge = Some(true);

    let mtn = Model3D {
        id: "m1".into(),
        name: "M".into(),
        category: "terrain".into(),
        transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 600.0,
        depth: 450.0,
        height: 220.0,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: Some("alpine".into()),
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let mut layout = empty_test_layout();
    layout.pieces = vec![p];
    layout.models = Some(vec![mtn]);

    let marked = apply_auto_tunnel_all_mountains(&mut layout);
    assert_eq!(marked, 1);
    assert_eq!(layout.pieces[0].has_tunnel, Some(true));
    assert_eq!(layout.pieces[0].has_bridge, Some(false));
}

#[test]
fn multi_track_parallel_tunnels_merge_into_wide_portal() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let a0 = create_piece("a0".into(), entry, Transform { x: -360.0, y: -32.0, rotation: 0.0 }, None);
    let a1 = create_piece("a1".into(), entry, Transform { x: 0.0, y: -32.0, rotation: 0.0 }, None);
    let b0 = create_piece("b0".into(), entry, Transform { x: -360.0, y: 32.0, rotation: 0.0 }, None);
    let b1 = create_piece("b1".into(), entry, Transform { x: 0.0, y: 32.0, rotation: 0.0 }, None);
    let mtn = Model3D {
        id: "m1".into(),
        name: "Twin".into(),
        category: "terrain".into(),
        transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 500.0,
        depth: 500.0,
        height: 240.0,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let mut layout = empty_test_layout();
    layout.pieces = vec![a0, a1, b0, b1];
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
    layout.models = Some(vec![mtn.clone()]);

    let layer_info = HashMap::new();
    let chains = build_connected_corridor_chains(&layout, &catalogs, &layer_info);
    assert_eq!(chains.len(), 2);
    let runs = compute_tunnel_runs_for_mountain(&mtn, 0.0, &chains, &[&mtn], &layer_info);
    assert_eq!(runs.len(), 2);
    let sp0 = runs[0].start_portal.as_ref().unwrap();
    let sp1 = runs[1].start_portal.as_ref().unwrap();
    assert_eq!(sp0.prof.num_tracks, 2);
    assert_eq!(sp1.prof.num_tracks, 2);
    assert!(sp0.prof.inner_width > 110.0);
}

#[test]
fn overlapping_mountains_suppress_interior_portal() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let p0 = create_piece("p0".into(), entry, Transform { x: -420.0, y: 0.0, rotation: 0.0 }, None);
    let p1 = create_piece("p1".into(), entry, Transform { x: -60.0, y: 0.0, rotation: 0.0 }, None);
    let p2 = create_piece("p2".into(), entry, Transform { x: 300.0, y: 0.0, rotation: 0.0 }, None);

    let m1 = Model3D {
        id: "m1".into(),
        name: "Left".into(),
        category: "terrain".into(),
        transform: Transform { x: -120.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 440.0,
        depth: 400.0,
        height: 220.0,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let m2 = Model3D {
        id: "m2".into(),
        name: "Right".into(),
        category: "terrain".into(),
        transform: Transform { x: 140.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 440.0,
        depth: 400.0,
        height: 220.0,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let mut layout = empty_test_layout();
    layout.pieces = vec![p0, p1, p2];
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
    layout.models = Some(vec![m1.clone(), m2.clone()]);

    let layer_info = HashMap::new();
    let chains = build_connected_corridor_chains(&layout, &catalogs, &layer_info);
    let runs_m1 = compute_tunnel_runs_for_mountain(&m1, 0.0, &chains, &[&m1, &m2], &layer_info);
    assert_eq!(runs_m1.len(), 1);
    assert!(!runs_m1[0].start_portal.as_ref().unwrap().suppressed_by_overlap);
    assert!(runs_m1[0].end_portal.as_ref().unwrap().suppressed_by_overlap);
}

#[test]
fn elevated_layer_plate_clips_terrain_without_falling_back_to_lower_table() {
    use railbench_core::types::{Layer, LayerPlate};

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
        width: 600.0,
        depth: 400.0,
        x: Some(900.0),
        y: Some(400.0),
        thickness_mm: Some(12.0),
        position_mode: Some("custom".into()),
        style: None,
    };
    let mut layout = empty_test_layout();
    layout.baseboard = Some(bb.clone());
    layout.layers = Some(vec![
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
    ]);

    let polys_l2 = collect_terrain_support_polygons(&layout, Some("layer-2"), &[bb]);
    assert_eq!(polys_l2.len(), 1, "elevated layer with plates must clip strictly to its plates");
    let max_x = polys_l2[0].iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    assert!((max_x - 300.0).abs() < 1e-6);

    // Brush stroke outside table bounds must clamp cleanly onto the table edge
    let mut seq = 1u64;
    let mut brush = TerrainBrushState::default();
    brush.set_active(true, Some(TerrainBrushMode::Stamp), None);
    brush.start_stroke(Vec2 { x: 5000.0, y: 0.0 }, &mut layout);
    let mtn_id = brush.finish_stroke(&mut layout, &mut seq, Some("layer-1"), false).unwrap();
    let m = layout.models.as_ref().unwrap().iter().find(|m| m.id == mtn_id).unwrap();
    assert!((m.transform.x - 1200.0).abs() <= 1.0, "stamp outside table must clamp to x=1200, got {}", m.transform.x);
}

#[test]
fn unified_heightfield_raise_lower_smooth_flatten_biome_and_board_clipping() {
    let mut layout = empty_test_layout();
    let bb = BaseboardConfig {
        id: Some("b1".into()),
        name: None,
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1200.0,
        depth: 800.0,
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
    layout.baseboard = Some(bb.clone());
    layout.baseboards = Some(vec![bb]);

    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24188").unwrap();
    let p = create_piece(
        "buried-track".into(),
        entry,
        Transform { x: -90.0, y: 0.0, rotation: 0.0 },
        None,
    );
    layout.pieces.push(p);

    // 1. Raise a 90mm+ mountain peak over the track at origin
    for _ in 0..5 {
        assert!(apply_heightfield_brush(
            &mut layout,
            SculptOp::Raise,
            Vec2 { x: 0.0, y: 0.0 },
            200.0,
            0.85,
            240.0,
            railbench_core::terrain::BIOME_AUTO,
        ));
    }
    let h_peak = {
        let hf = layout.terrain_field.as_ref().unwrap();
        railbench_core::terrain::sample_heightfield_bilinear(hf, 0.0, 0.0)
    };
    assert!(h_peak > 75.0, "expected raised peak > 75mm, got {h_peak}");

    // 2. Brush centered outside the board boundary (x=700 vs half-width=600) must leave far edge clipped cleanly
    apply_heightfield_brush(
        &mut layout,
        SculptOp::Raise,
        Vec2 { x: 590.0, y: 0.0 },
        180.0,
        0.9,
        240.0,
        railbench_core::terrain::BIOME_ROCK,
    );
    assert!(apply_auto_tunnel_for_heightfield(&mut layout) >= 1);
    assert_eq!(layout.pieces[0].has_tunnel, Some(true));

    // 3. Smooth, Flatten, Lower, Biome(Water), and Reset
    assert!(apply_heightfield_brush(
        &mut layout,
        SculptOp::Smooth,
        Vec2 { x: 0.0, y: 0.0 },
        180.0,
        0.8,
        240.0,
        0,
    ));
    assert!(apply_heightfield_brush(
        &mut layout,
        SculptOp::Flatten(45.0),
        Vec2 { x: 0.0, y: 0.0 },
        160.0,
        1.0,
        240.0,
        0,
    ));
    let h_flat = railbench_core::terrain::sample_heightfield_bilinear(
        layout.terrain_field.as_ref().unwrap(),
        0.0,
        0.0,
    );
    assert!((h_flat - 45.0).abs() < 28.0, "flatten should pull peak toward 45mm, got {h_flat}");

    assert!(apply_heightfield_brush(
        &mut layout,
        SculptOp::PaintBiome(railbench_core::terrain::BIOME_WATER),
        Vec2 { x: -300.0, y: 200.0 },
        140.0,
        0.8,
        240.0,
        railbench_core::terrain::BIOME_WATER,
    ));
    assert_eq!(
        railbench_core::terrain::sample_heightfield_material(
            layout.terrain_field.as_ref().unwrap(),
            -300.0,
            200.0,
        ),
        railbench_core::terrain::BIOME_WATER
    );
    assert!(apply_heightfield_brush(
        &mut layout,
        SculptOp::Reset,
        Vec2 { x: -300.0, y: 200.0 },
        160.0,
        1.0,
        240.0,
        0,
    ));
}

#[test]
fn terrain_modifiers_only_affect_active_layer_and_respect_lock() {
    let mut layout = empty_test_layout();
    let bb = BaseboardConfig {
        id: Some("b1".into()),
        name: None,
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1200.0,
        depth: 800.0,
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
    layout.baseboard = Some(bb.clone());
    layout.baseboards = Some(vec![bb]);
    layout.layers = Some(vec![
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
                x: Some(400.0),
                y: Some(250.0),
                thickness_mm: Some(10.0),
                position_mode: Some("custom".into()),
                style: Some("wood".into()),
            }]),
            terrain_field: None,
        },
    ]);

    // Sculpt on layer-2 only affects layer-2 inside its 400x300 plate
    layout.active_layer_id = Some("layer-2".into());
    assert!(apply_heightfield_brush(
        &mut layout,
        SculptOp::Raise,
        Vec2 { x: 0.0, y: 0.0 },
        120.0,
        1.0,
        240.0,
        0,
    ));
    let layers = layout.layers.as_ref().unwrap();
    assert!(layers[0].terrain_field.is_none(), "layer-1 must stay untouched");
    let hf2 = layers[1].terrain_field.as_ref().expect("layer-2 heightfield created");
    assert!(
        railbench_core::terrain::sample_heightfield_bilinear(hf2, 0.0, 0.0) > 10.0,
        "center of layer-2 plate should rise"
    );
    // Outside layer-2 plate bounds (x=450 is on baseboard but off layer-2 400x300 plate)
    assert!(!apply_heightfield_brush(
        &mut layout,
        SculptOp::Raise,
        Vec2 { x: 450.0, y: 0.0 },
        40.0,
        1.0,
        240.0,
        0,
    ));

    // Switch back to layer-1 and sculpt at (-300, 200); layer-2 must remain unchanged
    let h2_before = railbench_core::terrain::sample_heightfield_bilinear(
        layout.layers.as_ref().unwrap()[1].terrain_field.as_ref().unwrap(),
        0.0,
        0.0,
    );
    layout.active_layer_id = Some("layer-1".into());
    assert!(apply_heightfield_brush(
        &mut layout,
        SculptOp::Raise,
        Vec2 { x: -300.0, y: 200.0 },
        120.0,
        1.0,
        240.0,
        0,
    ));
    let h2_after = railbench_core::terrain::sample_heightfield_bilinear(
        layout.layers.as_ref().unwrap()[1].terrain_field.as_ref().unwrap(),
        0.0,
        0.0,
    );
    assert!((h2_before - h2_after).abs() < 1e-4, "layer-2 must not change when layer-1 is edited");

    // Lock layer-2 and verify fill, smooth, clear, and sculpt return false cleanly
    layout.active_layer_id = Some("layer-2".into());
    layout.layers.as_mut().unwrap()[1].locked = true;
    assert!(!apply_heightfield_brush(
        &mut layout,
        SculptOp::Raise,
        Vec2 { x: 0.0, y: 0.0 },
        100.0,
        1.0,
        240.0,
        0,
    ));
    assert!(!fill_heightfield_biome(
        &mut layout,
        railbench_core::terrain::BIOME_ROCK,
    ));
    assert!(!smooth_entire_heightfield(&mut layout));
    assert!(!clear_heightfield(&mut layout));
}
