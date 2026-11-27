use super::*;
use crate::model_tools::BUILTIN_MODELS;
use railbench_core::types::Transform;

fn mk_model(id: Option<&str>, w: f64, d: f64, h: f64) -> Model3D {
    Model3D {
        id: "test-m".into(),
        name: id.unwrap_or("generic").into(),
        category: "railway".into(),
        transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: w,
        depth: d,
        height: h,
        model_type: "builtin".into(),
        builtin_id: id.map(|s| s.to_string()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    }
}

#[test]
fn all_16_builtin_models_generate_valid_detailed_meshes() {
    for def in BUILTIN_MODELS {
        let m = mk_model(Some(def.id), def.width, def.depth, def.height);
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);

        assert!(verts.len() >= 100, "{} vertex count too small: {}", def.id, verts.len());
        assert!(idxs.len() >= 150 && idxs.len() % 3 == 0, "{} index count invalid: {}", def.id, idxs.len());
        let max_i = *idxs.iter().max().unwrap_or(&0) as usize;
        assert!(max_i < verts.len(), "{} out-of-bounds index {} >= {}", def.id, max_i, verts.len());

        for v in &verts {
            let vn = v.normal_f32();
            let vc = v.color_f32();
            let vm = v.mat_f32();
            assert!(v.pos.iter().all(|x| x.is_finite()), "{} non-finite pos", def.id);
            assert!(vn.iter().all(|x| x.is_finite()), "{} non-finite normal", def.id);
            assert!(vc.iter().all(|x| x.is_finite() && *x >= 0.0 && *x <= 1.0), "{} invalid color", def.id);
            let nlen = (vn[0] * vn[0] + vn[1] * vn[1] + vn[2] * vn[2]).sqrt();
            assert!((0.94..=1.06).contains(&nlen), "{} unnormalized normal len {}", def.id, nlen);
            assert!((0.03..=1.0).contains(&vm[0]), "{} bad roughness {}", def.id, vm[0]);
            assert!((0.0..=1.0).contains(&vm[1]), "{} bad metallic {}", def.id, vm[1]);
        }
    }

    // Alias & fallback check
    for id in [Some("bahnsteig"), None] {
        let m = mk_model(id, 100.0, 60.0, 80.0);
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);
        assert!(verts.len() >= 100 && !idxs.is_empty());
    }
}

#[test]
fn lit_models_emit_emissive_and_night_window_materials() {
    let ids = [
        "station",
        "town-house",
        "signal-box",
        "platform-lamp",
        "light-signal",
        "skyscraper",
        "platform",
        "diesel-fuel-station",
        "engine-shed",
        "turntable-trainhall",
    ];
    for id in ids {
        let def = crate::model_tools::find_builtin_def(id).unwrap();
        let m = mk_model(Some(id), def.width, def.depth, def.height);
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);
        assert!(
            verts.iter().any(|v| v.mat_f32()[2].abs() > 0.01),
            "{} must emit emissive or night-glow window vertices",
            id
        );
    }
}

#[test]
fn curved_models_have_non_axis_aligned_normals() {
    for id in ["water-tower", "pine-tree", "deciduous-tree", "diesel-fuel-station", "platform-lamp", "industry-building"] {
        let def = crate::model_tools::find_builtin_def(id).unwrap();
        let m = mk_model(Some(id), def.width, def.depth, def.height);
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);
        let oblique_normals = verts
            .iter()
            .filter(|v| {
                let vn = v.normal_f32();
                let ax = vn[0].abs();
                let az = vn[2].abs();
                ax > 0.15 && az > 0.15
            })
            .count();
        assert!(oblique_normals >= 20, "{} should have smooth curved normals, found {}", id, oblique_normals);
    }
}

#[test]
fn engine_shed_and_roundhouse_preserve_open_track_doorways() {
    // Engine shed: clear opening at x = -120 across y in [12..82] (including catenary!), z in [-14..14]
    let shed = mk_model(Some("engine-shed"), 240.0, 72.0, 105.0);
    let mut s_verts = Vec::new();
    let mut s_idxs = Vec::new();
    push_builtin_model_3d(&mut s_verts, &mut s_idxs, &shed, 0.0);
    for tri in s_idxs.chunks_exact(3) {
        let p = [s_verts[tri[0] as usize].pos, s_verts[tri[1] as usize].pos, s_verts[tri[2] as usize].pos];
        let cx = (p[0][0] + p[1][0] + p[2][0]) / 3.0;
        let cy = (p[0][1] + p[1][1] + p[2][1]) / 3.0;
        let cz = (p[0][2] + p[1][2] + p[2][2]) / 3.0;
        assert!(
            !(cx < -114.0 && (12.0..=82.0).contains(&cy) && cz.abs() < 15.0),
            "engine-shed doorway blocked at ({cx:.1}, {cy:.1}, {cz:.1})"
        );
    }

    // Turntable trainhall: 3 open portals at (-40.54, -159.26), (0, -155), (+40.54, -159.26)
    let hall = mk_model(Some("turntable-trainhall"), 320.0, 310.0, 115.0);
    let mut h_verts = Vec::new();
    let mut h_idxs = Vec::new();
    push_builtin_model_3d(&mut h_verts, &mut h_idxs, &hall, 0.0);
    let min_z = h_verts.iter().map(|v| v.pos[2]).fold(f32::INFINITY, f32::min);
    let max_z = h_verts.iter().map(|v| v.pos[2]).fold(f32::NEG_INFINITY, f32::max);
    assert!(min_z <= -155.0 && max_z >= 150.0, "unexpected roundhouse z span: {min_z}..{max_z}");
    for &(px, pz) in &[(-40.54_f32, -159.26_f32), (0.0, -155.0), (40.54, -159.26)] {
        for tri in h_idxs.chunks_exact(3) {
            let p = [h_verts[tri[0] as usize].pos, h_verts[tri[1] as usize].pos, h_verts[tri[2] as usize].pos];
            let cx = (p[0][0] + p[1][0] + p[2][0]) / 3.0;
            let cy = (p[0][1] + p[1][1] + p[2][1]) / 3.0;
            let cz = (p[0][2] + p[1][2] + p[2][2]) / 3.0;
            assert!(
                !((cx - px).abs() < 8.0 && (cz - pz).abs() < 6.0 && (12.0..=82.0).contains(&cy)),
                "roundhouse stall ({px}, {pz}) doorway blocked at ({cx:.1}, {cy:.1}, {cz:.1})"
            );
        }
    }
}

