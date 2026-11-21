use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{Elevation, Layer, Layout, Transform};

use crate::catalog::{builtin_catalogs, create_piece, find_entry, is_road_id};
use crate::viewer3d::{build_scene_mesh_3d, Scene3DOptions, SceneMesh3D, Viewer3DEnv};

trait MeshTestUtil {
    fn total_vert_count(&self) -> usize;
}

impl MeshTestUtil for SceneMesh3D {
    fn total_vert_count(&self) -> usize {
        self.vertices.len()
            + self
                .instanced_tracks
                .iter()
                .map(|p| p.vertices.len() * p.instances.len())
                .sum::<usize>()
    }
}

fn base_layout(pieces: Vec<railbench_core::types::TrackPiece>, layers: Option<Vec<Layer>>) -> Layout {
    Layout {
        name: "BridgeTest".into(),
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
        layers,
        active_layer_id: None,
        baseboard: None,
        baseboards: None,
    }
}

fn minimal_opts() -> Scene3DOptions {
    Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: false,
        ..Default::default()
    }
}

#[test]
fn straight_railway_bridge_generates_warren_truss_piers_and_abutments() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24188").expect("24188 present");
    let plain = create_piece("p0".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let mut bridged = create_piece("p1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    bridged.elevation = Some(Elevation { start: 85.0, end: 85.0 });
    bridged.has_bridge = Some(true);

    let opts = minimal_opts();
    let m_plain = build_scene_mesh_3d(&base_layout(vec![plain], None), &catalogs, &opts);
    let m_bridge = build_scene_mesh_3d(&base_layout(vec![bridged], None), &catalogs, &opts);

    assert!(!m_bridge.vertices.is_empty());
    assert!(m_bridge.total_vert_count() > m_plain.total_vert_count() + 1000);
    assert!(m_bridge.vertices.iter().all(|v| {
        v.pos.iter().all(|c| c.is_finite()) && v.normal_f32().iter().all(|c| c.is_finite())
    }));

    let max_y = m_bridge.vertices.iter().map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);
    let min_y = m_bridge.vertices.iter().map(|v| v.pos[1]).fold(f32::INFINITY, f32::min);
    assert!(max_y > 165.0, "expected Warren truss top chord above y=165, got {max_y}");
    assert!(min_y <= 0.0, "expected stone piers/abutments grounded to y<=0, got {min_y}");
}

#[test]
fn curved_railway_bridge_generates_plate_girder_viaduct_without_truss() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24130").expect("24130 present");
    let plain = create_piece("c0".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let mut bridged = create_piece("c1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    bridged.elevation = Some(Elevation { start: 85.0, end: 85.0 });
    bridged.has_bridge = Some(true);

    let opts = minimal_opts();
    let m_plain = build_scene_mesh_3d(&base_layout(vec![plain], None), &catalogs, &opts);
    let m_bridge = build_scene_mesh_3d(&base_layout(vec![bridged], None), &catalogs, &opts);

    assert!(m_bridge.total_vert_count() > m_plain.total_vert_count() + 600);
    let max_y = m_bridge.vertices.iter().map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);
    let min_y = m_bridge.vertices.iter().map(|v| v.pos[1]).fold(f32::INFINITY, f32::min);
    assert!(
        max_y > 95.0 && max_y < 125.0,
        "curved plate-girder bridge should stay below truss height: got max_y={max_y}"
    );
    assert!(min_y <= 0.0, "curved viaduct pier/abutments should reach ground: min_y={min_y}");
}

