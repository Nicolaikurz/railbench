use super::*;

pub(super) fn draw_pdf_plan_page(
    layout: &Layout,
    catalogs: &[Catalog],
    opts: &BlueprintExportOptions,
    filter_layer_id: Option<&str>,
    subtitle: &str,
    page_footer: &str,
    page_w: f64,
    page_h: f64,
) -> String {
    let margin = 36.0_f64;
    let header_h = 38.0_f64;
    let footer_h = 22.0_f64;
    let (min_x, max_x, min_y, max_y) = layout_bounds(layout, catalogs);
    let span_x = (max_x - min_x).max(100.0);
    let span_y = (max_y - min_y).max(100.0);
    let scale = ((page_w - margin * 2.0) / span_x)
        .min((page_h - margin * 2.0 - header_h - footer_h) / span_y);
    let off_x = (page_w - span_x * scale) * 0.5;
    let off_y = footer_h + (page_h - header_h - footer_h - span_y * scale) * 0.5;

    let to_pdf = |wx: f64, wy: f64| -> (f64, f64) {
        let px = off_x + (wx - min_x) * scale;
        let py = page_h - header_h - (off_y + (wy - min_y) * scale) + footer_h;
        (px, py)
    };

    let mut s = String::new();
    let title = pdf_escape(&format!("RAILbench: {} ({subtitle})", layout.name));
    let _ = writeln!(
        s,
        "0.08 0.11 0.18 rg BT /F1 12 Tf {margin:.1} {:.1} Td ({title}) Tj ET",
        page_h - 28.0
    );
    let _ = writeln!(
        s,
        "0.35 0.40 0.48 rg BT /F1 9 Tf {margin:.1} {:.1} Td (Scale {} | {} pieces | {}) Tj ET",
        page_h - 41.0,
        pdf_escape(&layout.scale),
        layout.pieces.len(),
        pdf_escape(page_footer)
    );
    let _ = writeln!(
        s,
        "0.2 0.25 0.35 RG 1.0 w {margin:.1} {:.1} m {:.1} {:.1} l S",
        page_h - 46.0,
        page_w - margin,
        page_h - 46.0
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
            let _ = write!(s, "0.94 0.95 0.97 rg 0.50 0.56 0.65 RG 0.9 w ");
            for (k, pt) in poly.iter().enumerate() {
                let (px, py) = to_pdf(pt.x, pt.y);
                let _ = write!(s, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
            }
            let _ = writeln!(s, "h B");
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
                    let _ = writeln!(stream, "0.88 0.90 0.93 rg 0.72 0.75 0.80 RG 0.8 w");
                    stroke_circle(stream, outer_r, "B");
                    let _ = writeln!(stream, "0.80 0.84 0.88 rg 0.68 0.72 0.76 RG 0.7 w");
                    stroke_circle(stream, pit_r, "B");
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
                    if opts.monochrome {
                        let _ = writeln!(stream, "0.08 0.10 0.14 RG 0.8 w");
                    } else {
                        let _ = writeln!(stream, "0.02 0.45 0.76 RG 0.9 w");
                    }
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
                            "BT /F1 5.5 Tf {lx:.1} {ly:.1} Td ({}) Tj ET",
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
                    let _ = writeln!(stream, "0.88 0.90 0.93 rg 0.72 0.75 0.80 RG 0.8 w");
                    write_poly(stream, &outer_cs, "B");
                    let _ = writeln!(stream, "0.80 0.84 0.88 rg 0.68 0.72 0.76 RG 0.7 w");
                    write_poly(stream, &pit_cs, "B");
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
                        Vec2 {
                            x: p.transform.x + rp.x,
                            y: p.transform.y + rp.y,
                        }
                    };
                    let _ = writeln!(stream, "0.58 0.64 0.72 RG 0.7 w");
                    for &rx in &[80.0, l * 0.5, l - 80.0] {
                        let a = to_w(rx, top_z + 12.0);
                        let b = to_w(rx, bot_z - 12.0);
                        let (ax, ay) = to_pdf(a.x, a.y);
                        let (bx, by) = to_pdf(b.x, b.y);
                        let _ = writeln!(stream, "{ax:.2} {ay:.2} m {bx:.2} {by:.2} l S");
                    }

                    if opts.monochrome {
                        let _ = writeln!(stream, "0.08 0.10 0.14 RG 0.8 w");
                    } else {
                        let _ = writeln!(stream, "0.02 0.45 0.76 RG 0.9 w");
                    }
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
                            "BT /F1 5.5 Tf {lx:.1} {ly:.1} Td ({}) Tj ET",
                            pdf_escape(&p.catalog_id)
                        );
                    }
                }
                continue;
            }

            if ghost {
                let _ = writeln!(stream, "1 J 0.84 0.87 0.90 RG {ballast_w:.2} w");
            } else if opts.monochrome {
                let _ = writeln!(stream, "1 J 0.72 0.75 0.80 RG {ballast_w:.2} w");
            } else if is_road {
                let _ = writeln!(stream, "1 J 0.32 0.37 0.44 RG {ballast_w:.2} w");
            } else {
                let _ = writeln!(stream, "1 J 0.45 0.52 0.60 RG {ballast_w:.2} w");
            }
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
            if !ghost {
                if opts.monochrome {
                    let _ = writeln!(stream, "0.08 0.10 0.14 RG 0.7 w");
                } else {
                    let _ = writeln!(stream, "0.02 0.45 0.76 RG 0.8 w");
                }
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
                if opts.show_labels && p.connectors.len() >= 2 && scale >= 0.08 {
                    let (lx, ly) = to_pdf(
                        (p.connectors[0].x + p.connectors[1].x) * 0.5,
                        (p.connectors[0].y + p.connectors[1].y) * 0.5,
                    );
                    let _ = writeln!(
                        stream,
                        "0.08 0.12 0.20 rg BT /F1 5.5 Tf {lx:.1} {ly:.1} Td ({}) Tj ET",
                        pdf_escape(&p.catalog_id)
                    );
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
            let cs = model_corners_world(m);
            let _ = write!(s, "0.86 0.91 0.98 rg 0.22 0.30 0.42 RG 0.8 w ");
            for (k, pt) in cs.iter().enumerate() {
                let (px, py) = to_pdf(pt.x, pt.y);
                let _ = write!(s, "{px:.2} {py:.2} {} ", if k == 0 { "m" } else { "l" });
            }
            let _ = writeln!(s, "h B");
        }
    }

    s
}

