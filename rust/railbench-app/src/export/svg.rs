use super::*;

pub(crate) fn sample_track_branches_world(piece: &TrackPiece, entry: Option<&CatalogEntry>) -> Vec<Vec<Vec2>> {
    if let Some(e) = entry {
        let to_w = |lx: f64, ly: f64| -> Vec2 {
            let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
            Vec2 {
                x: piece.transform.x + rp.x,
                y: piece.transform.y + rp.y,
            }
        };
        if e.kind == "turntable" {
            let outer_r = crate::turntables_3d::turntable_outer_radius(piece, entry);
            let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
            let pit_r = 139.0 * scale_f;
            let slot_angles: Vec<f64> = e
                .local_connectors
                .as_ref()
                .filter(|lc| !lc.is_empty())
                .map(|lc| lc.iter().map(|c| c.angle).collect())
                .unwrap_or_else(|| (0..30).map(|i| i as f64 * 12.0).collect());
            let mut out = Vec::with_capacity(1 + slot_angles.len());
            out.push(vec![to_w(-pit_r, 0.0), to_w(pit_r, 0.0)]);
            for deg in slot_angles {
                let rad = deg2rad(deg);
                let (ca, sa) = (rad.cos(), rad.sin());
                out.push(vec![to_w(pit_r * ca, pit_r * sa), to_w(outer_r * ca, outer_r * sa)]);
            }
            return out;
        }
        if e.kind == "transfer-table" {
            let l = e.length.unwrap_or(360.0);
            let pit_left = 36.0;
            let pit_right = l - 36.0;
            let track_zs = crate::turntables_3d::extract_transfer_track_zs(entry);
            let bridge_z = track_zs[track_zs.len() / 2] as f64;
            let mut out = Vec::with_capacity(1 + track_zs.len() * 2);
            out.push(vec![to_w(pit_left, bridge_z), to_w(pit_right, bridge_z)]);
            for &tzf in &track_zs {
                let tz = tzf as f64;
                out.push(vec![to_w(0.0, tz), to_w(pit_left, tz)]);
                out.push(vec![to_w(pit_right, tz), to_w(l, tz)]);
            }
            return out;
        }
    }

    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let steps = 16usize;
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], steps);
            let mut out = Vec::with_capacity(steps + 1);
            for k in 0..=steps {
                let rp = rotate(Vec2 { x: flat[k * 5], y: flat[k * 5 + 1] }, piece.transform.rotation);
                out.push(Vec2 {
                    x: piece.transform.x + rp.x,
                    y: piece.transform.y + rp.y,
                });
            }
            return vec![out];
        }
    }

    let c = &piece.connectors;
    match c.len() {
        0 => vec![],
        1 => {
            let rad = deg2rad(c[0].angle + 180.0);
            vec![vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[0].x + rad.cos() * 70.0, y: c[0].y + rad.sin() * 70.0 },
            ]]
        }
        2 => vec![sample_arc_between_connectors(
            c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 14,
        )],
        3 => vec![
            sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 12),
            sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 12),
        ],
        4 => {
            let d13 = (c[1].x - c[3].x).hypot(c[1].y - c[3].y);
            let d02 = (c[0].x - c[2].x).hypot(c[0].y - c[2].y);
            if (d13 - d02).abs() > 30.0 {
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 10),
                    sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[3].x, c[3].y, c[3].angle, 10),
                ]
            } else {
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 8),
                    sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[3].x, c[3].y, c[3].angle, 8),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[3].x, c[3].y, c[3].angle, 8),
                    sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[1].x, c[1].y, c[1].angle, 8),
                ]
            }
        }
        n => {
            let mut ring: Vec<Vec2> = c.iter().map(|cn| Vec2 { x: cn.x, y: cn.y }).collect();
            if let Some(first) = c.first() {
                ring.push(Vec2 { x: first.x, y: first.y });
            }
            let bridge = vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[n / 2].x, y: c[n / 2].y },
            ];
            vec![ring, bridge]
        }
    }
}

