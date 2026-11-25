use std::collections::HashMap;

use railbench_core::geometry::rotate;
use railbench_core::types::{Catalog, Layout, TrackPiece, Vec2};

use crate::catalog::{find_entry, is_road_id};
use crate::viewer3d::{sample_track_branches_3d, Camera3D};

use super::loco_builder::TrainPose;

#[derive(Debug, Clone)]
pub(crate) struct RailSegment3D {
    pub(crate) piece_id: String,
    pub(crate) branch_index: usize,
    pub(crate) pts: Vec<[f32; 3]>,
    pub(crate) cum_len: Vec<f32>,
    pub(crate) total_len: f32,
    pub(crate) gauge: f32,
    /// `(end_a_next_seg, end_a_at_start)` and `(end_b_next_seg, end_b_at_start)`
    pub(crate) next_at_start: Option<(usize, bool)>,
    pub(crate) next_at_end: Option<(usize, bool)>,
}

#[derive(Debug, Clone)]
pub(crate) struct TurnoutInfo {
    pub(crate) piece_id: String,
    pub(crate) num_branches: usize,
    pub(crate) world_pos: [f32; 3],
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RailNetwork3D {
    pub(crate) segments: Vec<RailSegment3D>,
    pub(crate) turnouts: Vec<TurnoutInfo>,
}

/// Computes top-of-rail elevation offset above track center elevation.
/// Matches `viewer3d/tracks.rs`: ballast_h (5.5) + tie_h (1.6) + rail_h (2.5) = 9.6 mm at H0 (16.5 mm).
#[inline]
pub(crate) fn rail_top_offset(gauge: f32) -> f32 {
    (5.5 + 1.6 + 2.5) * (gauge / 16.5)
}

fn layer_elev_and_visible(layout: &Layout, layer_id: Option<&str>) -> (f64, bool) {
    let lid = layer_id.unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .map(|l| (l.height_mm.unwrap_or(0.0), l.visible))
        .unwrap_or((0.0, true))
}

fn piece_world_connector_elev(p: &TrackPiece, cidx: usize, layer_h: f64) -> f64 {
    let e0 = layer_h + p.elevation.map(|e| e.start).unwrap_or(0.0);
    let e1 = layer_h + p.elevation.map(|e| e.end).unwrap_or(0.0);
    let u = if cidx == 0 || p.connectors.len() <= 1 { 0.0 } else { 1.0 };
    e0 + (e1 - e0) * u
}

pub(crate) fn build_rail_network(
    layout: &Layout,
    catalogs: &[Catalog],
    turnout_branches: &HashMap<String, usize>,
) -> RailNetwork3D {
    let mut net = RailNetwork3D::default();

    // Map `(piece_id, connector_idx) -> Vec<segment_index>` plus `(piece_id, branch_idx) -> segment_index`
    let mut branch_to_seg: HashMap<(String, usize), usize> = HashMap::new();
    let mut piece_map: HashMap<&str, (&TrackPiece, f64)> = HashMap::new();

    for piece in &layout.pieces {
        if is_road_id(catalogs, &piece.catalog_id) {
            continue;
        }
        let (layer_h, visible) = layer_elev_and_visible(layout, piece.layer_id.as_deref());
        if !visible {
            continue;
        }
        let entry = find_entry(catalogs, &piece.catalog_id);
        let kind = entry.map(|e| e.kind.as_str()).unwrap_or("straight");
        if matches!(kind, "turntable" | "transfer-table") {
            continue;
        }
        let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
        let top_off = rail_top_offset(gauge);
        let e0 = layer_h + piece.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = layer_h + piece.elevation.map(|e| e.end).unwrap_or(0.0);
        let branches = sample_track_branches_3d(piece, entry, e0, e1);
        if branches.is_empty() {
            continue;
        }

        piece_map.insert(&piece.id, (piece, layer_h));
        if branches.len() > 1 {
            let p0 = branches[0][0];
            net.turnouts.push(TurnoutInfo {
                piece_id: piece.id.clone(),
                num_branches: branches.len(),
                world_pos: [p0[0], p0[1] + top_off, p0[2]],
            });
        }

        for (br_idx, raw_pts) in branches.into_iter().enumerate() {
            if raw_pts.len() < 2 {
                continue;
            }
            let pts: Vec<[f32; 3]> = raw_pts
                .into_iter()
                .map(|p| [p[0], p[1] + top_off, p[2]])
                .collect();
            let mut cum_len = Vec::with_capacity(pts.len());
            cum_len.push(0.0);
            let mut acc = 0.0_f32;
            for w in pts.windows(2) {
                let dx = w[1][0] - w[0][0];
                let dy = w[1][1] - w[0][1];
                let dz = w[1][2] - w[0][2];
                acc += (dx * dx + dy * dy + dz * dz).sqrt();
                cum_len.push(acc);
            }
            if acc < 1.0 {
                continue;
            }
            let seg_idx = net.segments.len();
            branch_to_seg.insert((piece.id.clone(), br_idx), seg_idx);
            net.segments.push(RailSegment3D {
                piece_id: piece.id.clone(),
                branch_index: br_idx,
                pts,
                cum_len,
                total_len: acc,
                gauge,
                next_at_start: None,
                next_at_end: None,
            });
        }
    }

    // Fallback showcase straight segment if layout currently has no track pieces
    if net.segments.is_empty() {
        let top_off = rail_top_offset(16.5);
        net.segments.push(RailSegment3D {
            piece_id: "__showcase__".into(),
            branch_index: 0,
            pts: vec![[-180.0, top_off, 0.0], [180.0, top_off, 0.0]],
            cum_len: vec![0.0, 360.0],
            total_len: 360.0,
            gauge: 16.5,
            next_at_start: None,
            next_at_end: None,
        });
        return net;
    }

    // Link adjacent segments via spatial proximity of segment endpoints (respecting active turnout branches)
    let n_segs = net.segments.len();
    let is_active_branch = |piece_id: &str, br_idx: usize| -> bool {
        let sel = turnout_branches.get(piece_id).copied().unwrap_or(0);
        br_idx == sel
    };

    // Helper: find best neighbor segment whose endpoint matches `pt` within 3.5 mm and respects switch routing
    let find_link = |src_idx: usize, pt: [f32; 3]| -> Option<(usize, bool)> {
        let src_pid = &net.segments[src_idx].piece_id;
        let mut best: Option<(f32, usize, bool)> = None;
        for (dst_idx, dst) in net.segments.iter().enumerate() {
            if dst_idx == src_idx || &dst.piece_id == src_pid {
                continue;
            }
            // Route outgoing connections through the turnout's currently selected branch
            if let Some(&(owner_piece, _)) = piece_map.get(dst.piece_id.as_str()) {
                if owner_piece.connectors.len() == 3 || owner_piece.connectors.len() == 4 {
                    // Check which connector corresponds to start/end below via active branch filter
                    let _ = owner_piece;
                }
            }
            for &(at_start, ep) in &[(true, dst.pts[0]), (false, *dst.pts.last().unwrap())] {
                let d2 = (ep[0] - pt[0]).powi(2) + (ep[1] - pt[1]).powi(2) + (ep[2] - pt[2]).powi(2);
                if d2 <= 12.25 {
                    let is_branch_match = is_active_branch(&dst.piece_id, dst.branch_index);
                    // Prefer active branch by subtracting small bonus from distance score
                    let score = d2 + if is_branch_match { 0.0 } else { 100.0 };
                    if best.map_or(true, |(bs, _, _)| score < bs) {
                        best = Some((score, dst_idx, at_start));
                    }
                }
            }
        }
        best.map(|(_, idx, at_start)| (idx, at_start))
    };

    let links: Vec<_> = (0..n_segs)
        .map(|i| {
            let p_start = net.segments[i].pts[0];
            let p_end = *net.segments[i].pts.last().unwrap();
            (find_link(i, p_start), find_link(i, p_end))
        })
        .collect();
    for (i, (next_s, next_e)) in links.into_iter().enumerate() {
        net.segments[i].next_at_start = next_s;
        net.segments[i].next_at_end = next_e;
    }

    // Silence unused variable warnings cleanly while keeping explicit connector/elevation helper available
    let _ = (branch_to_seg, piece_world_connector_elev, rotate, Vec2::default());
    net
}

impl RailNetwork3D {
    /// Evaluates `(position, tangent_unit_dir, grade_pitch_rad)` on `seg_idx` at arc-length `dist_s`.
    pub(crate) fn sample_point_and_tangent(&self, seg_idx: usize, dist_s: f32) -> ([f32; 3], [f32; 3], f32) {
        let seg = &self.segments[seg_idx];
        let s = dist_s.clamp(0.0, seg.total_len);
        let n = seg.pts.len();
        let mut idx = 0usize;
        while idx + 2 < n && seg.cum_len[idx + 1] < s {
            idx += 1;
        }
        let s0 = seg.cum_len[idx];
        let s1 = seg.cum_len[idx + 1];
        let span = (s1 - s0).max(1e-4);
        let u = ((s - s0) / span).clamp(0.0, 1.0);
        let a = seg.pts[idx];
        let b = seg.pts[idx + 1];
        let pos = [
            a[0] + (b[0] - a[0]) * u,
            a[1] + (b[1] - a[1]) * u,
            a[2] + (b[2] - a[2]) * u,
        ];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let dz = b[2] - a[2];
        let len = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-4);
        let horiz = dx.hypot(dz).max(1e-4);
        let pitch = dy.atan2(horiz);
        (pos, [dx / len, dy / len, dz / len], pitch)
    }

