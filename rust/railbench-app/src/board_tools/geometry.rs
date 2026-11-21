use super::*;

pub fn get_baseboard_polygon(bb: &BaseboardConfig) -> Vec<Vec2> {
    let BoardBounds { x, y, width: w, depth: d } = get_baseboard_bounds(bb);
    let shape = bb.shape.as_deref().unwrap_or("rect");
    let mut poly = match shape {
        "triangle" => match bb.triangle_type.as_deref().unwrap_or("corner-tl") {
            "corner-tr" => vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d },
            ],
            "corner-br" => vec![
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x, y: y + d },
            ],
            "corner-bl" => vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x, y: y + d },
            ],
            "isosceles" => vec![
                Vec2 { x: x + w * 0.5, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x, y: y + d },
            ],
            _ => vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x, y: y + d },
            ],
        },
        "l-shape" => {
            let is_left = bb.l_side.as_deref().unwrap_or("left") == "left";
            let w_wing = bb.l_wing_width.unwrap_or(800.0).min(w * 0.85);
            let d_main = bb.l_wing_depth.unwrap_or(800.0).min(d * 0.85);
            if is_left {
                vec![
                    Vec2 { x, y },
                    Vec2 { x: x + w, y },
                    Vec2 { x: x + w, y: y + d_main },
                    Vec2 { x: x + w_wing, y: y + d_main },
                    Vec2 { x: x + w_wing, y: y + d },
                    Vec2 { x, y: y + d },
                ]
            } else {
                vec![
                    Vec2 { x, y },
                    Vec2 { x: x + w, y },
                    Vec2 { x: x + w, y: y + d },
                    Vec2 { x: x + w - w_wing, y: y + d },
                    Vec2 { x: x + w - w_wing, y: y + d_main },
                    Vec2 { x, y: y + d_main },
                ]
            }
        }
        "u-shape" => {
            let w_left = bb.u_left_wing_width.unwrap_or(700.0).min(w * 0.45);
            let w_right = bb.u_right_wing_width.unwrap_or(700.0).min(w * 0.45);
            let d_back = bb.u_back_depth.unwrap_or(600.0).min(d * 0.85);
            vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x: x + w - w_right, y: y + d },
                Vec2 { x: x + w - w_right, y: y + d_back },
                Vec2 { x: x + w_left, y: y + d_back },
                Vec2 { x: x + w_left, y: y + d },
                Vec2 { x, y: y + d },
            ]
        }
        "t-shape" => {
            let d_bar = bb.t_bar_depth.unwrap_or(600.0).min(d * 0.85);
            let w_stem = bb.t_stem_width.unwrap_or(800.0).min(w * 0.85);
            let stem_left = x + (w - w_stem) * 0.5;
            let stem_right = stem_left + w_stem;
            vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d_bar },
                Vec2 { x: stem_right, y: y + d_bar },
                Vec2 { x: stem_right, y: y + d },
                Vec2 { x: stem_left, y: y + d },
                Vec2 { x: stem_left, y: y + d_bar },
                Vec2 { x, y: y + d_bar },
            ]
        }
        "custom" => {
            if let Some(cp) = bb.custom_polygon.as_ref().filter(|p| p.len() >= 3) {
                let mut max_px = 0.0_f64;
                let mut max_py = 0.0_f64;
                for pt in cp {
                    max_px = max_px.max(pt.x);
                    max_py = max_py.max(pt.y);
                }
                let sx = if max_px > 1.0 { w / max_px } else { 1.0 };
                let sy = if max_py > 1.0 { d / max_py } else { 1.0 };
                cp.iter()
                    .map(|pt| Vec2 {
                        x: ((x + pt.x * sx) * 10.0).round() / 10.0,
                        y: ((y + pt.y * sy) * 10.0).round() / 10.0,
                    })
                    .collect()
            } else {
                vec![
                    Vec2 { x, y },
                    Vec2 { x: x + w, y },
                    Vec2 { x: x + w, y: y + d },
                    Vec2 { x, y: y + d },
                ]
            }
        }
        _ => vec![
            Vec2 { x, y },
            Vec2 { x: x + w, y },
            Vec2 { x: x + w, y: y + d },
            Vec2 { x, y: y + d },
        ],
    };

    let rot = bb.rotation.unwrap_or(0.0);
    if rot.abs() > 1e-6 {
        let cx = x + w * 0.5;
        let cy = y + d * 0.5;
        let rad = deg2rad(rot);
        let cos = rad.cos();
        let sin = rad.sin();
        for pt in &mut poly {
            let dx = pt.x - cx;
            let dy = pt.y - cy;
            pt.x = ((cx + dx * cos - dy * sin) * 10.0).round() / 10.0;
            pt.y = ((cy + dx * sin + dy * cos) * 10.0).round() / 10.0;
        }
    }

    poly
}