#[test]
fn platform_lamp_neck_connects_to_shade() {
    let lamp = mk_model(Some("platform-lamp"), 20.0, 20.0, 70.0);
    let mut verts = Vec::new();
    let mut idxs = Vec::new();
    push_builtin_model_3d(&mut verts, &mut idxs, &lamp, 0.0);
    assert!(
        verts.iter().any(|v| (10.0..=14.0).contains(&v.pos[0]) && v.pos[1] >= 65.5),
        "platform-lamp swan neck must arch seamlessly across x=10..14 above the shade"
    );
}

#[test]
fn roundhouse_and_water_tower_radial_alignment() {
    let hall = mk_model(Some("turntable-trainhall"), 320.0, 310.0, 115.0);
    let mut h_verts = Vec::new();
    let mut h_idxs = Vec::new();
    push_builtin_model_3d(&mut h_verts, &mut h_idxs, &hall, 0.0);
    for v in &h_verts {
        let dz = v.pos[2] - (-350.0);
        assert!(dz > 140.0, "roundhouse vertex behind turntable front: z={}", v.pos[2]);
        let ray_deg = v.pos[0].abs().atan2(dz).to_degrees();
        assert!(
            ray_deg <= 20.5,
            "roundhouse vertex twisted outside ±18 deg fan: ({:.1}, {:.1}) -> {ray_deg:.1} deg",
            v.pos[0],
            v.pos[2]
        );
    }
    assert!(
        h_verts.iter().any(|v| v.mat_f32()[1] > 0.7 && (v.pos[1] - 9.6).abs() < 0.25 && v.pos[2].abs() < 120.0),
        "roundhouse interior rails must sit at y = 9.6 mm matching turntable exit"
    );

    let tower = mk_model(Some("water-tower"), 85.0, 85.0, 170.0);
    let mut t_verts = Vec::new();
    let mut t_idxs = Vec::new();
    push_builtin_model_3d(&mut t_verts, &mut t_idxs, &tower, 0.0);
    for v in &t_verts {
        let vm = v.mat_f32();
        if (109.0..=117.0).contains(&v.pos[1]) && vm[2] < -0.1 {
            let r = v.pos[0].hypot(v.pos[2]);
            assert!(
                r <= 40.8,
                "water-tower tank window corner protruding radially at y={:.1}: r={r:.2}",
                v.pos[1]
            );
        }
        if (157.2..=160.8).contains(&v.pos[1]) && vm[2] >= -0.01 {
            let r = v.pos[0].hypot(v.pos[2]);
            assert!(r <= 9.6, "water-tower cupola solid ring blocks lantern glass at y={:.1}, r={r:.2}", v.pos[1]);
        }
    }
    for tri in t_idxs.chunks_exact(3) {
        let vs = [&t_verts[tri[0] as usize], &t_verts[tri[1] as usize], &t_verts[tri[2] as usize]];
        if vs[0].mat_f32()[2] >= -0.01 {
            let min_y = vs.iter().map(|v| v.pos[1]).fold(f32::INFINITY, f32::min);
            let max_y = vs.iter().map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);
            let min_r = vs.iter().map(|v| v.pos[0].hypot(v.pos[2])).fold(f32::INFINITY, f32::min);
            assert!(
                !(min_y < 156.0 && max_y > 162.0 && min_r > 9.3),
                "water-tower opaque cylinder spans across cupola glass at r={min_r:.2}"
            );
        }
    }
    assert!(
        t_verts
            .iter()
            .any(|v| (156.2..=161.8).contains(&v.pos[1]) && v.mat_f32()[2] < -0.1 && v.pos[0].hypot(v.pos[2]) >= 8.3),
        "water-tower cupola glowing lantern glass must be exposed"
    );
}

