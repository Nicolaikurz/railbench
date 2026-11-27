use std::collections::HashMap;

use super::super::*;

const GRID_CELL_MM: f32 = 60.0;

#[derive(Clone, Copy)]
struct TrackClearSample {
    x: f32,
    y: f32,
    z: f32,
    r: f32,
}

#[derive(Clone, Copy)]
struct CatenaryAnchor {
    anchor: [f32; 3],
    nx: f32,
    nz: f32,
    preferred_side: f32,
    mast_off: f32,
    contact_h: f32,
    messenger_h: f32,
    sf: f32,
}

/// Global 3D catenary mast & portal ("Doppelhalter" / Quertragwerk) placement solver:
/// 1. Indexes every track in the layout into a 2D/3D spatial grid so no mast pole ever lands on rails/ballast.
/// 2. Automatically groups parallel electrified tracks (and boxed-in single tracks between adjacent tracks)
///    into overhead double/multi-track portal holders with outer posts placed clear of all tracks.
#[derive(Default)]
pub(in crate::viewer3d) struct CatenaryMast3DContext {
    track_grid: HashMap<(i32, i32), Vec<TrackClearSample>>,
    pending_anchors: Vec<CatenaryAnchor>,
    placed_poles: Vec<[f32; 3]>,
    single_mast_count: usize,
    portal_count: usize,
}

impl CatenaryMast3DContext {
    /// Builds a spatial track-clearance index over all visible non-road rail pieces in `layout`.
    pub(in crate::viewer3d) fn from_layout(
        layout: &Layout,
        catalogs: &[Catalog],
        layer_info: &HashMap<&str, (f64, bool)>,
    ) -> Self {
        let mut ctx = Self::default();
        for piece in &layout.pieces {
            if is_road_id(catalogs, &piece.catalog_id) {
                continue;
            }
            let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
            let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
            if !vis {
                continue;
            }
            let e0 = lh + piece.elevation.map(|e| e.start).unwrap_or(0.0);
            let e1 = lh + piece.elevation.map(|e| e.end).unwrap_or(0.0);
            let entry = find_entry(catalogs, &piece.catalog_id);
            let kind_str = entry.map(|e| e.kind.as_str()).unwrap_or("straight");
            let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
            let sf = (gauge / 16.5).max(0.25);
            let ballast_bot_w = (gauge / 16.5) * 42.0;
            // Half-ballast shoulder + 3 mm buffer (~24 mm in H0; strictly below mast_off ~26.9 mm)
            let clear_r = (ballast_bot_w * 0.5 + 3.0 * sf).max(24.0 * sf);

            if kind_str == "turntable" {
                let outer_r =
                    crate::turntables_3d::turntable_outer_radius(piece, entry) as f32 + 6.0 * sf;
                ctx.add_track_sample([
                    piece.transform.x as f32,
                    e0 as f32,
                    piece.transform.y as f32,
                ], outer_r);
                continue;
            }
            if kind_str == "transfer-table" {
                let len = entry.and_then(|e| e.length).unwrap_or(360.0) as f32;
                let rot = (piece.transform.rotation as f32).to_radians();
                let (cr, sr) = (rot.cos(), rot.sin());
                let zs = crate::turntables_3d::extract_transfer_track_zs(entry);
                let steps = ((len / (18.0 * sf)).ceil() as usize).max(2);
                for bz in zs {
                    for s in 0..=steps {
                        let lx = (s as f32 / steps as f32) * len;
                        let wx = piece.transform.x as f32 + lx * cr - bz * sr;
                        let wz = piece.transform.y as f32 + lx * sr + bz * cr;
                        ctx.add_track_sample([wx, e0 as f32, wz], clear_r);
                    }
                }
                continue;
            }

            let branches = sample_track_branches_3d(piece, entry, e0, e1);
            let step_mm = (16.0 * sf).max(4.0);
            for pts in branches {
                if pts.len() < 2 {
                    continue;
                }
                for w in pts.windows(2) {
                    let dx = w[1][0] - w[0][0];
                    let dy = w[1][1] - w[0][1];
                    let dz = w[1][2] - w[0][2];
                    let seg_len = dx.hypot(dz);
                    let steps = ((seg_len / step_mm).ceil() as usize).max(1);
                    for s in 0..=steps {
                        let u = s as f32 / steps as f32;
                        ctx.add_track_sample(
                            [w[0][0] + dx * u, w[0][1] + dy * u, w[0][2] + dz * u],
                            clear_r,
                        );
                    }
                }
            }
        }
        ctx
    }