pub fn layer_plate_as_baseboard(
    plate: &LayerPlate,
    target_board: Option<&BaseboardConfig>,
) -> BaseboardConfig {
    let table_bounds = target_board
        .map(get_baseboard_bounds)
        .unwrap_or(BoardBounds {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            depth: 0.0,
        });
    BaseboardConfig {
        id: Some(plate.id.clone()),
        name: plate.name.clone(),
        enabled: true,
        shape: Some(plate.shape.clone().unwrap_or_else(|| "rect".into())),
        custom_polygon: plate.custom_polygon.clone(),
        triangle_type: plate.triangle_type.clone(),
        width: plate.width.max(50.0),
        depth: plate.depth.max(50.0),
        rotation: Some(plate.rotation.unwrap_or(0.0)),
        l_wing_width: plate.l_wing_width,
        l_wing_depth: plate.l_wing_depth,
        l_side: plate.l_side.clone(),
        u_left_wing_width: plate.u_left_wing_width,
        u_right_wing_width: plate.u_right_wing_width,
        u_back_depth: plate.u_back_depth,
        t_bar_depth: plate.t_bar_depth,
        t_stem_width: plate.t_stem_width,
        position_mode: "custom".into(),
        custom_x: Some(table_bounds.x + plate.x.unwrap_or(0.0)),
        custom_y: Some(table_bounds.y + plate.y.unwrap_or(0.0)),
        style: plate
            .style
            .clone()
            .or_else(|| target_board.map(|b| b.style.clone()))
            .unwrap_or_else(|| "wood".into()),
        corner_radius: Some(8.0),
        show_dimensions: Some(false),
    }
}

pub fn get_layer_plate_polygon(
    plate: &LayerPlate,
    target_board: Option<&BaseboardConfig>,
) -> Vec<Vec2> {
    let dummy = layer_plate_as_baseboard(plate, target_board);
    let mut poly = get_baseboard_polygon(&dummy);
    if let Some(tb) = target_board {
        let rot = tb.rotation.unwrap_or(0.0);
        if rot.abs() > 1e-6 {
            let b = get_baseboard_bounds(tb);
            let cx = b.x + tb.width * 0.5;
            let cy = b.y + tb.depth * 0.5;
            let rad = deg2rad(rot);
            let cos = rad.cos();
            let sin = rad.sin();
            for pt in &mut poly {
                let dx = pt.x - cx;
                let dy = pt.y - cy;
                pt.x = ((cx + dx * cos - dy * sin) * 10.0).round() / 10.0;
                pt.y = ((cy + dx * sin + dy * cos) * 10.0).round() / 10.0;
            }
        }
    }
    poly
}

