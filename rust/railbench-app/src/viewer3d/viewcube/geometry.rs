use std::collections::HashMap;

use eframe::egui::{Color32, Pos2};

use super::super::{cross3, dot3, normalize3, Camera3D};

const CHAMFER_A: f32 = 0.64;
const CHAMFER_B: f32 = 0.87;
const PERSPECTIVE_CAM_DIST: f32 = 4.6 / 0.40;

pub(super) type EdgeKey = ((i32, i32, i32), (i32, i32, i32));

#[derive(Clone)]
pub(super) struct CubeFacet {
    pub(super) verts: Vec<[f32; 3]>,
    pub(super) normal: [f32; 3],
    pub(super) label: Option<&'static str>,
}

pub(super) struct VisibleFacet {
    pub(super) depth: f32,
    pub(super) nz: f32,
    pub(super) screen_area: f32,
    pub(super) world_verts: Vec<[f32; 3]>,
    pub(super) screen_poly: Vec<Pos2>,
    pub(super) normal: [f32; 3],
    pub(super) label: Option<&'static str>,
}

/// Builds a watertight 26-facet chamfered cube (6 octagons + 12 edge bevels + 8 corner hexagons)
/// with outward unit normals and counter-clockwise vertex winding.
pub(super) fn build_chamfered_cube_facets(is_de: bool) -> Vec<CubeFacet> {
    let mut facets = Vec::with_capacity(26);
    let a = CHAMFER_A;
    let b = CHAMFER_B;

    // 1. Six octagonal main faces (+Z, -Z, +X, -X, +Y, -Y)
    let oct_coords: [(f32, f32); 8] = [
        (b, a),
        (a, b),
        (-a, b),
        (-b, a),
        (-b, -a),
        (-a, -b),
        (a, -b),
        (b, -a),
    ];
    let main_faces: [([f32; 3], [f32; 3], [f32; 3], &str); 6] = [
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], if is_de { "VOR" } else { "FRONT" }),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], if is_de { "HIN" } else { "BACK" }),
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0], if is_de { "REC" } else { "RIGHT" }),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], if is_de { "LIN" } else { "LEFT" }),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0], if is_de { "OBN" } else { "TOP" }),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], if is_de { "UNT" } else { "BOT" }),
    ];
    for (n, u_ax, v_ax, lbl) in main_faces {
        let verts = oct_coords
            .iter()
            .map(|&(u, v)| {
                [
                    n[0] + u_ax[0] * u + v_ax[0] * v,
                    n[1] + u_ax[1] * u + v_ax[1] * v,
                    n[2] + u_ax[2] * u + v_ax[2] * v,
                ]
            })
            .collect();
        facets.push(CubeFacet {
            verts,
            normal: n,
            label: Some(lbl),
        });
    }

    // 2. Twelve rectangular edge bevels
    let basis = [[1.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for i in 0..3 {
        for j in (i + 1)..3 {
            let k = 3 - i - j;
            for &si in &[-1.0_f32, 1.0] {
                for &sj in &[-1.0_f32, 1.0] {
                    let n_raw = [
                        basis[i][0] * si + basis[j][0] * sj,
                        basis[i][1] * si + basis[j][1] * sj,
                        basis[i][2] * si + basis[j][2] * sj,
                    ];
                    let normal = normalize3(n_raw);
                    let mk = |ci: f32, cj: f32, ck: f32| -> [f32; 3] {
                        [
                            basis[i][0] * ci + basis[j][0] * cj + basis[k][0] * ck,
                            basis[i][1] * ci + basis[j][1] * cj + basis[k][1] * ck,
                            basis[i][2] * ci + basis[j][2] * cj + basis[k][2] * ck,
                        ]
                    };
                    let mut verts = vec![
                        mk(si, sj * b, -a),
                        mk(si, sj * b, a),
                        mk(si * b, sj, a),
                        mk(si * b, sj, -a),
                    ];
                    let e1 = [
                        verts[1][0] - verts[0][0],
                        verts[1][1] - verts[0][1],
                        verts[1][2] - verts[0][2],
                    ];
                    let e2 = [
                        verts[2][0] - verts[1][0],
                        verts[2][1] - verts[1][1],
                        verts[2][2] - verts[1][2],
                    ];
                    if dot3(cross3(e1, e2), normal) < 0.0 {
                        verts.reverse();
                    }
                    facets.push(CubeFacet {
                        verts,
                        normal,
                        label: None,
                    });
                }
            }
        }
    }

    // 3. Eight hexagonal corner bevels
    for &sx in &[-1.0_f32, 1.0] {
        for &sy in &[-1.0_f32, 1.0] {
            for &sz in &[-1.0_f32, 1.0] {
                let normal = normalize3([sx, sy, sz]);
                let mut verts = vec![
                    [sx * a, sy * b, sz],
                    [sx * b, sy * a, sz],
                    [sx, sy * a, sz * b],
                    [sx, sy * b, sz * a],
                    [sx * b, sy, sz * a],
                    [sx * a, sy, sz * b],
                ];
                if sx * sy * sz < 0.0 {
                    verts.reverse();
                }
                facets.push(CubeFacet {
                    verts,
                    normal,
                    label: None,
                });
            }
        }
    }

    facets
}

/// Checks whether `p` lies inside or on the boundary of convex 2D polygon `poly`.
pub(super) fn point_in_polygon_2d(p: Pos2, poly: &[Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut pos = false;
    let mut neg = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if cross > 1e-3 {
            pos = true;
        } else if cross < -1e-3 {
            neg = true;
        }
        if pos && neg {
            return false;
        }
    }
    pos || neg
}

/// Signed screen area of a projected front-facing CCW 3D polygon (`+Y` down in egui -> negated shoelace sum).
pub(super) fn screen_front_area_2d(poly: &[Pos2]) -> f32 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    -0.5 * sum
}

