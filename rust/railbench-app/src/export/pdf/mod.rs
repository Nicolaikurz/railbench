use super::*;

pub(super) fn hex_to_pdf_rgb(hex: &str) -> (f64, f64, f64) {
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

pub(super) fn pdf_track_color(kind: &str, is_road: bool, is_bridge: bool, is_tunnel: bool, monochrome: bool) -> (f64, f64, f64) {
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

pub(super) struct PdfBadge<'a> {
    name: &'a str,
    color_rgb: (f64, f64, f64),
    elev_line: String,
    count_line: String,
}

pub(super) fn write_pdf_page_header(
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

pub(super) fn write_pdf_page_footer(
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


mod plan_page;

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
        page_streams.push(plan_page::draw_pdf_plan_page(
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
        page_streams.push(plan_page::draw_pdf_plan_page(
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