pub fn is_point_in_polygon(pt: Vec2, poly: &[Vec2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[j];
        let intersect =
            ((a.y > pt.y) != (b.y > pt.y)) && (pt.x < ((b.x - a.x) * (pt.y - a.y)) / (b.y - a.y) + a.x);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn is_point_inside_baseboard(pt: Vec2, bb: &BaseboardConfig) -> bool {
    if !bb.enabled {
        return false;
    }
    let poly = get_baseboard_polygon(bb);
    is_point_in_polygon(pt, &poly)
}

pub(super) fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let l2 = vx * vx + vy * vy;
    if l2 < 1e-9 {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * vx + (p.y - a.y) * vy) / l2).clamp(0.0, 1.0);
    (p.x - (a.x + t * vx)).hypot(p.y - (a.y + t * vy))
}

pub fn is_point_near_baseboard_edge(pt: Vec2, bb: &BaseboardConfig, tol: f64) -> bool {
    if !bb.enabled {
        return false;
    }
    let poly = get_baseboard_polygon(bb);
    if poly.len() < 3 {
        return false;
    }
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        if dist_to_segment(pt, poly[j], poly[i]) <= tol {
            return true;
        }
        j = i;
    }
    false
}

pub fn compute_baseboard_leg_positions(
    bb: &BaseboardConfig,
    leg_radius: f64,
    base_inset: f64,
) -> Vec<Vec2> {
    let poly = get_baseboard_polygon(bb);
    if poly.len() < 3 {
        return Vec::new();
    }
    let n = poly.len();
    let mut placed: Vec<Vec2> = Vec::new();
    for i in 0..n {
        let p_prev = poly[(i + n - 1) % n];
        let p_curr = poly[i];
        let p_next = poly[(i + 1) % n];

        let v1x = p_curr.x - p_prev.x;
        let v1y = p_curr.y - p_prev.y;
        let l1 = v1x.hypot(v1y);
        let v2x = p_next.x - p_curr.x;
        let v2y = p_next.y - p_curr.y;
        let l2 = v2x.hypot(v2y);
        if l1 < 1e-4 || l2 < 1e-4 {
            continue;
        }
        let u1x = v1x / l1;
        let u1y = v1y / l1;
        let u2x = v2x / l2;
        let u2y = v2y / l2;
        let n1x = -u1y;
        let n1y = u1x;
        let n2x = -u2y;
        let n2y = u2x;

        let min_edge = l1.min(l2);
        let inset = base_inset.min((leg_radius + 5.0).max(min_edge * 0.25));

        let bis_x = n1x + n2x;
        let bis_y = n1y + n2y;
        let bis_len = bis_x.hypot(bis_y);
        let (mut lx, mut ly) = (p_curr.x + bis_x * inset, p_curr.y + bis_y * inset);
        if bis_len > 1e-4 {
            let bis_scale = (inset * 2.8).min((2.0 * inset) / bis_len);
            lx = p_curr.x + (bis_x / bis_len) * bis_scale;
            ly = p_curr.y + (bis_y / bis_len) * bis_scale;
            if !is_point_in_polygon(Vec2 { x: lx, y: ly }, &poly) {
                lx = p_curr.x - (bis_x / bis_len) * bis_scale;
                ly = p_curr.y - (bis_y / bis_len) * bis_scale;
            }
        }
        let pt = Vec2 { x: lx, y: ly };
        if placed.iter().any(|p| (p.x - pt.x).hypot(p.y - pt.y) < 80.0) {
            continue;
        }
        if is_point_in_polygon(pt, &poly) {
            placed.push(pt);
        }
    }
    placed
}

// ============================================================================
//  Concave Polygon + Holes Triangulator (Pure Rust Ear-Clipping + Hole Bridging)
// ============================================================================

pub fn signed_polygon_area(poly: &[Vec2]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        sum += a.x * b.y - b.x * a.y;
    }
    sum * 0.5
}

pub(super) fn clean_polygon_ring(pts: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(pts.len());
    for &p in pts {
        if out
            .last()
            .map_or(true, |last| (p.x - last.x).hypot(p.y - last.y) > 0.1)
        {
            out.push(p);
        }
    }
    if out.len() >= 2 && (out[0].x - out.last().unwrap().x).hypot(out[0].y - out.last().unwrap().y) <= 0.1 {
        out.pop();
    }
    out
}

