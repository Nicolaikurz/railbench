use super::super::*;
use super::*;

pub(super) fn draw_pdf_plan_page(
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

