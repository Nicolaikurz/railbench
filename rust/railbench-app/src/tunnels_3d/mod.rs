use railbench_core::geometry::build_tunnel_sweep_positions;
use railbench_core::types::{CatalogEntry, Layout, TrackPiece};

use crate::terrain_tools::{
    create_tunnel_profile, is_point_inside_any_mountain, tunnel_profile_points, TunnelProfile,
};
use crate::viewer3d::{
    cross3, dot3, normalize3, push_quad_mat, push_tri, sample_track_branches_3d, Vertex3D,
    MAT_DEFAULT,
};

// Material descriptors (roughness <= 0.92 so WGSL gravel micro-texture stays on ballast only)
const MAT_STONE: [f32; 3] = [0.88, 0.02, 0.0];
const MAT_CONCRETE: [f32; 3] = [0.84, 0.02, 0.0];
const MAT_TUNNEL_TERRAIN: [f32; 3] = [0.91, 0.0, 0.0];
const MAT_STEEL_DARK: [f32; 3] = [0.42, 0.65, 0.0];
const MAT_PORCELAIN: [f32; 3] = [0.26, 0.05, 0.0];
const MAT_WARM_SCONCE: [f32; 3] = [0.25, 0.05, -0.82];
const MAT_ROAD_LED: [f32; 3] = [0.20, 0.05, 0.85];

// Dressed stone & modern concrete color palette
const STONE_MAIN: [f32; 3] = [0.44, 0.43, 0.42];
const STONE_DARK: [f32; 3] = [0.33, 0.33, 0.34];
const STONE_LIGHT: [f32; 3] = [0.58, 0.56, 0.53];
const STONE_VOUSSOIR_A: [f32; 3] = [0.53, 0.51, 0.48];
const STONE_VOUSSOIR_B: [f32; 3] = [0.46, 0.45, 0.43];
const STONE_PLAQUE: [f32; 3] = [0.19, 0.20, 0.22];
const LINING_REVEAL_RAIL: [f32; 3] = [0.22, 0.23, 0.25];
const LINING_REVEAL_ROAD: [f32; 3] = [0.38, 0.40, 0.43];

const CONCRETE_PORTAL: [f32; 3] = [0.60, 0.62, 0.64];
const CONCRETE_TRIM: [f32; 3] = [0.70, 0.72, 0.74];
const CONCRETE_DARK: [f32; 3] = [0.45, 0.47, 0.50];

// ============================================================================
//  Portal-Local Orthonormal Coordinate Frame
// ============================================================================

struct PortalFrame {
    origin: [f32; 3],
    right: [f32; 2], // local +X (transverse right when facing into tunnel)
    fwd: [f32; 2],   // local +Z (inward into bore; -Z = outward approach cutting)
}

impl PortalFrame {
    fn new(world_pos: [f32; 3], inward_dir_xz: [f32; 2]) -> Self {
        let len = inward_dir_xz[0].hypot(inward_dir_xz[1]).max(1e-6);
        let fwd = [inward_dir_xz[0] / len, inward_dir_xz[1] / len];
        let right = [fwd[1], -fwd[0]];
        Self {
            origin: world_pos,
            right,
            fwd,
        }
    }

    fn to_world(&self, lx: f32, ly: f32, lz: f32) -> [f32; 3] {
        [
            self.origin[0] + self.right[0] * lx + self.fwd[0] * lz,
            self.origin[1] + ly,
            self.origin[2] + self.right[1] * lx + self.fwd[1] * lz,
        ]
    }

    fn to_norm(&self, nx: f32, ny: f32, nz: f32) -> [f32; 3] {
        normalize3([
            self.right[0] * nx + self.fwd[0] * nz,
            ny,
            self.right[1] * nx + self.fwd[1] * nz,
        ])
    }

