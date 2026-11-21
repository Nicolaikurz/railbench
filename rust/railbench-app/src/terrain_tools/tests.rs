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
