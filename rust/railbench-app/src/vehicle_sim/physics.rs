use railbench_core::types::{Catalog, CatalogEntry, Layout};

use crate::model_tools::scale_factor_from_h0;

use super::track_graph::{
    branch_endpoint_connectors, find_special_exit_connector, norm_deg_360, recompute_vehicle_poses,
    shortest_angle_diff_deg, turntable_slot_angles, walk_track_graph_cached,
    recompute_vehicle_poses_cached, TrackWalkCache,
};
use super::{VehicleInstance, VehicleRenderPose, VehicleSimState};

#[derive(Clone, Copy)]
pub(super) struct Obb2D {
    cx: f64,
    cy: f64,
    cz: f64,
    hx: f64,
    hy: f64,
    ux: f64,
    uy: f64,
    owner_idx: usize,
}

impl Obb2D {
    pub(super) fn from_pose(p: &VehicleRenderPose, owner_idx: usize, margin_len: f64, margin_w: f64) -> Self {
        let rad = p.heading_deg.to_radians();
        Self {
            cx: p.x,
            cy: p.y,
            cz: p.z,
            hx: (p.length_mm * 0.5 + margin_len).max(2.0),
            hy: (p.width_mm * 0.5 + margin_w).max(2.0),
            ux: rad.cos(),
            uy: rad.sin(),
            owner_idx,
        }
    }
}

pub(super) fn obb_overlaps(a: &Obb2D, b: &Obb2D, z_clear_mm: f64) -> bool {
    if (a.cz - b.cz).abs() > z_clear_mm {
        return false;
    }
    let dx = b.cx - a.cx;
    let dy = b.cy - a.cy;
    let max_r = a.hx + a.hy + b.hx + b.hy;
    if dx * dx + dy * dy > max_r * max_r {
        return false;
    }
    let axes = [(a.ux, a.uy), (-a.uy, a.ux), (b.ux, b.uy), (-b.uy, b.ux)];
    for &(ax, ay) in &axes {
        let dist = (dx * ax + dy * ay).abs();
        let ra = a.hx * (a.ux * ax + a.uy * ay).abs() + a.hy * (-a.uy * ax + a.ux * ay).abs();
        let rb = b.hx * (b.ux * ax + b.uy * ay).abs() + b.hy * (-b.uy * ax + b.ux * ay).abs();
        if dist > ra + rb {
            return false;
        }
    }
    true
}

/// Computes total trailing consist distance from locomotive center to the rear bumper of the last wagon.
fn consist_rear_bumper_dist(v: &VehicleInstance, sf: f64) -> f64 {
    let l = v.kind.def().length_h0_mm * sf;
    let coupler_gap = (3.5 + 1.4 * v.coupler_strain as f64).clamp(1.8, 6.5) * sf;
    let mut dist = l * 0.5;
    for &wk in &v.coupled_cars {
        dist += coupler_gap + wk.def().length_h0_mm * sf;
    }
    dist
}

impl VehicleSimState {
    pub fn has_moving_bridges(&self) -> bool {
        for (pid, &tgt) in &self.turntable_targets_deg {
            let cur = self.turntable_angles_deg.get(pid).copied().unwrap_or(0.0);
            if shortest_angle_diff_deg(cur, tgt).abs() > 0.05 {
                return true;
            }
        }
        for (pid, &tgt) in &self.transfer_targets_mm {
            let cur = self.transfer_offsets_mm.get(pid).copied().unwrap_or(tgt);
            if (tgt - cur).abs() > 0.1 {
                return true;
            }
        }
        false
    }

    pub fn is_piece_bridge_moving(&self, piece_id: &str) -> bool {
        if let Some(&tgt) = self.turntable_targets_deg.get(piece_id) {
            let cur = self.turntable_angles_deg.get(piece_id).copied().unwrap_or(0.0);
            if shortest_angle_diff_deg(cur, tgt).abs() > 0.05 {
                return true;
            }
        }
        if let Some(&tgt) = self.transfer_targets_mm.get(piece_id) {
            let cur = self.transfer_offsets_mm.get(piece_id).copied().unwrap_or(tgt);
            if (tgt - cur).abs() > 0.1 {
                return true;
            }
        }
        false
    }