    #[allow(clippy::too_many_arguments)]
    fn push_box(
        &self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
        x0: f32,
        x1: f32,
        y0: f32,
        y1: f32,
        z0: f32,
        z1: f32,
        color: [f32; 3],
        mat: [f32; 3],
    ) {
        let (ax0, ax1) = (x0.min(x1), x0.max(x1));
        let (ay0, ay1) = (y0.min(y1), y0.max(y1));
        let (az0, az1) = (z0.min(z1), z0.max(z1));
        let p = |x: f32, y: f32, z: f32| self.to_world(x, y, z);

        // Top (+Y), Bottom (-Y), Front (-Z), Back (+Z), Left (-X), Right (+X)
        push_quad_mat(verts, idxs, p(ax0, ay1, az0), p(ax1, ay1, az0), p(ax1, ay1, az1), p(ax0, ay1, az1), self.to_norm(0.0, 1.0, 0.0), color, mat);
        push_quad_mat(verts, idxs, p(ax0, ay0, az1), p(ax1, ay0, az1), p(ax1, ay0, az0), p(ax0, ay0, az0), self.to_norm(0.0, -1.0, 0.0), color, mat);
        push_quad_mat(verts, idxs, p(ax0, ay0, az0), p(ax1, ay0, az0), p(ax1, ay1, az0), p(ax0, ay1, az0), self.to_norm(0.0, 0.0, -1.0), color, mat);
        push_quad_mat(verts, idxs, p(ax1, ay0, az1), p(ax0, ay0, az1), p(ax0, ay1, az1), p(ax1, ay1, az1), self.to_norm(0.0, 0.0, 1.0), color, mat);
        push_quad_mat(verts, idxs, p(ax0, ay0, az1), p(ax0, ay0, az0), p(ax0, ay1, az0), p(ax0, ay1, az1), self.to_norm(-1.0, 0.0, 0.0), color, mat);
        push_quad_mat(verts, idxs, p(ax1, ay0, az0), p(ax1, ay0, az1), p(ax1, ay1, az1), p(ax1, ay1, az0), self.to_norm(1.0, 0.0, 0.0), color, mat);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_angled_box(
        &self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
        center_lx: f32,
        center_ly: f32,
        center_lz: f32,
        local_yaw: f32,
        size: [f32; 3],
        color: [f32; 3],
        mat: [f32; 3],
    ) {
        let hx = size[0] * 0.5;
        let hy = size[1] * 0.5;
        let hz = size[2] * 0.5;
        let c = local_yaw.cos();
        let s = local_yaw.sin();
        let pt = |x: f32, y: f32, z: f32| -> [f32; 3] {
            let lx = center_lx + x * c - z * s;
            let lz = center_lz + x * s + z * c;
            self.to_world(lx, center_ly + y, lz)
        };
        let rn = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
            self.to_norm(nx * c - nz * s, ny, nx * s + nz * c)
        };

        push_quad_mat(verts, idxs, pt(-hx, hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 1.0, 0.0), color, mat);
        push_quad_mat(verts, idxs, pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(-hx, -hy, -hz), rn(0.0, -1.0, 0.0), color, mat);
        push_quad_mat(verts, idxs, pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 0.0, 1.0), color, mat);
        push_quad_mat(verts, idxs, pt(hx, -hy, -hz), pt(-hx, -hy, -hz), pt(-hx, hy, -hz), pt(hx, hy, -hz), rn(0.0, 0.0, -1.0), color, mat);
        push_quad_mat(verts, idxs, pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), rn(1.0, 0.0, 0.0), color, mat);
        push_quad_mat(verts, idxs, pt(-hx, -hy, -hz), pt(-hx, -hy, hz), pt(-hx, hy, hz), pt(-hx, hy, -hz), rn(-1.0, 0.0, 0.0), color, mat);
    }
}

// ============================================================================
//  Public Entry Point 1: Arched Tunnel Portals (Railway Stone / Road Concrete)
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_piece_tunnel_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    is_road: bool,
    e0: f64,
    e1: f64,
    layout: &Layout,
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
    piece_map: &std::collections::HashMap<&str, &TrackPiece>,
) {
    let raw_branches = sample_track_branches_3d(piece, entry, e0, e1);
    let Some(main_br) = raw_branches.first().filter(|b| b.len() >= 2) else {
        return;
    };
    let mid = main_br[main_br.len() / 2];
    if is_point_inside_any_mountain(mid[0] as f64, mid[2] as f64, mid[1] as f64, layout, layer_info, 0.84) {
        return;
    }

    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5);
    let prof = create_tunnel_profile(is_road, piece.has_catenary == Some(true), gauge, 1, 0.0);

    let neighbor_has_tunnel_or_mtn = |c_idx: usize, elev_at_c: f64| -> bool {
        let Some(conn) = piece.connectors.get(c_idx) else {
            return false;
        };
        if let Some(ref link) = conn.connected_to {
            if let Some(other) = piece_map.get(link.piece_id.as_str()).copied() {
                if other.has_tunnel == Some(true) {
                    return true;
                }
            }
        }
        is_point_inside_any_mountain(conn.x, conn.y, elev_at_c, layout, layer_info, 0.84)
    };

    let open_start = !neighbor_has_tunnel_or_mtn(0, e0);
    let open_end = piece.connectors.len() >= 2 && !neighbor_has_tunnel_or_mtn(1, e1);

    for (br_idx, br) in raw_branches.iter().enumerate() {
        if br.len() < 2 {
            continue;
        }
        let mut path_xyz = Vec::with_capacity(br.len() * 3);
        for p in br {
            path_xyz.push(p[0]);
            path_xyz.push(p[1]);
            path_xyz.push(p[2]);
        }

        // 1. Detailed vaulted interior bore
        push_tunnel_bore_3d(verts, idxs, &path_xyz, &prof);

        // 2. Layered rocky/grassy earthen hillside cover (trimmed back cleanly behind portal faces)
        let trim_s = if br_idx == 0 && open_start { 11.0 } else { 0.0 };
        let trim_e = if br_idx == 0 && open_end { 11.0 } else { 0.0 };
        let hill_path = trim_path_ends(&path_xyz, trim_s, trim_e);
        push_hillside_cover_3d(verts, idxs, &hill_path, &prof);
    }

    if open_start {
        let p0 = main_br[0];
        let p1 = main_br[1];
        let dx = p1[0] - p0[0];
        let dz = p1[2] - p0[2];
        if dx.hypot(dz) > 1e-4 {
            push_arched_tunnel_portal_3d(verts, idxs, p0, [dx, dz], e0.max(0.0) as f32, &prof);
        }
    }
    if open_end {
        let p_end = main_br[main_br.len() - 1];
        let p_prev = main_br[main_br.len() - 2];
        let dx = p_prev[0] - p_end[0];
        let dz = p_prev[2] - p_end[2];
        if dx.hypot(dz) > 1e-4 {
            push_arched_tunnel_portal_3d(verts, idxs, p_end, [dx, dz], e1.max(0.0) as f32, &prof);
        }
    }
}

