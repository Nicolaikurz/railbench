use super::*;

// ============================================================================
//  3. NEM 102 / NEM 103 CLEARANCE & COLLISION CHECKER
// ============================================================================

struct SampledPieceGeom {
    id: String,
    gauge: f64,
    has_catenary: bool,
    min_radius: Option<f64>,
    connected_to: HashSet<String>,
    /// Branches of `(x, y, world_elev_mm)`
    branches: Vec<Vec<(f64, f64, f64)>>,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

fn seg_seg_closest_points(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
) -> (f64, f64, f64, Vec2) {
    // Parametric closest distance squared between segments AB and CD, returning (dist, s, t, mid)
    let ux = bx - ax;
    let uy = by - ay;
    let vx = dx - cx;
    let vy = dy - cy;
    let wx = ax - cx;
    let wy = ay - cy;

    let a = ux * ux + uy * uy;
    let b = ux * vx + uy * vy;
    let c = vx * vx + vy * vy;
    let d = ux * wx + uy * wy;
    let e = vx * wx + vy * wy;
    let denom = a * c - b * b;

    let (mut s_n, mut s_d);
    let (mut t_n, t_d);

    if denom < 1e-8 {
        s_n = 0.0;
        s_d = 1.0;
        t_n = e;
        t_d = c;
    } else {
        s_n = b * e - c * d;
        s_d = denom;
        t_n = a * e - b * d;
        t_d = c;
        if s_n < 0.0 {
            s_n = 0.0;
            t_n = e;
        } else if s_n > s_d {
            s_n = s_d;
            t_n = e + b;
        }
    }

    if t_n < 0.0 {
        t_n = 0.0;
        if -d < 0.0 {
            s_n = 0.0;
        } else if -d > a {
            s_n = s_d;
        } else {
            s_n = -d;
            s_d = a;
        }
    } else if t_n > t_d {
        t_n = t_d;
        if (-d + b) < 0.0 {
            s_n = 0.0;
        } else if (-d + b) > a {
            s_n = s_d;
        } else {
            s_n = -d + b;
            s_d = a;
        }
    }

    let sc = if s_d.abs() < 1e-8 { 0.0 } else { (s_n / s_d).clamp(0.0, 1.0) };
    let tc = if t_d.abs() < 1e-8 { 0.0 } else { (t_n / t_d).clamp(0.0, 1.0) };
    let px = ax + sc * ux;
    let py = ay + sc * uy;
    let qx = cx + tc * vx;
    let qy = cy + tc * vy;
    let dist = (px - qx).hypot(py - qy);
    (
        dist,
        sc,
        tc,
        Vec2 {
            x: (px + qx) * 0.5,
            y: (py + qy) * 0.5,
        },
    )
}

pub fn check_nem_clearances(layout: &Layout, catalogs: &[Catalog]) -> Vec<ClearanceViolation> {
    let layers = layer_height_map(layout);
    let mut geoms: Vec<SampledPieceGeom> = Vec::with_capacity(layout.pieces.len());

    for p in &layout.pieces {
        if is_road_id(catalogs, &p.catalog_id) {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let (lh, visible) = layers.get(lid).copied().unwrap_or((0.0, true));
        if !visible {
            continue;
        }
        let entry = find_entry(catalogs, &p.catalog_id);
        let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5);
        let e0 = lh + p.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = lh + p.elevation.map(|e| e.end).unwrap_or(e0 - lh);

        let raw_branches = sample_track_branches_world(p, entry);
        let mut branches = Vec::with_capacity(raw_branches.len());
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);

        for br in raw_branches {
            if br.len() < 2 {
                continue;
            }
            let n = (br.len() - 1).max(1) as f64;
            let mut pts = Vec::with_capacity(br.len());
            for (k, pt) in br.iter().enumerate() {
                min_x = min_x.min(pt.x);
                max_x = max_x.max(pt.x);
                min_y = min_y.min(pt.y);
                max_y = max_y.max(pt.y);
                let t = k as f64 / n;
                pts.push((pt.x, pt.y, e0 + (e1 - e0) * t));
            }
            branches.push(pts);
        }
        if !min_x.is_finite() {
            continue;
        }

        let mut connected_to = HashSet::new();
        for c in &p.connectors {
            if let Some(ref ct) = c.connected_to {
                connected_to.insert(ct.piece_id.clone());
            }
        }
        let min_radius = p
            .custom_radius
            .or_else(|| entry.and_then(|e| e.radius.or(e.branch_radius)))
            .map(|r| r.abs());

        geoms.push(SampledPieceGeom {
            id: p.id.clone(),
            gauge,
            has_catenary: p.has_catenary == Some(true),
            min_radius,
            connected_to,
            branches,
            min_x,
            max_x,
            min_y,
            max_y,
        });
    }