#[inline]
pub(super) fn cross2(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

#[inline]
pub(super) fn point_in_tri_ccw(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let c1 = cross2(a, b, p);
    let c2 = cross2(b, c, p);
    let c3 = cross2(c, a, p);
    c1 >= -1e-6 && c2 >= -1e-6 && c3 >= -1e-6
}

pub(super) fn find_visible_outer_vertex(outer: &[Vec2], m: Vec2) -> Option<usize> {
    let n = outer.len();
    let mut best_x = f64::INFINITY;
    let mut best_edge = None;
    for i in 0..n {
        let a = outer[i];
        let b = outer[(i + 1) % n];
        if (a.y <= m.y && b.y > m.y) || (b.y <= m.y && a.y > m.y) {
            let t = (m.y - a.y) / (b.y - a.y);
            let ix = a.x + t * (b.x - a.x);
            if ix >= m.x - 1e-6 && ix < best_x {
                best_x = ix;
                best_edge = Some(i);
            }
        }
    }
    let e_idx = best_edge?;
    let ea = outer[e_idx];
    let eb = outer[(e_idx + 1) % n];
    let p_idx = if ea.x >= eb.x { e_idx } else { (e_idx + 1) % n };
    let p_vert = outer[p_idx];
    let ix_pt = Vec2 { x: best_x, y: m.y };

    if (p_vert.x - ix_pt.x).hypot(p_vert.y - ix_pt.y) <= 1e-5 {
        return Some(p_idx);
    }

    // Check if triangle (M, I, P) contains reflex vertices; pick minimum angle to +X axis
    let area = cross2(m, ix_pt, p_vert);
    let mut chosen = p_idx;
    let mut min_angle = (p_vert.y - m.y).abs().atan2((p_vert.x - m.x).max(1e-9));
    let mut min_dist = (p_vert.x - m.x).hypot(p_vert.y - m.y);

    for (vi, &v) in outer.iter().enumerate() {
        if vi == p_idx || (v.x - m.x).hypot(v.y - m.y) <= 1e-5 {
            continue;
        }
        let inside = if area >= 0.0 {
            point_in_tri_ccw(v, m, ix_pt, p_vert)
        } else {
            point_in_tri_ccw(v, m, p_vert, ix_pt)
        };
        if !inside {
            continue;
        }
        let prev = outer[(vi + n - 1) % n];
        let next = outer[(vi + 1) % n];
        // Outer polygon is CCW (area > 0), so reflex vertices have cross2(prev, v, next) < 0
        if cross2(prev, v, next) < 1e-5 {
            let ang = (v.y - m.y).abs().atan2((v.x - m.x).max(1e-9));
            let d = (v.x - m.x).hypot(v.y - m.y);
            if ang < min_angle - 1e-6 || ((ang - min_angle).abs() <= 1e-6 && d < min_dist) {
                min_angle = ang;
                min_dist = d;
                chosen = vi;
            }
        }
    }
    Some(chosen)
}

pub fn triangulate_polygon_with_holes(
    outer_raw: &[Vec2],
    holes_raw: &[Vec<Vec2>],
) -> Vec<[Vec2; 3]> {
    let mut outer = clean_polygon_ring(outer_raw);
    if outer.len() < 3 || signed_polygon_area(&outer).abs() < 1e-3 {
        return Vec::new();
    }
    if signed_polygon_area(&outer) < 0.0 {
        outer.reverse();
    }

    // Prepare valid CW holes strictly inside the outer polygon, sorted rightmost first
    let mut holes: Vec<(f64, usize, Vec<Vec2>)> = Vec::new();
    for h_raw in holes_raw {
        let mut h = clean_polygon_ring(h_raw);
        let h_area = signed_polygon_area(&h);
        if h.len() < 3 || h_area.abs() < 1.0 {
            continue;
        }
        if h_area > 0.0 {
            h.reverse();
        }
        if !h.iter().all(|&pt| is_point_in_polygon(pt, &outer)) {
            continue;
        }
        let mut m_idx = 0usize;
        for i in 1..h.len() {
            if h[i].x > h[m_idx].x || ((h[i].x - h[m_idx].x).abs() < 1e-9 && h[i].y < h[m_idx].y) {
                m_idx = i;
            }
        }
        holes.push((h[m_idx].x, m_idx, h));
    }
    holes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_mx, m_idx, h) in holes {
        let m = h[m_idx];
        let Some(v_idx) = find_visible_outer_vertex(&outer, m) else {
            continue;
        };
        let mut combined = Vec::with_capacity(outer.len() + h.len() + 2);
        combined.extend_from_slice(&outer[..=v_idx]);
        for k in 0..h.len() {
            combined.push(h[(m_idx + k) % h.len()]);
        }
        combined.push(m);
        combined.push(outer[v_idx]);
        if v_idx + 1 < outer.len() {
            combined.extend_from_slice(&outer[v_idx + 1..]);
        }
        outer = combined;
    }

    // Ear clipping on merged CCW polygon
    let mut poly = outer;
    let mut tris = Vec::with_capacity(poly.len().saturating_sub(2));
    let mut guard = poly.len() * poly.len() + 32;

    while poly.len() > 3 && guard > 0 {
        guard -= 1;
        let n = poly.len();
        let mut cut_idx: Option<usize> = None;

        for i in 0..n {
            let a = poly[(i + n - 1) % n];
            let b = poly[i];
            let c = poly[(i + 1) % n];
            if cross2(a, b, c) <= 1e-6 {
                continue;
            }
            let mut blocked = false;
            for j in 0..n {
                if j == i || j == (i + n - 1) % n || j == (i + 1) % n {
                    continue;
                }
                let p = poly[j];
                if (p.x - a.x).hypot(p.y - a.y) < 1e-5
                    || (p.x - b.x).hypot(p.y - b.y) < 1e-5
                    || (p.x - c.x).hypot(p.y - c.y) < 1e-5
                {
                    continue;
                }
                if point_in_tri_ccw(p, a, b, c) {
                    blocked = true;
                    break;
                }
            }
            if !blocked {
                cut_idx = Some(i);
                break;
            }
        }

        if let Some(i) = cut_idx {
            let a = poly[(i + n - 1) % n];
            let b = poly[i];
            let c = poly[(i + 1) % n];
            if cross2(a, b, c).abs() > 1e-5 {
                tris.push([a, b, c]);
            }
            poly.remove(i);
        } else {
            // Degenerate/collinear fallback: clip tightest nearly-convex vertex to keep progressing
            let mut best_i = 0usize;
            let mut best_score = f64::INFINITY;
            for i in 0..n {
                let a = poly[(i + n - 1) % n];
                let b = poly[i];
                let c = poly[(i + 1) % n];
                let cr = cross2(a, b, c);
                if cr >= -1e-4 && cr.abs() < best_score {
                    best_score = cr.abs();
                    best_i = i;
                }
            }
            let a = poly[(best_i + n - 1) % n];
            let b = poly[best_i];
            let c = poly[(best_i + 1) % n];
            if cross2(a, b, c).abs() > 1e-4 {
                tris.push([a, b, c]);
            }
            poly.remove(best_i);
        }
    }

    if poly.len() == 3 && cross2(poly[0], poly[1], poly[2]).abs() > 1e-5 {
        tris.push([poly[0], poly[1], poly[2]]);
    }

    tris
}

