use std::collections::BTreeMap;
use std::fmt::Write as _;

use railbench_core::types::{Catalog, Layout};

use crate::catalog::find_entry;

/// Exports Bill of Materials (Stückliste) as UTF-8 CSV.
pub fn export_bom_csv(layout: &Layout, catalogs: &[Catalog]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &layout.pieces {
        *counts.entry(p.catalog_id.as_str()).or_insert(0) += 1;
    }

    let mut csv = String::from("CatalogID,Count,Name,Category\n");
    for (cid, cnt) in counts {
        let entry = find_entry(catalogs, cid);
        let name = entry.map(|e| e.name.as_str()).unwrap_or("");
        let cat = entry.map(|e| e.category.as_str()).unwrap_or("");
        let _ = writeln!(csv, "\"{}\",{},\"{}\",\"{}\"", cid, cnt, name, cat);
    }
    csv
}

fn layout_bounds(layout: &Layout) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in &layout.pieces {
        for c in &p.connectors {
            min_x = min_x.min(c.x);
            max_x = max_x.max(c.x);
            min_y = min_y.min(c.y);
            max_y = max_y.max(c.y);
        }
    }
    if let Some(bb) = &layout.baseboard {
        if bb.enabled {
            let cx = bb.custom_x.unwrap_or(0.0);
            let cy = bb.custom_y.unwrap_or(0.0);
            min_x = min_x.min(cx - bb.width * 0.5);
            max_x = max_x.max(cx + bb.width * 0.5);
            min_y = min_y.min(cy - bb.depth * 0.5);
            max_y = max_y.max(cy + bb.depth * 0.5);
        }
    }
    if !min_x.is_finite() {
        return (-500.0, 500.0, -300.0, 300.0);
    }
    (min_x - 80.0, max_x + 80.0, min_y - 80.0, max_y + 80.0)
}

/// Exports a crisp 1:1 millimeter vector SVG blueprint.
pub fn export_layout_svg(layout: &Layout) -> String {
    let (min_x, max_x, min_y, max_y) = layout_bounds(layout);
    let w = (max_x - min_x).max(100.0);
    let h = (max_y - min_y).max(100.0);

    let mut svg = String::new();
    let _ = writeln!(
        svg,
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{min_x:.1} {min_y:.1} {w:.1} {h:.1}" width="{w:.1}mm" height="{h:.1}mm">
  <rect x="{min_x:.1}" y="{min_y:.1}" width="{w:.1}" height="{h:.1}" fill="#f8fafc"/>"##
    );

    if let Some(bb) = &layout.baseboard {
        if bb.enabled {
            let bx = bb.custom_x.unwrap_or(0.0) - bb.width * 0.5;
            let by = bb.custom_y.unwrap_or(0.0) - bb.depth * 0.5;
            let _ = writeln!(
                svg,
                r##"  <rect x="{bx:.1}" y="{by:.1}" width="{:.1}" height="{:.1}" fill="#e2e8f0" stroke="#334155" stroke-width="3"/>"##,
                bb.width, bb.depth
            );
        }
    }

    for p in &layout.pieces {
        if p.connectors.len() >= 2 {
            let a = &p.connectors[0];
            let b = &p.connectors[1];
            let _ = writeln!(
                svg,
                r##"  <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#64748b" stroke-width="26" stroke-linecap="round"/>"##,
                a.x, a.y, b.x, b.y
            );
            let _ = writeln!(
                svg,
                r##"  <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#0f172a" stroke-width="3"/>"##,
                a.x, a.y, b.x, b.y
            );
            if p.connectors.len() >= 3 {
                let c = &p.connectors[2];
                let _ = writeln!(
                    svg,
                    r##"  <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#64748b" stroke-width="24" stroke-linecap="round"/>"##,
                    a.x, a.y, c.x, c.y
                );
                let _ = writeln!(
                    svg,
                    r##"  <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#0f172a" stroke-width="3"/>"##,
                    a.x, a.y, c.x, c.y
                );
            }
            let mx = (a.x + b.x) * 0.5;
            let my = (a.y + b.y) * 0.5;
            let _ = writeln!(
                svg,
                r##"  <text x="{mx:.1}" y="{my:.1}" font-family="sans-serif" font-size="14" fill="#ffffff" text-anchor="middle" dominant-baseline="central">{}</text>"##,
                p.catalog_id
            );
        }
    }

    let _ = writeln!(svg, "</svg>");
    svg
}

