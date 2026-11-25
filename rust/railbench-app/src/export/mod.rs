use std::collections::BTreeMap;
use std::fmt::Write as _;

use railbench_core::geometry::{
    deg2rad, rotate, sample_arc_between_connectors, sample_flex_spline_points,
};
use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{
    BaseboardConfig, Catalog, CatalogEntry, Elevation, Layer, LayerPlate, Layout, Model3D,
    TrackPiece, Transform, Vec2,
};

use crate::board_tools::{get_baseboard_polygon, get_baseboards_slice};
use crate::catalog::{create_piece, find_entry, is_road_id};

#[derive(Debug, Clone)]
pub struct BlueprintExportOptions {
    pub landscape: bool,
    pub include_overview: bool,
    pub include_layer_pages: bool,
    pub selected_layer_ids: Vec<String>,
    pub include_bom_page: bool,
    pub ghost_other_layers: bool,
    pub show_baseboards: bool,
    pub show_models: bool,
    pub show_labels: bool,
    pub show_grid: bool,
    pub show_electrical: bool,
    pub monochrome: bool,
}

impl Default for BlueprintExportOptions {
    fn default() -> Self {
        Self {
            landscape: true,
            include_overview: true,
            include_layer_pages: false,
            selected_layer_ids: Vec::new(),
            include_bom_page: true,
            ghost_other_layers: true,
            show_baseboards: true,
            show_models: true,
            show_labels: true,
            show_grid: true,
            show_electrical: true,
            monochrome: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PieceSummaryItem {
    pub catalog_id: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub count: usize,
    pub specification: String,
    pub total_length_meters: f64,
}

#[derive(Debug, Clone)]
pub struct ModelSummaryItem {
    pub name: String,
    pub category: String,
    pub model_type: String,
    pub dimensions: String,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct PieceCategorySummary {
    pub items: Vec<PieceSummaryItem>,
    pub total_pieces: usize,
    pub total_length_meters: f64,
}

#[derive(Debug, Clone)]
pub struct PieceListSummary {
    pub total_pieces: usize,
    pub total_length_meters: f64,
    pub total_objects: usize,
    pub tracks: PieceCategorySummary,
    pub roads: PieceCategorySummary,
    pub buildings: Vec<ModelSummaryItem>,
    pub total_buildings: usize,
}

fn piece_length_and_spec(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    is_road: bool,
) -> (f64, String) {
    if let Some(cl) = piece.custom_length {
        if cl > 1.0 {
            let ang = piece.custom_angle.unwrap_or(0.0);
            let spec = if ang.abs() > 0.1 {
                format!("L = {:.0} mm / {:.1} deg", cl, ang)
            } else {
                format!("L = {:.0} mm", cl)
            };
            return (cl / 1000.0, spec);
        }
    }
    if let Some(e) = entry {
        if let Some(len) = e.length {
            let spec = if is_road {
                format!("L = {:.0} mm (B = {:.0} mm)", len, e.road_width.unwrap_or(60.0))
            } else {
                format!("L = {:.0} mm", len)
            };
            return (len / 1000.0, spec);
        }
        if let (Some(r), Some(a)) = (e.radius, e.angle) {
            let len_m = (r * deg2rad(a.abs())) / 1000.0;
            return (len_m, format!("R = {:.0} mm / {:.1} deg", r, a));
        }
    }
    if piece.connectors.len() >= 2 {
        let d = (piece.connectors[0].x - piece.connectors[1].x)
            .hypot(piece.connectors[0].y - piece.connectors[1].y);
        if d > 1.0 {
            return (d / 1000.0, format!("L = {:.0} mm", d));
        }
    }
    (0.0, String::new())
}

pub fn get_layer_piece_list_summary(
    layout: &Layout,
    catalogs: &[Catalog],
    layer_id: &str,
    is_de: bool,
) -> PieceCategorySummary {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut sample_piece: BTreeMap<&str, &TrackPiece> = BTreeMap::new();
    for p in &layout.pieces {
        let p_lid = p.layer_id.as_deref().unwrap_or("layer-1");
        if p_lid != layer_id {
            continue;
        }
        sample_piece.entry(p.catalog_id.as_str()).or_insert(p);
        *counts.entry(p.catalog_id.as_str()).or_insert(0) += 1;
    }
    let mut items = Vec::with_capacity(counts.len());
    let mut total_pieces = 0usize;
    let mut total_m = 0.0_f64;
    for (cid, cnt) in counts {
        let entry = find_entry(catalogs, cid);
        let is_road_item = is_road_id(catalogs, cid);
        let name = entry
            .map(|e| if is_de { e.name.as_str() } else { e.name_en.as_deref().unwrap_or(&e.name) })
            .unwrap_or(cid)
            .to_string();
        let category = entry
            .map(|e| e.category.clone())
            .unwrap_or_else(|| if is_road_item { "Road" } else { "Track" }.into());
        let kind = entry.map(|e| e.kind.clone()).unwrap_or_else(|| "unknown".into());
        let (unit_m, spec) = sample_piece
            .get(cid)
            .map(|p| piece_length_and_spec(p, entry, is_road_item))
            .unwrap_or((0.0, String::new()));
        let len_m = ((unit_m * cnt as f64) * 100.0).round() / 100.0;
        total_pieces += cnt;
        total_m += len_m;
        items.push(PieceSummaryItem {
            catalog_id: cid.to_string(),
            name,
            category,
            kind,
            count: cnt,
            specification: spec,
            total_length_meters: len_m,
        });
    }
    PieceCategorySummary {
        items,
        total_pieces,
        total_length_meters: (total_m * 100.0).round() / 100.0,
    }
}

pub fn get_piece_list_summary(
    layout: &Layout,
    catalogs: &[Catalog],
    is_de: bool,
) -> PieceListSummary {
    let mut track_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut road_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut sample_piece: BTreeMap<&str, &TrackPiece> = BTreeMap::new();

    for p in &layout.pieces {
        sample_piece.entry(p.catalog_id.as_str()).or_insert(p);
        if is_road_id(catalogs, &p.catalog_id) {
            *road_counts.entry(p.catalog_id.as_str()).or_insert(0) += 1;
        } else {
            *track_counts.entry(p.catalog_id.as_str()).or_insert(0) += 1;
        }
    }

    let build_cat = |counts: BTreeMap<&str, usize>, is_road_list: bool| -> PieceCategorySummary {
        let mut items = Vec::with_capacity(counts.len());
        let mut total_pieces = 0usize;
        let mut total_m = 0.0_f64;
        for (cid, cnt) in counts {
            let entry = find_entry(catalogs, cid);
            let name = entry
                .map(|e| {
                    if is_de {
                        e.name.as_str()
                    } else {
                        e.name_en.as_deref().unwrap_or(&e.name)
                    }
                })
                .unwrap_or(cid)
                .to_string();
            let category = entry
                .map(|e| e.category.clone())
                .unwrap_or_else(|| if is_road_list { "Road" } else { "Track" }.into());
            let kind = entry.map(|e| e.kind.clone()).unwrap_or_else(|| "unknown".into());
            let (unit_m, spec) = sample_piece
                .get(cid)
                .map(|p| piece_length_and_spec(p, entry, is_road_list))
                .unwrap_or((0.0, String::new()));
            let len_m = ((unit_m * cnt as f64) * 100.0).round() / 100.0;
            total_pieces += cnt;
            total_m += len_m;
            items.push(PieceSummaryItem {
                catalog_id: cid.to_string(),
                name,
                category,
                kind,
                count: cnt,
                specification: spec,
                total_length_meters: len_m,
            });
        }
        PieceCategorySummary {
            items,
            total_pieces,
            total_length_meters: (total_m * 100.0).round() / 100.0,
        }
    };

    let tracks = build_cat(track_counts, false);
    let roads = build_cat(road_counts, true);

    let mut model_groups: BTreeMap<String, ModelSummaryItem> = BTreeMap::new();
    let mut total_buildings = 0usize;
    if let Some(models) = &layout.models {
        total_buildings = models.len();
        for m in models {
            let key = format!(
                "{}|{}|{:.0}|{:.0}|{:.0}",
                m.name, m.category, m.width, m.depth, m.height
            );
            model_groups
                .entry(key)
                .and_modify(|it| it.count += 1)
                .or_insert_with(|| ModelSummaryItem {
                    name: m.name.clone(),
                    category: m.category.clone(),
                    model_type: m.model_type.clone(),
                    dimensions: format!("{:.0} x {:.0} x {:.0} mm", m.width, m.depth, m.height),
                    count: 1,
                });
        }
    }
    let buildings: Vec<ModelSummaryItem> = model_groups.into_values().collect();
    let total_pieces = tracks.total_pieces + roads.total_pieces;
    let total_length_meters =
        ((tracks.total_length_meters + roads.total_length_meters) * 100.0).round() / 100.0;

    PieceListSummary {
        total_pieces,
        total_length_meters,
        total_objects: total_pieces + total_buildings,
        tracks,
        roads,
        buildings,
        total_buildings,
    }
}

/// Exports a rich multi-section Bill of Materials CSV matching TS `exportPieceListCSV`.
pub fn export_bom_csv(layout: &Layout, catalogs: &[Catalog], is_de: bool) -> String {
    let sum = get_piece_list_summary(layout, catalogs, is_de);
    let mut csv = String::new();
    let crlf = "\r\n";

    let _ = write!(
        csv,
        "{};{}{}{};{}{}{};{} ({:.2} m){}{};{} ({:.2} m){}{};{}{}",
        if is_de { "Stueckliste fuer Gleisplan:" } else { "Parts List for Layout:" },
        layout.name,
        crlf,
        if is_de { "Massstab:" } else { "Scale:" },
        layout.scale,
        crlf,
        if is_de { "Gleise gesamt:" } else { "Total tracks:" },
        sum.tracks.total_pieces,
        sum.tracks.total_length_meters,
        crlf,
        if is_de { "Strassen gesamt:" } else { "Total roads:" },
        sum.roads.total_pieces,
        sum.roads.total_length_meters,
        crlf,
        if is_de { "Gebaeude & 3D-Modelle:" } else { "Buildings & 3D Models:" },
        sum.total_buildings,
        crlf
    );
    let _ = write!(csv, "{crlf}");

    if !sum.tracks.items.is_empty() {
        let _ = write!(
            csv,
            "{}{}",
            if is_de { "--- 1. Gleise & Weichen ---" } else { "--- 1. Railway Tracks ---" },
            crlf
        );
        let _ = write!(
            csv,
            "{}{}",
            if is_de {
                "Art.-Nr.;Bezeichnung;Kategorie;Gleistyp;Spezifikation;Anzahl;Laenge (m)"
            } else {
                "Part No.;Name;Category;Kind;Specification;Qty;Track Length (m)"
            },
            crlf
        );
        for it in &sum.tracks.items {
            let _ = write!(
                csv,
                "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{:.2}{}",
                it.catalog_id,
                it.name,
                it.category,
                it.kind,
                it.specification,
                it.count,
                it.total_length_meters,
                crlf
            );
        }
        let _ = write!(csv, "{crlf}");
    }

    if !sum.roads.items.is_empty() {
        let _ = write!(
            csv,
            "{}{}",
            if is_de { "--- 2. Strassen & Fahrbahnen ---" } else { "--- 2. Roads & Streets ---" },
            crlf
        );
        let _ = write!(
            csv,
            "{}{}",
            if is_de {
                "Art.-Nr.;Bezeichnung;Kategorie;Typ;Spezifikation;Anzahl;Laenge (m)"
            } else {
                "Part No.;Name;Category;Kind;Specification;Qty;Road Length (m)"
            },
            crlf
        );
        for it in &sum.roads.items {
            let _ = write!(
                csv,
                "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{:.2}{}",
                it.catalog_id,
                it.name,
                it.category,
                it.kind,
                it.specification,
                it.count,
                it.total_length_meters,
                crlf
            );
        }
        let _ = write!(csv, "{crlf}");
    }

    if !sum.buildings.is_empty() {
        let _ = write!(
            csv,
            "{}{}",
            if is_de { "--- 3. Gebaeude & 3D-Objekte ---" } else { "--- 3. Buildings & 3D Objects ---" },
            crlf
        );
        let _ = write!(
            csv,
            "{}{}",
            if is_de {
                "Modellname;Kategorie;Typ;Masse (B x T x H mm);Anzahl"
            } else {
                "Model Name;Category;Type;Dimensions (W x D x H mm);Qty"
            },
            crlf
        );
        for it in &sum.buildings {
            let _ = write!(
                csv,
                "\"{}\";\"{}\";\"{}\";\"{}\";{}{}",
                it.name, it.category, it.model_type, it.dimensions, it.count, crlf
            );
        }
    }

    csv
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

mod xml;
#[allow(unused_imports)]
use xml::*;
pub use xml::{export_layout_xml, import_layout_xml};
mod svg;
#[allow(unused_imports)]
use svg::*;
pub use svg::{export_layout_svg_with_options};
pub(crate) use svg::{layout_bounds, sample_track_branches_world};
mod pdf;
#[allow(unused_imports)]
use pdf::*;
pub use pdf::{export_layout_pdf_with_options};
#[cfg(test)]
mod tests;
