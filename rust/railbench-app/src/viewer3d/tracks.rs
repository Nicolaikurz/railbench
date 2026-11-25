use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn push_track_piece_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    layout: &Layout,
    is_road: bool,
    e0: f64,
    e1: f64,
    color_mode: ColorMode,
    layer_color: Option<[f32; 3]>,
    cat_ctx: &mut CatenaryMast3DContext,
) {
    let kind_str = entry.map(|e| e.kind.as_str()).unwrap_or("straight");
    match kind_str {
        "turntable" => {
            crate::turntables_3d::push_turntable_3d(verts, idxs, piece, entry, e0);
            return;
        }
        "transfer-table" => {
            crate::turntables_3d::push_transfer_table_3d(verts, idxs, piece, entry, layout, e0);
            return;
        }
        _ => {}
    }

    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
    let branches = sample_track_branches_3d(piece, entry, e0, e1);
    let is_buffer_piece = piece.connectors.len() == 1 || entry.map_or(false, crate::catalog::is_buffer_stop_entry);

    if is_road {
        let rw = entry.and_then(|e| e.road_width).unwrap_or(60.0) as f32;
        let asphalt = [0.22, 0.25, 0.29];
        let stripe = [0.92, 0.94, 0.96];
        for (br_idx, pts) in branches.iter().enumerate() {
            push_trapezoid_polyline_3d(verts, idxs, pts, rw, rw + 4.0, 2.2, asphalt);
            let stripe_pts: Vec<[f32; 3]> = pts.iter().map(|p| [p[0], p[1] + 2.2, p[2]]).collect();
            push_box_polyline_mat_3d(verts, idxs, &stripe_pts, 2.2, 0.4, stripe, MAT_DEFAULT);
            if piece.has_bridge == Some(true) && pts.len() >= 2 {
                crate::bridges_3d::push_road_bridge_3d(verts, idxs, piece, layout, pts, br_idx, rw);
            }
        }
        return;
    }

    let ballast_top_w = (gauge / 16.5) * 26.8;
    let ballast_bot_w = (gauge / 16.5) * 42.0;
    let ballast_h = 5.5_f32;
    let tie_w = (gauge / 16.5) * 23.2;
    let tie_thick = 2.6_f32;
    let tie_embed = 1.0_f32;
    let tie_h = 1.6_f32;
    let rail_h = 2.5_f32;
    let rail_w = 1.35_f32;

    // Matches Electron `viewer-3d-track.ts`: realistic #78716c ballast in Off mode,
    // direct blueprint article color in Types mode, and layer color in Layers mode.
    let base_ballast_col = if piece.has_tunnel == Some(true) {
        [0.25, 0.24, 0.23]
    } else {
        [0x78 as f32 / 255.0, 0x71 as f32 / 255.0, 0x6c as f32 / 255.0]
    };
    let ballast_col = match color_mode {
        ColorMode::Off => base_ballast_col,
        ColorMode::Types => {
            let _ = is_buffer_piece;
            let c = crate::blueprint_colors::get_track_blueprint_color(&piece.catalog_id).rgb_f32();
            if piece.has_tunnel == Some(true) {
                [c[0] * 0.70, c[1] * 0.70, c[2] * 0.70]
            } else {
                c
            }
        }
        ColorMode::Layers => {
            let lc = layer_color.unwrap_or([0x38 as f32 / 255.0, 0xbd as f32 / 255.0, 0xf8 as f32 / 255.0]);
            if piece.has_tunnel == Some(true) {
                [lc[0] * 0.70, lc[1] * 0.70, lc[2] * 0.70]
            } else {
                lc
            }
        }
    };
    let tie_col = [0.23, 0.16, 0.11];
    let tie_plate_col = [0.28, 0.25, 0.23];
    let rail_web_col = [0.36, 0.33, 0.31];
    let rail_col = [0.86, 0.89, 0.92];

    // Pass 1: Continuous Mitered Multi-Slope 3D Ballast Roadbed (with sub-ballast shoulder, beveled crown & sealed end-caps) + Bridge deck
    for (br_idx, pts) in branches.iter().enumerate() {
        if pts.len() < 2 {
            continue;
        }
        push_ballast_bed_polyline_3d(verts, idxs, pts, ballast_top_w, ballast_bot_w, ballast_h, ballast_col);
        if piece.has_bridge == Some(true) {
            crate::bridges_3d::push_railway_bridge_3d(
                verts,
                idxs,
                piece,
                entry,
                layout,
                pts,
                br_idx,
                gauge,
                ballast_bot_w,
                ballast_h,
                tie_h,
            );
        }
    }

    // Pass 2: 3D Wooden Sleepers/Ties embedded into the ballast crib + steel rail baseplates (Rippenplatten)
    let tie_step = (gauge / 16.5) * 9.0;
    let hg = gauge * 0.5;
    let plate_half = (gauge / 16.5) * 1.45;
    let plate_thick = 3.2_f32;
    let plate_h = 0.45_f32;
    for pts in &branches {
        if pts.len() < 2 {
            continue;
        }
        let mut accum = 0.0_f32;
        for w in pts.windows(2) {
            let a = w[0];
            let b = w[1];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let dz = b[2] - a[2];
            let seg_len = dx.hypot(dz);
            if seg_len < 0.1 {
                continue;
            }
            let nx = -dz / seg_len;
            let nz = dx / seg_len;
            let mut d = tie_step - accum;
            while d <= seg_len {
                let u = d / seg_len;
                let tx = a[0] + dx * u;
                let ty_base = a[1] + dy * u + ballast_h - tie_embed;
                let ty_top = a[1] + dy * u + ballast_h + tie_h;
                let tz = a[2] + dz * u;
                let p0 = [tx + nx * tie_w * 0.5, ty_base, tz + nz * tie_w * 0.5];
                let p1 = [tx - nx * tie_w * 0.5, ty_base, tz - nz * tie_w * 0.5];
                push_box_segment(verts, idxs, p0, p1, tie_thick, tie_h + tie_embed, tie_col);

                // Left & right steel tie plates (Rippenplatten) on top of the sleeper under each rail
                for side in [-1.0_f32, 1.0] {
                    let rx = tx + nx * (hg * side);
                    let rz = tz + nz * (hg * side);
                    let pa = [rx + nx * plate_half, ty_top - 0.05, rz + nz * plate_half];
                    let pb = [rx - nx * plate_half, ty_top - 0.05, rz - nz * plate_half];
                    push_box_segment(verts, idxs, pa, pb, plate_thick, plate_h, tie_plate_col);
                }
                d += tie_step;
            }
            accum = seg_len - (d - tie_step);
        }
    }

    // Pass 3: Twin Continuous Mitered 3D Profiled Rails (dark weathered steel web/foot + bright polished running head) + Overhead Catenary
    for (br_idx, pts) in branches.iter().enumerate() {
        if pts.len() < 2 {
            continue;
        }
        let n_pts = pts.len();
        let mut left_web_pts = Vec::with_capacity(n_pts);
        let mut right_web_pts = Vec::with_capacity(n_pts);
        let mut left_head_pts = Vec::with_capacity(n_pts);
        let mut right_head_pts = Vec::with_capacity(n_pts);
        let web_h = rail_h * 0.64;
        let head_h = rail_h * 0.36;
        for i in 0..n_pts {
            let (nx, nz, _) = polyline_miter_normal(pts, i);
            let p = pts[i];
            let ry_web = p[1] + ballast_h + tie_h;
            let ry_head = ry_web + web_h;
            left_web_pts.push([p[0] + nx * hg, ry_web, p[2] + nz * hg]);
            right_web_pts.push([p[0] - nx * hg, ry_web, p[2] - nz * hg]);
            left_head_pts.push([p[0] + nx * hg, ry_head, p[2] + nz * hg]);
            right_head_pts.push([p[0] - nx * hg, ry_head, p[2] - nz * hg]);
        }
        push_box_polyline_mat_3d(verts, idxs, &left_web_pts, rail_w * 0.72, web_h, rail_web_col, MAT_RAIL_WEB);
        push_box_polyline_mat_3d(verts, idxs, &right_web_pts, rail_w * 0.72, web_h, rail_web_col, MAT_RAIL_WEB);
        push_box_polyline_mat_3d(verts, idxs, &left_head_pts, rail_w, head_h, rail_col, MAT_METAL_RAIL);
        push_box_polyline_mat_3d(verts, idxs, &right_head_pts, rail_w, head_h, rail_col, MAT_METAL_RAIL);

        if piece.has_catenary == Some(true) {
            let contact_h = 72.0_f32;
            let messenger_h = 85.0_f32;
            let mast_off = if piece.has_bridge == Some(true) {
                ballast_bot_w * 0.5 + 9.5 * (gauge / 16.5)
            } else {
                (ballast_bot_w * 0.60).max(24.0)
            };
            let wire_copper = [0.85, 0.47, 0.04];
            let wire_msg = [0.58, 0.64, 0.72];

            // Preferred outer side of curve/turnout so contact wire stays on the chord side
            let preferred_side = match kind_str {
                "curve" => {
                    if entry.and_then(|e| e.angle).unwrap_or(30.0) < 0.0 {
                        1.0_f32
                    } else {
                        -1.0_f32
                    }
                }
                "turnout-left" | "curved-turnout-left" => {
                    if br_idx == 0 { 1.0 } else { -1.0 }
                }
                "turnout-right" | "curved-turnout-right" => {
                    if br_idx == 0 { -1.0 } else { 1.0 }
                }
                _ => {
                    if br_idx % 2 == 0 { 1.0 } else { -1.0 }
                }
            };

            let p_start = pts[0];
            let p_end = pts[n_pts - 1];
            let (nx_start, nz_start, _) = polyline_miter_normal(pts, 0);
            let (nx_end, nz_end, _) = polyline_miter_normal(pts, n_pts - 1);

            // Place exterior masts at open-air ends (tunneled pieces use overhead ceiling brackets inside the bore)
            if piece.has_tunnel != Some(true) {
                if br_idx == 0 {
                    cat_ctx.try_place_mast(
                        verts,
                        idxs,
                        p_start,
                        nx_start,
                        nz_start,
                        preferred_side,
                        mast_off,
                        contact_h,
                        messenger_h,
                        &piece.id,
                        layout,
                    );
                }
                if !is_buffer_piece {
                    cat_ctx.try_place_mast(
                        verts,
                        idxs,
                        p_end,
                        nx_end,
                        nz_end,
                        preferred_side,
                        mast_off,
                        contact_h,
                        messenger_h,
                        &piece.id,
                        layout,
                    );
                }
            }

            // Straight spans vs curved spans: in real railways (and TS viewer-3d-catenary.ts),
            // catenary wires run in straight chords between consecutive masts!
            // For curves/splines, we place an intermediate mast at the midpoint when the curve angle/length is large,
            // or subdivide into chord spans between hang points so every hang point & dropper lies 100% on the wire.
            let is_curved_branch = n_pts > 2 && {
                let mid_p = pts[n_pts / 2];
                let line_x = (p_start[0] + p_end[0]) * 0.5;
                let line_z = (p_start[2] + p_end[2]) * 0.5;
                (mid_p[0] - line_x).hypot(mid_p[2] - line_z) > 1.5
            };

            if is_curved_branch {
                // Sample chord spans along the curve (4..7 spans depending on arc length)
                let total_arc: f32 = pts
                    .windows(2)
                    .map(|w| (w[1][0] - w[0][0]).hypot(w[1][2] - w[0][2]))
                    .sum();
                let chord_spans = ((total_arc / 42.0).round() as usize).clamp(4, 8);
                let sag_max = if total_arc >= 30.0 {
                    (total_arc * 0.032).min(6.2)
                } else {
                    0.0
                };
                for s in 0..chord_spans {
                    let u0 = s as f32 / chord_spans as f32;
                    let u1 = (s + 1) as f32 / chord_spans as f32;
                    let idx0 = ((u0 * (n_pts - 1) as f32).round() as usize).min(n_pts - 1);
                    let idx1 = ((u1 * (n_pts - 1) as f32).round() as usize).min(n_pts - 1);
                    let a = pts[idx0];
                    let b = pts[idx1];
                    let sag0 = 4.0 * sag_max * u0 * (1.0 - u0);
                    let sag1 = 4.0 * sag_max * u1 * (1.0 - u1);

                    let wa = [a[0], a[1] + contact_h, a[2]];
                    let wb = [b[0], b[1] + contact_h, b[2]];
                    let ma = [a[0], a[1] + messenger_h - sag0, a[2]];
                    let mb = [b[0], b[1] + messenger_h - sag1, b[2]];

                    push_box_segment(verts, idxs, wa, wb, 0.8, 0.8, wire_copper);
                    push_box_segment(verts, idxs, ma, mb, 0.65, 0.65, wire_msg);

                    // Dropper at the exact midpoint of this chord segment `(wa+wb)/2` to `(ma+mb)/2`
                    let drop_bot = [
                        (wa[0] + wb[0]) * 0.5,
                        (wa[1] + wb[1]) * 0.5 + 0.4,
                        (wa[2] + wb[2]) * 0.5,
                    ];
                    let drop_top = [
                        (ma[0] + mb[0]) * 0.5,
                        (ma[1] + mb[1]) * 0.5 + 0.3,
                        (ma[2] + mb[2]) * 0.5,
                    ];
                    let drop_h = (drop_top[1] - drop_bot[1]).max(1.0);
                    push_box(
                        verts,
                        idxs,
                        [drop_bot[0], drop_bot[1] + drop_h * 0.5, drop_bot[2]],
                        [0.45, drop_h, 0.45],
                        wire_msg,
                    );
                }
            } else {
                // Straight span between p_start and p_end
                let horiz_len = (p_end[0] - p_start[0]).hypot(p_end[2] - p_start[2]);
                let sag_max = if horiz_len >= 30.0 {
                    (horiz_len * 0.032).min(6.2)
                } else {
                    0.0
                };
                let msg_segs = if sag_max > 0.1 {
                    ((horiz_len / 35.0).round() as usize).clamp(4, 10)
                } else {
                    1
                };
                let wa = [p_start[0], p_start[1] + contact_h, p_start[2]];
                let wb = [p_end[0], p_end[1] + contact_h, p_end[2]];
                push_box_segment(verts, idxs, wa, wb, 0.8, 0.8, wire_copper);

                for s in 0..msg_segs {
                    let u0 = s as f32 / msg_segs as f32;
                    let u1 = (s + 1) as f32 / msg_segs as f32;
                    let sag0 = 4.0 * sag_max * u0 * (1.0 - u0);
                    let sag1 = 4.0 * sag_max * u1 * (1.0 - u1);
                    let ma = [
                        p_start[0] + (p_end[0] - p_start[0]) * u0,
                        p_start[1] + (p_end[1] - p_start[1]) * u0 + messenger_h - sag0,
                        p_start[2] + (p_end[2] - p_start[2]) * u0,
                    ];
                    let mb = [
                        p_start[0] + (p_end[0] - p_start[0]) * u1,
                        p_start[1] + (p_end[1] - p_start[1]) * u1 + messenger_h - sag1,
                        p_start[2] + (p_end[2] - p_start[2]) * u1,
                    ];
                    push_box_segment(verts, idxs, ma, mb, 0.65, 0.65, wire_msg);

                    // Dropper at interior vertices `u1` (for s + 1 < msg_segs) so dropper top meets the exact messenger vertex!
                    if s + 1 < msg_segs {
                        let cw_y = p_start[1] + (p_end[1] - p_start[1]) * u1 + contact_h + 0.4;
                        let msg_y = mb[1] + 0.3;
                        let drop_h = (msg_y - cw_y).max(1.0);
                        push_box(
                            verts,
                            idxs,
                            [mb[0], cw_y + drop_h * 0.5, mb[2]],
                            [0.45, drop_h, 0.45],
                            wire_msg,
                        );
                    }
                }
            }
        }
    }

    // Catenary power feeder box & section isolator markers
    if piece.catenary_power_feed == Some(true) {
        if let Some(pts) = branches.first().filter(|b| !b.is_empty()) {
            let m = pts[0];
            let (nx, nz, _) = polyline_miter_normal(pts, 0);
            let fx = m[0] + nx * 25.0;
            let fz = m[2] + nz * 25.0;
            push_box(verts, idxs, [fx, m[1] + 5.0, fz], [5.0, 10.0, 5.0], [0.01, 0.52, 0.78]);
            push_box(verts, idxs, [fx, m[1] + 45.0, fz], [1.2, 80.0, 1.2], [0.06, 0.09, 0.16]);
            push_box_segment(verts, idxs, [fx, m[1] + 85.0, fz], [m[0], m[1] + 85.0, m[2]], 1.0, 1.0, [0.01, 0.52, 0.78]);
            push_box(verts, idxs, [m[0], m[1] + 78.5, m[2]], [1.0, 13.0, 1.0], [0.01, 0.52, 0.78]);
        }
    }
    if piece.catenary_isolator == Some(true) {
        if let Some(pts) = branches.first().filter(|b| b.len() >= 2) {
            let iso_idx = piece.catenary_isolated_connector_index.unwrap_or(0);
            let k = if iso_idx == 1 {
                (pts.len() * 4 / 5).min(pts.len() - 1)
            } else {
                (pts.len() / 5).max(1).min(pts.len() - 1)
            };
            let m = pts[k];
            push_rotated_box_mat(verts, idxs, [m[0], m[1] + 72.4, m[2]], 0.0, [0.0, 0.0, 0.0], [5.0, 3.0, 5.0], [0.86, 0.15, 0.15], [0.22, 0.05, 0.90]);
            push_rotated_box_mat(verts, idxs, [m[0], m[1] + 85.3, m[2]], 0.0, [0.0, 0.0, 0.0], [4.5, 2.4, 4.5], [0.60, 0.20, 0.07], MAT_DEFAULT);
        }
    }

    // 3D Buffer Stop (Prellbock: full Rawie / DB steel A-frame with rail clamps, oak buffer beam, twin cylindrical buffers & Sh2 halt sign mounted on the rails)
    if is_buffer_piece {
        if let Some(pts) = branches.first().filter(|b| b.len() >= 2) {
            let p_end = pts[pts.len() - 1];
            let p_prev = pts[pts.len() - 2];
            let dx = p_end[0] - p_prev[0];
            let dy = p_end[1] - p_prev[1];
            let dz = p_end[2] - p_prev[2];
            let seg_len = dx.hypot(dz).max(1e-4);
            let ux = dx / seg_len;
            let uz = dz / seg_len;

            // In build_buffer_stop_with_signal, local -Z faces the approaching train (towards p_prev)
            // and local +Z points towards the dead-end (p_end).
            // With rot_xz(x, z, rot): world_x = x*cos(rot) - z*sin(rot), world_z = x*sin(rot) + z*cos(rot),
            // so local +Z (0, 1) maps to (-sin(rot), cos(rot)) = (ux, uz) => rot = (-ux).atan2(uz).
            let rot = (-ux).atan2(uz);
            let scale = (gauge / 16.5).max(0.35);
            let back_offset = 15.0 * scale;
            let bx = p_end[0] - ux * back_offset;
            let bz = p_end[2] - uz * back_offset;
            let by_track = p_end[1] - dy * (back_offset / seg_len).clamp(0.0, 1.0);
            let by = by_track + ballast_h + tie_h;
            let illuminated = entry.map_or(false, crate::catalog::is_illuminated_buffer_entry);

            crate::models3d_mesh::build_buffer_stop_with_signal(
                verts,
                idxs,
                [bx, by, bz],
                rot,
                [25.0 * scale, 25.0 * scale, 35.0 * scale],
                illuminated,
            );
        }
    }

    // Turnout switch tie rod, DB Wn 1 / Wn 2 Weichenlaterne & digital decoder housing in 3D
    if piece.has_lantern == Some(true) || piece.has_decoder == Some(true) {
        let is_left = matches!(kind_str, "turnout-left" | "curved-turnout-left");
        let side_y = if is_left { -22.0 } else { 22.0 };
        let rp = rotate(Vec2 { x: 26.0, y: side_y }, piece.transform.rotation);
        let lx = (piece.transform.x + rp.x) as f32;
        let lz = (piece.transform.y + rp.y) as f32;
        let ly = e0 as f32;
        let yaw = (piece.transform.rotation as f32).to_radians();

        let rp_track = rotate(Vec2 { x: 26.0, y: 0.0 }, piece.transform.rotation);
        let tx = (piece.transform.x + rp_track.x) as f32;
        let tz = (piece.transform.y + rp_track.y) as f32;

        crate::models3d_mesh::build_turnout_accessories_3d(
            verts,
            idxs,
            [lx, ly, lz],
            [tx, ly, tz],
            yaw,
            kind_str,
            piece.has_lantern == Some(true),
            piece.has_decoder == Some(true),
        );
    }

    // Trackside DB Lichthauptsignal(s) in 3D (full detailed model scaled to track gauge)
    if piece.has_signal == Some(true) {
        if let Some(main_pts) = branches.first().filter(|b| b.len() >= 2) {
            let lat_dist = gauge * 0.95 + 7.5;
            let sig_scale = (gauge / 16.5).clamp(0.45, 2.2) * 0.62;
            let sig_size = [25.0 * sig_scale, 85.0 * sig_scale, 25.0 * sig_scale];
            let side_cfg = piece.signal_side.as_deref().unwrap_or("right");
            let dir_cfg = piece.signal_direction.as_deref().unwrap_or("forward");
            let sides: &[f32] = match side_cfg {
                "left" => &[1.0],
                "both" => &[-1.0, 1.0],
                _ => &[-1.0],
            };
            let dirs: &[bool] = match dir_cfg {
                "backward" => &[false],
                "both" => &[true, false],
                _ => &[true],
            };
            for &side_f in sides {
                for &is_fwd in dirs {
                    let frac = if is_fwd { 0.35 } else { 0.65 };
                    let idx = ((main_pts.len() - 1) as f32 * frac).round() as usize;
                    let idx = idx.clamp(0, main_pts.len() - 2);
                    let a = main_pts[idx];
                    let b = main_pts[idx + 1];
                    let dx = b[0] - a[0];
                    let dz = b[2] - a[2];
                    let len = dx.hypot(dz).max(1e-4);
                    let nx = -dz / len;
                    let nz = dx / len;
                    let sx = a[0] + nx * lat_dist * side_f;
                    let sy = a[1];
                    let sz = a[2] + nz * lat_dist * side_f;
                    // In build_light_signal, local +Z is the front face with the lenses facing the approaching train.
                    // For a forward signal, approaching trains travel along (dx, dz), so the lenses must face (-dx, -dz).
                    // With rot_xz(0, 1, yaw) = (-sin(yaw), cos(yaw)), setting (-sin(yaw), cos(yaw)) = (face_x, face_z)
                    // gives yaw = (-face_x).atan2(face_z).
                    let face_x = if is_fwd { -dx } else { dx };
                    let face_z = if is_fwd { -dz } else { dz };
                    let yaw = (-face_x).atan2(face_z);
                    crate::models3d_mesh::build_light_signal(verts, idxs, [sx, sy, sz], yaw, sig_size);
                }
            }
        }
    }
}