/// Quantizes a 3D cube vertex for exact undirected edge deduplication across adjacent facets.
pub(super) fn quantize_vert_key(v: [f32; 3]) -> (i32, i32, i32) {
    (
        (v[0] * 1000.0).round() as i32,
        (v[1] * 1000.0).round() as i32,
        (v[2] * 1000.0).round() as i32,
    )
}

/// Linearly interpolates between two RGB colors returning an opaque `Color32`.
pub(super) fn lerp_rgb_opaque(a: [f32; 3], b: [f32; 3], t: f32) -> Color32 {
    let k = t.clamp(0.0, 1.0);
    let r = a[0] + (b[0] - a[0]) * k;
    let g = a[1] + (b[1] - a[1]) * k;
    let bl = a[2] + (b[2] - a[2]) * k;
    Color32::from_rgb(
        r.clamp(0.0, 255.0).round() as u8,
        g.clamp(0.0, 255.0).round() as u8,
        bl.clamp(0.0, 255.0).round() as u8,
    )
}

/// Aligns `camera` so its eye direction matches the outward `normal` of the clicked ViewCube facet.
pub(super) fn apply_facet_normal_to_camera(camera: &mut Camera3D, normal: [f32; 3]) {
    let n = normalize3(normal);
    if n[0].hypot(n[2]) > 0.05 {
        camera.yaw = n[0].atan2(n[2]);
    }
    camera.pitch = n[1].clamp(-1.0, 1.0).asin().clamp(-1.48, 1.48);
}

pub(super) fn camera_basis(camera: &Camera3D) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let cp = camera.pitch.cos();
    let sp = camera.pitch.sin();
    let cy = camera.yaw.cos();
    let sy = camera.yaw.sin();
    let right = [cy, 0.0, -sy];
    let up = [-sp * sy, cp, -sp * cy];
    let view_z = [cp * sy, sp, cp * cy];
    (right, up, view_z)
}

