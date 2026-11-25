use super::*;
use railbench_core::types::{Elevation, Layout, Transform};

use crate::catalog::{builtin_catalogs, create_piece, find_entry};
use crate::viewer3d::{build_scene_mesh_3d, Scene3DOptions, Viewer3DEnv};

fn minimal_layout(pieces: Vec<TrackPiece>) -> Layout {
    Layout {
        name: "TunnelTest".into(),
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
    }
}

#[test]
fn east_west_tunnel_portal_cornice_and_wing_walls_align_with_headwall() {
    let prof = create_tunnel_profile(false, true, 16.5, 1, 0.0);
    let mut verts = Vec::new();
    let mut idxs = Vec::new();
    push_arched_tunnel_portal_3d(&mut verts, &mut idxs, [0.0, 0.0, 0.0], [1.0, 0.0], 0.0, &prof);

    assert!(!verts.is_empty());
    assert!(verts.iter().all(|v| v.pos.iter().all(|c| c.is_finite()) && v.normal_f32().iter().all(|c| c.is_finite())));

    // Top cornice must sit directly above headwall (+X), not mirrored to -X
    let max_y = verts.iter().map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);
    let cornice_verts: Vec<&Vertex3D> = verts.iter().filter(|v| v.pos[1] > max_y - 3.0).collect();
    let avg_x: f32 = cornice_verts.iter().map(|v| v.pos[0]).sum::<f32>() / cornice_verts.len() as f32;
    assert!(avg_x > 2.0, "cornice must center over headwall (+X), got avg_x={avg_x}");

    // Wing walls extend outward into approach cutting (-X)
    let min_x = verts.iter().map(|v| v.pos[0]).fold(f32::INFINITY, f32::min);
    assert!(min_x < -20.0, "wing walls must flare into approach cutting (-X), got min_x={min_x}");

    // Portal arch opening must stay 100% clear for trains + catenary
    let c_half = prof.inner_width * 0.5 - 2.0;
    let blocked = verts.iter().any(|v| {
        v.pos[2].abs() < c_half && v.pos[1] > 2.0 && v.pos[1] < prof.wall_h - 2.0 && v.pos[0] > -2.0 && v.pos[0] < 20.0
    });
    assert!(!blocked, "tunnel portal mouth must remain clear of blocking geometry");
}

#[test]
fn electrified_tunnel_clears_catenary_height_and_uses_ceiling_hangers() {
    let prof = create_tunnel_profile(false, true, 16.5, 1, 0.0);
    assert!(prof.inner_width >= 74.0);
    assert!(prof.total_h >= 112.0, "catenary vault must clear 92 mm masts, got {}", prof.total_h);

    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let mut t = create_piece("t0".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    t.has_tunnel = Some(true);
    t.has_catenary = Some(true);
    t.elevation = Some(Elevation { start: 0.0, end: 0.0 });

    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: false,
        ..Default::default()
    };
    let mesh = build_scene_mesh_3d(&minimal_layout(vec![t]), &catalogs, &opts);
    assert!(mesh.vertices.len() > 1500);
    assert!(mesh.vertices.iter().all(|v| v.pos.iter().all(|c| c.is_finite()) && v.normal_f32().iter().all(|c| c.is_finite())));
}

#[test]
fn road_tunnel_generates_concrete_portal_and_lighting() {
    let catalogs = builtin_catalogs();
    let road_entry = catalogs
        .iter()
        .filter(|c| c.is_road == Some(true))
        .flat_map(|c| c.entries.iter())
        .find(|e| e.kind == "straight" && e.length.unwrap_or(0.0) >= 150.0)
        .expect("road straight catalog entry");

    let mut r = create_piece("rt0".into(), road_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    r.has_tunnel = Some(true);

    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: false,
        ..Default::default()
    };
    let mesh = build_scene_mesh_3d(&minimal_layout(vec![r]), &catalogs, &opts);
    assert!(mesh.vertices.len() > 600);
    assert!(mesh.vertices.iter().any(|v| v.mat_f32()[2] > 0.5), "expected emissive highway tunnel LED lighting");
    assert!(mesh.vertices.iter().all(|v| v.pos.iter().all(|c| c.is_finite()) && v.normal_f32().iter().all(|c| c.is_finite())));
}