#[test]
fn platform_has_open_stairwell_tapered_ramps_and_clear_clock() {
    let plat = mk_model(Some("platform"), 320.0, 54.0, 55.0);
    let mut verts = Vec::new();
    let mut idxs = Vec::new();
    push_builtin_model_3d(&mut verts, &mut idxs, &plat, 0.0);
    for v in &verts {
        if v.pos[0].abs() > 143.0 {
            assert!(
                v.pos[1] <= 9.5,
                "platform ramp has elevated geometry at x={:.1}, y={:.2}",
                v.pos[0],
                v.pos[1]
            );
        }
    }
    for tri in idxs.chunks_exact(3) {
        let p = [verts[tri[0] as usize].pos, verts[tri[1] as usize].pos, verts[tri[2] as usize].pos];
        let n = verts[tri[0] as usize].normal_f32();
        let cx = (p[0][0] + p[1][0] + p[2][0]) / 3.0;
        let cy = (p[0][1] + p[1][1] + p[2][1]) / 3.0;
        let cz = (p[0][2] + p[1][2] + p[2][2]) / 3.0;
        if n[1] > 0.9 && cx.abs() < 12.0 && cz.abs() < 7.0 {
            assert!(
                !(9.85..=10.15).contains(&cy),
                "platform deck slab seals underpass stairwell at ({cx:.1}, {cy:.1}, {cz:.1})"
            );
        }
    }
    assert!(
        verts.iter().any(|v| (19.0..=26.0).contains(&v.pos[0]) && (32.0..=39.0).contains(&v.pos[1]) && v.pos[2].abs() < 3.0),
        "platform double-sided clock should sit clear at x = 22.5"
    );
}

#[test]
fn town_house_beams_do_not_cross_windows() {
    let house = mk_model(Some("town-house"), 110.0, 90.0, 125.0);
    let mut verts = Vec::new();
    let mut idxs = Vec::new();
    push_builtin_model_3d(&mut verts, &mut idxs, &house, 0.0);
    for tri in idxs.chunks_exact(3) {
        let v0 = &verts[tri[0] as usize];
        let v0_m = v0.mat_f32();
        let v0_c = v0.color_f32();
        let is_dark_beam = v0_m[2].abs() < 0.01 && v0_c[0] < 0.28 && v0_c[1] < 0.12 && v0_c[2] < 0.04;
        if !is_dark_beam {
            continue;
        }
        let p = [v0.pos, verts[tri[1] as usize].pos, verts[tri[2] as usize].pos];
        let cx = (p[0][0] + p[1][0] + p[2][0]) / 3.0;
        let cy = (p[0][1] + p[1][1] + p[2][1]) / 3.0;
        let cz = (p[0][2] + p[1][2] + p[2][2]) / 3.0;
        if cz.abs() > 41.0 && (53.5..=64.5).contains(&cy) {
            for &wx in &[-36.0_f32, 0.0, 36.0] {
                assert!(
                    (cx - wx).abs() >= 3.5,
                    "town-house timber beam crosses upper window {wx} at ({cx:.1}, {cy:.1}, {cz:.1})"
                );
            }
        }
    }
}

#[test]
fn engine_shed_and_roundhouse_emit_transparent_roofs_in_3d() {
    for (id, w, d, h) in [
        ("engine-shed", 240.0, 72.0, 105.0),
        ("turntable-trainhall", 320.0, 310.0, 115.0),
    ] {
        let m = mk_model(Some(id), w, d, h);
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        let mut glass_idxs = Vec::new();
        push_builtin_model_3d_with_glass(&mut verts, &mut idxs, Some(&mut glass_idxs), &m, 0.0);

        assert!(!glass_idxs.is_empty(), "{id} should emit transparent roof glass indices");
        assert_eq!(glass_idxs.len() % 3, 0, "{id} glass_idxs must be triangle list");
        for &gi in &glass_idxs {
            let v = &verts[gi as usize];
            assert!(
                v.color[3] < 128,
                "{id} roof glass vertex alpha should be translucent (<128), got {}",
                v.color[3]
            );
            assert!(
                v.pos[1] >= 95.0,
                "{id} glass vertex should sit at roof height, got y={}",
                v.pos[1]
            );
        }

        if id == "engine-shed" {
            // Confirm no wide opaque roof triangles seal the interior track bay
            for tri in idxs.chunks_exact(3) {
                let p = [verts[tri[0] as usize].pos, verts[tri[1] as usize].pos, verts[tri[2] as usize].pos];
                let min_y = p.iter().map(|v| v[1]).fold(f32::INFINITY, f32::min);
                let span_x = p.iter().map(|v| v[0]).fold(f32::NEG_INFINITY, f32::max)
                    - p.iter().map(|v| v[0]).fold(f32::INFINITY, f32::min);
                let span_z = p.iter().map(|v| v[2]).fold(f32::NEG_INFINITY, f32::max)
                    - p.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
                assert!(
                    !(min_y >= 96.5 && span_x > 25.0 && span_z > 12.0),
                    "engine-shed opaque triangle spans roof above tracks: y>={min_y:.1}, dx={span_x:.1}, dz={span_z:.1}"
                );
            }
        }
    }
}