    let mut violations_map: HashMap<(String, String, u8), ClearanceViolation> = HashMap::new();

    for i in 0..geoms.len() {
        for j in (i + 1)..geoms.len() {
            let a = &geoms[i];
            let b = &geoms[j];
            if a.connected_to.contains(&b.id) || b.connected_to.contains(&a.id) {
                continue;
            }
            // Skip pairs sharing an immediate turnout/crossing junction neighbor (diverging branches naturally fan out)
            if a.connected_to.iter().any(|n| b.connected_to.contains(n)) {
                continue;
            }

            let scale_f = ((a.gauge + b.gauge) * 0.5 / 16.5).clamp(0.35, 3.6);
            let tight_curve = a.min_radius.map_or(false, |r| r <= 460.0 * scale_f)
                || b.min_radius.map_or(false, |r| r <= 460.0 * scale_f);
            let req_h = (if tight_curve { 52.0 } else { 44.0 }) * scale_f;

            if a.max_x + req_h < b.min_x
                || b.max_x + req_h < a.min_x
                || a.max_y + req_h < b.min_y
                || b.max_y + req_h < a.min_y
            {
                continue;
            }

            let mut worst_v: Option<(f64, f64, Vec2)> = None;
            let mut worst_h: Option<(f64, f64, Vec2)> = None;

            for br_a in &a.branches {
                for sa in br_a.windows(2) {
                    for br_b in &b.branches {
                        for sb in br_b.windows(2) {
                            let (dist2d, sc, tc, mid) = seg_seg_closest_points(
                                sa[0].0, sa[0].1, sa[1].0, sa[1].1, sb[0].0, sb[0].1, sb[1].0,
                                sb[1].1,
                            );
                            if dist2d > req_h {
                                continue;
                            }
                            let ea = sa[0].2 + (sa[1].2 - sa[0].2) * sc;
                            let eb = sb[0].2 + (sb[1].2 - sb[0].2) * tc;
                            let dz = (ea - eb).abs();
                            let lower_has_cat = if ea <= eb { a.has_catenary } else { b.has_catenary };
                            let req_v = (if lower_has_cat { 92.0 } else { 75.0 }) * scale_f;

                            if dist2d < (a.gauge + b.gauge) * 0.55 {
                                if dz >= 1.5 && dz < req_v {
                                    if worst_v.map_or(true, |(cur, _, _)| dz < cur) {
                                        worst_v = Some((dz, req_v, mid));
                                    }
                                }
                            } else if dz < req_v * 0.65 && dist2d < req_h {
                                if worst_h.map_or(true, |(cur, _, _)| dist2d < cur) {
                                    worst_h = Some((dist2d, req_h, mid));
                                }
                            }
                        }
                    }
                }
            }

            if let Some((actual_v, req_v, pos)) = worst_v {
                let key = (a.id.clone(), b.id.clone(), 0);
                violations_map.insert(
                    key,
                    ClearanceViolation {
                        kind: ClearanceKind::VerticalHeadroom,
                        pos,
                        piece_a: a.id.clone(),
                        piece_b: b.id.clone(),
                        actual_mm: actual_v,
                        required_mm: req_v,
                        label: format!("NEM 102: {actual_v:.0}<{req_v:.0}mm"),
                    },
                );
            }
            if let Some((actual_h, req_h_v, pos)) = worst_h {
                let key = (a.id.clone(), b.id.clone(), 1);
                violations_map.insert(
                    key,
                    ClearanceViolation {
                        kind: ClearanceKind::HorizontalCurve,
                        pos,
                        piece_a: a.id.clone(),
                        piece_b: b.id.clone(),
                        actual_mm: actual_h,
                        required_mm: req_h_v,
                        label: format!("NEM 103: {actual_h:.0}<{req_h_v:.0}mm"),
                    },
                );
            }
        }
    }