fn push_hillside_cover_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    path_xyz: &[f32],
    prof: &TunnelProfile,
) {
    let n_pts = path_xyz.len() / 3;
    if n_pts < 2 {
        return;
    }

    let c = prof.inner_width * 0.5;
    let outer_hw = c + prof.pier;
    let crest_h = prof.total_h + 18.0;
    let toe_hw = outer_hw + 22.0;
    let mid_hw = outer_hw + 8.0;
    let rock_h = prof.wall_h * 0.62;
    let grass_h = prof.total_h * 0.94;

    // Cross-section from left grounded toe -> rocky cliff -> grassy hill dome -> right grounded toe
    let raw_profile: [([f32; 2], [f32; 3]); 9] = [
        ([-toe_hw, -1.5], [0.36, 0.35, 0.32]),
        ([-mid_hw, rock_h], [0.40, 0.39, 0.36]),
        ([-outer_hw * 0.85, grass_h], [0.27, 0.40, 0.14]),
        ([-outer_hw * 0.45, crest_h - 3.5], [0.25, 0.42, 0.14]),
        ([0.0, crest_h], [0.24, 0.44, 0.15]),
        ([outer_hw * 0.45, crest_h - 3.5], [0.25, 0.42, 0.14]),
        ([outer_hw * 0.85, grass_h], [0.27, 0.40, 0.14]),
        ([mid_hw, rock_h], [0.40, 0.39, 0.36]),
        ([toe_hw, -1.5], [0.36, 0.35, 0.32]),
    ];

    let mut sec_flat = Vec::with_capacity(raw_profile.len() * 2);
    for (pt, _) in &raw_profile {
        sec_flat.push(pt[0]);
        sec_flat.push(pt[1]);
    }
    let swept = build_tunnel_sweep_positions(path_xyz, &sec_flat);
    let n_sec = raw_profile.len();
    let n_ring = swept.len() / (n_sec * 3);

    for r_i in 0..n_ring.saturating_sub(1) {
        let mid_c = [
            0.5 * (path_xyz[r_i * 3] + path_xyz[(r_i + 1) * 3]),
            0.5 * (path_xyz[r_i * 3 + 1] + path_xyz[(r_i + 1) * 3 + 1]) + prof.wall_h * 0.35,
            0.5 * (path_xyz[r_i * 3 + 2] + path_xyz[(r_i + 1) * 3 + 2]),
        ];
        for s_j in 0..(n_sec - 1) {
            let b0 = (r_i * n_sec + s_j) * 3;
            let b1 = (r_i * n_sec + s_j + 1) * 3;
            let b2 = ((r_i + 1) * n_sec + s_j + 1) * 3;
            let b3 = ((r_i + 1) * n_sec + s_j) * 3;
            let p0 = [swept[b0], swept[b0 + 1], swept[b0 + 2]];
            let p1 = [swept[b1], swept[b1 + 1], swept[b1 + 2]];
            let p2 = [swept[b2], swept[b2 + 1], swept[b2 + 2]];
            let p3 = [swept[b3], swept[b3 + 1], swept[b3 + 2]];
            let c0 = raw_profile[s_j].1;
            let c1 = raw_profile[s_j + 1].1;
            let col = [0.5 * (c0[0] + c1[0]), 0.5 * (c0[1] + c1[1]), 0.5 * (c0[2] + c1[2])];
            push_oriented_quad_toward(verts, idxs, p0, p1, p2, p3, mid_c, false, col, MAT_TUNNEL_TERRAIN);
        }
    }
}