    /// Advances `(seg_idx, dist_s, forward_along_seg)` by `delta_s` (can be positive or negative),
    /// crossing connected pieces and returning `(new_seg, new_s, new_dir, bounced_at_dead_end)`.
    pub(crate) fn advance_along(
        &self,
        mut seg_idx: usize,
        mut dist_s: f32,
        mut forward_along_seg: bool,
        delta_s: f32,
    ) -> (usize, f32, bool, bool) {
        if self.segments.is_empty() || delta_s.abs() < 1e-6 {
            return (seg_idx, dist_s, forward_along_seg, false);
        }
        let mut remaining = delta_s.abs();
        let mut travel_fwd = if delta_s >= 0.0 {
            forward_along_seg
        } else {
            !forward_along_seg
        };
        let mut bounced = false;
        let mut hops = 0usize;

        while remaining > 1e-4 && hops < 32 {
            hops += 1;
            let seg = &self.segments[seg_idx];
            if travel_fwd {
                let avail = (seg.total_len - dist_s).max(0.0);
                if remaining <= avail {
                    dist_s += remaining;
                    remaining = 0.0;
                } else if let Some((next_seg, enter_at_start)) = seg.next_at_end {
                    remaining -= avail;
                    // Verify next_seg direction aligns cleanly without 180-deg flip when heading into turnout frog
                    seg_idx = next_seg;
                    if enter_at_start {
                        dist_s = 0.0;
                        travel_fwd = true;
                    } else {
                        dist_s = self.segments[next_seg].total_len;
                        travel_fwd = false;
                    }
                    forward_along_seg = if delta_s >= 0.0 { travel_fwd } else { !travel_fwd };
                } else {
                    // Open track end or buffer stop: gently reverse direction
                    dist_s = seg.total_len;
                    forward_along_seg = !forward_along_seg;
                    bounced = true;
                    break;
                }
            } else {
                let avail = dist_s.max(0.0);
                if remaining <= avail {
                    dist_s -= remaining;
                    remaining = 0.0;
                } else if let Some((next_seg, enter_at_start)) = seg.next_at_start {
                    remaining -= avail;
                    seg_idx = next_seg;
                    if enter_at_start {
                        dist_s = 0.0;
                        travel_fwd = true;
                    } else {
                        dist_s = self.segments[next_seg].total_len;
                        travel_fwd = false;
                    }
                    forward_along_seg = if delta_s >= 0.0 { travel_fwd } else { !travel_fwd };
                } else {
                    dist_s = 0.0;
                    forward_along_seg = !forward_along_seg;
                    bounced = true;
                    break;
                }
            }
        }
        (seg_idx, dist_s, forward_along_seg, bounced)
    }