pub(super) fn draw_pdf_bom_page(
    layout: &Layout,
    catalogs: &[Catalog],
    page_w: f64,
    page_h: f64,
    is_de: bool,
    page_footer: &str,
) -> String {
    let sum = get_piece_list_summary(layout, catalogs, is_de);
    let margin = 40.0_f64;
    let mut s = String::new();
    let heading = pdf_escape(&format!("{} - Bill of Materials ({})", layout.name, layout.scale));
    let _ = writeln!(
        s,
        "0.08 0.11 0.18 rg BT /F1 13 Tf {margin:.1} {:.1} Td ({heading}) Tj ET",
        page_h - 34.0
    );
    let sub = pdf_escape(&format!(
        "Tracks: {} ({:.2} m) | Roads: {} ({:.2} m) | Models: {} | {}",
        sum.tracks.total_pieces,
        sum.tracks.total_length_meters,
        sum.roads.total_pieces,
        sum.roads.total_length_meters,
        sum.total_buildings,
        page_footer
    ));
    let _ = writeln!(
        s,
        "0.35 0.42 0.50 rg BT /F1 9 Tf {margin:.1} {:.1} Td ({sub}) Tj ET",
        page_h - 49.0
    );

    let mut y = page_h - 72.0;
    let _ = writeln!(
        s,
        "0.92 0.94 0.97 rg {margin:.1} {:.1} {:.1} 16 re f",
        y - 4.0,
        page_w - margin * 2.0
    );
    let _ = writeln!(
        s,
        "0.08 0.12 0.20 rg BT /F1 9 Tf {:.1} {y:.1} Td (Part No.) Tj ET BT /F1 9 Tf {:.1} {y:.1} Td (Description) Tj ET BT /F1 9 Tf {:.1} {y:.1} Td (Spec) Tj ET BT /F1 9 Tf {:.1} {y:.1} Td (Qty) Tj ET BT /F1 9 Tf {:.1} {y:.1} Td (Length) Tj ET",
        margin + 6.0,
        margin + 90.0,
        margin + 320.0,
        page_w - margin - 105.0,
        page_w - margin - 52.0
    );
    y -= 18.0;

    for it in sum.tracks.items.iter().chain(sum.roads.items.iter()) {
        if y < 44.0 {
            break;
        }
        let desc = it.name.chars().take(38).collect::<String>();
        let spec = it.specification.chars().take(24).collect::<String>();
        let _ = writeln!(
            s,
            "0.12 0.16 0.24 rg BT /F1 8.5 Tf {:.1} {y:.1} Td ({}) Tj ET BT /F1 8.5 Tf {:.1} {y:.1} Td ({}) Tj ET BT /F1 8.5 Tf {:.1} {y:.1} Td ({}) Tj ET BT /F1 8.5 Tf {:.1} {y:.1} Td ({}x) Tj ET BT /F1 8.5 Tf {:.1} {y:.1} Td ({:.2} m) Tj ET",
            margin + 6.0,
            pdf_escape(&it.catalog_id),
            margin + 90.0,
            pdf_escape(&desc),
            margin + 320.0,
            pdf_escape(&spec),
            page_w - margin - 105.0,
            it.count,
            page_w - margin - 52.0,
            it.total_length_meters
        );
        y -= 14.0;
    }

    for bm in &sum.buildings {
        if y < 44.0 {
            break;
        }
        let desc = bm.name.chars().take(38).collect::<String>();
        let _ = writeln!(
            s,
            "0.22 0.28 0.36 rg BT /F1 8.5 Tf {:.1} {y:.1} Td (3D) Tj ET BT /F1 8.5 Tf {:.1} {y:.1} Td ({}) Tj ET BT /F1 8.5 Tf {:.1} {y:.1} Td ({}) Tj ET BT /F1 8.5 Tf {:.1} {y:.1} Td ({}x) Tj ET",
            margin + 6.0,
            margin + 90.0,
            pdf_escape(&desc),
            margin + 320.0,
            pdf_escape(&bm.dimensions),
            page_w - margin - 105.0,
            bm.count
        );
        y -= 14.0;
    }

    s
}