pub(crate) fn sample_track_branches_3d(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    e0: f64,
    e1: f64,
) -> Vec<Vec<[f32; 3]>> {
    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let steps = 32;
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], steps);
            let mut out = Vec::with_capacity(steps + 1);
            for k in 0..=steps {
                let u = k as f64 / steps as f64;
                let rp = rotate(Vec2 { x: flat[k * 5], y: flat[k * 5 + 1] }, piece.transform.rotation);
                out.push([
                    (piece.transform.x + rp.x) as f32,
                    (e0 + (e1 - e0) * u) as f32,
                    (piece.transform.y + rp.y) as f32,
                ]);
            }
            return vec![out];
        }
    }

    let lift = |pts2d: Vec<Vec2>| -> Vec<[f32; 3]> {
        let n = pts2d.len().saturating_sub(1).max(1);
        pts2d
            .into_iter()
            .enumerate()
            .map(|(k, p)| {
                let u = k as f64 / n as f64;
                [p.x as f32, (e0 + (e1 - e0) * u) as f32, p.y as f32]
            })
            .collect()
    };

    let c = &piece.connectors;
    match c.len() {
        0 => vec![],
        1 => {
            let stub_len = piece
                .custom_length
                .or_else(|| entry.and_then(|e| e.length))
                .unwrap_or(70.0)
                .max(18.0);
            let rad = (c[0].angle + 180.0).to_radians();
            vec![lift(vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[0].x + rad.cos() * stub_len, y: c[0].y + rad.sin() * stub_len },
            ])]
        }
        2 => vec![lift(sample_arc_between_connectors(
            c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 28,
        ))],
        3 => vec![
            lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 24)),
            lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 24)),
        ],
        4 => {
            let a0 = c[0].angle.to_radians();
            let v0x = a0.cos();
            let v0y = a0.sin();
            let facing_opposite = c[1..4]
                .iter()
                .filter(|ci| {
                    let ai = ci.angle.to_radians();
                    v0x * ai.cos() + v0y * ai.sin() < -0.15
                })
                .count();
            if facing_opposite == 3 {
                vec![
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 24)),
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 24)),
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[3].x, c[3].y, c[3].angle, 24)),
                ]
            } else {
                vec![
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 24)),
                    lift(sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[3].x, c[3].y, c[3].angle, 24)),
                ]
            }
        }
        _ => {
            let half = c.len() / 2;
            vec![lift(vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[half].x, y: c[half].y },
            ])]
        }
    }
}

// ============================================================================
//  Detailed 3D Buildings, Trees, Signals & Smooth-Normal Alpine Mountains
// ============================================================================