pub(super) fn collect_visible_facets(
    facets: Vec<CubeFacet>,
    center: Pos2,
    camera: &Camera3D,
) -> Vec<VisibleFacet> {
    let (right, up, view_z) = camera_basis(camera);
    let project = |pt: [f32; 3]| -> (Pos2, f32) {
        let x_cam = dot3(pt, right);
        let y_cam = dot3(pt, up);
        let z_cam = dot3(pt, view_z);
        let scale = 30.5 * (PERSPECTIVE_CAM_DIST / (PERSPECTIVE_CAM_DIST - z_cam));
        (Pos2::new(center.x + x_cam * scale, center.y - y_cam * scale), z_cam)
    };

    let mut visible: Vec<VisibleFacet> = facets
        .into_iter()
        .filter_map(|f| {
            let nz = dot3(f.normal, view_z);
            let plane_d = dot3(f.verts[0], f.normal);
            // Exact perspective half-space test against camera eye at `PERSPECTIVE_CAM_DIST * view_z`.
            if PERSPECTIVE_CAM_DIST * nz - plane_d <= 1e-4 {
                return None;
            }
            let mut spts = Vec::with_capacity(f.verts.len());
            let mut z_sum = 0.0_f32;
            for v in &f.verts {
                let (sp, zv) = project(*v);
                spts.push(sp);
                z_sum += zv;
            }
            let screen_area = screen_front_area_2d(&spts);
            if screen_area <= 0.0 {
                return None;
            }
            let depth = z_sum / f.verts.len() as f32;
            Some(VisibleFacet {
                depth,
                nz,
                screen_area,
                world_verts: f.verts,
                screen_poly: spts,
                normal: f.normal,
                label: f.label,
            })
        })
        .collect();
    visible.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));
    visible
}

pub(super) fn dedup_visible_facet_edges(
    visible: &[VisibleFacet],
) -> Vec<(Pos2, Pos2, u8)> {
    let mut edge_map: HashMap<EdgeKey, usize> = HashMap::with_capacity(96);
    let mut edges: Vec<(Pos2, Pos2, u8)> = Vec::with_capacity(96);
    for f in visible {
        let n = f.world_verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let k0 = quantize_vert_key(f.world_verts[i]);
            let k1 = quantize_vert_key(f.world_verts[j]);
            let key = if k0 <= k1 { (k0, k1) } else { (k1, k0) };
            if let Some(&idx) = edge_map.get(&key) {
                edges[idx].2 = edges[idx].2.saturating_add(1);
            } else {
                let idx = edges.len();
                edge_map.insert(key, idx);
                edges.push((f.screen_poly[i], f.screen_poly[j], 1));
            }
        }
    }
    edges
}