    let mut out: Vec<ClearanceViolation> = violations_map.into_values().collect();
    out.sort_by(|a, b| a.piece_a.cmp(&b.piece_a).then(a.piece_b.cmp(&b.piece_b)));
    out
}

pub fn draw_nem_clearance_overlay_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    violations: &[ClearanceViolation],
    is_de: bool,
) {
    if violations.is_empty() {
        return;
    }
    for v in violations {
        let sp = camera.world_to_screen(rect, v.pos.x, v.pos.y);
        if !rect.expand(48.0).contains(sp) {
            continue;
        }
        let col = match v.kind {
            ClearanceKind::VerticalHeadroom => Color32::from_rgb(239, 68, 68),
            ClearanceKind::HorizontalCurve => Color32::from_rgb(245, 158, 11),
        };
        let bg = Color32::from_rgba_unmultiplied(15, 23, 42, 230);
        painter.circle_stroke(sp, 14.0, Stroke::new(2.2_f32, col));
        painter.circle_filled(sp, 4.5, col);
        let tag_pos = Pos2::new(sp.x, sp.y - 22.0);
        let tag_r = Rect::from_center_size(tag_pos, EguiVec2::new(112.0, 17.0));
        painter.rect_filled(tag_r, 4.0, bg);
        painter.rect_stroke(tag_r, 4.0, Stroke::new(1.1_f32, col), egui::StrokeKind::Inside);
        painter.text(
            tag_pos,
            egui::Align2::CENTER_CENTER,
            &v.label,
            FontId::proportional(9.5),
            Color32::WHITE,
        );
    }

    let banner = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.min.y + 68.0),
        EguiVec2::new(360.0, 24.0),
    );
    painter.rect_filled(banner, 6.0, Color32::from_rgba_unmultiplied(120, 53, 15, 235));
    painter.rect_stroke(
        banner,
        6.0,
        Stroke::new(1.2_f32, Color32::from_rgb(251, 191, 36)),
        egui::StrokeKind::Inside,
    );
    let msg = if is_de {
        format!("Lichtraumprofil NEM 102/103: {} Konflikt(e)", violations.len())
    } else {
        format!("NEM 102/103 Clearance: {} conflict(s)", violations.len())
    };
    painter.text(
        banner.center(),
        egui::Align2::CENTER_CENTER,
        msg,
        FontId::proportional(11.0),
        Color32::from_rgb(254, 243, 199),
    );
}

// ============================================================================
//  4. REACHABILITY HEATMAP (Arm-Reach Analysis)
// ============================================================================