// ============================================================================
//  Multi-Table Magnetic Snapping (port of src/core/baseboard-snapping.ts)
// ============================================================================

pub fn snap_baseboard(
    cand: &BaseboardConfig,
    cand_x: f64,
    cand_y: f64,
    others: &[BaseboardConfig],
    threshold: f64,
) -> BaseboardSnapResult {
    let w_a = cand.width;
    let d_a = cand.depth;
    let mut best_x = cand_x;
    let mut best_y = cand_y;
    let mut min_diff_x = threshold + 1.0;
    let mut min_diff_y = threshold + 1.0;
    let mut snapped_x = false;
    let mut snapped_y = false;
    let mut guides = Vec::new();

    let enabled_others: Vec<&BaseboardConfig> = others
        .iter()
        .filter(|b| b.id != cand.id && b.enabled)
        .collect();
    if enabled_others.is_empty() {
        return BaseboardSnapResult {
            x: cand_x,
            y: cand_y,
            snapped_x: false,
            snapped_y: false,
            guides,
        };
    }

    for &b in &enabled_others {
        let bb = get_baseboard_bounds(b);
        let left_b = bb.x;
        let right_b = bb.x + bb.width;
        let top_b = bb.y;
        let bot_b = bb.y + bb.depth;
        let mid_xb = bb.x + bb.width * 0.5;
        let mid_yb = bb.y + bb.depth * 0.5;

        for (target_x, diff) in [
            (right_b, (cand_x - right_b).abs()),
            (left_b - w_a, (cand_x + w_a - left_b).abs()),
            (left_b, (cand_x - left_b).abs()),
            (right_b - w_a, (cand_x + w_a - right_b).abs()),
            (mid_xb - w_a * 0.5, (cand_x + w_a * 0.5 - mid_xb).abs()),
        ] {
            if diff < min_diff_x {
                min_diff_x = diff;
                best_x = target_x;
                snapped_x = true;
            }
        }

        for (target_y, diff) in [
            (bot_b, (cand_y - bot_b).abs()),
            (top_b - d_a, (cand_y + d_a - top_b).abs()),
            (top_b, (cand_y - top_b).abs()),
            (bot_b - d_a, (cand_y + d_a - bot_b).abs()),
            (mid_yb - d_a * 0.5, (cand_y + d_a * 0.5 - mid_yb).abs()),
        ] {
            if diff < min_diff_y {
                min_diff_y = diff;
                best_y = target_y;
                snapped_y = true;
            }
        }
    }

    // Polygon vertex-to-vertex snap refinement for L/U/T/custom boards
    let mut temp = cand.clone();
    temp.position_mode = "custom".into();
    temp.custom_x = Some(best_x);
    temp.custom_y = Some(best_y);
    let poly_a = get_baseboard_polygon(&temp);
    'outer: for &b in &enabled_others {
        let poly_b = get_baseboard_polygon(b);
        for &va in &poly_a {
            for &vb in &poly_b {
                let dx = (va.x - vb.x).abs();
                let dy = (va.y - vb.y).abs();
                if dx < threshold && dy < threshold && dx + dy < min_diff_x + min_diff_y {
                    best_x += vb.x - va.x;
                    best_y += vb.y - va.y;
                    snapped_x = true;
                    snapped_y = true;
                    break 'outer;
                }
            }
        }
    }

    if snapped_x {
        for &b in &enabled_others {
            let bb = get_baseboard_bounds(b);
            let right_b = bb.x + bb.width;
            let left_b = bb.x;
            for edge_pos in [left_b, right_b] {
                if (best_x - edge_pos).abs() < 1.5 || (best_x + w_a - edge_pos).abs() < 1.5 {
                    guides.push(BaseboardSnapGuide {
                        is_x: true,
                        pos: edge_pos,
                        from: best_y.min(bb.y) - 60.0,
                        to: (best_y + d_a).max(bb.y + bb.depth) + 60.0,
                    });
                    break;
                }
            }
        }
    }

    if snapped_y {
        for &b in &enabled_others {
            let bb = get_baseboard_bounds(b);
            let top_b = bb.y;
            let bot_b = bb.y + bb.depth;
            for edge_pos in [top_b, bot_b] {
                if (best_y - edge_pos).abs() < 1.5 || (best_y + d_a - edge_pos).abs() < 1.5 {
                    guides.push(BaseboardSnapGuide {
                        is_x: false,
                        pos: edge_pos,
                        from: best_x.min(bb.x) - 60.0,
                        to: (best_x + w_a).max(bb.x + bb.width) + 60.0,
                    });
                    break;
                }
            }
        }
    }

    BaseboardSnapResult {
        x: best_x.round(),
        y: best_y.round(),
        snapped_x,
        snapped_y,
        guides,
    }
}