#[test]
fn bridge_pier_collision_avoidance_clears_lower_crossing_track() {
    let catalogs = builtin_catalogs();
    let e188 = find_entry(&catalogs, "24188").expect("24188 present");
    let e360 = find_entry(&catalogs, "24360").expect("24360 present");

    // Lower track along Z passing right underneath the midpoint of the upper bridge (x = 94.15)
    let lower = create_piece(
        "lower".into(),
        e360,
        Transform { x: 94.15, y: -180.0, rotation: 90.0 },
        Some("layer-1".into()),
    );
    // Upper 24188 bridge spanning x = 0..188.3 at y_elev = 85 mm on layer-2
    let mut upper = create_piece(
        "upper".into(),
        e188,
        Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        Some("layer-2".into()),
    );
    upper.elevation = Some(Elevation { start: 85.0, end: 85.0 });
    upper.has_bridge = Some(true);

    let layers = Some(vec![
        Layer {
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
        },
        Layer {
            id: "layer-2".into(),
            name: "Viaduct".into(),
            color: None,
            visible: true,
            locked: false,
            height_mm: Some(0.0),
            wood_plate_mode: None,
            wood_plate_thickness_mm: None,
            wood_plate_style: None,
            plates: None,
        },
    ]);

    let mesh = build_scene_mesh_3d(&base_layout(vec![lower, upper], layers), &catalogs, &minimal_opts());
    // Check non-instanced vertices (the upper bridge) have NO pier obstructing lower track at (94.15, 0)
    let blocked = mesh.vertices.iter().any(|v| {
        (v.pos[0] - 94.15).abs() < 18.0
            && v.pos[2].abs() < 18.0
            && v.pos[1] > 12.0
            && v.pos[1] < 65.0
    });
    assert!(!blocked, "bridge pier dropped onto crossing lower track at x=94.15, z=0!");
}

#[test]
fn connected_straight_bridges_omit_interior_abutments() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24188").expect("24188 present");
    let mut s1 = create_piece("s1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    s1.elevation = Some(Elevation { start: 85.0, end: 85.0 });
    s1.has_bridge = Some(true);

    let mut s2 = create_piece("s2".into(), entry, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
    s2.elevation = Some(Elevation { start: 85.0, end: 85.0 });
    s2.has_bridge = Some(true);

    let mut layout = base_layout(vec![s1, s2], None);
    update_layout_connections(&mut layout, 8.0, 20.0, 1.0);

    let mesh = build_scene_mesh_3d(&layout, &catalogs, &minimal_opts());
    // Mid-joint at x = 188.3 must have no abutment wall below deck level
    let interior_wall = mesh.vertices.iter().any(|v| {
        (v.pos[0] - 188.3).abs() < 5.0
            && v.pos[2].abs() < 18.0
            && v.pos[1] > 10.0
            && v.pos[1] < 70.0
    });
    assert!(!interior_wall, "connected bridge spans must omit interior abutments at shared joint");
}

#[test]
fn road_bridge_generates_concrete_deck_guardrails_and_piers() {
    let catalogs = builtin_catalogs();
    let road_entry = catalogs
        .iter()
        .filter(|c| c.is_road == Some(true))
        .flat_map(|c| c.entries.iter())
        .find(|e| e.kind == "straight" && e.length.unwrap_or(0.0) >= 150.0)
        .expect("road straight catalog entry");
    assert!(is_road_id(&catalogs, &road_entry.id));

    let plain = create_piece("r0".into(), road_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let mut bridged = create_piece("r1".into(), road_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    bridged.elevation = Some(Elevation { start: 85.0, end: 85.0 });
    bridged.has_bridge = Some(true);

    let opts = minimal_opts();
    let m_plain = build_scene_mesh_3d(&base_layout(vec![plain], None), &catalogs, &opts);
    let m_bridge = build_scene_mesh_3d(&base_layout(vec![bridged], None), &catalogs, &opts);

    assert!(m_bridge.total_vert_count() > m_plain.total_vert_count() + 400);
    let min_y = m_bridge.vertices.iter().map(|v| v.pos[1]).fold(f32::INFINITY, f32::min);
    let max_y = m_bridge.vertices.iter().map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);
    assert!(min_y <= 0.0, "road bridge piers/abutments must ground to y<=0: min_y={min_y}");
    assert!(max_y > 96.0, "road bridge guardrails should rise above deck y=85: max_y={max_y}");
}