    pub fn set_turntable_target(&mut self, piece_id: &str, target_deg: f64) {
        self.turntable_angles_deg.entry(piece_id.to_string()).or_insert(0.0);
        self.turntable_targets_deg
            .insert(piece_id.to_string(), norm_deg_360(target_deg));
    }

    pub fn step_turntable(&mut self, piece_id: &str, entry: Option<&CatalogEntry>, delta_slots: isize) {
        let slots = turntable_slot_angles(entry);
        if slots.is_empty() {
            return;
        }
        let cur = self
            .turntable_targets_deg
            .get(piece_id)
            .or_else(|| self.turntable_angles_deg.get(piece_id))
            .copied()
            .unwrap_or(0.0);
        let cur_idx = slots
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                shortest_angle_diff_deg(cur, **a)
                    .abs()
                    .partial_cmp(&shortest_angle_diff_deg(cur, **b).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        let n = slots.len() as isize;
        let next_idx = ((cur_idx as isize + delta_slots).rem_euclid(n)) as usize;
        self.set_turntable_target(piece_id, slots[next_idx]);
    }

    pub fn step_transfer_table(&mut self, piece_id: &str, entry: Option<&CatalogEntry>, delta_bays: isize) {
        let zs = crate::turntables_3d::extract_transfer_track_zs(entry);
        if zs.is_empty() {
            return;
        }
        let def_z = zs[zs.len() / 2] as f64;
        let cur = *self.transfer_offsets_mm.entry(piece_id.to_string()).or_insert(def_z);
        let tgt = self.transfer_targets_mm.get(piece_id).copied().unwrap_or(cur);
        let cur_idx = zs
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((**a as f64) - tgt)
                    .abs()
                    .partial_cmp(&((**b as f64) - tgt).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        let next_idx = (cur_idx as isize + delta_bays).clamp(0, (zs.len() - 1) as isize) as usize;
        self.transfer_targets_mm
            .insert(piece_id.to_string(), zs[next_idx] as f64);
    }

    /// Smoothly animates turntable rotations & transfer table shifts, carrying any parked locomotive with the bridge.
    pub fn step_bridges(&mut self, dt_secs: f64, layout: &Layout, catalogs: &[Catalog]) {
        let dt = dt_secs.clamp(0.0, 0.1);
        let mut any_moved = false;
        let mut moved_ids: Vec<String> = Vec::new();

        for (pid, &tgt) in &self.turntable_targets_deg {
            let cur = self.turntable_angles_deg.entry(pid.clone()).or_insert(0.0);
            let diff = shortest_angle_diff_deg(*cur, tgt);
            if diff.abs() > 0.05 {
                let max_step = 36.0 * dt;
                if diff.abs() <= max_step {
                    *cur = norm_deg_360(tgt);
                } else {
                    *cur = norm_deg_360(*cur + diff.signum() * max_step);
                }
                any_moved = true;
                moved_ids.push(pid.clone());
            }
        }
        for (pid, &tgt) in &self.transfer_targets_mm {
            let cur = self.transfer_offsets_mm.entry(pid.clone()).or_insert(tgt);
            let diff = tgt - *cur;
            if diff.abs() > 0.1 {
                let max_step = 95.0 * dt;
                if diff.abs() <= max_step {
                    *cur = tgt;
                } else {
                    *cur += diff.signum() * max_step;
                }
                any_moved = true;
                moved_ids.push(pid.clone());
            }
        }
        if any_moved {
            self.bridge_mesh_version = self.bridge_mesh_version.wrapping_add(1);
            for v in &mut self.vehicles {
                if moved_ids.iter().any(|id| id == &v.piece_id) {
                    recompute_vehicle_poses(
                        v,
                        layout,
                        catalogs,
                        &self.turnout_states,
                        &self.turntable_angles_deg,
                        &self.transfer_offsets_mm,
                    );
                }
            }
        }
    }

    /// Advances all vehicles by `dt_secs` with consist force physics, collision avoidance,
    /// turnout/turntable/transfer-table routing, and track-following bogie kinematics.
    pub fn step(&mut self, dt_secs: f64, layout: &Layout, catalogs: &[Catalog]) {
        if !self.running || self.vehicles.is_empty() {
            return;
        }
        let needs_sanitize = self.vehicles.iter().any(|v| {
            layout
                .pieces
                .iter()
                .find(|p| p.id == v.piece_id)
                .is_none_or(|p| !Self::piece_matches_domain(catalogs, p, v.domain))
        });
        if needs_sanitize {
            self.sanitize_for_layout(layout, catalogs);
            if !self.running || self.vehicles.is_empty() {
                return;
            }
        }
        let dt = dt_secs.clamp(0.0, 0.1);
        let sf = scale_factor_from_h0(&layout.scale).max(0.1);
        let mut walk_cache = TrackWalkCache::new(
            layout,
            catalogs,
            &self.turntable_angles_deg,
            &self.transfer_offsets_mm,
        );

        let mut all_obbs: Vec<Obb2D> = Vec::with_capacity(self.vehicles.len() * 3);
        for (idx, veh) in self.vehicles.iter().enumerate() {
            for p in &veh.cached_poses {
                all_obbs.push(Obb2D::from_pose(p, idx, 0.0, 0.0));
            }
        }

        let n_veh = self.vehicles.len();
        for vi in 0..n_veh {
            let (veh_sf, _) = walk_cache.track_scale_and_gauge(
                &self.vehicles[vi].piece_id,
                self.vehicles[vi].domain,
            );
            let veh_z_clear = 26.0 * veh_sf;

            if self.is_piece_bridge_moving(&self.vehicles[vi].piece_id) {
                let v = &mut self.vehicles[vi];
                v.velocity_mm_s = 0.0;
                v.current_speed_mm_s = 0.0;
                v.blocked_ahead = false;
                recompute_vehicle_poses_cached(v, &mut walk_cache, &self.turnout_states);
                all_obbs.retain(|o| o.owner_idx != vi);
                for p in &v.cached_poses {
                    all_obbs.push(Obb2D::from_pose(p, vi, 0.0, 0.0));
                }
                continue;
            }

            let blocked_lookahead = {
                let v = &self.vehicles[vi];
                let def = v.kind.def();
                let orient_sign = if v.forward { 1.0 } else { -1.0 };
                let moving_back =
                    v.velocity_mm_s < -0.5 || (v.velocity_mm_s.abs() <= 0.5 && v.reverse);
                let travel_sign = if moving_back { -orient_sign } else { orient_sign };
                let lead_bumper_offset = if moving_back {
                    -orient_sign * consist_rear_bumper_dist(v, veh_sf)
                } else {
                    orient_sign * (def.length_h0_mm * veh_sf * 0.5)
                };
                let look_dist =
                    (24.0 * veh_sf + v.current_speed_mm_s * 0.65).clamp(24.0 * veh_sf, 160.0 * veh_sf);
                let pw = def.width_h0_mm * veh_sf * 0.44;
                let mut hit = false;
                for &frac in &[0.22_f64, 0.58, 1.0] {
                    let probe_delta = lead_bumper_offset + travel_sign * (look_dist * frac);
                    let (px, py, pz, tang) = walk_track_graph_cached(
                        &mut walk_cache,
                        &self.turnout_states,
                        &v.path_history,
                        &v.piece_id,
                        v.branch_idx,
                        v.dist_mm,
                        probe_delta,
                        v.domain,
                    );
                    let rad = tang.to_radians();
                    let probe_obb = Obb2D {
                        cx: px,
                        cy: py,
                        cz: pz,
                        hx: (10.0 * veh_sf).max(4.0),
                        hy: pw,
                        ux: rad.cos(),
                        uy: rad.sin(),
                        owner_idx: vi,
                    };
                    if all_obbs
                        .iter()
                        .any(|o| o.owner_idx != vi && obb_overlaps(&probe_obb, o, veh_z_clear))
                    {
                        hit = true;
                        break;
                    }
                }
                hit
            };

            let v = &mut self.vehicles[vi];
            v.blocked_ahead = blocked_lookahead;
            let def = v.kind.def();
            if def.tractive_kn <= 0.0 && v.throttle < 0.01 && v.current_speed_mm_s < 0.5 {
                v.velocity_mm_s = 0.0;
                v.current_speed_mm_s = 0.0;
                continue;
            }
            let max_v = def.max_speed_mm_s * veh_sf;
            let dir_sign = if v.reverse { -1.0 } else { 1.0 };
            let v_target = if blocked_lookahead {
                0.0
            } else {
                dir_sign * (v.throttle.clamp(0.0, 1.0) as f64) * max_v
            };

            let prev_pid = v.piece_id.clone();
            let prev_br = v.branch_idx;
            let prev_dist = v.dist_mm;
            let prev_fwd = v.forward;
            let prev_hist_len = v.path_history.len();
            let prev_wheel_angle = v.wheel_angle;

            if v.kind == super::VehicleKind::Br96Mallet && v.steam_on {
                v.steam_time = (v.steam_time + dt as f32).rem_euclid(1000.0);
            }

            // 1. Longitudinal consist force physics (kN / metric tons)
            let m_total = v.total_mass_tons().max(1.0);
            let brake_total_kn = def.brake_kn
                + v.coupled_cars.iter().map(|w| w.def().brake_kn).sum::<f64>();
            let v_proto = (v.velocity_mm_s / sf) * 0.087;
            let abs_vp = v_proto.abs();
            let f_grade = -m_total * 9.81 * v.pitch_rad.sin();
            let f_drag = if abs_vp > 0.02 {
                let sgn = v_proto.signum();
                -(0.016 * m_total + 0.00045 * m_total * abs_vp + 0.008 * abs_vp * abs_vp) * sgn
            } else {
                0.0
            };

            let v_err = v_target - v.velocity_mm_s;
            let same_dir = v.velocity_mm_s.abs() < 1.5 || (v_target * v.velocity_mm_s >= 0.0);
            let want_accel =
                !blocked_lookahead && same_dir && v_target.abs() > v.velocity_mm_s.abs() + 0.5;
            let want_brake =
                blocked_lookahead || !same_dir || (v_target.abs() + 0.5 < v.velocity_mm_s.abs());

            let f_drive = if want_accel {
                let throttle_frac = (v.throttle.clamp(0.0, 1.0) as f64).max(0.15);
                let speed_taper = 1.0 / (1.0 + 0.032 * abs_vp);
                let err_gain = (v_err.abs() / (max_v * 0.18).max(1.0)).clamp(0.15, 1.0);
                dir_sign * def.tractive_kn * throttle_frac * speed_taper * err_gain
            } else if want_brake {
                if blocked_lookahead {
                    if v.velocity_mm_s.abs() > 0.2 {
                        -v.velocity_mm_s.signum() * brake_total_kn * 1.35
                    } else {
                        -f_grade
                    }
                } else if v.throttle < 0.01 && same_dir && v.velocity_mm_s.abs() > 8.0 * sf {
                    -v.velocity_mm_s.signum() * brake_total_kn * 0.22
                } else {
                    let brake_dir = if v.velocity_mm_s.abs() > 0.2 {
                        -v.velocity_mm_s.signum()
                    } else {
                        dir_sign
                    };
                    let err_gain = (v_err.abs() / (max_v * 0.22).max(1.0)).clamp(0.25, 1.0);
                    brake_dir * brake_total_kn * 0.85 * err_gain
                }
            } else {
                -f_grade - f_drag + (v_err / max_v.max(1.0)) * def.tractive_kn * 0.6
            };

            let f_net = f_drive + f_grade + f_drag;
            let a_proto = f_net / m_total;
            let a_model = (a_proto / 0.087) * sf * 2.2;
            let mut next_v = v.velocity_mm_s + a_model * dt;

            if (blocked_lookahead || v.throttle < 0.01)
                && next_v.abs() < 3.0 * sf
                && v.velocity_mm_s.abs() < 8.0 * sf
            {
                next_v = 0.0;
            } else if blocked_lookahead && v.velocity_mm_s * next_v <= 0.0 {
                next_v = 0.0;
            } else if same_dir && (v_target - v.velocity_mm_s) * (v_target - next_v) < 0.0 {
                next_v = v_target;
            }
            v.velocity_mm_s = next_v.clamp(-max_v * 1.15, max_v * 1.15);
            v.current_speed_mm_s = v.velocity_mm_s.abs();

            let target_strain = (a_proto * 0.65).clamp(-1.0, 1.0) as f32;
            v.coupler_strain += (target_strain - v.coupler_strain) * (dt * 6.0).min(1.0) as f32;

            // 2. Advance along connected track/street graph
            let ds = v.velocity_mm_s * dt;
            let mut remain = ds.abs();
            let backing_up = ds < 0.0;
            let mut hops = 0usize;

            while remain > 1e-6 && hops < 8 {
                let Some(cur_idx) = walk_cache.piece_idx(&v.piece_id) else {
                    break;
                };
                let piece = walk_cache.piece(cur_idx);
                let (is_special, entry, br_idx, seg_len, first_pt, last_pt) = {
                    let cached = walk_cache.get_or_sample(cur_idx);
                    if cached.branches.is_empty() {
                        break;
                    }
                    let bi = v.branch_idx.min(cached.branches.len() - 1);
                    let sl = cached.lengths[bi].max(1.0);
                    (
                        cached.is_special,
                        cached.entry,
                        bi,
                        sl,
                        cached.branches[bi].first().copied(),
                        cached.branches[bi].last().copied(),
                    )
                };
                let move_pos_dist = v.forward ^ backing_up;

                let exit_conn_opt = if move_pos_dist {
                    if is_special {
                        last_pt.and_then(|pt| find_special_exit_connector(piece, pt))
                    } else {
                        Some(branch_endpoint_connectors(piece, entry, br_idx).1)
                    }
                } else if is_special {
                    first_pt.and_then(|pt| find_special_exit_connector(piece, pt))
                } else {
                    Some(branch_endpoint_connectors(piece, entry, br_idx).0)
                };
                let exit_link = exit_conn_opt
                    .and_then(|ec| walk_cache.find_connected_next(cur_idx, ec, v.domain));

                if is_special && exit_link.is_none() {
                    let mid = seg_len * 0.5;
                    let crosses_mid = if move_pos_dist {
                        v.dist_mm <= mid + 1.0 && v.dist_mm + remain >= mid
                    } else {
                        v.dist_mm >= mid - 1.0 && v.dist_mm - remain <= mid
                    };
                    if crosses_mid {
                        v.dist_mm = mid;
                        v.velocity_mm_s = 0.0;
                        v.current_speed_mm_s = 0.0;
                        break;
                    }
                }

                let fits_on_piece = if move_pos_dist {
                    v.dist_mm + remain <= seg_len
                } else {
                    v.dist_mm - remain >= 0.0
                };
                if fits_on_piece {
                    v.dist_mm = if move_pos_dist {
                        v.dist_mm + remain
                    } else {
                        v.dist_mm - remain
                    };
                    break;
                }

                remain -= if move_pos_dist {
                    (seg_len - v.dist_mm).max(0.0)
                } else {
                    v.dist_mm.max(0.0)
                };
                let boundary_dist = if move_pos_dist { seg_len } else { 0.0 };

                if let Some((next_idx, entry_conn)) = exit_link {
                    let want_road = v.domain == super::VehicleDomain::Road;
                    let (next_is_road, next_is_special, next_pid) = {
                        let pid = walk_cache.piece(next_idx).id.clone();
                        let c = walk_cache.get_or_sample(next_idx);
                        (c.is_road, c.is_special, pid)
                    };
                    if next_is_road != want_road || (want_road && next_is_special) {
                        v.dist_mm = boundary_dist;
                        v.velocity_mm_s = 0.0;
                        v.current_speed_mm_s = 0.0;
                        v.reverse = !v.reverse;
                        break;
                    }
                    match walk_cache.resolve_transition(
                        &self.turnout_states,
                        &v.path_history,
                        next_idx,
                        entry_conn,
                        false,
                    ) {
                        Ok((_, br, _raw_dist, at_start, auto_st)) => {
                            let is_next_special = walk_cache.get_or_sample(next_idx).is_special;
                            v.piece_id = next_pid.clone();
                            v.branch_idx = br;
                            if is_next_special {
                                v.forward = at_start ^ backing_up;
                                v.dist_mm = _raw_dist;
                            } else {
                                if let Some(new_st) = auto_st {
                                    let prev = self.turnout_states.get(&next_pid).copied().unwrap_or(0);
                                    if prev != new_st {
                                        self.turnout_states.insert(next_pid, new_st);
                                        self.turnout_version = self.turnout_version.wrapping_add(1);
                                    }
                                }
                                if at_start {
                                    v.forward = !backing_up;
                                    v.dist_mm = 0.0;
                                } else {
                                    v.forward = backing_up;
                                    v.dist_mm = walk_cache
                                        .get_or_sample(next_idx)
                                        .lengths
                                        .get(br)
                                        .copied()
                                        .unwrap_or(0.0);
                                }
                            }
                            v.path_history.push((v.piece_id.clone(), v.branch_idx));
                            if v.path_history.len() > 24 {
                                v.path_history.remove(0);
                            }
                            hops += 1;
                        }
                        Err(()) => {
                            v.dist_mm = boundary_dist;
                            v.velocity_mm_s = 0.0;
                            v.current_speed_mm_s = 0.0;
                            break;
                        }
                    }
                } else {
                    v.dist_mm = boundary_dist;
                    v.velocity_mm_s = 0.0;
                    v.current_speed_mm_s = 0.0;
                    if !is_special {
                        v.reverse = !v.reverse;
                    }
                    break;
                }
            }

            if let Some(lk) = v.kind.train_loco_kind() {
                let r_wheel = lk.wheel_radius_h0() * (veh_sf as f32).max(0.25);
                v.wheel_angle = (v.wheel_angle - (ds as f32) / r_wheel).rem_euclid(std::f32::consts::TAU);
            }

            // 3. Recompute bogie kinematics & 4. Hard anti-penetration OBB guard
            recompute_vehicle_poses_cached(v, &mut walk_cache, &self.turnout_states);
            if ds.abs() > 1e-6 {
                let collided = v.cached_poses.iter().any(|p| {
                    let my_obb = Obb2D::from_pose(p, vi, 3.0 * veh_sf, -1.0 * veh_sf);
                    all_obbs
                        .iter()
                        .any(|o| o.owner_idx != vi && obb_overlaps(&my_obb, o, veh_z_clear))
                });
                if collided {
                    v.piece_id = prev_pid;
                    v.branch_idx = prev_br;
                    v.dist_mm = prev_dist;
                    v.forward = prev_fwd;
                    v.wheel_angle = prev_wheel_angle;
                    v.path_history.truncate(prev_hist_len);
                    v.velocity_mm_s = 0.0;
                    v.current_speed_mm_s = 0.0;
                    v.blocked_ahead = true;
                    recompute_vehicle_poses_cached(v, &mut walk_cache, &self.turnout_states);
                }
            }

            all_obbs.retain(|o| o.owner_idx != vi);
            for p in &v.cached_poses {
                all_obbs.push(Obb2D::from_pose(p, vi, 0.0, 0.0));
            }
        }
    }
}