    fn cell_coords(wx: f32, wz: f32) -> (i32, i32) {
        (
            (wx / GRID_CELL_MM).floor() as i32,
            (wz / GRID_CELL_MM).floor() as i32,
        )
    }

    fn add_track_sample(&mut self, pos: [f32; 3], r: f32) {
        let key = Self::cell_coords(pos[0], pos[2]);
        self.track_grid.entry(key).or_default().push(TrackClearSample {
            x: pos[0],
            y: pos[1],
            z: pos[2],
            r,
        });
    }

    pub(in crate::viewer3d) fn is_pole_clear(&self, wx: f32, wy: f32, wz: f32) -> bool {
        let (cx, cz) = Self::cell_coords(wx, wz);
        for dx in -1..=1 {
            for dz in -1..=1 {
                if let Some(list) = self.track_grid.get(&(cx + dx, cz + dz)) {
                    for pt in list {
                        if (pt.y - wy).abs() > 55.0 {
                            continue;
                        }
                        let ddx = pt.x - wx;
                        let ddz = pt.z - wz;
                        if ddx * ddx + ddz * ddz < pt.r * pt.r {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    fn nearest_other_track_dist(
        &self,
        wx: f32,
        wy: f32,
        wz: f32,
        own_anchor: [f32; 3],
        own_sf: f32,
    ) -> f32 {
        let (cx, cz) = Self::cell_coords(wx, wz);
        let own_ignore_r2 = (28.0 * own_sf).powi(2);
        let mut best_d2 = f32::INFINITY;
        for dx in -2..=2 {
            for dz in -2..=2 {
                if let Some(list) = self.track_grid.get(&(cx + dx, cz + dz)) {
                    for pt in list {
                        if (pt.y - wy).abs() > 55.0 {
                            continue;
                        }
                        let odx = pt.x - own_anchor[0];
                        let odz = pt.z - own_anchor[2];
                        if odx * odx + odz * odz <= own_ignore_r2 {
                            continue;
                        }
                        let ddx = pt.x - wx;
                        let ddz = pt.z - wz;
                        let d2 = ddx * ddx + ddz * ddz;
                        if d2 < best_d2 {
                            best_d2 = d2;
                        }
                    }
                }
            }
        }
        best_d2.sqrt()
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::viewer3d) fn register_anchor(
        &mut self,
        anchor: [f32; 3],
        nx: f32,
        nz: f32,
        preferred_side: f32,
        mast_off: f32,
        contact_h: f32,
        messenger_h: f32,
        sf: f32,
    ) {
        let dedup_r = 34.0 * sf;
        if self.pending_anchors.iter().any(|pa| {
            (pa.anchor[0] - anchor[0]).hypot(pa.anchor[2] - anchor[2]) < dedup_r
                && (pa.anchor[1] - anchor[1]).abs() < 25.0
        }) {
            return;
        }
        let n_len = nx.hypot(nz);
        if n_len < 1e-4 {
            return;
        }
        self.pending_anchors.push(CatenaryAnchor {
            anchor,
            nx: nx / n_len,
            nz: nz / n_len,
            preferred_side: if preferred_side >= 0.0 { 1.0 } else { -1.0 },
            mast_off,
            contact_h,
            messenger_h,
            sf,
        });
    }

    /// Resolves all registered catenary anchors into collision-free single masts or
    /// double/multi-track overhead portal holders ("Doppelhalter" / Quertragwerke).
    pub(in crate::viewer3d) fn finalize_masts(
        &mut self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
    ) {
        let anchors = std::mem::take(&mut self.pending_anchors);
        if anchors.is_empty() {
            return;
        }
        let n_len = anchors.len();
        let mut used = vec![false; n_len];

        for i in 0..n_len {
            if used[i] {
                continue;
            }
            let a0 = anchors[i];
            let group_indices = self.find_parallel_group(&anchors, &used, i);

            if group_indices.len() >= 2 {
                let group_anchors: Vec<CatenaryAnchor> =
                    group_indices.iter().map(|&gi| anchors[gi]).collect();
                if self.try_emit_group_portal(verts, idxs, &group_anchors) {
                    for &gi in &group_indices {
                        used[gi] = true;
                    }
                    continue;
                }
            }

            used[i] = true;
            self.emit_single_or_bridged(verts, idxs, a0);
        }
    }

    fn find_parallel_group(
        &self,
        anchors: &[CatenaryAnchor],
        used: &[bool],
        seed_idx: usize,
    ) -> Vec<usize> {
        let seed = anchors[seed_idx];
        let n = [seed.nx, seed.nz];
        let t = [-seed.nz, seed.nx];
        let max_lon = 34.0 * seed.sf;
        let min_step = 28.0 * seed.sf;
        let max_step = 115.0 * seed.sf;
        let max_span = 280.0 * seed.sf;

        let mut cands: Vec<(f32, usize)> = vec![(0.0, seed_idx)];
        for (j, aj) in anchors.iter().enumerate() {
            if j == seed_idx || used[j] {
                continue;
            }
            if (aj.anchor[1] - seed.anchor[1]).abs() > 20.0 * seed.sf {
                continue;
            }
            let ndot = (aj.nx * n[0] + aj.nz * n[1]).abs();
            if ndot < 0.85 {
                continue;
            }
            let dx = aj.anchor[0] - seed.anchor[0];
            let dz = aj.anchor[2] - seed.anchor[2];
            let lon = (dx * t[0] + dz * t[1]).abs();
            if lon > max_lon {
                continue;
            }
            let u = dx * n[0] + dz * n[1];
            cands.push((u, j));
        }

        if cands.len() < 2 {
            return vec![seed_idx];
        }
        cands.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let seed_pos = cands.iter().position(|&(_, idx)| idx == seed_idx).unwrap_or(0);

        let mut start = seed_pos;
        let mut end = seed_pos;
        while start > 0 {
            let gap = cands[start].0 - cands[start - 1].0;
            if gap >= min_step && gap <= max_step && (cands[end].0 - cands[start - 1].0) <= max_span
            {
                start -= 1;
            } else {
                break;
            }
        }
        while end + 1 < cands.len() {
            let gap = cands[end + 1].0 - cands[end].0;
            if gap >= min_step && gap <= max_step && (cands[end + 1].0 - cands[start].0) <= max_span
            {
                end += 1;
            } else {
                break;
            }
        }

        cands[start..=end].iter().map(|&(_, idx)| idx).collect()
    }

    fn try_emit_group_portal(
        &mut self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
        group: &[CatenaryAnchor],
    ) -> bool {
        let a0 = group[0];
        let n = [a0.nx, a0.nz];
        let sf = a0.sf;

        let inv = 1.0 / group.len() as f32;
        let cx = group.iter().map(|a| a.anchor[0]).sum::<f32>() * inv;
        let cy = group.iter().map(|a| a.anchor[1]).fold(f32::NEG_INFINITY, f32::max);
        let cz = group.iter().map(|a| a.anchor[2]).sum::<f32>() * inv;

        let mut u_min = f32::INFINITY;
        let mut u_max = f32::NEG_INFINITY;
        let mut max_off = a0.mast_off;
        for a in group {
            let u = (a.anchor[0] - cx) * n[0] + (a.anchor[2] - cz) * n[1];
            u_min = u_min.min(u);
            u_max = u_max.max(u);
            max_off = max_off.max(a.mast_off);
        }

        let step = 6.0 * sf;
        let left_pole = (0..=11).find_map(|k| {
            let u = u_min - max_off - (k as f32) * step;
            let px = cx + n[0] * u;
            let pz = cz + n[1] * u;
            self.is_pole_clear(px, cy, pz).then_some([px, cy, pz])
        });
        let right_pole = (0..=11).find_map(|k| {
            let u = u_max + max_off + (k as f32) * step;
            let px = cx + n[0] * u;
            let pz = cz + n[1] * u;
            self.is_pole_clear(px, cy, pz).then_some([px, cy, pz])
        });

        let (Some(lp), Some(rp)) = (left_pole, right_pole) else {
            return false;
        };

        self.push_portal_holder_3d(verts, idxs, lp, rp, group);
        true
    }

    fn emit_single_or_bridged(
        &mut self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
        a: CatenaryAnchor,
    ) {
        let sf = a.sf;
        let pref_pos = [
            a.anchor[0] + a.nx * a.mast_off * a.preferred_side,
            a.anchor[1],
            a.anchor[2] + a.nz * a.mast_off * a.preferred_side,
        ];
        let opp_pos = [
            a.anchor[0] - a.nx * a.mast_off * a.preferred_side,
            a.anchor[1],
            a.anchor[2] - a.nz * a.mast_off * a.preferred_side,
        ];
        let clear_pref = self.is_pole_clear(pref_pos[0], pref_pos[1], pref_pos[2]);
        let clear_opp = self.is_pole_clear(opp_pos[0], opp_pos[1], opp_pos[2]);

        let chosen_single = match (clear_pref, clear_opp) {
            (true, true) => {
                let d_pref =
                    self.nearest_other_track_dist(pref_pos[0], pref_pos[1], pref_pos[2], a.anchor, sf);
                let d_opp =
                    self.nearest_other_track_dist(opp_pos[0], opp_pos[1], opp_pos[2], a.anchor, sf);
                if d_pref < 42.0 * sf && d_opp > d_pref + 12.0 * sf {
                    Some(opp_pos)
                } else {
                    Some(pref_pos)
                }
            }
            (true, false) => Some(pref_pos),
            (false, true) => Some(opp_pos),
            (false, false) => None,
        };

        if let Some(pole) = chosen_single {
            self.push_single_mast_3d(verts, idxs, a, pole);
            return;
        }

        // Both immediate sides collide with adjacent tracks/turnouts: span outward to a Doppelhalter portal
        let step = 6.0 * sf;
        let max_extra = 110.0 * sf;
        let max_k = (max_extra / step).ceil() as usize;
        let left_pole = (1..=max_k).find_map(|k| {
            let d = a.mast_off + (k as f32) * step;
            let px = a.anchor[0] - a.nx * d;
            let pz = a.anchor[2] - a.nz * d;
            self.is_pole_clear(px, a.anchor[1], pz)
                .then_some([px, a.anchor[1], pz])
        });
        let right_pole = (1..=max_k).find_map(|k| {
            let d = a.mast_off + (k as f32) * step;
            let px = a.anchor[0] + a.nx * d;
            let pz = a.anchor[2] + a.nz * d;
            self.is_pole_clear(px, a.anchor[1], pz)
                .then_some([px, a.anchor[1], pz])
        });

        if let (Some(lp), Some(rp)) = (left_pole, right_pole) {
            self.push_portal_holder_3d(verts, idxs, lp, rp, &[a]);
            return;
        }

        // If only one side cleared within reasonable cantilever reach (~mast_off + 30*sf), use it
        let max_cantilever = a.mast_off + 30.0 * sf;
        if let Some(lp) = left_pole.filter(|p| (p[0] - a.anchor[0]).hypot(p[2] - a.anchor[2]) <= max_cantilever) {
            self.push_single_mast_3d(verts, idxs, a, lp);
        } else if let Some(rp) = right_pole.filter(|p| (p[0] - a.anchor[0]).hypot(p[2] - a.anchor[2]) <= max_cantilever) {
            self.push_single_mast_3d(verts, idxs, a, rp);
        }
    }

    fn push_single_mast_3d(
        &mut self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
        a: CatenaryAnchor,
        pole: [f32; 3],
    ) {
        let [mx, my, mz] = pole;
        let anchor = a.anchor;
        let contact_h = a.contact_h;
        let messenger_h = a.messenger_h;
        self.placed_poles.push(pole);
        self.single_mast_count += 1;

        let mast_col = [0.20, 0.25, 0.33];
        let insulator_brown = [0.52, 0.30, 0.06];
        let pole_h = messenger_h + 3.0;
        // Concrete foundation pad + steel lattice mast pole
        push_box(verts, idxs, [mx, my + 4.0, mz], [6.5, 8.0, 6.5], [0.46, 0.48, 0.52]);
        push_box(verts, idxs, [mx, my + 4.0 + pole_h * 0.5, mz], [3.0, pole_h, 3.0], mast_col);

        // 1. Upper horizontal Spitzenausleger arm at messenger_h
        push_box_segment(
            verts,
            idxs,
            [mx, my + messenger_h, mz],
            [anchor[0], my + messenger_h, anchor[2]],
            1.5,
            1.5,
            mast_col,
        );
        push_box(
            verts,
            idxs,
            [anchor[0], my + messenger_h + 0.6, anchor[2]],
            [2.0, 2.0, 2.0],
            mast_col,
        );

        // 2. Diagonal Auslegerrohr from mast (y = contact_h - 3.0) up to upper arm (at 16% out from track)
        let top_join_x = anchor[0] * 0.84 + mx * 0.16;
        let top_join_z = anchor[2] * 0.84 + mz * 0.16;
        let mast_diag_y = my + contact_h - 3.0;
        push_box_segment(
            verts,
            idxs,
            [mx, mast_diag_y, mz],
            [top_join_x, my + messenger_h, top_join_z],
            1.2,
            1.2,
            mast_col,
        );

        // 3. Lower registration arm (Seitenhalter) from mid-diagonal directly to contact wire
        let mid_diag_x = anchor[0] * 0.42 + mx * 0.58;
        let mid_diag_z = anchor[2] * 0.42 + mz * 0.58;
        let mid_diag_y = my + contact_h + 5.0;
        push_box_segment(
            verts,
            idxs,
            [mid_diag_x, mid_diag_y, mid_diag_z],
            [anchor[0], my + contact_h, anchor[2]],
            1.0,
            1.0,
            mast_col,
        );
        push_box(
            verts,
            idxs,
            [anchor[0], my + contact_h + 0.5, anchor[2]],
            [2.2, 1.6, 1.6],
            mast_col,
        );

        // 4. Three ceramic insulators positioned strictly ON the three arms
        let ins_upper_x = anchor[0] * 0.15 + mx * 0.85;
        let ins_upper_z = anchor[2] * 0.15 + mz * 0.85;
        push_box(
            verts,
            idxs,
            [ins_upper_x, my + messenger_h + 0.75, ins_upper_z],
            [2.8, 2.8, 2.8],
            insulator_brown,
        );

        let ins_diag_u = 0.82_f32;
        let ins_diag_x = top_join_x * (1.0 - ins_diag_u) + mx * ins_diag_u;
        let ins_diag_z = top_join_z * (1.0 - ins_diag_u) + mz * ins_diag_u;
        let ins_diag_y = (my + messenger_h) * (1.0 - ins_diag_u) + mast_diag_y * ins_diag_u;
        push_box(verts, idxs, [ins_diag_x, ins_diag_y + 0.6, ins_diag_z], [2.8, 2.8, 2.8], insulator_brown);

        let ins_reg_x = anchor[0] * 0.35 + mid_diag_x * 0.65;
        let ins_reg_z = anchor[2] * 0.35 + mid_diag_z * 0.65;
        let ins_reg_y = (my + contact_h) * 0.35 + mid_diag_y * 0.65;
        push_box(verts, idxs, [ins_reg_x, ins_reg_y + 0.5, ins_reg_z], [2.4, 2.4, 2.4], insulator_brown);

        // Every 6th single mast: Radspannwerk (tension wheel + hanging concrete counterweight stack)
        if self.single_mast_count % 6 == 0 {
            let d = (mx - anchor[0]).hypot(mz - anchor[2]).max(1.0);
            let wx = mx + ((mx - anchor[0]) / d) * 4.2;
            let wz = mz + ((mz - anchor[2]) / d) * 4.2;
            push_box(verts, idxs, [wx, my + contact_h + 8.0, wz], [4.6, 4.6, 1.8], [0.25, 0.28, 0.32]);
            push_box(verts, idxs, [wx, my + 28.0, wz], [4.0, 22.0, 4.0], [0.42, 0.44, 0.46]);
        }
    }

    fn push_portal_holder_3d(
        &mut self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
        left_pole: [f32; 3],
        right_pole: [f32; 3],
        group: &[CatenaryAnchor],
    ) {
        self.placed_poles.push(left_pole);
        self.placed_poles.push(right_pole);
        self.portal_count += 1;

        let a0 = group[0];
        let sf = a0.sf;
        let base_y = left_pole[1].max(right_pole[1]);
        let messenger_h = a0.messenger_h;
        let contact_h = a0.contact_h;
        let pole_h = messenger_h + 6.0;
        let top_chord_y = base_y + messenger_h + 2.6;
        let bot_chord_y = base_y + messenger_h - 0.6;
        let steady_y = base_y + contact_h + 5.0;

        let mast_col = [0.20, 0.25, 0.33];
        let truss_col = [0.24, 0.30, 0.39];
        let pad_col = [0.46, 0.48, 0.52];
        let insulator_brown = [0.52, 0.30, 0.06];

        // 1. Two outer foundation pads + heavier steel lattice portal masts
        for pole in [left_pole, right_pole] {
            push_box(
                verts,
                idxs,
                [pole[0], pole[1] + 4.0, pole[2]],
                [7.2 * sf, 8.0, 7.2 * sf],
                pad_col,
            );
            push_box(
                verts,
                idxs,
                [pole[0], pole[1] + 4.0 + pole_h * 0.5, pole[2]],
                [3.4 * sf, pole_h, 3.4 * sf],
                mast_col,
            );
        }

        // 2. Upper truss cross-beam + lower steady wire cross-beam spanning outside-to-outside
        push_box_segment(
            verts,
            idxs,
            [left_pole[0], top_chord_y, left_pole[2]],
            [right_pole[0], top_chord_y, right_pole[2]],
            1.4,
            1.4,
            truss_col,
        );
        push_box_segment(
            verts,
            idxs,
            [left_pole[0], bot_chord_y, left_pole[2]],
            [right_pole[0], bot_chord_y, right_pole[2]],
            1.3,
            1.3,
            truss_col,
        );
        push_box_segment(
            verts,
            idxs,
            [left_pole[0], steady_y, left_pole[2]],
            [right_pole[0], steady_y, right_pole[2]],
            1.0,
            1.0,
            mast_col,
        );

        let sx = right_pole[0] - left_pole[0];
        let sz = right_pole[2] - left_pole[2];
        let span_len = sx.hypot(sz).max(1.0);
        let panels = ((span_len / (28.0 * sf)).round() as usize).clamp(3, 12);
        for k in 1..panels {
            let u = k as f32 / panels as f32;
            let px = left_pole[0] + sx * u;
            let pz = left_pole[2] + sz * u;
            push_box_segment(verts, idxs, [px, bot_chord_y, pz], [px, top_chord_y, pz], 0.9, 0.9, truss_col);
        }

        // Angled corner knee braces at both outer masts
        let brace_u = (14.0 * sf / span_len).clamp(0.08, 0.25);
        let brace_y = base_y + contact_h - 4.0;
        let l_bx = left_pole[0] + sx * brace_u;
        let l_bz = left_pole[2] + sz * brace_u;
        let r_bx = right_pole[0] - sx * brace_u;
        let r_bz = right_pole[2] - sz * brace_u;
        push_box_segment(
            verts,
            idxs,
            [left_pole[0], brace_y, left_pole[2]],
            [l_bx, bot_chord_y, l_bz],
            1.1,
            1.1,
            mast_col,
        );
        push_box_segment(
            verts,
            idxs,
            [right_pole[0], brace_y, right_pole[2]],
            [r_bx, bot_chord_y, r_bz],
            1.1,
            1.1,
            mast_col,
        );

        // 3. Drop arms, wire clamps, and ceramic insulators for every track under the portal
        let span_dir_x = sx / span_len;
        let span_dir_z = sz / span_len;
        for ga in group {
            let t_proj = (((ga.anchor[0] - left_pole[0]) * span_dir_x
                + (ga.anchor[2] - left_pole[2]) * span_dir_z)
                / span_len)
                .clamp(0.05, 0.95);
            let bx = left_pole[0] + sx * t_proj;
            let bz = left_pole[2] + sz * t_proj;
            let msg_y = ga.anchor[1] + ga.messenger_h;
            let cnt_y = ga.anchor[1] + ga.contact_h;

            // Messenger wire hanger & clamp (angled if anchor is staggered along track)
            push_box_segment(
                verts,
                idxs,
                [bx, bot_chord_y, bz],
                [ga.anchor[0], msg_y, ga.anchor[2]],
                1.0,
                1.0,
                mast_col,
            );
            push_box(verts, idxs, [ga.anchor[0], msg_y + 0.5, ga.anchor[2]], [2.0, 2.0, 2.0], mast_col);

            // Registration drop arm from steady beam down to contact wire clamp
            push_box_segment(
                verts,
                idxs,
                [bx, steady_y, bz],
                [ga.anchor[0], cnt_y, ga.anchor[2]],
                0.95,
                0.95,
                mast_col,
            );
            push_box(verts, idxs, [ga.anchor[0], cnt_y + 0.4, ga.anchor[2]], [2.2, 1.6, 1.6], mast_col);

            // Pair of ceramic insulators flanking the track hanger on top & steady beams
            for side in [-1.0_f32, 1.0] {
                let ix = bx + span_dir_x * (7.5 * sf * side);
                let iz = bz + span_dir_z * (7.5 * sf * side);
                if (ix - left_pole[0]).hypot(iz - left_pole[2]) > 4.0 * sf
                    && (ix - right_pole[0]).hypot(iz - right_pole[2]) > 4.0 * sf
                {
                    push_box(verts, idxs, [ix, bot_chord_y, iz], [2.4, 2.4, 2.4], insulator_brown);
                    push_box(verts, idxs, [ix, steady_y, iz], [2.2, 2.2, 2.2], insulator_brown);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};
    use railbench_core::types::Transform;

    fn make_layout(pieces: Vec<TrackPiece>) -> Layout {
        Layout {
            name: "CatenaryTest".into(),
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

    fn run_catenary(layout: &Layout, catalogs: &[Catalog]) -> CatenaryMast3DContext {
        let mut layer_info = HashMap::new();
        layer_info.insert("layer-1", (0.0, true));
        let mut ctx = CatenaryMast3DContext::from_layout(layout, catalogs, &layer_info);
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        for piece in &layout.pieces {
            let entry = find_entry(catalogs, &piece.catalog_id);
            let is_road = is_road_id(catalogs, &piece.catalog_id);
            crate::viewer3d::tracks::push_track_piece_3d(
                &mut verts,
                &mut idxs,
                piece,
                entry,
                layout,
                is_road,
                0.0,
                0.0,
                ColorMode::Off,
                None,
                0,
                0.0,
                None,
                false,
                &mut ctx,
            );
        }
        ctx.finalize_masts(&mut verts, &mut idxs);
        ctx
    }

    #[test]
    fn catenary_parallel_double_track_creates_doppelhalter_portals_outside_tracks() {
        let cats = builtin_catalogs();
        let e24188 = find_entry(&cats, "24188").expect("24188 in catalog");

        let mut t1 = create_piece("t1".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        t1.has_catenary = Some(true);
        let mut t2 = create_piece("t2".into(), e24188, Transform { x: 0.0, y: 64.3, rotation: 0.0 }, None);
        t2.has_catenary = Some(true);

        let layout = make_layout(vec![t1, t2]);
        let ctx = run_catenary(&layout, &cats);

        assert!(ctx.portal_count >= 2, "parallel tracks should emit Doppelhalter portals, got {}", ctx.portal_count);
        assert_eq!(ctx.single_mast_count, 0, "parallel tracks should share portals instead of inner single masts");
        assert_eq!(ctx.placed_poles.len(), ctx.portal_count * 2);
        for pole in &ctx.placed_poles {
            assert!(ctx.is_pole_clear(pole[0], pole[1], pole[2]), "pole {pole:?} collided with track");
            assert!(pole[2] < -20.0 || pole[2] > 84.3, "pole {pole:?} should stand outside [0, 64.3]");
        }
    }

    #[test]
    fn catenary_avoids_adjacent_non_electrified_track_and_portals_when_boxed_in() {
        let cats = builtin_catalogs();
        let e24188 = find_entry(&cats, "24188").expect("24188 in catalog");

        // Part A: Electrified track at y=0, non-electrified track at y=+45 -> single mast flips to z < 0
        let mut cat_mid = create_piece("t_cat".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        cat_mid.has_catenary = Some(true);
        let plain_pos = create_piece("t_pos".into(), e24188, Transform { x: 0.0, y: 45.0, rotation: 0.0 }, None);
        let layout_a = make_layout(vec![cat_mid.clone(), plain_pos.clone()]);
        let ctx_a = run_catenary(&layout_a, &cats);

        assert_eq!(ctx_a.single_mast_count, 2);
        assert_eq!(ctx_a.portal_count, 0);
        for pole in &ctx_a.placed_poles {
            assert!(ctx_a.is_pole_clear(pole[0], pole[1], pole[2]));
            assert!(pole[2] < -20.0, "expected single mast on open -Z side away from y=45, got {pole:?}");
        }

        // Part B: Boxed-in middle track with plain tracks on both sides (y=-45 and y=+45) -> upgrades to portal!
        let plain_neg = create_piece("t_neg".into(), e24188, Transform { x: 0.0, y: -45.0, rotation: 0.0 }, None);
        let layout_b = make_layout(vec![cat_mid, plain_pos, plain_neg]);
        let ctx_b = run_catenary(&layout_b, &cats);

        assert!(ctx_b.portal_count >= 2, "boxed-in center track must upgrade to portal holders, got {}", ctx_b.portal_count);
        assert_eq!(ctx_b.single_mast_count, 0);
        for pole in &ctx_b.placed_poles {
            assert!(ctx_b.is_pole_clear(pole[0], pole[1], pole[2]), "boxed-in portal pole {pole:?} hit a track");
            assert!(pole[2].abs() > 65.0, "portal pole {pole:?} must span outside both outer tracks (±45)");
        }
    }

    #[test]
    fn catenary_turnout_masts_clear_both_branches() {
        let cats = builtin_catalogs();
        let e24611 = find_entry(&cats, "24611").expect("24611 left turnout in catalog");
        let mut sw = create_piece("sw1".into(), e24611, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        sw.has_catenary = Some(true);

        let layout = make_layout(vec![sw]);
        let ctx = run_catenary(&layout, &cats);

        assert!(!ctx.placed_poles.is_empty());
        for pole in &ctx.placed_poles {
            assert!(
                ctx.is_pole_clear(pole[0], pole[1], pole[2]),
                "turnout mast {pole:?} collided with turnout branch"
            );
        }
    }
}