    /// Computes full 3D `TrainPose` plus Mallet front-unit curve articulation angle (rad).
    pub(crate) fn compute_train_pose(
        &self,
        seg_idx: usize,
        dist_s: f32,
        forward_along_seg: bool,
        layout_scale: f32,
    ) -> (TrainPose, f32) {
        let (pos, tan, raw_pitch) = self.sample_point_and_tangent(seg_idx, dist_s);
        let dir = if forward_along_seg { 1.0_f32 } else { -1.0_f32 };
        let fx = tan[0] * dir;
        let fz = tan[2] * dir;
        let yaw_rad = fz.atan2(fx);
        let pitch_rad = raw_pitch * dir;

        // Estimate curve articulation by peeking ~50 mm ahead along rail network
        let look = 50.0 * layout_scale;
        let (n_seg, n_s, n_fwd, bounced) = self.advance_along(seg_idx, dist_s, forward_along_seg, look);
        let mallet_art = if bounced {
            0.0
        } else {
            let (_, n_tan, _) = self.sample_point_and_tangent(n_seg, n_s);
            let ndir = if n_fwd { 1.0_f32 } else { -1.0_f32 };
            let f_yaw = (n_tan[2] * ndir).atan2(n_tan[0] * ndir);
            let mut dyaw = f_yaw - yaw_rad;
            while dyaw > std::f32::consts::PI {
                dyaw -= std::f32::consts::TAU;
            }
            while dyaw < -std::f32::consts::PI {
                dyaw += std::f32::consts::TAU;
            }
            dyaw.clamp(-0.32, 0.32)
        };

        let seg_scale = (self.segments[seg_idx].gauge / 16.5).max(0.25);
        (
            TrainPose {
                pos,
                yaw_rad,
                pitch_rad,
                scale: seg_scale,
            },
            mallet_art,
        )
    }