// ============================================================================
//  Table & Layer Plate Track Cutouts (port of src/core/table-cutout.ts)
// ============================================================================

pub fn get_track_clearance_width(entry: Option<&CatalogEntry>) -> f64 {
    let Some(e) = entry else { return 54.0 };
    if e.is_road == Some(true) {
        return e.road_width.unwrap_or(60.0) + 16.0;
    }
    let gauge = e.gauge.unwrap_or(16.5);
    (gauge * 3.2).round().max(26.0)
}

pub fn piece_has_catenary_overhead(piece: &TrackPiece) -> bool {
    piece.has_catenary == Some(true)
        || piece.catenary_power_feed == Some(true)
        || piece.catenary_isolator == Some(true)
}

pub fn get_piece_clearance_width(piece: &TrackPiece, entry: Option<&CatalogEntry>) -> f64 {
    let base_w = get_track_clearance_width(entry);
    if entry.map_or(false, |e| e.is_road == Some(true)) || !piece_has_catenary_overhead(piece) {
        return base_w;
    }
    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5);
    let sc = (gauge / 16.5).clamp(0.45, 2.5);
    let ballast_bot_w = 42.0 * sc;
    let mast_off = if piece.has_bridge == Some(true) {
        ballast_bot_w * 0.5 + 9.5 * sc
    } else {
        (ballast_bot_w * 0.60).max(24.0)
    };
    let cat_w = ((mast_off + 3.5 + 7.3 * sc) * 2.0).ceil();
    base_w.max(cat_w)
}

