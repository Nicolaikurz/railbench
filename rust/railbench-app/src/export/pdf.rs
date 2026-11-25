use super::*;

fn hex_to_pdf_rgb(hex: &str) -> (f64, f64, f64) {
    let h = hex.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
        }
    }
    (0.22, 0.74, 0.97)
}

fn pdf_track_color(kind: &str, is_road: bool, is_bridge: bool, is_tunnel: bool, monochrome: bool) -> (f64, f64, f64) {
    if monochrome {
        return (0.12, 0.14, 0.18);
    }
    if is_road {
        return (0.28, 0.33, 0.41);
    }
    if is_bridge {
        return (0.09, 0.56, 0.55);
    }
    if is_tunnel {
        return (0.45, 0.38, 0.62);
    }
    match kind {
        "curve" => (0.15, 0.39, 0.92),
        "turnout-left" | "turnout-right" | "curved-turnout-left" | "curved-turnout-right" | "three-way" | "double-slip" => {
            (0.85, 0.47, 0.02)
        }
        "crossing" => (0.58, 0.25, 0.82),
        "flex" => (0.03, 0.57, 0.70),
        "feeder" => (0.09, 0.64, 0.29),
        _ => (0.02, 0.45, 0.76),
    }
}

struct PdfBadge<'a> {
    name: &'a str,
    color_rgb: (f64, f64, f64),
    elev_line: String,
    count_line: String,
}

fn write_pdf_page_header(
    s: &mut String,
    page_w: f64,
    page_h: f64,
    margin: f64,
    title: &str,
    subtitle: &str,
    badge: &PdfBadge<'_>,
) -> f64 {
    let top_y = page_h - margin;

    // Brand tag (matches Electron print-pdf.ts: #0284c7 bold)
    let _ = writeln!(
        s,
        "0.01 0.52 0.78 rg BT /F2 9 Tf {margin:.1} {:.1} Td (RAILbench Track Planner {}) Tj ET",
        top_y - 4.0,
        crate::version::APP_VERSION_TAG
    );
    // Bold layout title (#0f172a)
    let _ = writeln!(
        s,
        "0.06 0.09 0.16 rg BT /F2 16 Tf {margin:.1} {:.1} Td ({}) Tj ET",
        top_y - 22.0,
        pdf_escape(title)
    );
    // Subtitle (#64748b)
    let _ = writeln!(
        s,
        "0.39 0.45 0.55 rg BT /F1 8.5 Tf {margin:.1} {:.1} Td ({}) Tj ET",
        top_y - 35.0,
        pdf_escape(subtitle)
    );

    // Top-right Layer / Inventory Badge Card (#f8fafc fill, #e2e8f0 border, thick left accent bar)
    let badge_w = 185.0_f64.min((page_w - margin * 2.0) * 0.40);
    let badge_h = 40.0_f64;
    let badge_x = page_w - margin - badge_w;
    let badge_y = top_y - 38.0;
    let _ = writeln!(
        s,
        "0.97 0.98 0.99 rg 0.89 0.91 0.94 RG 0.7 w {badge_x:.1} {badge_y:.1} {badge_w:.1} {badge_h:.1} re B"
    );
    let (cr, cg, cb) = badge.color_rgb;
    let _ = writeln!(
        s,
        "{cr:.3} {cg:.3} {cb:.3} rg {badge_x:.1} {badge_y:.1} 3.5 {badge_h:.1} re f"
    );
    let tx = badge_x + 9.0;
    let b_name: String = badge.name.chars().take(28).collect();
    let _ = writeln!(
        s,
        "0.06 0.09 0.16 rg BT /F2 9 Tf {tx:.1} {:.1} Td ({}) Tj ET",
        badge_y + 27.5,
        pdf_escape(&b_name)
    );
    let _ = writeln!(
        s,
        "0.20 0.25 0.33 rg BT /F1 7.8 Tf {tx:.1} {:.1} Td ({}) Tj ET",
        badge_y + 16.0,
        pdf_escape(&badge.elev_line)
    );
    let _ = writeln!(
        s,
        "0.39 0.45 0.55 rg BT /F1 7.2 Tf {tx:.1} {:.1} Td ({}) Tj ET",
        badge_y + 5.5,
        pdf_escape(&badge.count_line)
    );

    // Crisp dark header divider rule (#0f172a)
    let rule_y = top_y - 45.0;
    let _ = writeln!(
        s,
        "0.06 0.09 0.16 RG 1.4 w {margin:.1} {rule_y:.1} m {:.1} {rule_y:.1} l S",
        page_w - margin
    );
    rule_y
}

fn write_pdf_page_footer(
    s: &mut String,
    page_w: f64,
    margin: f64,
    left_text: &str,
    right_page_text: &str,
) -> f64 {
    let footer_rule_y = margin + 22.0;
    let _ = writeln!(
        s,
        "0.80 0.84 0.88 RG 0.7 w {margin:.1} {footer_rule_y:.1} m {:.1} {footer_rule_y:.1} l S",
        page_w - margin
    );
    let left_trimmed: String = left_text.chars().take(105).collect();
    let _ = writeln!(
        s,
        "0.28 0.33 0.41 rg BT /F1 7.8 Tf {margin:.1} {:.1} Td ({}) Tj ET",
        margin + 9.0,
        pdf_escape(&left_trimmed)
    );
    let approx_w = right_page_text.len() as f64 * 4.4;
    let rx = (page_w - margin - approx_w).max(margin + 200.0);
    let _ = writeln!(
        s,
        "0.39 0.45 0.55 rg BT /F1 7.8 Tf {rx:.1} {:.1} Td ({}) Tj ET",
        margin + 9.0,
        pdf_escape(right_page_text)
    );
    footer_rule_y
}