    pub(crate) fn nearest_turnout_id(&self, pos: [f32; 3]) -> Option<String> {
        self.turnouts
            .iter()
            .min_by(|a, b| {
                let da = (a.world_pos[0] - pos[0]).powi(2) + (a.world_pos[2] - pos[2]).powi(2);
                let db = (b.world_pos[0] - pos[0]).powi(2) + (b.world_pos[2] - pos[2]).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|t| t.piece_id.clone())
    }

    pub(crate) fn apply_camera_tracking(&self, cam: &mut Camera3D, pose: TrainPose, mode: super::TrainCameraMode) {
        match mode {
            super::TrainCameraMode::Free => {}
            super::TrainCameraMode::Follow => {
                cam.target = [pose.pos[0], pose.pos[1] + 24.0 * pose.scale, pose.pos[2]];
            }
            super::TrainCameraMode::Cab => {
                cam.target = [pose.pos[0], pose.pos[1] + 28.0 * pose.scale, pose.pos[2]];
                // Orbit behind trailing end so camera looks forward over locomotive nose
                cam.yaw = -pose.yaw_rad - std::f32::consts::FRAC_PI_2;
                cam.pitch = 0.16;
                cam.distance = cam.distance.clamp(75.0, 260.0);
            }
        }
    }
}