/// Zero-dependency pure-Rust vector PDF 1.4 blueprint exporter (A4 Landscape, 842 x 595 pt).
pub fn export_layout_pdf(layout: &Layout) -> Vec<u8> {
    let page_w = 842.0_f64;
    let page_h = 595.0_f64;
    let margin = 36.0_f64;

    let (min_x, max_x, min_y, max_y) = layout_bounds(layout);
    let span_x = (max_x - min_x).max(100.0);
    let span_y = (max_y - min_y).max(100.0);
    let scale = ((page_w - margin * 2.0) / span_x).min((page_h - margin * 2.0 - 24.0) / span_y);
    let off_x = (page_w - span_x * scale) * 0.5;
    let off_y = (page_h - 24.0 - span_y * scale) * 0.5;

    let to_pdf = |wx: f64, wy: f64| -> (f64, f64) {
        let px = off_x + (wx - min_x) * scale;
        let py = page_h - 24.0 - (off_y + (wy - min_y) * scale);
        (px, py)
    };

    let mut stream = String::new();
    // Title header
    let safe_title = layout.name.replace(['(', ')', '\\'], " ");
    let _ = writeln!(
        stream,
        "0.1 0.12 0.18 rg BT /F1 12 Tf {margin:.1} {:.1} Td (RAILbench Blueprint: {safe_title} [Scale {} | {} pieces]) Tj ET",
        page_h - 22.0,
        layout.scale,
        layout.pieces.len()
    );

    // Baseboard rectangle
    if let Some(bb) = &layout.baseboard {
        if bb.enabled {
            let cx = bb.custom_x.unwrap_or(0.0);
            let cy = bb.custom_y.unwrap_or(0.0);
            let (x0, y0) = to_pdf(cx - bb.width * 0.5, cy + bb.depth * 0.5);
            let rw = bb.width * scale;
            let rh = bb.depth * scale;
            let _ = writeln!(
                stream,
                "0.92 0.94 0.96 rg 0.25 0.30 0.38 RG 1.2 w {x0:.2} {y0:.2} {rw:.2} {rh:.2} re B"
            );
        }
    }

    // Track lines
    let ballast_pt = (24.0 * scale).clamp(1.5, 10.0);
    let _ = writeln!(stream, "1 J 0.45 0.50 0.58 RG {ballast_pt:.2} w");
    for p in &layout.pieces {
        if p.connectors.len() >= 2 {
            let (x0, y0) = to_pdf(p.connectors[0].x, p.connectors[0].y);
            let (x1, y1) = to_pdf(p.connectors[1].x, p.connectors[1].y);
            let _ = writeln!(stream, "{x0:.2} {y0:.2} m {x1:.2} {y1:.2} l S");
            if p.connectors.len() >= 3 {
                let (x2, y2) = to_pdf(p.connectors[2].x, p.connectors[2].y);
                let _ = writeln!(stream, "{x0:.2} {y0:.2} m {x2:.2} {y2:.2} l S");
            }
        }
    }
    let _ = writeln!(stream, "0.08 0.12 0.20 RG 0.8 w");
    for p in &layout.pieces {
        if p.connectors.len() >= 2 {
            let (x0, y0) = to_pdf(p.connectors[0].x, p.connectors[0].y);
            let (x1, y1) = to_pdf(p.connectors[1].x, p.connectors[1].y);
            let _ = writeln!(stream, "{x0:.2} {y0:.2} m {x1:.2} {y1:.2} l S");
        }
    }

    // Assemble valid PDF 1.4 byte stream with exact xref offsets
    let mut pdf = Vec::<u8>::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::<usize>::new();

    let objects = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string(),
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_string(),
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w:.0} {page_h:.0}] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n"
        ),
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_string(),
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n{stream}endstream\nendobj\n",
            stream.len()
        ),
    ];

    for obj in &objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj.as_bytes());
    }

    let xref_pos = pdf.len();
    let _ = write!(
        &mut String::new(),
        ""
    );
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