/// Renders a single seamless welded triangle-mesh curved arrow arc plus crisp continuous perimeter.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chamfered_cube_has_26_watertight_coplanar_outward_facets() {
        let facets = build_chamfered_cube_facets(false);
        assert_eq!(facets.len(), 26);

        let oct_count = facets.iter().filter(|f| f.verts.len() == 8).count();
        let edge_count = facets.iter().filter(|f| f.verts.len() == 4).count();
        let corner_count = facets.iter().filter(|f| f.verts.len() == 6).count();
        assert_eq!(oct_count, 6);
        assert_eq!(edge_count, 12);
        assert_eq!(corner_count, 8);

        for f in &facets {
            let n_len = (f.normal[0] * f.normal[0] + f.normal[1] * f.normal[1] + f.normal[2] * f.normal[2]).sqrt();
            assert!((n_len - 1.0).abs() < 1e-4);

            // Coplanarity & outward offset check
            let d0 = dot3(f.verts[0], f.normal);
            assert!(d0 > 0.5, "facet plane must lie outside origin");
            for v in &f.verts[1..] {
                let d = dot3(*v, f.normal);
                assert!((d - d0).abs() < 1e-4, "vertices must be coplanar");
            }

            // Counter-clockwise outward winding check
            let e1 = [
                f.verts[1][0] - f.verts[0][0],
                f.verts[1][1] - f.verts[0][1],
                f.verts[1][2] - f.verts[0][2],
            ];
            let e2 = [
                f.verts[2][0] - f.verts[1][0],
                f.verts[2][1] - f.verts[1][1],
                f.verts[2][2] - f.verts[1][2],
            ];
            assert!(
                dot3(cross3(e1, e2), f.normal) > 0.0,
                "facet winding must be CCW outward"
            );
        }
    }

    #[test]
    fn point_in_polygon_2d_classifies_inside_and_outside_correctly() {
        let oct = vec![
            Pos2::new(10.0, 0.0),
            Pos2::new(20.0, 0.0),
            Pos2::new(30.0, 10.0),
            Pos2::new(30.0, 20.0),
            Pos2::new(20.0, 30.0),
            Pos2::new(10.0, 30.0),
            Pos2::new(0.0, 20.0),
            Pos2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon_2d(Pos2::new(15.0, 15.0), &oct));
        assert!(!point_in_polygon_2d(Pos2::new(1.0, 1.0), &oct));
        assert!(!point_in_polygon_2d(Pos2::new(40.0, 15.0), &oct));
    }

    #[test]
    fn apply_facet_normal_aligns_camera_eye_direction_with_facet_normal() {
        let cases: [([f32; 3], f32, f32); 6] = [
            ([0.0, 0.0, 1.0], 0.0, 0.0),
            ([1.0, 0.0, 0.0], std::f32::consts::FRAC_PI_2, 0.0),
            ([-1.0, 0.0, 0.0], -std::f32::consts::FRAC_PI_2, 0.0),
            ([0.0, 0.0, -1.0], std::f32::consts::PI, 0.0),
            ([0.0, 1.0, 0.0], 0.62, 1.48),
            ([1.0, 1.0, 1.0], std::f32::consts::FRAC_PI_4, (1.0_f32 / 3.0_f32.sqrt()).asin()),
        ];
        for (normal, exp_yaw, exp_pitch) in cases {
            let mut cam = Camera3D::default();
            apply_facet_normal_to_camera(&mut cam, normal);
            assert!((cam.yaw - exp_yaw).abs() < 1e-4, "yaw mismatch for {normal:?}");
            assert!((cam.pitch - exp_pitch).abs() < 1e-4, "pitch mismatch for {normal:?}");
        }
    }

    #[test]
    fn screen_front_area_and_edge_dedup_separate_silhouette_from_creases() {
        let mut cam = Camera3D::default();
        cam.yaw = 0.62;
        cam.pitch = 0.58;
        let center = Pos2::new(100.0, 100.0);
        let visible = collect_visible_facets(build_chamfered_cube_facets(false), center, &cam);
        assert!(!visible.is_empty());
        for f in &visible {
            assert!(
                screen_front_area_2d(&f.screen_poly) > 0.0,
                "every kept facet must have positive front-facing screen area"
            );
        }

        let edges = dedup_visible_facet_edges(&visible);
        let silhouette_count = edges.iter().filter(|e| e.2 == 1).count();
        let crease_count = edges.iter().filter(|e| e.2 == 2).count();
        let over_shared = edges.iter().filter(|e| e.2 > 2).count();
        assert_eq!(over_shared, 0, "chamfered cube is manifold: no edge shared >2 times");
        assert!(silhouette_count > 0, "expected non-empty outer silhouette edges");
        assert!(crease_count > 0, "expected non-empty internal crease edges");
    }

    #[test]
    fn viewcube_silhouette_is_watertight_across_all_rotations() {
        let center = Pos2::new(100.0, 100.0);
        let base_facets = build_chamfered_cube_facets(false);
        let mut angles = Vec::new();
        for yi in 0..60 {
            let yaw = (yi as f32 * 6.0).to_radians();
            for pi in -14..=14 {
                let pitch = (pi as f32 * 6.0).to_radians();
                angles.push((yaw, pitch));
            }
        }
        // Include exact cardinal & grazing bevel angles near perspective horizon
        let horizon = (1.0_f32 / PERSPECTIVE_CAM_DIST).asin();
        for y_deg in [0.0_f32, 45.0, 90.0, 135.0, 180.0] {
            for p_rad in [0.0, horizon, -horizon, horizon + 0.002, horizon - 0.002, 1.48, -1.48] {
                angles.push((y_deg.to_radians(), p_rad));
            }
        }

        for (yaw, pitch) in angles {
            let mut cam = Camera3D::default();
            cam.yaw = yaw;
            cam.pitch = pitch;
            let visible = collect_visible_facets(base_facets.clone(), center, &cam);
            assert!(!visible.is_empty());

            let mut edge_counts: HashMap<EdgeKey, u8> = HashMap::with_capacity(96);
            for f in &visible {
                assert!(f.screen_area > 0.0, "backfacing facet leaked at yaw={yaw} pitch={pitch}");
                let n = f.world_verts.len();
                for i in 0..n {
                    let k0 = quantize_vert_key(f.world_verts[i]);
                    let k1 = quantize_vert_key(f.world_verts[(i + 1) % n]);
                    let key = if k0 <= k1 { (k0, k1) } else { (k1, k0) };
                    let c = edge_counts.entry(key).or_insert(0);
                    *c = c.saturating_add(1);
                }
            }

            let mut adj: HashMap<(i32, i32, i32), Vec<(i32, i32, i32)>> = HashMap::new();
            for ((k0, k1), cnt) in edge_counts {
                assert!(cnt <= 2, "edge shared {cnt} times at yaw={yaw} pitch={pitch}");
                if cnt == 1 {
                    adj.entry(k0).or_default().push(k1);
                    adj.entry(k1).or_default().push(k0);
                }
            }
            assert!(!adj.is_empty(), "empty silhouette at yaw={yaw} pitch={pitch}");
            for (vk, nbrs) in &adj {
                assert_eq!(
                    nbrs.len(),
                    2,
                    "broken silhouette vertex {vk:?} (deg={}) at yaw={yaw} pitch={pitch}",
                    nbrs.len()
                );
            }
            // Verify single connected silhouette loop
            let start = *adj.keys().next().unwrap();
            let mut visited = std::collections::HashSet::with_capacity(adj.len());
            let mut stack = vec![start];
            while let Some(cur) = stack.pop() {
                if visited.insert(cur) {
                    for &nxt in &adj[&cur] {
                        if !visited.contains(&nxt) {
                            stack.push(nxt);
                        }
                    }
                }
            }
            assert_eq!(
                visited.len(),
                adj.len(),
                "disconnected silhouette components at yaw={yaw} pitch={pitch}"
            );
        }
    }

    #[test]
    fn viewcube_renders_cleanly_in_light_and_dark_mode() {
        use eframe::egui::{self, Rect};
        use super::super::draw_viewcube_widget;
        let vp = Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0));
        let layout = crate::make_empty_layout();
        for &(dark_mode, is_de) in &[(false, false), (true, true)] {
            let ctx = egui::Context::default();
            let mut cam = Camera3D::default();
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(vp);
            let _ = ctx.run(raw, |c| {
                egui::CentralPanel::default().show(c, |ui| {
                    draw_viewcube_widget(ui, vp, 68.0, &mut cam, &layout, is_de, dark_mode);
                });
            });
            let cube_rect = ctx
                .memory(|m| m.area_rect(egui::Id::new("hud_3d_viewcube")))
                .expect("viewcube area rect should be recorded");
            assert!(vp.contains_rect(cube_rect), "viewcube rect {cube_rect:?} should fit in viewport {vp:?}");
        }
    }
}