fn draw_pdf_plan_page(
    layout: &Layout,
    catalogs: &[Catalog],
    opts: &BlueprintExportOptions,
    filter_layer_id: Option<&str>,
    subtitle: &str,
    badge: &PdfBadge<'_>,
    footer_left: &str,
    page_footer: &str,
    page_w: f64,
    page_h: f64,
) -> String {
    let margin = 28.0_f64;
    let mut s = String::with_capacity(8192);

    // Pure white page background
    let _ = writeln!(s, "1 1 1 rg 0 0 {page_w:.1} {page_h:.1} re f");

    let header_rule_y = write_pdf_page_header(&mut s, page_w, page_h, margin, &layout.name, subtitle, badge);
    let footer_rule_y = write_pdf_page_footer(&mut s, page_w, margin, footer_left, page_footer);

    // Framed plan viewport between header and footer
    let body_x = margin;
    let body_y = footer_rule_y + 8.0;
    let body_w = page_w - margin * 2.0;
    let body_h = (header_rule_y - 8.0 - body_y).max(80.0);
    let _ = writeln!(
        s,
        "0.89 0.91 0.94 RG 0.7 w {body_x:.1} {body_y:.1} {body_w:.1} {body_h:.1} re S"
    );

    let (min_x, max_x, min_y, max_y) = layout_bounds(layout, catalogs);
    let span_x = (max_x - min_x).max(100.0);
    let span_y = (max_y - min_y).max(100.0);
    let inner_pad = 14.0_f64;
    let scale = ((body_w - inner_pad * 2.0) / span_x).min((body_h - inner_pad * 2.0) / span_y);
    let off_x = body_x + (body_w - span_x * scale) * 0.5;
    let off_y = body_y + (body_h - span_y * scale) * 0.5;

    let to_pdf = |wx: f64, wy: f64| -> (f64, f64) {
        let px = off_x + (wx - min_x) * scale;
        let py = body_y + body_h - (off_y - body_y) - (wy - min_y) * scale;
        (px, py)
    };

    if opts.show_grid {
        let _ = writeln!(s, "q {body_x:.1} {body_y:.1} {body_w:.1} {body_h:.1} re W n");
        let mut gx = (min_x / 100.0).floor() * 100.0;
        while gx <= max_x {
            let is_major = (gx / 500.0 - (gx / 500.0).round()).abs() < 1e-6;
            let (px, _) = to_pdf(gx, min_y);
            if is_major {
                let _ = writeln!(s, "0.80 0.84 0.89 RG 0.45 w {px:.2} {body_y:.2} m {px:.2} {:.2} l S", body_y + body_h);
            } else {
                let _ = writeln!(s, "0.91 0.93 0.96 RG 0.25 w {px:.2} {body_y:.2} m {px:.2} {:.2} l S", body_y + body_h);
            }
            gx += 100.0;
        }
        let mut gy = (min_y / 100.0).floor() * 100.0;
        while gy <= max_y {
            let is_major = (gy / 500.0 - (gy / 500.0).round()).abs() < 1e-6;
            let (_, py) = to_pdf(min_x, gy);
            if is_major {
                let _ = writeln!(s, "0.80 0.84 0.89 RG 0.45 w {body_x:.2} {py:.2} m {:.2} {py:.2} l S", body_x + body_w);
            } else {
                let _ = writeln!(s, "0.91 0.93 0.96 RG 0.25 w {body_x:.2} {py:.2} m {:.2} {py:.2} l S", body_x + body_w);
            }
            gy += 100.0;
        }
        let _ = writeln!(s, "Q");
    }

    if opts.show_baseboards {
        for bb in get_baseboards_slice(layout) {
            if !bb.enabled {
                continue;
            }
            let poly = get_baseboard_polygon(&bb);
            if poly.len() < 3 {
                continue;
            }
            let _ = write!(s, "0.97 0.98 0.99 rg 0.58 0.64 0.72 RG 0.9 w ");
            for (k, pt) in poly.iter().enumerate() {
                let (px, py) = to_pdf(pt.x, pt.y);
                let _ = write!(s, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
            }
            let _ = writeln!(s, "h B");
        }
    }

    // Topographic terrain & mountain underlay drawn UNDERNEATH tracks so all tracks/tunnels stay 100% visible
    let write_ellipse = |st: &mut String, cx: f64, cy: f64, rx: f64, ry: f64, rot_deg: f64, op: &str| {
        let steps = 32usize;
        for k in 0..steps {
            let a = (k as f64 / steps as f64) * std::f64::consts::TAU;
            let rp = rotate(Vec2 { x: rx * a.cos(), y: ry * a.sin() }, rot_deg);
            let (px, py) = to_pdf(cx + rp.x, cy + rp.y);
            let _ = write!(st, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
        }
        let _ = writeln!(st, "h {op}");
    };

    if let Some(models) = &layout.models {
        for m in models {
            let m_lid = m.layer_id.as_deref().unwrap_or("layer-1");
            if let Some(flid) = filter_layer_id {
                if flid != m_lid {
                    continue;
                }
            }
            let is_mountain = m.builtin_id.as_deref() == Some("mountain");
            let is_ground = m.builtin_id.as_deref() == Some("ground-terrain");
            if !is_mountain && !is_ground {
                continue;
            }
            let sc = m.scale.unwrap_or(1.0);
            let rx = (m.width * sc * 0.5).max(12.0);
            let ry = (m.depth * sc * 0.5).max(12.0);
            let (cx, cy, rot) = (m.transform.x, m.transform.y, m.transform.rotation);

            if is_ground {
                let (fr, fg, fb) = if opts.monochrome {
                    (0.95, 0.95, 0.95)
                } else {
                    match m.ground_terrain_type.as_deref().unwrap_or("grass") {
                        "water" => (0.87, 0.95, 0.99),
                        "sand" => (0.98, 0.95, 0.87),
                        "dirt" => (0.94, 0.90, 0.85),
                        _ => (0.92, 0.97, 0.91),
                    }
                };
                let _ = writeln!(s, "{fr:.3} {fg:.3} {fb:.3} rg");
                if let Some(nodes) = &m.mountain_path {
                    if !nodes.is_empty() {
                        for nd in nodes {
                            let rp = rotate(Vec2 { x: nd.x, y: nd.y }, rot);
                            let r = nd.radius.max(10.0);
                            write_ellipse(&mut s, cx + rp.x, cy + rp.y, r, r, 0.0, "f");
                        }
                        continue;
                    }
                }
                write_ellipse(&mut s, cx, cy, rx, ry, rot, "f");
            } else {
                // Mountain: soft airy topographic fill + inner contour rings (Höhenlinien) UNDERNEATH tracks
                let (fr, fg, fb) = if opts.monochrome { (0.94, 0.94, 0.94) } else { (0.93, 0.96, 0.92) };
                let (cr, cg, cb) = if opts.monochrome { (0.68, 0.70, 0.72) } else { (0.52, 0.67, 0.50) };
                let _ = writeln!(s, "{fr:.3} {fg:.3} {fb:.3} rg");
                if let Some(nodes) = &m.mountain_path {
                    if !nodes.is_empty() {
                        for nd in nodes {
                            let rp = rotate(Vec2 { x: nd.x, y: nd.y }, rot);
                            let r = nd.radius.max(12.0);
                            write_ellipse(&mut s, cx + rp.x, cy + rp.y, r, r, 0.0, "f");
                        }
                        // Inner topographic contour rings on each ridge node
                        let _ = writeln!(s, "{cr:.3} {cg:.3} {cb:.3} RG 0.45 w [2.5 2.0] 0 d");
                        for nd in nodes {
                            let rp = rotate(Vec2 { x: nd.x, y: nd.y }, rot);
                            let r = nd.radius.max(12.0);
                            write_ellipse(&mut s, cx + rp.x, cy + rp.y, r * 0.68, r * 0.68, 0.0, "S");
                            write_ellipse(&mut s, cx + rp.x, cy + rp.y, r * 0.36, r * 0.36, 0.0, "S");
                        }
                        let _ = writeln!(s, "[] 0 d");
                        continue;
                    }
                }
                write_ellipse(&mut s, cx, cy, rx, ry, rot, "f");
                let _ = writeln!(s, "{cr:.3} {cg:.3} {cb:.3} RG 0.45 w [2.5 2.0] 0 d");
                write_ellipse(&mut s, cx, cy, rx * 0.70, ry * 0.70, rot, "S");
                write_ellipse(&mut s, cx, cy, rx * 0.40, ry * 0.40, rot, "S");
                let _ = writeln!(s, "[] 0 d");
            }
        }
    }

    let draw_pieces = |stream: &mut String, ghost: bool| {
        let ballast_w = (22.0 * scale).clamp(1.2, 8.5);
        for p in &layout.pieces {
            let p_lid = p.layer_id.as_deref().unwrap_or("layer-1");
            let matches = filter_layer_id.map_or(true, |lid| lid == p_lid);
            if ghost && matches {
                continue;
            }
            if !ghost && !matches {
                continue;
            }
            let entry = find_entry(catalogs, &p.catalog_id);
            let kind = entry.map(|e| e.kind.as_str()).unwrap_or("");
            let is_road = is_road_id(catalogs, &p.catalog_id);

            if kind == "turntable" {
                let outer_r = crate::turntables_3d::turntable_outer_radius(p, entry);
                let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
                let pit_r = 139.0 * scale_f;
                let ring_rail_r = 124.0 * scale_f;
                let (cx, cy) = (p.transform.x, p.transform.y);
                let stroke_circle = |st: &mut String, rad: f64, op: &str| {
                    let steps = 36usize;
                    for k in 0..steps {
                        let a = (k as f64 / steps as f64) * std::f64::consts::TAU;
                        let (px, py) = to_pdf(cx + rad * a.cos(), cy + rad * a.sin());
                        let _ = write!(st, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
                    }
                    let _ = writeln!(st, "h {op}");
                };
                if ghost {
                    let _ = writeln!(stream, "0.90 0.92 0.95 rg 0.78 0.81 0.86 RG 0.7 w");
                    stroke_circle(stream, outer_r, "B");
                    stroke_circle(stream, pit_r, "S");
                } else {
                    if opts.monochrome {
                        let _ = writeln!(stream, "0.78 0.81 0.85 rg 0.30 0.35 0.42 RG 0.9 w");
                    } else {
                        let _ = writeln!(stream, "0.42 0.47 0.54 rg 0.28 0.33 0.41 RG 0.9 w");
                    }
                    stroke_circle(stream, outer_r, "B");
                    if opts.monochrome {
                        let _ = writeln!(stream, "0.88 0.91 0.94 rg 0.35 0.40 0.48 RG 0.8 w");
                    } else {
                        let _ = writeln!(stream, "0.12 0.16 0.23 rg 0.38 0.45 0.54 RG 0.8 w");
                    }
                    stroke_circle(stream, pit_r, "B");
                    let _ = writeln!(stream, "0.58 0.64 0.72 RG 0.7 w");
                    stroke_circle(stream, ring_rail_r, "S");
                    let (tr, tg, tb) = pdf_track_color(kind, false, false, false, opts.monochrome);
                    let _ = writeln!(stream, "{tr:.3} {tg:.3} {tb:.3} RG 0.9 w");
                    for br in sample_track_branches_world(p, entry) {
                        if br.len() < 2 {
                            continue;
                        }
                        for (k, pt) in br.iter().enumerate() {
                            let (px, py) = to_pdf(pt.x, pt.y);
                            let _ = write!(stream, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
                        }
                        let _ = writeln!(stream, "S");
                    }
                    if opts.show_labels && scale >= 0.08 {
                        let lrp = rotate(Vec2 { x: 0.0, y: -pit_r * 0.45 }, p.transform.rotation);
                        let (lx, ly) = to_pdf(cx + lrp.x, cy + lrp.y);
                        if opts.monochrome {
                            let _ = writeln!(stream, "0.08 0.12 0.20 rg ");
                        } else {
                            let _ = writeln!(stream, "0.95 0.97 0.99 rg ");
                        }
                        let _ = writeln!(
                            stream,
                            "BT /F2 5.5 Tf {lx:.1} {ly:.1} Td ({}) Tj ET",
                            pdf_escape(&p.catalog_id)
                        );
                    }
                }
                continue;
            }

            if kind == "transfer-table" {
                let outer_cs = crate::turntables_3d::transfer_table_outer_world_corners(p, entry);
                let pit_cs = crate::turntables_3d::transfer_table_world_rect_corners(
                    p,
                    entry.expect("transfer-table entry exists"),
                    0.0,
                    0.0,
                );
                let write_poly = |st: &mut String, pts: &[Vec2; 4], op: &str| {
                    for (k, pt) in pts.iter().enumerate() {
                        let (px, py) = to_pdf(pt.x, pt.y);
                        let _ = write!(st, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
                    }
                    let _ = writeln!(st, "h {op}");
                };
                if ghost {
                    let _ = writeln!(stream, "0.90 0.92 0.95 rg 0.78 0.81 0.86 RG 0.7 w");
                    write_poly(stream, &outer_cs, "B");
                    write_poly(stream, &pit_cs, "S");
                } else {
                    if opts.monochrome {
                        let _ = writeln!(stream, "0.78 0.81 0.85 rg 0.30 0.35 0.42 RG 0.9 w");
                    } else {
                        let _ = writeln!(stream, "0.42 0.47 0.54 rg 0.28 0.33 0.41 RG 0.9 w");
                    }
                    write_poly(stream, &outer_cs, "B");
                    if opts.monochrome {
                        let _ = writeln!(stream, "0.88 0.91 0.94 rg 0.35 0.40 0.48 RG 0.8 w");
                    } else {
                        let _ = writeln!(stream, "0.12 0.16 0.23 rg 0.38 0.45 0.54 RG 0.8 w");
                    }
                    write_poly(stream, &pit_cs, "B");
                    let (_, l, top_z, bot_z) = crate::turntables_3d::transfer_table_local_bounds(entry);
                    let to_w = |lx: f64, ly: f64| -> Vec2 {
                        let rp = rotate(Vec2 { x: lx, y: ly }, p.transform.rotation);
                        Vec2 { x: p.transform.x + rp.x, y: p.transform.y + rp.y }
                    };
                    let _ = writeln!(stream, "0.58 0.64 0.72 RG 0.7 w");
                    for &rx in &[80.0, l * 0.5, l - 80.0] {
                        let a = to_w(rx, top_z + 12.0);
                        let b = to_w(rx, bot_z - 12.0);
                        let (ax, ay) = to_pdf(a.x, a.y);
                        let (bx, by) = to_pdf(b.x, b.y);
                        let _ = writeln!(stream, "{ax:.2} {ay:.2} m {bx:.2} {by:.2} l S");
                    }
                    let (tr, tg, tb) = pdf_track_color(kind, false, false, false, opts.monochrome);
                    let _ = writeln!(stream, "{tr:.3} {tg:.3} {tb:.3} RG 0.9 w");
                    for br in sample_track_branches_world(p, entry) {
                        if br.len() < 2 {
                            continue;
                        }
                        for (k, pt) in br.iter().enumerate() {
                            let (px, py) = to_pdf(pt.x, pt.y);
                            let _ = write!(stream, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
                        }
                        let _ = writeln!(stream, "S");
                    }
                    if opts.show_labels && scale >= 0.08 {
                        let track_zs = crate::turntables_3d::extract_transfer_track_zs(entry);
                        let bridge_z = track_zs[track_zs.len() / 2] as f64;
                        let lp = to_w(l * 0.5, bridge_z - 56.0);
                        let (lx, ly) = to_pdf(lp.x, lp.y);
                        if opts.monochrome {
                            let _ = writeln!(stream, "0.08 0.12 0.20 rg ");
                        } else {
                            let _ = writeln!(stream, "0.95 0.97 0.99 rg ");
                        }
                        let _ = writeln!(
                            stream,
                            "BT /F2 5.5 Tf {lx:.1} {ly:.1} Td ({}) Tj ET",
                            pdf_escape(&p.catalog_id)
                        );
                    }
                }
                continue;
            }

            let (tr, tg, tb) = pdf_track_color(
                kind,
                is_road,
                p.has_bridge == Some(true),
                p.has_tunnel == Some(true),
                opts.monochrome,
            );
            if ghost {
                let _ = writeln!(stream, "1 J 0.87 0.89 0.92 RG {ballast_w:.2} w");
            } else if opts.monochrome {
                let _ = writeln!(stream, "1 J 0.78 0.80 0.84 RG {ballast_w:.2} w");
            } else if is_road {
                let _ = writeln!(stream, "1 J 0.34 0.39 0.46 RG {ballast_w:.2} w");
            } else {
                // Soft tinted roadbed halo matching track type color
                let br = 0.72 + tr * 0.22;
                let bg = 0.74 + tg * 0.20;
                let bb = 0.78 + tb * 0.18;
                let _ = writeln!(stream, "1 J {br:.3} {bg:.3} {bb:.3} RG {ballast_w:.2} w");
            }
            let branches = sample_track_branches_world(p, entry);
            for br in &branches {
                if br.len() < 2 {
                    continue;
                }
                for (k, pt) in br.iter().enumerate() {
                    let (px, py) = to_pdf(pt.x, pt.y);
                    let _ = write!(stream, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
                }
                let _ = writeln!(stream, "S");
            }
            if !ghost {
                let rail_w = (ballast_w * 0.26).clamp(0.7, 1.8);
                let is_tun = p.has_tunnel == Some(true);
                if is_tun {
                    let _ = writeln!(stream, "{tr:.3} {tg:.3} {tb:.3} RG {rail_w:.2} w [3.5 2.5] 0 d");
                } else {
                    let _ = writeln!(stream, "{tr:.3} {tg:.3} {tb:.3} RG {rail_w:.2} w");
                }
                for br in &branches {
                    if br.len() < 2 {
                        continue;
                    }
                    for (k, pt) in br.iter().enumerate() {
                        let (px, py) = to_pdf(pt.x, pt.y);
                        let _ = write!(stream, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
                    }
                    let _ = writeln!(stream, "S");
                }
                if is_tun {
                    let _ = writeln!(stream, "[] 0 d");
                }
                if opts.show_labels && p.connectors.len() >= 2 && scale >= 0.08 {
                    let (lx, ly) = to_pdf(
                        (p.connectors[0].x + p.connectors[1].x) * 0.5,
                        (p.connectors[0].y + p.connectors[1].y) * 0.5,
                    );
                    let lbl = &p.catalog_id;
                    let pill_w = (lbl.len() as f64 * 3.3 + 4.0).max(16.0);
                    let pill_h = 7.5_f64;
                    let _ = writeln!(
                        stream,
                        "1 1 1 rg 0.82 0.85 0.90 RG 0.35 w {:.1} {:.1} {pill_w:.1} {pill_h:.1} re B",
                        lx - pill_w * 0.5,
                        ly - pill_h * 0.5
                    );
                    let _ = writeln!(
                        stream,
                        "0.06 0.09 0.16 rg BT /F2 5.2 Tf {:.1} {:.1} Td ({}) Tj ET",
                        lx - pill_w * 0.5 + 2.0,
                        ly - 1.8,
                        pdf_escape(lbl)
                    );
                }
                if opts.show_electrical && !p.connectors.is_empty() && scale >= 0.05 {
                    let mx_w = p.connectors.iter().map(|c| c.x).sum::<f64>() / p.connectors.len() as f64;
                    let my_w = p.connectors.iter().map(|c| c.y).sum::<f64>() / p.connectors.len() as f64;
                    let (ex, ey) = to_pdf(mx_w, my_w);
                    if p.power_feed == Some(true) {
                        if opts.monochrome {
                            let _ = writeln!(stream, "0.10 0.12 0.16 rg ");
                        } else {
                            let _ = writeln!(stream, "0.09 0.64 0.29 rg ");
                        }
                        let _ = writeln!(
                            stream,
                            "BT /F2 5.5 Tf {:.1} {:.1} Td (+PWR) Tj ET",
                            ex - 8.0,
                            ey + 5.5
                        );
                    }
                    if p.is_isolator == Some(true) {
                        let iso_c = p
                            .connectors
                            .get(p.isolated_connector_index.unwrap_or(0))
                            .unwrap_or(&p.connectors[0]);
                        let rad = (iso_c.angle + 90.0).to_radians();
                        let (dx, dy) = (rad.cos() * 14.0, rad.sin() * 14.0);
                        let (ax, ay) = to_pdf(iso_c.x - dx, iso_c.y - dy);
                        let (bx, by) = to_pdf(iso_c.x + dx, iso_c.y + dy);
                        if opts.monochrome {
                            let _ = writeln!(stream, "0.10 0.12 0.16 RG 1.4 w {ax:.2} {ay:.2} m {bx:.2} {by:.2} l S");
                        } else {
                            let _ = writeln!(stream, "0.86 0.15 0.15 RG 1.4 w {ax:.2} {ay:.2} m {bx:.2} {by:.2} l S");
                        }
                    }
                    if p.has_signal == Some(true) {
                        if opts.monochrome {
                            let _ = writeln!(stream, "0.10 0.12 0.16 rg ");
                        } else {
                            let _ = writeln!(stream, "0.85 0.47 0.02 rg ");
                        }
                        let _ = writeln!(
                            stream,
                            "BT /F2 5.5 Tf {:.1} {:.1} Td (SIG) Tj ET",
                            ex + 6.0,
                            ey + 5.5
                        );
                    }
                    if let Some(el) = p.elevation {
                        let avg_z = (el.start + el.end) * 0.5;
                        if avg_z.abs() >= 1.0 {
                            let sign = if avg_z >= 0.0 { "+" } else { "" };
                            let _ = writeln!(
                                stream,
                                "0.01 0.41 0.63 rg BT /F1 4.8 Tf {:.1} {:.1} Td ({sign}{avg_z:.0}mm) Tj ET",
                                ex - 8.0,
                                ey - 7.5
                            );
                        }
                    }
                }
            }
        }
    };

    if filter_layer_id.is_some() && opts.ghost_other_layers {
        draw_pieces(&mut s, true);
    }
    draw_pieces(&mut s, false);

    if let Some(models) = &layout.models {
        for m in models {
            let m_lid = m.layer_id.as_deref().unwrap_or("layer-1");
            if let Some(flid) = filter_layer_id {
                if flid != m_lid {
                    continue;
                }
            }
            if m.builtin_id.as_deref() == Some("ground-terrain") {
                continue;
            }
            if m.builtin_id.as_deref() == Some("mountain") {
                // Stroke-only dashed topographic outline on top (NO fill!) so tracks underneath remain 100% visible
                let sc = m.scale.unwrap_or(1.0);
                let rx = (m.width * sc * 0.5).max(12.0);
                let ry = (m.depth * sc * 0.5).max(12.0);
                let (cx, cy, rot) = (m.transform.x, m.transform.y, m.transform.rotation);
                let (or, og, ob) = if opts.monochrome { (0.32, 0.35, 0.38) } else { (0.24, 0.45, 0.26) };
                let _ = writeln!(s, "{or:.3} {og:.3} {ob:.3} RG 0.9 w [4.0 2.5] 0 d");
                if let Some(nodes) = &m.mountain_path {
                    if !nodes.is_empty() {
                        for nd in nodes {
                            let rp = rotate(Vec2 { x: nd.x, y: nd.y }, rot);
                            let r = nd.radius.max(12.0);
                            write_ellipse(&mut s, cx + rp.x, cy + rp.y, r, r, 0.0, "S");
                        }
                    } else {
                        write_ellipse(&mut s, cx, cy, rx, ry, rot, "S");
                    }
                } else {
                    write_ellipse(&mut s, cx, cy, rx, ry, rot, "S");
                }
                let _ = writeln!(s, "[] 0 d");
                continue;
            }
            if !opts.show_models {
                continue;
            }
            // Render real 2D top-view architectural building mesh (matches 2D canvas view!)
            let top_mesh = crate::canvas2d::build_model_top_view_mesh_2d(m);
            if !top_mesh.tris.is_empty() {
                let rad = m.transform.rotation.to_radians();
                let (sin_r, cos_r) = rad.sin_cos();
                for (pts, cols) in &top_mesh.tris {
                    let r_avg = (cols[0].r() as f64 + cols[1].r() as f64 + cols[2].r() as f64) / 765.0;
                    let g_avg = (cols[0].g() as f64 + cols[1].g() as f64 + cols[2].g() as f64) / 765.0;
                    let b_avg = (cols[0].b() as f64 + cols[1].b() as f64 + cols[2].b() as f64) / 765.0;
                    let (cr, cg, cb) = if opts.monochrome {
                        let lum = (0.299 * r_avg + 0.587 * g_avg + 0.114 * b_avg).clamp(0.12, 0.96);
                        (lum, lum, lum)
                    } else {
                        (r_avg, g_avg, b_avg)
                    };
                    let mut p2 = [(0.0_f64, 0.0_f64); 3];
                    for k in 0..3 {
                        let lx = pts[k][0] as f64;
                        let ly = pts[k][1] as f64;
                        let wx = m.transform.x + lx * cos_r - ly * sin_r;
                        let wy = m.transform.y + lx * sin_r + ly * cos_r;
                        p2[k] = to_pdf(wx, wy);
                    }
                    let _ = writeln!(
                        s,
                        "{cr:.3} {cg:.3} {cb:.3} rg {:.2} {:.2} m {:.2} {:.2} l {:.2} {:.2} l h f",
                        p2[0].0, p2[0].1, p2[1].0, p2[1].1, p2[2].0, p2[2].1
                    );
                }
            } else {
                let cs = model_corners_world(m);
                let _ = write!(s, "0.90 0.94 0.99 rg 0.25 0.33 0.45 RG 0.8 w ");
                for (k, pt) in cs.iter().enumerate() {
                    let (px, py) = to_pdf(pt.x, pt.y);
                    let _ = write!(s, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
                }
                let _ = writeln!(s, "h B");
            }
        }
    }

    // Scale Bar (Maßstabsbalken) in bottom-left of plan viewport (matches Electron print.ts:200-210)
    let bar_mm = if span_x > 1500.0 { 500.0 } else { 200.0 };
    let bar_pt = (bar_mm * scale).clamp(32.0, 140.0);
    let bx = body_x + 12.0;
    let by = body_y + 12.0;
    let _ = writeln!(
        s,
        "0.20 0.25 0.33 RG 1.2 w {bx:.1} {by:.1} m {:.1} {by:.1} l {bx:.1} {:.1} m {bx:.1} {:.1} l {:.1} {:.1} m {:.1} {:.1} l S",
        bx + bar_pt,
        by - 3.5,
        by + 3.5,
        bx + bar_pt,
        by - 3.5,
        bx + bar_pt,
        by + 3.5
    );
    let _ = writeln!(
        s,
        "0.20 0.25 0.33 rg BT /F2 7 Tf {:.1} {:.1} Td ({:.0} mm) Tj ET",
        bx + bar_pt + 5.0,
        by - 2.3,
        bar_mm
    );

    s
}

/// Renders one or more styled Bill of Materials pages matching Electron's print-pdf.ts table layout.
pub(super) fn draw_pdf_bom_pages(
    layout: &Layout,
    catalogs: &[Catalog],
    page_w: f64,
    page_h: f64,
    is_de: bool,
    start_page_num: usize,
    total_pages_hint: usize,
) -> Vec<String> {
    let sum = get_piece_list_summary(layout, catalogs, is_de);
    let margin = 28.0_f64;

    // Collect unified table rows: (part_no, desc, category, spec, qty_str, len_str)
    let mut rows: Vec<(String, String, String, String, String, String)> = Vec::new();
    for it in sum.tracks.items.iter().chain(sum.roads.items.iter()) {
        rows.push((
            it.catalog_id.clone(),
            it.name.chars().take(40).collect(),
            it.category.chars().take(18).collect(),
            it.specification.chars().take(24).collect(),
            format!("{}", it.count),
            format!("{:.2} m", it.total_length_meters),
        ));
    }
    for bm in &sum.buildings {
        rows.push((
            "3D".into(),
            bm.name.chars().take(40).collect(),
            bm.category.chars().take(18).collect(),
            bm.dimensions.chars().take(24).collect(),
            format!("{}", bm.count),
            "-".into(),
        ));
    }

    let grand_pieces = sum.tracks.total_pieces + sum.roads.total_pieces + sum.total_buildings;
    let grand_meters = sum.tracks.total_length_meters + sum.roads.total_length_meters;

    let row_h = 16.0_f64;
    let header_h = 20.0_f64;
    let total_row_h = 20.0_f64;
    let table_top_y = page_h - margin - 54.0;
    let table_bottom_min = margin + 32.0 + total_row_h;
    let max_rows_per_page = (((table_top_y - header_h - table_bottom_min) / row_h).floor() as usize).max(1);

    let chunks: Vec<&[(String, String, String, String, String, String)]> = if rows.is_empty() {
        vec![&[]]
    } else {
        rows.chunks(max_rows_per_page).collect()
    };

    let actual_total = total_pages_hint + chunks.len().saturating_sub(1);
    let mut pages = Vec::with_capacity(chunks.len());

    let table_w = page_w - margin * 2.0;
    let col_part = margin + 6.0;
    let col_desc = margin + 72.0;
    let col_cat = margin + table_w * 0.44;
    let col_spec = margin + table_w * 0.60;
    let col_qty = page_w - margin - 88.0;
    let col_len = page_w - margin - 42.0;

    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let mut s = String::with_capacity(4096);
        let _ = writeln!(s, "1 1 1 rg 0 0 {page_w:.1} {page_h:.1} re f");

        let title = if is_de {
            format!("{} - Stueckliste", layout.name)
        } else {
            format!("{} - Bill of Materials", layout.name)
        };
        let subtitle = format!(
            "Scale: {}  |  Feed Voltage: {:.0} V",
            layout.scale,
            layout.feed_voltage.unwrap_or(16.0)
        );
        let badge = PdfBadge {
            name: if is_de { "Stueckliste / Inventar" } else { "Bill of Materials" },
            color_rgb: (0.06, 0.73, 0.51),
            elev_line: format!("{grand_pieces} Pieces Total"),
            count_line: format!("{grand_meters:.2} m total length"),
        };
        let _ = write_pdf_page_header(&mut s, page_w, page_h, margin, &title, &subtitle, &badge);

        let footer_left = format!(
            "Layout: {} ({})  |  Feed Voltage: {:.0}V",
            layout.name,
            layout.scale,
            layout.feed_voltage.unwrap_or(16.0)
        );
        let page_str = format!("Page {} of {}", start_page_num + chunk_idx, actual_total);
        let _ = write_pdf_page_footer(&mut s, page_w, margin, &footer_left, &page_str);

        // Table Header (#f1f5f9 fill, #cbd5e1 border)
        let mut y = table_top_y;
        let _ = writeln!(
            s,
            "0.95 0.96 0.98 rg 0.80 0.84 0.88 RG 0.7 w {margin:.1} {:.1} {table_w:.1} {header_h:.1} re B",
            y - header_h
        );
        let ty = y - 13.5;
        let (h_part, h_desc, h_cat, h_spec, h_qty, h_len) = if is_de {
            ("Art.-Nr.", "Bezeichnung", "Kategorie", "Geometrie / Mass", "Menge", "Laenge")
        } else {
            ("Part No.", "Description", "Category", "Specification", "Qty", "Length")
        };
        let _ = writeln!(
            s,
            "0.06 0.09 0.16 rg BT /F2 8.2 Tf {col_part:.1} {ty:.1} Td ({h_part}) Tj ET BT /F2 8.2 Tf {col_desc:.1} {ty:.1} Td ({h_desc}) Tj ET BT /F2 8.2 Tf {col_cat:.1} {ty:.1} Td ({h_cat}) Tj ET BT /F2 8.2 Tf {col_spec:.1} {ty:.1} Td ({h_spec}) Tj ET BT /F2 8.2 Tf {col_qty:.1} {ty:.1} Td ({h_qty}) Tj ET BT /F2 8.2 Tf {col_len:.1} {ty:.1} Td ({h_len}) Tj ET"
        );
        y -= header_h;

        // Zebra-striped table rows
        for (idx, (part, desc, cat, spec, qty, len)) in chunk.iter().enumerate() {
            let row_bottom = y - row_h;
            if idx % 2 == 1 {
                let _ = writeln!(
                    s,
                    "0.97 0.98 0.99 rg 0.89 0.91 0.94 RG 0.45 w {margin:.1} {row_bottom:.1} {table_w:.1} {row_h:.1} re B"
                );
            } else {
                let _ = writeln!(
                    s,
                    "0.89 0.91 0.94 RG 0.45 w {margin:.1} {row_bottom:.1} {table_w:.1} {row_h:.1} re S"
                );
            }
            let rty = y - 11.2;
            let _ = writeln!(
                s,
                "0.06 0.09 0.16 rg BT /F2 7.8 Tf {col_part:.1} {rty:.1} Td ({}) Tj ET 0.20 0.25 0.33 rg BT /F1 7.8 Tf {col_desc:.1} {rty:.1} Td ({}) Tj ET BT /F1 7.5 Tf {col_cat:.1} {rty:.1} Td ({}) Tj ET BT /F1 7.5 Tf {col_spec:.1} {rty:.1} Td ({}) Tj ET 0.06 0.09 0.16 rg BT /F2 7.8 Tf {col_qty:.1} {rty:.1} Td ({}) Tj ET 0.20 0.25 0.33 rg BT /F1 7.8 Tf {col_len:.1} {rty:.1} Td ({}) Tj ET",
                pdf_escape(part),
                pdf_escape(desc),
                pdf_escape(cat),
                pdf_escape(spec),
                pdf_escape(qty),
                pdf_escape(len)
            );
            y -= row_h;
        }

        // Grand Total row on last BOM page (#e2e8f0 fill, dark #0f172a border)
        if chunk_idx + 1 == chunks.len() {
            let tot_bottom = y - total_row_h;
            let _ = writeln!(
                s,
                "0.89 0.91 0.94 rg 0.06 0.09 0.16 RG 1.0 w {margin:.1} {tot_bottom:.1} {table_w:.1} {total_row_h:.1} re B"
            );
            let tty = y - 13.5;
            let grand_lbl = if is_de { "Gesamtsumme:" } else { "Grand Total:" };
            let _ = writeln!(
                s,
                "0.06 0.09 0.16 rg BT /F2 8.5 Tf {col_part:.1} {tty:.1} Td ({grand_lbl}) Tj ET BT /F2 8.5 Tf {col_qty:.1} {tty:.1} Td ({grand_pieces}) Tj ET BT /F2 8.5 Tf {col_len:.1} {tty:.1} Td ({grand_meters:.2} m) Tj ET"
            );
        }

        pages.push(s);
    }

    pages
}

/// Pure-Rust Multi-Page Vector PDF 1.4 Blueprint & Parts List Exporter (Electron-parity layout).
pub fn export_layout_pdf_with_options(
    layout: &Layout,
    catalogs: &[Catalog],
    opts: &BlueprintExportOptions,
    is_de: bool,
) -> Vec<u8> {
    let (page_w, page_h) = if opts.landscape {
        (842.0_f64, 595.0_f64)
    } else {
        (595.0_f64, 842.0_f64)
    };

    let mut layer_pages_meta: Vec<(String, String, f64, String)> = Vec::new();
    if opts.include_layer_pages {
        if let Some(layers) = &layout.layers {
            let mut sorted: Vec<_> = layers
                .iter()
                .filter(|l| {
                    if opts.selected_layer_ids.is_empty() {
                        l.visible
                    } else {
                        opts.selected_layer_ids.contains(&l.id)
                    }
                })
                .collect();
            sorted.sort_by(|a, b| {
                a.height_mm
                    .unwrap_or(0.0)
                    .partial_cmp(&b.height_mm.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for l in sorted {
                layer_pages_meta.push((
                    l.id.clone(),
                    l.name.clone(),
                    l.height_mm.unwrap_or(0.0),
                    l.color.clone().unwrap_or_else(|| "#38bdf8".into()),
                ));
            }
        }
    }

    let include_overview = opts.include_overview || (layer_pages_meta.is_empty() && !opts.include_bom_page);
    let plan_page_count = (include_overview as usize) + layer_pages_meta.len();
    let bom_pages = if opts.include_bom_page {
        draw_pdf_bom_pages(
            layout,
            catalogs,
            page_w,
            page_h,
            is_de,
            plan_page_count + 1,
            plan_page_count + 1,
        )
    } else {
        Vec::new()
    };
    let total_pages = (plan_page_count + bom_pages.len()).max(1);

    let mut page_streams: Vec<String> = Vec::with_capacity(total_pages);
    let mut page_num = 1usize;

    if include_overview {
        let sum = get_piece_list_summary(layout, catalogs, is_de);
        let layer_cnt = layout.layers.as_ref().map_or(1, Vec::len);
        let total_len = sum.tracks.total_length_meters + sum.roads.total_length_meters;
        let total_pcs = sum.tracks.total_pieces + sum.roads.total_pieces;
        let badge = PdfBadge {
            name: if is_de { "Alle Ebenen kombiniert" } else { "All Layers Combined" },
            color_rgb: (0.01, 0.52, 0.78),
            elev_line: if is_de { format!("{layer_cnt} Ebenen") } else { format!("{layer_cnt} Layers") },
            count_line: format!("{total_pcs} tracks  |  {total_len:.2} m total"),
        };
        let active_layers_str = layout
            .layers
            .as_ref()
            .map(|ls| {
                ls.iter()
                    .map(|l| format!("{} ({:.0} mm)", l.name, l.height_mm.unwrap_or(0.0)))
                    .collect::<Vec<_>>()
                    .join("  |  ")
            })
            .unwrap_or_else(|| "Layer 1 (0 mm)".into());
        let footer_left = if is_de {
            format!("Aktive Ebenen: {active_layers_str}")
        } else {
            format!("Active Layers: {active_layers_str}")
        };
        let subtitle = if is_de {
            format!("Gesamtuebersicht  |  Spur {}", layout.scale)
        } else {
            format!("Overall Layout Overview  |  Scale {}", layout.scale)
        };
        let footer_right = format!("Page {page_num} of {total_pages}");
        page_streams.push(draw_pdf_plan_page(
            layout,
            catalogs,
            opts,
            None,
            &subtitle,
            &badge,
            &footer_left,
            &footer_right,
            page_w,
            page_h,
        ));
        page_num += 1;
    }

    for (lid, lname, lh, lcol) in &layer_pages_meta {
        let lsum = get_layer_piece_list_summary(layout, catalogs, lid, is_de);
        let h_sign = if *lh > 0.0 {
            format!("+{lh:.0}")
        } else {
            format!("{lh:.0}")
        };
        let badge = PdfBadge {
            name: lname,
            color_rgb: hex_to_pdf_rgb(lcol),
            elev_line: if is_de {
                format!("Hoehe: {h_sign} mm")
            } else {
                format!("Height: {h_sign} mm")
            },
            count_line: format!("{} tracks  |  {:.2} m", lsum.total_pieces, lsum.total_length_meters),
        };
        let chips = lsum
            .items
            .iter()
            .take(10)
            .map(|it| format!("{}: {}x", it.catalog_id, it.count))
            .collect::<Vec<_>>()
            .join("   ");
        let footer_left = if chips.is_empty() {
            if is_de { "Keine Gleise auf dieser Ebene".into() } else { "No tracks assigned to this layer".into() }
        } else if is_de {
            format!("Teile auf dieser Ebene: {chips}")
        } else {
            format!("Parts on this level: {chips}")
        };
        let subtitle = if is_de {
            format!("Ebene: {lname}  |  Spur {}", layout.scale)
        } else {
            format!("Layer: {lname}  |  Scale {}", layout.scale)
        };
        let footer_right = format!("Page {page_num} of {total_pages}");
        page_streams.push(draw_pdf_plan_page(
            layout,
            catalogs,
            opts,
            Some(lid.as_str()),
            &subtitle,
            &badge,
            &footer_left,
            &footer_right,
            page_w,
            page_h,
        ));
        page_num += 1;
    }

    page_streams.extend(bom_pages);

    // Assemble PDF 1.4 with both /F1 (Helvetica) and /F2 (Helvetica-Bold)
    let mut pdf = Vec::<u8>::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::<usize>::new();

    let page_ids: Vec<String> = (0..page_streams.len())
        .map(|i| format!("{} 0 R", 5 + i * 2))
        .collect();

    let mut objects = vec![
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string(),
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            page_ids.join(" "),
            page_streams.len()
        ),
        "3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_string(),
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n".to_string(),
    ];

    for (i, stream) in page_streams.iter().enumerate() {
        let page_obj = 5 + i * 2;
        let cont_obj = page_obj + 1;
        objects.push(format!(
            "{page_obj} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w:.0} {page_h:.0}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {cont_obj} 0 R >>\nendobj\n"
        ));
        objects.push(format!(
            "{cont_obj} 0 obj\n<< /Length {} >>\nstream\n{stream}endstream\nendobj\n",
            stream.len()
        ));
    }

    for obj in &objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj.as_bytes());
    }

    let xref_pos = pdf.len();
    let mut xref = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for off in offsets {
        let _ = writeln!(xref, "{off:010} 00000 n ");
    }
    let _ = writeln!(
        xref,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF",
        objects.len() + 1
    );
    pdf.extend_from_slice(xref.as_bytes());
    pdf
}