pub(crate) fn layout_bounds(layout: &Layout, catalogs: &[Catalog]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in &layout.pieces {
        let entry = find_entry(catalogs, &p.catalog_id);
        match entry.map(|e| e.kind.as_str()) {
            Some("turntable") => {
                let r = crate::turntables_3d::turntable_outer_radius(p, entry);
                min_x = min_x.min(p.transform.x - r);
                max_x = max_x.max(p.transform.x + r);
                min_y = min_y.min(p.transform.y - r);
                max_y = max_y.max(p.transform.y + r);
            }
            Some("transfer-table") => {
                for pt in crate::turntables_3d::transfer_table_outer_world_corners(p, entry) {
                    min_x = min_x.min(pt.x);
                    max_x = max_x.max(pt.x);
                    min_y = min_y.min(pt.y);
                    max_y = max_y.max(pt.y);
                }
            }
            _ => {}
        }
        for c in &p.connectors {
            min_x = min_x.min(c.x);
            max_x = max_x.max(c.x);
            min_y = min_y.min(c.y);
            max_y = max_y.max(c.y);
        }
    }
    for bb in get_baseboards_slice(layout) {
        if !bb.enabled {
            continue;
        }
        for pt in get_baseboard_polygon(&bb) {
            min_x = min_x.min(pt.x);
            max_x = max_x.max(pt.x);
            min_y = min_y.min(pt.y);
            max_y = max_y.max(pt.y);
        }
    }
    if let Some(models) = &layout.models {
        for m in models {
            let hw = m.width * m.scale.unwrap_or(1.0) * 0.5;
            let hd = m.depth * m.scale.unwrap_or(1.0) * 0.5;
            min_x = min_x.min(m.transform.x - hw);
            max_x = max_x.max(m.transform.x + hw);
            min_y = min_y.min(m.transform.y - hd);
            max_y = max_y.max(m.transform.y + hd);
        }
    }
    if !min_x.is_finite() {
        return (-500.0, 500.0, -300.0, 300.0);
    }
    (min_x - 80.0, max_x + 80.0, min_y - 80.0, max_y + 80.0)
}

pub(super) fn model_corners_world(m: &railbench_core::types::Model3D) -> [Vec2; 4] {
    let sc = m.scale.unwrap_or(1.0);
    let hw = m.width * sc * 0.5;
    let hd = m.depth * sc * 0.5;
    let raw = [
        Vec2 { x: -hw, y: -hd },
        Vec2 { x: hw, y: -hd },
        Vec2 { x: hw, y: hd },
        Vec2 { x: -hw, y: hd },
    ];
    raw.map(|p| {
        let rp = rotate(p, m.transform.rotation);
        Vec2 {
            x: m.transform.x + rp.x,
            y: m.transform.y + rp.y,
        }
    })
}