fn point_in_polygon(pt: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let xi = poly[i].x;
        let yi = poly[i].y;
        let xj = poly[j].x;
        let yj = poly[j].y;
        let intersect = ((yi > pt.y) != (yj > pt.y))
            && (pt.x < (xj - xi) * (pt.y - yi) / ((yj - yi).abs().max(1e-9) * (yj - yi).signum()) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn min_dist_to_polygon_edges(pt: Vec2, poly: &[Vec2]) -> f64 {
    let n = poly.len();
    if n < 2 {
        return f64::INFINITY;
    }
    let mut best = f64::INFINITY;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let vx = b.x - a.x;
        let vy = b.y - a.y;
        let len_sq = vx * vx + vy * vy;
        let t = if len_sq < 1e-6 {
            0.0
        } else {
            (((pt.x - a.x) * vx + (pt.y - a.y) * vy) / len_sq).clamp(0.0, 1.0)
        };
        let dx = pt.x - (a.x + t * vx);
        let dy = pt.y - (a.y + t * vy);
        best = best.min(dx.hypot(dy));
    }
    best
}

#[cfg(test)]
pub fn compute_unreachable_area_pct(layout: &Layout, max_reach_mm: f64) -> f32 {
    let boards = get_baseboards_slice(layout);
    let mut total = 0usize;
    let mut unreachable = 0usize;
    let step = 65.0_f64;

    for b in boards {
        if !b.enabled {
            continue;
        }
        let poly = get_baseboard_polygon(&b);
        if poly.len() < 3 {
            continue;
        }
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &poly {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let mut x = min_x + step * 0.5;
        while x < max_x {
            let mut y = min_y + step * 0.5;
            while y < max_y {
                let pt = Vec2 { x, y };
                if point_in_polygon(pt, &poly) {
                    total += 1;
                    if min_dist_to_polygon_edges(pt, &poly) > max_reach_mm {
                        unreachable += 1;
                    }
                }
                y += step;
            }
            x += step;
        }
    }
    if total == 0 {
        0.0
    } else {
        (unreachable as f32 / total as f32) * 100.0
    }
}

pub fn draw_reachability_heatmap_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    layout: &Layout,
    max_reach_mm: f64,
    is_de: bool,
) -> f32 {
    let boards = get_baseboards_slice(layout);
    let step = 65.0_f64;
    let mut total = 0usize;
    let mut unreachable = 0usize;

    for b in boards {
        if !b.enabled {
            continue;
        }
        let poly = get_baseboard_polygon(&b);
        if poly.len() < 3 {
            continue;
        }
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &poly {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let mut x = min_x + step * 0.5;
        while x < max_x {
            let mut y = min_y + step * 0.5;
            while y < max_y {
                let pt = Vec2 { x, y };
                if point_in_polygon(pt, &poly) {
                    total += 1;
                    let d = min_dist_to_polygon_edges(pt, &poly);
                    if d > max_reach_mm {
                        unreachable += 1;
                    }
                    let col = if d <= max_reach_mm * 0.72 {
                        Color32::from_rgba_unmultiplied(34, 197, 94, 42)
                    } else if d <= max_reach_mm {
                        Color32::from_rgba_unmultiplied(245, 158, 11, 56)
                    } else {
                        Color32::from_rgba_unmultiplied(239, 68, 68, 78)
                    };
                    let p0 = camera.world_to_screen(rect, x - step * 0.5, y - step * 0.5);
                    let p1 = camera.world_to_screen(rect, x + step * 0.5, y + step * 0.5);
                    let cell_r = Rect::from_two_pos(p0, p1);
                    if cell_r.intersects(rect) {
                        painter.rect_filled(cell_r, 0.0, col);
                    }
                }
                y += step;
            }
            x += step;
        }
    }

    let pct = if total == 0 {
        0.0
    } else {
        (unreachable as f32 / total as f32) * 100.0
    };

    // Legend card bottom-left
    let leg_r = Rect::from_min_size(Pos2::new(rect.min.x + 18.0, rect.max.y - 44.0), EguiVec2::new(310.0, 26.0));
    painter.rect_filled(leg_r, 6.0, Color32::from_rgba_unmultiplied(15, 23, 42, 220));
    painter.rect_stroke(
        leg_r,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, Color32::from_rgb(71, 85, 105)),
        egui::StrokeKind::Inside,
    );
    let txt = if is_de {
        format!(
            "Greifweite <= {:.0} cm  |  {:.0}% nicht erreichbar",
            max_reach_mm / 10.0,
            pct
        )
    } else {
        format!(
            "Reachability <= {:.0} cm  |  {:.0}% out of reach",
            max_reach_mm / 10.0,
            pct
        )
    };
    painter.text(
        leg_r.center(),
        egui::Align2::CENTER_CENTER,
        txt,
        FontId::proportional(11.0),
        Color32::WHITE,
    );

    pct
}

