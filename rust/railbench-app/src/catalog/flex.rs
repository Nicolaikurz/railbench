use super::*;
use railbench_core::geometry::norm_angle_deg;

pub fn flex_max_length(catalogs: &[Catalog], catalog_id: &str) -> f64 {
    find_entry(catalogs, catalog_id)
        .and_then(|e| e.length)
        .unwrap_or(900.0)
        .max(200.0)
}

pub(super) fn polyline_length(pts: &[Vec2]) -> f64 {
    let mut total = 0.0;
    for w in pts.windows(2) {
        total += (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
    }
    total
}

pub(super) fn signed_angle_diff_deg(from_deg: f64, to_deg: f64) -> f64 {
    let mut d = (to_deg - from_deg) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    d
}

/// Recomputes a flex piece's connector 1 (`c[1]`) from `custom_length` (mm) and `custom_angle` (deg)
/// while keeping connector 0 (`c[0]`) anchored at its current world position & angle.
pub fn apply_flex_geometry(piece: &mut TrackPiece, catalogs: &[Catalog], length_mm: f64, angle_deg: f64) {
    let len = length_mm.clamp(25.0, flex_max_length(catalogs, &piece.catalog_id));
    let ang = angle_deg.clamp(-180.0, 180.0);

    piece.custom_length = Some(len);
    piece.custom_angle = Some(ang);
    piece.custom_offsets = None;
    piece.custom_chord_dist = None;

    if piece.connectors.len() < 2 {
        return;
    }
    let x0 = piece.connectors[0].x;
    let y0 = piece.connectors[0].y;
    let entry_heading = norm_angle_deg(piece.connectors[0].angle + 180.0);

    let (local_x, local_y) = if ang.abs() < 0.25 {
        piece.custom_radius = None;
        (len, 0.0)
    } else {
        let rad = deg2rad(ang.abs());
        let r = len / rad;
        piece.custom_radius = Some(r);
        let dir = if ang < 0.0 { -1.0 } else { 1.0 };
        (r * rad.sin(), dir * r * (1.0 - rad.cos()))
    };

    let rp = rotate(Vec2 { x: local_x, y: local_y }, entry_heading);
    piece.connectors[1].x = x0 + rp.x;
    piece.connectors[1].y = y0 + rp.y;
    piece.connectors[1].angle = norm_angle_deg(entry_heading + ang);
    piece.connectors[1].connected_to = None;
    piece.transform.x = x0;
    piece.transform.y = y0;
    piece.transform.rotation = entry_heading;
}

/// Free-drags either endpoint (`moving_conn_idx`: `0` or `1`) of a flex track to `world_pos`,
/// keeping the opposite connector anchored while bending into a smooth circular arc and
/// shortening/lengthening within `[25.0, flex_max_length]`.
pub fn apply_flex_endpoint_drag(
    piece: &mut TrackPiece,
    catalogs: &[Catalog],
    moving_conn_idx: usize,
    world_pos: Vec2,
) {
    if piece.connectors.len() < 2 || moving_conn_idx > 1 {
        return;
    }
    let anchor_idx = 1 - moving_conn_idx;
    let ax = piece.connectors[anchor_idx].x;
    let ay = piece.connectors[anchor_idx].y;
    let anchor_heading = norm_angle_deg(piece.connectors[anchor_idx].angle + 180.0);

    let local = rotate(Vec2 { x: world_pos.x - ax, y: world_pos.y - ay }, -anchor_heading);
    let chord = local.x.hypot(local.y).max(25.0);
    let max_alpha = deg2rad(87.5);
    let alpha = local.y.atan2(local.x.max(5.0)).clamp(-max_alpha, max_alpha);
    let bend_deg = (2.0 * alpha).to_degrees().clamp(-175.0, 175.0);
    let raw_arc_len = if alpha.abs() < 0.01 {
        chord
    } else {
        chord * (alpha / alpha.sin())
    };
    let max_len = flex_max_length(catalogs, &piece.catalog_id);
    let len = raw_arc_len.clamp(25.0, max_len);

    let (local_x, local_y, radius_opt) = if bend_deg.abs() < 0.25 {
        (len, 0.0, None)
    } else {
        let rad = deg2rad(bend_deg.abs());
        let r = len / rad;
        let dir = if bend_deg < 0.0 { -1.0 } else { 1.0 };
        (r * rad.sin(), dir * r * (1.0 - rad.cos()), Some(r))
    };

    let rp = rotate(Vec2 { x: local_x, y: local_y }, anchor_heading);
    piece.connectors[moving_conn_idx].x = ax + rp.x;
    piece.connectors[moving_conn_idx].y = ay + rp.y;
    piece.connectors[moving_conn_idx].angle = norm_angle_deg(anchor_heading + bend_deg);
    piece.connectors[moving_conn_idx].connected_to = None;

    piece.custom_offsets = None;
    piece.custom_chord_dist = None;
    piece.custom_length = Some(len);
    piece.custom_radius = radius_opt;
    let c0_heading = norm_angle_deg(piece.connectors[0].angle + 180.0);
    piece.custom_angle = Some(signed_angle_diff_deg(c0_heading, piece.connectors[1].angle));
    piece.transform.x = piece.connectors[0].x;
    piece.transform.y = piece.connectors[0].y;
    piece.transform.rotation = c0_heading;
}

/// Snaps either endpoint (`moving_conn_idx`: `0` or `1`) of a flex track onto a target connector
/// `(target_pos, target_angle)` with exact 0° tangent alignment, keeping the opposite connector
/// anchored and automatically bending (circular arc or smooth C1 S-curve) and shortening as needed.
/// Returns `true` if the target is within valid flex length `[20.0, flex_max_length]`.
pub fn snap_flex_endpoint(
    piece: &mut TrackPiece,
    catalogs: &[Catalog],
    moving_conn_idx: usize,
    target_pos: Vec2,
    target_angle: f64,
) -> bool {
    if piece.connectors.len() < 2 || moving_conn_idx > 1 {
        return false;
    }
    let anchor_idx = 1 - moving_conn_idx;
    let anchor = &piece.connectors[anchor_idx];
    let chord = (target_pos.x - anchor.x).hypot(target_pos.y - anchor.y);
    if chord < 18.0 {
        return false;
    }

    let snapped_angle = norm_angle_deg(target_angle + 180.0);
    let (x0, y0, a0, x1, y1, a1) = if moving_conn_idx == 1 {
        (anchor.x, anchor.y, anchor.angle, target_pos.x, target_pos.y, snapped_angle)
    } else {
        (target_pos.x, target_pos.y, snapped_angle, anchor.x, anchor.y, anchor.angle)
    };

    let pts = railbench_core::geometry::sample_arc_between_connectors(x0, y0, a0, x1, y1, a1, 40);
    let arc_len = polyline_length(&pts);
    let max_len = flex_max_length(catalogs, &piece.catalog_id);
    if !arc_len.is_finite() || arc_len < 20.0 || arc_len > max_len {
        return false;
    }

    piece.connectors[moving_conn_idx].x = target_pos.x;
    piece.connectors[moving_conn_idx].y = target_pos.y;
    piece.connectors[moving_conn_idx].angle = snapped_angle;
    piece.connectors[moving_conn_idx].connected_to = None;

    piece.custom_offsets = None;
    piece.custom_chord_dist = None;
    piece.custom_length = Some(arc_len);

    let c0_heading = norm_angle_deg(piece.connectors[0].angle + 180.0);
    let bend_deg = signed_angle_diff_deg(c0_heading, piece.connectors[1].angle);
    piece.custom_angle = Some(bend_deg);
    piece.custom_radius = if bend_deg.abs() >= 0.25 {
        Some(arc_len / deg2rad(bend_deg.abs()))
    } else {
        None
    };

    piece.transform.x = piece.connectors[0].x;
    piece.transform.y = piece.connectors[0].y;
    piece.transform.rotation = c0_heading;
    true
}

/// Returns the 5 interactive flex handles in world coordinates:
/// `[bend_25%, bend_50%, bend_75%, endpoint_c1, endpoint_c0]`.
pub fn flex_bend_handles_world(piece: &TrackPiece) -> Option<[Vec2; 5]> {
    if piece.connectors.len() < 2 {
        return None;
    }
    let c0 = &piece.connectors[0];
    let c1 = &piece.connectors[1];
    let (chord, rot, origin_x, origin_y, offsets) =
        if let (Some(offs), Some(ch)) = (&piece.custom_offsets, piece.custom_chord_dist) {
            if offs.len() >= 3 {
                (ch, piece.transform.rotation, piece.transform.x, piece.transform.y, [offs[0], offs[1], offs[2]])
            } else {
                let dx = c1.x - c0.x;
                let dy = c1.y - c0.y;
                (dx.hypot(dy).max(1.0), dy.atan2(dx).to_degrees(), c0.x, c0.y, [0.0, 0.0, 0.0])
            }
        } else {
            let dx = c1.x - c0.x;
            let dy = c1.y - c0.y;
            let ch = dx.hypot(dy).max(1.0);
            let ang = dy.atan2(dx).to_degrees();
            // If currently a circular arc / C1 Bezier, sample its 25/50/75% points as initial handle positions
            let pts = railbench_core::geometry::sample_arc_between_connectors(
                c0.x, c0.y, c0.angle, c1.x, c1.y, c1.angle, 12,
            );
            if pts.len() >= 13 {
                return Some([
                    pts[3],
                    pts[6],
                    pts[9],
                    Vec2 { x: c1.x, y: c1.y },
                    Vec2 { x: c0.x, y: c0.y },
                ]);
            }
            (ch, ang, c0.x, c0.y, [0.0, 0.0, 0.0])
        };

    let mut out = [Vec2 { x: 0.0, y: 0.0 }; 5];
    for (i, &t) in [0.25_f64, 0.50, 0.75].iter().enumerate() {
        let rp = rotate(Vec2 { x: chord * t, y: offsets[i] }, rot);
        out[i] = Vec2 {
            x: origin_x + rp.x,
            y: origin_y + rp.y,
        };
    }
    out[3] = Vec2 { x: c1.x, y: c1.y };
    out[4] = Vec2 { x: c0.x, y: c0.y };
    Some(out)
}

/// Dragging one of the 3 interior flex bend points (`handle_idx` in `0..3`) updates
/// `custom_offsets` (`[h1, h2, h3]`) and `custom_chord_dist` (1:1 with TS flextrack splines)
/// while preserving already-connected endpoint angles and updating `custom_length`.
pub fn apply_flex_spline_offset(piece: &mut TrackPiece, catalogs: &[Catalog], handle_idx: usize, world_pos: Vec2) {
    if piece.connectors.len() < 2 || handle_idx >= 3 {
        return;
    }
    let c0x = piece.connectors[0].x;
    let c0y = piece.connectors[0].y;
    let c1x = piece.connectors[1].x;
    let c1y = piece.connectors[1].y;
    let c0_locked = piece.connectors[0].connected_to.is_some();
    let c1_locked = piece.connectors[1].connected_to.is_some();

    // Ensure piece is initialized in spline coordinates anchored at c[0] -> c[1]
    if piece.custom_offsets.as_ref().map_or(true, |o| o.len() < 3) || piece.custom_chord_dist.is_none() {
        let dx = c1x - c0x;
        let dy = c1y - c0y;
        let chord = dx.hypot(dy).max(25.0);
        let chord_rot = norm_angle_deg(dy.atan2(dx).to_degrees());

        // Seed initial 3 offsets from current arc shape so there's no jump on first drag
        let pts = railbench_core::geometry::sample_arc_between_connectors(
            c0x, c0y, piece.connectors[0].angle, c1x, c1y, piece.connectors[1].angle, 12,
        );
        let mut init_offs = vec![0.0, 0.0, 0.0];
        if pts.len() >= 13 {
            for (i, &pi) in [3usize, 6, 9].iter().enumerate() {
                let loc = rotate(Vec2 { x: pts[pi].x - c0x, y: pts[pi].y - c0y }, -chord_rot);
                init_offs[i] = loc.y;
            }
        }
        piece.transform.x = c0x;
        piece.transform.y = c0y;
        piece.transform.rotation = chord_rot;
        piece.custom_chord_dist = Some(chord);
        piece.custom_offsets = Some(init_offs);
    }

    let max_len = flex_max_length(catalogs, &piece.catalog_id);
    let chord = piece.custom_chord_dist.unwrap_or(200.0).max(25.0);
    let rot = piece.transform.rotation;
    let local = rotate(
        Vec2 {
            x: world_pos.x - piece.transform.x,
            y: world_pos.y - piece.transform.y,
        },
        -rot,
    );
    let max_off = chord * 0.65;
    if let Some(offs) = &mut piece.custom_offsets {
        let was_straight = offs[0].abs() < 2.5 && offs[1].abs() < 2.5 && offs[2].abs() < 2.5;
        let clamped_off = ((local.y.clamp(-max_off, max_off)) * 10.0).round() / 10.0;
        if handle_idx == 1 && was_straight {
            // Center pull from straight creates a uniform symmetrical bow
            offs[1] = clamped_off;
            offs[0] = (clamped_off * 0.75 * 10.0).round() / 10.0;
            offs[2] = (clamped_off * 0.75 * 10.0).round() / 10.0;
        } else {
            offs[handle_idx] = clamped_off;
        }

        if offs[0].abs() < 2.5 && offs[1].abs() < 2.5 && offs[2].abs() < 2.5 {
            // Snap to straight when all 3 offsets are near zero
            piece.custom_offsets = None;
            piece.custom_chord_dist = None;
            piece.custom_radius = None;
            piece.custom_angle = None;
            piece.custom_length = Some(chord.clamp(25.0, max_len));
            if !c0_locked {
                piece.connectors[0].angle = norm_angle_deg(rot + 180.0);
            }
            if !c1_locked {
                piece.connectors[1].angle = norm_angle_deg(rot);
            }
            let end_rp = rotate(Vec2 { x: chord, y: 0.0 }, rot);
            piece.connectors[1].x = piece.transform.x + end_rp.x;
            piece.connectors[1].y = piece.transform.y + end_rp.y;
            return;
        }

        let measure_spline_len = |o: &[f64]| -> f64 {
            let flat = railbench_core::geometry::sample_flex_spline_points(chord, o[0], o[1], o[2], 32);
            let mut total = 0.0;
            let mut prev = (flat[0], flat[1]);
            for i in (5..flat.len()).step_by(5) {
                let cur = (flat[i], flat[i + 1]);
                total += (cur.0 - prev.0).hypot(cur.1 - prev.1);
                prev = cur;
            }
            total
        };

        let mut spline_len = measure_spline_len(offs);
        if spline_len > max_len && spline_len > chord {
            let excess_ratio = ((max_len - chord).max(0.0) / (spline_len - chord)).sqrt().clamp(0.0, 1.0);
            for v in offs.iter_mut() {
                *v = (*v * excess_ratio * 10.0).round() / 10.0;
            }
            spline_len = measure_spline_len(offs);
        }

        let (h1, h2, h3) = (offs[0], offs[1], offs[2]);
        if !c0_locked {
            let m0 = 2.0 * (4.0 * h1 - h2);
            let tan0_deg = m0.atan2(chord).to_degrees();
            piece.connectors[0].angle = norm_angle_deg(rot + tan0_deg + 180.0);
        }
        if !c1_locked {
            let m4 = -2.0 * (4.0 * h3 - h2);
            let tan1_deg = m4.atan2(chord).to_degrees();
            piece.connectors[1].angle = norm_angle_deg(rot + tan1_deg);
        }
        let end_rp = rotate(Vec2 { x: chord, y: 0.0 }, rot);
        piece.connectors[1].x = piece.transform.x + end_rp.x;
        piece.connectors[1].y = piece.transform.y + end_rp.y;
        piece.custom_length = Some(spline_len.min(max_len));

        // Populate matching custom_radius & custom_angle for symmetric bows
        let is_sym = (h1 - h3).abs() < 1.5 && (h1 - 0.75 * h2).abs() < 2.5 && h2.abs() >= 2.5;
        if is_sym {
            let abs_h = h2.abs();
            let half_d = chord * 0.5;
            let r = ((half_d * half_d + abs_h * abs_h) / (2.0 * abs_h)).max(80.0);
            let sin_ha = (half_d / r).min(1.0);
            let alpha_deg = (2.0 * sin_ha.asin()).to_degrees();
            piece.custom_radius = Some((r * 10.0).round() / 10.0);
            piece.custom_angle = Some(((-h2.signum() * alpha_deg) * 10.0).round() / 10.0);
        } else {
            piece.custom_radius = None;
            piece.custom_angle = None;
        }
    }
}

/// Flips a flextrack end-for-end so connector 0 and connector 1 swap places while preserving shape,
/// spline offsets, curvature direction, and ramp elevation.
pub fn flip_flex_track_piece(piece: &mut TrackPiece, catalogs: &[Catalog]) {
    if piece.connectors.len() < 2 {
        return;
    }
    let c0_conn = piece.connectors[0].connected_to.clone();
    let c1_conn = piece.connectors[1].connected_to.clone();
    let c1 = &piece.connectors[1];
    piece.transform = Transform {
        x: (c1.x * 10.0).round() / 10.0,
        y: (c1.y * 10.0).round() / 10.0,
        rotation: norm_angle_deg(c1.angle - 180.0),
    };
    if let Some(ang) = piece.custom_angle {
        piece.custom_angle = Some(-ang);
    }
    if let Some(offs) = &mut piece.custom_offsets {
        if offs.len() >= 3 {
            *offs = vec![-offs[2], -offs[1], -offs[0]];
        }
    }
    if let Some(el) = piece.elevation {
        piece.elevation = Some(railbench_core::types::Elevation {
            start: el.end,
            end: el.start,
        });
    }
    if let Some(entry) = find_entry(catalogs, &piece.catalog_id) {
        piece.connectors = compute_piece_world_connectors(piece, entry);
        if piece.connectors.len() >= 2 {
            piece.connectors[0].connected_to = c1_conn;
            piece.connectors[1].connected_to = c0_conn;
        }
    } else {
        piece.connectors.swap(0, 1);
    }
}

/// Resets a flextrack back to its catalog straight length while keeping whichever end is connected
/// anchored (if connector 1 is connected and connector 0 is free, flips first to anchor connector 1).
pub fn reset_flex_track_to_original(piece: &mut TrackPiece, catalogs: &[Catalog]) {
    if piece.connectors.len() >= 2
        && piece.connectors[1].connected_to.is_some()
        && piece.connectors[0].connected_to.is_none()
    {
        flip_flex_track_piece(piece, catalogs);
    }
    let old_c0 = piece.connectors.first().and_then(|c| c.connected_to.clone());
    piece.custom_length = None;
    piece.custom_radius = None;
    piece.custom_angle = None;
    piece.custom_offsets = None;
    piece.custom_chord_dist = None;
    if let Some(entry) = find_entry(catalogs, &piece.catalog_id) {
        piece.connectors = compute_world_connectors(entry, piece.transform);
    }
    if let (Some(c0), Some(ref conn)) = (piece.connectors.get_mut(0), old_c0) {
        c0.connected_to = Some(conn.clone());
    }
}

/// Bends an active flextrack to a target radius (in mm) while keeping `custom_length` (or `angle_deg` if provided)
/// and preserving connector 0 as anchor.
pub fn bend_flex_track(
    piece: &mut TrackPiece,
    catalogs: &[Catalog],
    radius_mm: f64,
    angle_deg_opt: Option<f64>,
) {
    let default_len = find_entry(catalogs, &piece.catalog_id)
        .and_then(|e| e.length)
        .unwrap_or(900.0);
    let cur_len = piece.custom_length.unwrap_or(default_len).clamp(25.0, flex_max_length(catalogs, &piece.catalog_id));
    if radius_mm <= 0.0 || radius_mm >= 10000.0 {
        apply_flex_geometry(piece, catalogs, cur_len, 0.0);
        return;
    }
    let r = radius_mm.clamp(120.0, 9999.0);
    let sign = angle_deg_opt
        .filter(|a| a.abs() > 0.05)
        .map(|a| a.signum())
        .or_else(|| piece.custom_angle.filter(|a| a.abs() > 0.05).map(|a| a.signum()))
        .unwrap_or(1.0);
    let target_deg = angle_deg_opt.unwrap_or_else(|| {
        let raw_deg = (cur_len / r).to_degrees().clamp(1.0, 180.0);
        sign * raw_deg
    });
    let len = if angle_deg_opt.is_some() {
        (r * deg2rad(target_deg.abs())).clamp(25.0, flex_max_length(catalogs, &piece.catalog_id))
    } else {
        cur_len
    };
    piece.custom_offsets = None;
    piece.custom_chord_dist = None;
    piece.custom_length = Some(len);
    piece.custom_radius = Some(r);
    piece.custom_angle = Some(target_deg);
    if let Some(entry) = find_entry(catalogs, &piece.catalog_id) {
        piece.connectors = compute_piece_world_connectors(piece, entry);
    }
}

/// Cuts a flextrack into two connected halves (preserving radius, curvature direction, and midpoint elevation).
pub fn cut_flex_track(
    piece: &TrackPiece,
    catalogs: &[Catalog],
    second_id: String,
) -> Option<(TrackPiece, TrackPiece)> {
    let entry = find_entry(catalogs, &piece.catalog_id)?;
    let total_len = piece
        .custom_length
        .or(entry.length)
        .unwrap_or(900.0);
    if total_len < 40.0 {
        return None;
    }
    let half_len = (total_len * 0.5 * 10.0).round() / 10.0;

    let (e_first, e_second) = match piece.elevation {
        Some(el) => {
            let mid = ((el.start + el.end) * 0.5 * 10.0).round() / 10.0;
            (
                Some(railbench_core::types::Elevation { start: el.start, end: mid }),
                Some(railbench_core::types::Elevation { start: mid, end: el.end }),
            )
        }
        None => (None, None),
    };

    let mut first = piece.clone();
    first.elevation = e_first;
    first.custom_offsets = None;
    first.custom_chord_dist = None;
    first.custom_length = Some(half_len);

    if let (Some(r), Some(a)) = (piece.custom_radius, piece.custom_angle) {
        if a.abs() > 0.25 && r > 0.0 {
            let half_a = (a * 0.5 * 10.0).round() / 10.0;
            first.custom_radius = Some(r);
            first.custom_angle = Some(half_a);
            first.connectors = compute_piece_world_connectors(&first, entry);
            let c1 = first.connectors.get(1)?.clone();
            let tr2 = Transform {
                x: c1.x,
                y: c1.y,
                rotation: norm_angle_deg(c1.angle),
            };
            let mut second = first.clone();
            second.id = second_id;
            second.transform = tr2;
            second.elevation = e_second;
            second.connectors = compute_piece_world_connectors(&second, entry);
            return Some((first, second));
        }
    }

    first.custom_radius = None;
    first.custom_angle = None;
    first.connectors = compute_piece_world_connectors(&first, entry);
    let c1 = first.connectors.get(1)?.clone();
    let tr2 = Transform {
        x: c1.x,
        y: c1.y,
        rotation: norm_angle_deg(c1.angle),
    };
    let mut second = first.clone();
    second.id = second_id;
    second.transform = tr2;
    second.elevation = e_second;
    second.connectors = compute_piece_world_connectors(&second, entry);
    Some((first, second))
}

#[derive(Debug, Clone, Copy)]
pub struct FlexRampInfo {
    pub length_mm: f64,
    pub start_elev_mm: f64,
    pub end_elev_mm: f64,
    pub rise_mm: f64,
    pub gradient_pct: f64,
    pub is_ramp: bool,
}

pub fn get_flex_track_ramp_info(piece: &TrackPiece, catalogs: &[Catalog]) -> FlexRampInfo {
    let length_mm = piece
        .custom_length
        .or_else(|| find_entry(catalogs, &piece.catalog_id).and_then(|e| e.length))
        .unwrap_or(900.0)
        .max(1.0);
    let start_elev_mm = piece.elevation.map_or(0.0, |e| e.start);
    let end_elev_mm = piece.elevation.map_or(0.0, |e| e.end);
    let rise_mm = end_elev_mm - start_elev_mm;
    let gradient_pct = ((rise_mm.abs() / length_mm) * 100.0 * 10.0).round() / 10.0;
    FlexRampInfo {
        length_mm,
        start_elev_mm,
        end_elev_mm,
        rise_mm,
        gradient_pct,
        is_ramp: rise_mm.abs() >= 0.5,
    }
}