/// Exports a crisp 1:1 millimeter vector SVG blueprint supporting curved tracks & multi-table shapes.
pub fn export_layout_svg_with_options(
    layout: &Layout,
    catalogs: &[Catalog],
    opts: &BlueprintExportOptions,
) -> String {
    let (min_x, max_x, min_y, max_y) = layout_bounds(layout, catalogs);
    let w = (max_x - min_x).max(100.0);
    let h = (max_y - min_y).max(100.0);

    let mut svg = String::new();
    let _ = writeln!(
        svg,
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{min_x:.1} {min_y:.1} {w:.1} {h:.1}" width="{w:.1}mm" height="{h:.1}mm">
  <rect x="{min_x:.1}" y="{min_y:.1}" width="{w:.1}" height="{h:.1}" fill="#ffffff"/>"##
    );

    if opts.show_baseboards {
        for bb in get_baseboards_slice(layout) {
            if !bb.enabled {
                continue;
            }
            let poly = get_baseboard_polygon(&bb);
            if poly.len() < 3 {
                continue;
            }
            let pts_s: String = poly
                .iter()
                .map(|p| format!("{:.1},{:.1}", p.x, p.y))
                .collect::<Vec<_>>()
                .join(" ");
            let fill = if opts.monochrome { "#f8fafc" } else { "#f1f5f9" };
            let _ = writeln!(
                svg,
                r##"  <polygon points="{pts_s}" fill="{fill}" stroke="#64748b" stroke-width="3"/>"##
            );
        }
    }

    if let Some(models) = &layout.models {
        for m in models {
            let cs = model_corners_world(m);
            let pts_s = cs
                .iter()
                .map(|p| format!("{:.1},{:.1}", p.x, p.y))
                .collect::<Vec<_>>()
                .join(" ");
            let fill = if opts.monochrome { "#e2e8f0" } else { "#dbeafe" };
            let _ = writeln!(
                svg,
                r##"  <polygon points="{pts_s}" fill="{fill}" fill-opacity="0.7" stroke="#334155" stroke-width="2"/>"##
            );
        }
    }

    for p in &layout.pieces {
        let entry = find_entry(catalogs, &p.catalog_id);
        let kind = entry.map(|e| e.kind.as_str()).unwrap_or("");
        let is_road = is_road_id(catalogs, &p.catalog_id);
        let ballast_col = if opts.monochrome {
            "#cbd5e1"
        } else if is_road {
            "#475569"
        } else {
            "#64748b"
        };
        let rail_col = if opts.monochrome { "#0f172a" } else { "#0284c7" };
        let ballast_w = if is_road { 42 } else { 26 };

        if kind == "turntable" {
            let outer_r = crate::turntables_3d::turntable_outer_radius(p, entry);
            let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
            let pit_r = 139.0 * scale_f;
            let ring_rail_r = 124.0 * scale_f;
            let (cx, cy) = (p.transform.x, p.transform.y);
            let pit_fill = if opts.monochrome { "#e2e8f0" } else { "#1e293b" };
            let _ = writeln!(
                svg,
                r##"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{outer_r:.1}" fill="{ballast_col}" stroke="#475569" stroke-width="2"/>"##
            );
            let _ = writeln!(
                svg,
                r##"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{pit_r:.1}" fill="{pit_fill}" stroke="#64748b" stroke-width="2"/>"##
            );
            let _ = writeln!(
                svg,
                r##"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{ring_rail_r:.1}" fill="none" stroke="#94a3b8" stroke-width="1.5"/>"##
            );
            for br in sample_track_branches_world(p, entry) {
                if br.len() < 2 {
                    continue;
                }
                let pts_s: String = br
                    .iter()
                    .map(|pt| format!("{:.1},{:.1}", pt.x, pt.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = writeln!(
                    svg,
                    r##"  <polyline points="{pts_s}" fill="none" stroke="{rail_col}" stroke-width="3" stroke-linecap="round"/>"##
                );
            }
            if opts.show_labels {
                let lrp = rotate(Vec2 { x: 0.0, y: -pit_r * 0.45 }, p.transform.rotation);
                let (lx, ly) = (cx + lrp.x, cy + lrp.y);
                let lbl_col = if opts.monochrome { "#0f172a" } else { "#f8fafc" };
                let _ = writeln!(
                    svg,
                    r##"  <text x="{lx:.1}" y="{ly:.1}" font-family="sans-serif" font-size="13" fill="{lbl_col}" text-anchor="middle" dominant-baseline="central">{}</text>"##,
                    escape_xml(&p.catalog_id)
                );
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
            let outer_s = outer_cs
                .iter()
                .map(|pt| format!("{:.1},{:.1}", pt.x, pt.y))
                .collect::<Vec<_>>()
                .join(" ");
            let pit_s = pit_cs
                .iter()
                .map(|pt| format!("{:.1},{:.1}", pt.x, pt.y))
                .collect::<Vec<_>>()
                .join(" ");
            let pit_fill = if opts.monochrome { "#e2e8f0" } else { "#1e293b" };
            let _ = writeln!(
                svg,
                r##"  <polygon points="{outer_s}" fill="{ballast_col}" stroke="#475569" stroke-width="2"/>"##
            );
            let _ = writeln!(
                svg,
                r##"  <polygon points="{pit_s}" fill="{pit_fill}" stroke="#64748b" stroke-width="2"/>"##
            );
            let (_, l, top_z, bot_z) = crate::turntables_3d::transfer_table_local_bounds(entry);
            let to_w = |lx: f64, ly: f64| -> Vec2 {
                let rp = rotate(Vec2 { x: lx, y: ly }, p.transform.rotation);
                Vec2 {
                    x: p.transform.x + rp.x,
                    y: p.transform.y + rp.y,
                }
            };
            for &rx in &[80.0, l * 0.5, l - 80.0] {
                let a = to_w(rx, top_z + 12.0);
                let b = to_w(rx, bot_z - 12.0);
                let _ = writeln!(
                    svg,
                    r##"  <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#94a3b8" stroke-width="1.5"/>"##,
                    a.x, a.y, b.x, b.y
                );
            }
            for br in sample_track_branches_world(p, entry) {
                if br.len() < 2 {
                    continue;
                }
                let pts_s: String = br
                    .iter()
                    .map(|pt| format!("{:.1},{:.1}", pt.x, pt.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = writeln!(
                    svg,
                    r##"  <polyline points="{pts_s}" fill="none" stroke="{rail_col}" stroke-width="3" stroke-linecap="round"/>"##
                );
            }
            if opts.show_labels {
                let track_zs = crate::turntables_3d::extract_transfer_track_zs(entry);
                let bridge_z = track_zs[track_zs.len() / 2] as f64;
                let lp = to_w(l * 0.5, bridge_z - 56.0);
                let lbl_col = if opts.monochrome { "#0f172a" } else { "#f8fafc" };
                let _ = writeln!(
                    svg,
                    r##"  <text x="{:.1}" y="{:.1}" font-family="sans-serif" font-size="13" fill="{lbl_col}" text-anchor="middle" dominant-baseline="central">{}</text>"##,
                    lp.x,
                    lp.y,
                    escape_xml(&p.catalog_id)
                );
            }
            continue;
        }

        for br in sample_track_branches_world(p, entry) {
            if br.len() < 2 {
                continue;
            }
            let pts_s: String = br
                .iter()
                .map(|pt| format!("{:.1},{:.1}", pt.x, pt.y))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(
                svg,
                r##"  <polyline points="{pts_s}" fill="none" stroke="{ballast_col}" stroke-width="{ballast_w}" stroke-linecap="round" stroke-linejoin="round"/>"##
            );
            let dash = if p.has_tunnel == Some(true) {
                r#" stroke-dasharray="12 8""#
            } else {
                ""
            };
            let _ = writeln!(
                svg,
                r##"  <polyline points="{pts_s}" fill="none" stroke="{rail_col}" stroke-width="3"{dash} stroke-linecap="round" stroke-linejoin="round"/>"##
            );
        }

        if opts.show_labels && p.connectors.len() >= 2 {
            let mx = (p.connectors[0].x + p.connectors[1].x) * 0.5;
            let my = (p.connectors[0].y + p.connectors[1].y) * 0.5;
            let _ = writeln!(
                svg,
                r##"  <text x="{mx:.1}" y="{my:.1}" font-family="sans-serif" font-size="13" fill="#0f172a" text-anchor="middle" dominant-baseline="central">{}</text>"##,
                escape_xml(&p.catalog_id)
            );
        }
    }

    if opts.show_models {
        if let Some(models) = &layout.models {
            for m in models {
                if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                    continue;
                }
                let top_mesh = crate::canvas2d::build_model_top_view_mesh_2d(m);
                if !top_mesh.tris.is_empty() {
                    let rad = m.transform.rotation.to_radians();
                    let (sin_r, cos_r) = rad.sin_cos();
                    for (pts, cols) in &top_mesh.tris {
                        let r_u8 = ((cols[0].r() as u16 + cols[1].r() as u16 + cols[2].r() as u16) / 3) as u8;
                        let g_u8 = ((cols[0].g() as u16 + cols[1].g() as u16 + cols[2].g() as u16) / 3) as u8;
                        let b_u8 = ((cols[0].b() as u16 + cols[1].b() as u16 + cols[2].b() as u16) / 3) as u8;
                        let (r, g, b) = if opts.monochrome {
                            let lum = ((r_u8 as u16 * 77 + g_u8 as u16 * 150 + b_u8 as u16 * 29) >> 8) as u8;
                            (lum, lum, lum)
                        } else {
                            (r_u8, g_u8, b_u8)
                        };
                        let mut p2 = [(0.0_f64, 0.0_f64); 3];
                        for k in 0..3 {
                            let lx = pts[k][0] as f64;
                            let ly = pts[k][1] as f64;
                            p2[k] = (
                                m.transform.x + lx * cos_r - ly * sin_r,
                                m.transform.y + lx * sin_r + ly * cos_r,
                            );
                        }
                        let _ = writeln!(
                            svg,
                            r##"  <polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="#{r:02x}{g:02x}{b:02x}"/>"##,
                            p2[0].0, p2[0].1, p2[1].0, p2[1].1, p2[2].0, p2[2].1
                        );
                    }
                }
            }
        }
    }

    let _ = writeln!(svg, "</svg>");
    svg
}

pub(super) fn pdf_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let c = if ch.is_ascii() { ch } else { '?' };
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out
}