pub fn piece_vertical_clearance(piece: &TrackPiece, scale: &str, deck_thickness: f64) -> f64 {
    let base = min_clearance_for_scale(scale);
    if piece_has_catenary_overhead(piece) {
        // Masts rise +92 mm above rail bed (messenger 85, contact 72) plus 6 mm wood clearance.
        base.max(deck_thickness + 98.0)
    } else {
        base
    }
}

pub(super) fn piece_passes_through_deck(
    piece: &TrackPiece,
    layer_map: &HashMap<&str, &Layer>,
    clearance: f64,
    deck_elev: f64,
    deck_thickness: f64,
) -> (bool, f64, f64) {
    let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
    let layer = layer_map.get(lid).copied();
    if let Some(l) = layer {
        if !l.visible {
            return (false, 0.0, 0.0);
        }
    }
    let base_h = layer.and_then(|l| l.height_mm).unwrap_or(0.0);
    let ps = piece.elevation.map_or(0.0, |e| e.start);
    let pe = piece.elevation.map_or(ps, |e| e.end);
    let start_h = base_h + ps;
    let end_h = base_h + pe;
    let min_elev = start_h.min(end_h);
    let max_elev = start_h.max(end_h);

    let goes_below = min_elev < deck_elev - 0.1;
    let is_sloped = (max_elev - min_elev).abs() >= 0.5;
    let overhead_reach = if piece_has_catenary_overhead(piece) { 96.0 } else { 0.0 };
    let intersects_slab = max_elev + overhead_reach > deck_elev - deck_thickness;
    let in_zone = max_elev > deck_elev - clearance + 0.1 && (is_sloped || intersects_slab);
    (goes_below && in_zone, min_elev, max_elev)
}