// ============================================================================
//  Geometry & Sampling Helpers
// ============================================================================

#[allow(clippy::too_many_arguments)]
fn push_oriented_quad_toward(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    ref_pt: [f32; 3],
    point_toward_ref: bool,
    color: [f32; 3],
    mat: [f32; 3],
) {
    let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let mut n = normalize3(cross3(e1, e2));
    let fc = [
        0.25 * (p0[0] + p1[0] + p2[0] + p3[0]),
        0.25 * (p0[1] + p1[1] + p2[1] + p3[1]),
        0.25 * (p0[2] + p1[2] + p2[2] + p3[2]),
    ];
    let to_ref = [ref_pt[0] - fc[0], ref_pt[1] - fc[1], ref_pt[2] - fc[2]];
    let d = dot3(n, to_ref);
    if (point_toward_ref && d < 0.0) || (!point_toward_ref && d > 0.0) {
        n = [-n[0], -n[1], -n[2]];
    }
    push_quad_mat(verts, idxs, p0, p1, p2, p3, n, color, mat);
}

fn tunnel_ceiling_y_at_x(prof: &TunnelProfile, lx: f32) -> f32 {
    let c = (prof.inner_width * 0.5 - 0.8).max(6.0);
    let rise = (prof.arch_rise - 0.8).max(2.0);
    let r_circle = (c * c + rise * rise) / (2.0 * rise);
    let cy = prof.wall_h + rise - r_circle;
    let x = lx.clamp(-c, c);
    let inside = (r_circle * r_circle - x * x).max(0.0);
    cy + inside.sqrt()
}

fn sample_path_xyz(path_xyz: &[f32], cum: &[f32], target_d: f32) -> ([f32; 3], [f32; 2]) {
    let n = path_xyz.len() / 3;
    let total = *cum.last().unwrap_or(&0.0);
    let d = target_d.clamp(0.0, total);
    let mut seg = 0usize;
    while seg + 1 < n - 1 && cum[seg + 1] < d {
        seg += 1;
    }
    let span = (cum[seg + 1] - cum[seg]).max(1e-5);
    let t = ((d - cum[seg]) / span).clamp(0.0, 1.0);
    let a = [path_xyz[seg * 3], path_xyz[seg * 3 + 1], path_xyz[seg * 3 + 2]];
    let b = [path_xyz[(seg + 1) * 3], path_xyz[(seg + 1) * 3 + 1], path_xyz[(seg + 1) * 3 + 2]];
    let pos = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    let dx = b[0] - a[0];
    let dz = b[2] - a[2];
    let len = dx.hypot(dz).max(1e-5);
    (pos, [dx / len, dz / len])
}

fn trim_path_ends(path_xyz: &[f32], trim_start: f32, trim_end: f32) -> Vec<f32> {
    let n = path_xyz.len() / 3;
    if n < 2 || (trim_start <= 0.1 && trim_end <= 0.1) {
        return path_xyz.to_vec();
    }
    let mut cum = vec![0.0_f32; n];
    for i in 1..n {
        let dx = path_xyz[i * 3] - path_xyz[(i - 1) * 3];
        let dz = path_xyz[i * 3 + 2] - path_xyz[(i - 1) * 3 + 2];
        cum[i] = cum[i - 1] + dx.hypot(dz);
    }
    let total = cum[n - 1];
    if total <= trim_start + trim_end + 8.0 {
        return path_xyz.to_vec();
    }
    let d_start = trim_start;
    let d_end = total - trim_end;
    let mut out = Vec::with_capacity(path_xyz.len());
    let (p_s, _) = sample_path_xyz(path_xyz, &cum, d_start);
    out.extend_from_slice(&p_s);
    for i in 0..n {
        if cum[i] > d_start + 0.5 && cum[i] < d_end - 0.5 {
            out.push(path_xyz[i * 3]);
            out.push(path_xyz[i * 3 + 1]);
            out.push(path_xyz[i * 3 + 2]);
        }
    }
    let (p_e, _) = sample_path_xyz(path_xyz, &cum, d_end);
    out.extend_from_slice(&p_e);
    out
}

// ============================================================================
//  Unit Tests
// ============================================================================

mod portals;
#[allow(unused_imports)]
use portals::*;
pub(crate) use portals::{push_arched_tunnel_portal_3d};
mod bore;
#[allow(unused_imports)]
use bore::*;
pub(crate) use bore::{push_tunnel_bore_3d};
#[cfg(test)]
mod tests;