/// Pure-Rust Multi-Page Vector PDF 1.4 Blueprint & Parts List Exporter.
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

    let mut page_streams: Vec<String> = Vec::new();
    let mut layer_pages_meta: Vec<(String, String, f64)> = Vec::new();
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
                layer_pages_meta.push((l.id.clone(), l.name.clone(), l.height_mm.unwrap_or(0.0)));
            }
        }
    }

    let total_pages = (opts.include_overview as usize)
        + layer_pages_meta.len()
        + (opts.include_bom_page as usize);
    let total_pages = total_pages.max(1);
    let mut page_num = 1usize;

    if opts.include_overview || (layer_pages_meta.is_empty() && !opts.include_bom_page) {
        let footer = format!("Page {page_num}/{total_pages}");
        page_streams.push(draw_pdf_plan_page(
            layout,
            catalogs,
            opts,
            None,
            "Overview",
            &footer,
            page_w,
            page_h,
        ));
        page_num += 1;
    }

    for (lid, lname, lh) in &layer_pages_meta {
        let sub = format!("Layer: {lname} ({lh:.0} mm)");
        let footer = format!("Page {page_num}/{total_pages}");
        page_streams.push(draw_pdf_plan_page(
            layout,
            catalogs,
            opts,
            Some(lid.as_str()),
            &sub,
            &footer,
            page_w,
            page_h,
        ));
        page_num += 1;
    }

    if opts.include_bom_page {
        let footer = format!("Page {page_num}/{total_pages}");
        page_streams.push(draw_pdf_bom_page(
            layout, catalogs, page_w, page_h, is_de, &footer,
        ));
    }

    let mut pdf = Vec::<u8>::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::<usize>::new();

    let page_ids: Vec<String> = (0..page_streams.len())
        .map(|i| format!("{} 0 R", 4 + i * 2))
        .collect();

    let mut objects = vec![
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string(),
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            page_ids.join(" "),
            page_streams.len()
        ),
        "3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_string(),
    ];

    for (i, stream) in page_streams.iter().enumerate() {
        let page_obj = 4 + i * 2;
        let cont_obj = page_obj + 1;
        objects.push(format!(
            "{page_obj} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w:.0} {page_h:.0}] /Resources << /Font << /F1 3 0 R >> >> /Contents {cont_obj} 0 R >>\nendobj\n"
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

