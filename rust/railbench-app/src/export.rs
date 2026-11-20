use std::collections::BTreeMap;
use std::fmt::Write as _;

use railbench_core::geometry::{
    deg2rad, rotate, sample_arc_between_connectors, sample_flex_spline_points,
};
use railbench_core::types::{Catalog, CatalogEntry, Layout, TrackPiece, Vec2};

use crate::board_tools::{get_baseboard_polygon, get_baseboards_slice};
use crate::catalog::{find_entry, is_road_id};

#[derive(Debug, Clone)]
pub struct BlueprintExportOptions {
    pub landscape: bool,
    pub include_overview: bool,
    pub include_layer_pages: bool,
    pub selected_layer_ids: Vec<String>,
    pub include_bom_page: bool,
    pub ghost_other_layers: bool,
    pub show_baseboards: bool,
    pub show_labels: bool,
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
            show_labels: true,
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

/// Exports complete RAILbench layout XML matching `src/io/serialize.ts::toXML`.
pub fn export_layout_xml(layout: &Layout) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let feed_v = layout.feed_voltage.unwrap_or(16.0);
    let _ = write!(
        xml,
        "<layout name=\"{}\" scale=\"{}\" feedVoltage=\"{feed_v}\"",
        escape_xml(&layout.name),
        escape_xml(&layout.scale)
    );
    if let Some(cv) = layout.catenary_feed_voltage {
        let _ = write!(xml, " catenaryFeedVoltage=\"{cv}\"");
    }
    if let Some(ref cm) = layout.catenary_circuit_mode {
        let _ = write!(xml, " catenaryCircuitMode=\"{}\"", escape_xml(cm));
    }
    if let Some(lm) = layout.loco_current_ma {
        let _ = write!(xml, " locoCurrentMa=\"{lm}\"");
    }
    if let Some(lc) = layout.loco_count {
        let _ = write!(xml, " locoCount=\"{lc}\"");
    }
    if let Some(tl) = layout.turnout_lantern_current_ma {
        let _ = write!(xml, " turnoutLanternCurrentMa=\"{tl}\"");
    }
    if let Some(td) = layout.turnout_decoder_current_ma {
        let _ = write!(xml, " turnoutDecoderCurrentMa=\"{td}\"");
    }
    xml.push_str(">\n");

    let boards = get_baseboards_slice(layout);
    if !boards.is_empty() {
        xml.push_str("  <baseboards>\n");
        for bb in &boards {
            let _ = writeln!(
                xml,
                "    <baseboard enabled=\"{}\" id=\"{}\" name=\"{}\" shape=\"{}\" triangleType=\"{}\" rotation=\"{}\" width=\"{}\" depth=\"{}\" positionMode=\"{}\" style=\"{}\" cornerRadius=\"{}\" showDimensions=\"{}\" customX=\"{}\" customY=\"{}\" lWingWidth=\"{}\" lWingDepth=\"{}\" lSide=\"{}\" uLeftWingWidth=\"{}\" uRightWingWidth=\"{}\" uBackDepth=\"{}\" tBarDepth=\"{}\" tStemWidth=\"{}\"/>",
                bb.enabled,
                escape_xml(bb.id.as_deref().unwrap_or("")),
                escape_xml(bb.name.as_deref().unwrap_or("Table")),
                escape_xml(bb.shape.as_deref().unwrap_or("rect")),
                escape_xml(bb.triangle_type.as_deref().unwrap_or("corner-tl")),
                bb.rotation.unwrap_or(0.0),
                bb.width,
                bb.depth,
                escape_xml(&bb.position_mode),
                escape_xml(&bb.style),
                bb.corner_radius.unwrap_or(12.0),
                bb.show_dimensions.unwrap_or(true),
                bb.custom_x.unwrap_or(0.0),
                bb.custom_y.unwrap_or(0.0),
                bb.l_wing_width.unwrap_or(800.0),
                bb.l_wing_depth.unwrap_or(800.0),
                escape_xml(bb.l_side.as_deref().unwrap_or("left")),
                bb.u_left_wing_width.unwrap_or(700.0),
                bb.u_right_wing_width.unwrap_or(700.0),
                bb.u_back_depth.unwrap_or(600.0),
                bb.t_bar_depth.unwrap_or(600.0),
                bb.t_stem_width.unwrap_or(800.0),
            );
        }
        xml.push_str("  </baseboards>\n");
    }

    if let Some(layers) = &layout.layers {
        if !layers.is_empty() {
            let active = layout
                .active_layer_id
                .as_deref()
                .unwrap_or(layers[0].id.as_str());
            let _ = writeln!(xml, "  <layers activeLayerId=\"{}\">", escape_xml(active));
            for lay in layers {
                let _ = write!(
                    xml,
                    "    <layer id=\"{}\" name=\"{}\" color=\"{}\" visible=\"{}\" locked=\"{}\" heightMm=\"{}\"",
                    escape_xml(&lay.id),
                    escape_xml(&lay.name),
                    escape_xml(lay.color.as_deref().unwrap_or("#38bdf8")),
                    lay.visible,
                    lay.locked,
                    lay.height_mm.unwrap_or(0.0)
                );
                if let Some(ref wm) = lay.wood_plate_mode {
                    let _ = write!(xml, " woodPlateMode=\"{}\"", escape_xml(wm));
                }
                if let Some(wt) = lay.wood_plate_thickness_mm {
                    let _ = write!(xml, " woodPlateThicknessMm=\"{wt}\"");
                }
                if let Some(ref ws) = lay.wood_plate_style {
                    let _ = write!(xml, " woodPlateStyle=\"{}\"", escape_xml(ws));
                }
                if let Some(plates) = &lay.plates {
                    if !plates.is_empty() {
                        xml.push_str(">\n");
                        for pl in plates {
                            let _ = write!(
                                xml,
                                "      <plate id=\"{}\" width=\"{}\" depth=\"{}\" x=\"{}\" y=\"{}\" thicknessMm=\"{}\"",
                                escape_xml(&pl.id),
                                pl.width,
                                pl.depth,
                                pl.x.unwrap_or(0.0),
                                pl.y.unwrap_or(0.0),
                                pl.thickness_mm.unwrap_or(10.0)
                            );
                            if let Some(ref pn) = pl.name {
                                let _ = write!(xml, " name=\"{}\"", escape_xml(pn));
                            }
                            if let Some(ref bid) = pl.baseboard_id {
                                let _ = write!(xml, " baseboardId=\"{}\"", escape_xml(bid));
                            }
                            if let Some(ref sh) = pl.shape {
                                let _ = write!(xml, " shape=\"{}\"", escape_xml(sh));
                            }
                            if let Some(ref st) = pl.style {
                                let _ = write!(xml, " style=\"{}\"", escape_xml(st));
                            }
                            xml.push_str("/>\n");
                        }
                        xml.push_str("    </layer>\n");
                        continue;
                    }
                }
                xml.push_str("/>\n");
            }
            xml.push_str("  </layers>\n");
        }
    }

    xml.push_str("  <pieces>\n");
    for p in &layout.pieces {
        let _ = write!(
            xml,
            "    <piece id=\"{}\" catalogId=\"{}\" x=\"{:.2}\" y=\"{:.2}\" rotation=\"{:.2}\"",
            escape_xml(&p.id),
            escape_xml(&p.catalog_id),
            p.transform.x,
            p.transform.y,
            p.transform.rotation
        );
        if let Some(ref lid) = p.layer_id {
            let _ = write!(xml, " layerId=\"{}\"", escape_xml(lid));
        }
        if let Some(ev) = p.elevation {
            let _ = write!(xml, " elevStart=\"{:.2}\" elevEnd=\"{:.2}\"", ev.start, ev.end);
        }
        if let Some(cl) = p.custom_length {
            let _ = write!(xml, " customLength=\"{cl:.1}\"");
        }
        if let Some(ca) = p.custom_angle {
            let _ = write!(xml, " customAngle=\"{ca:.2}\"");
        }
        if p.power_feed == Some(true) { xml.push_str(" powerFeed=\"true\""); }
        if p.is_isolator == Some(true) { xml.push_str(" isIsolator=\"true\""); }
        if p.has_catenary == Some(true) { xml.push_str(" hasCatenary=\"true\""); }
        if p.catenary_power_feed == Some(true) { xml.push_str(" catenaryPowerFeed=\"true\""); }
        if p.catenary_isolator == Some(true) { xml.push_str(" catenaryIsolator=\"true\""); }
        if p.has_bridge == Some(true) { xml.push_str(" hasBridge=\"true\""); }
        if p.has_tunnel == Some(true) { xml.push_str(" hasTunnel=\"true\""); }
        if p.has_lantern == Some(true) { xml.push_str(" hasLantern=\"true\""); }
        if p.has_decoder == Some(true) { xml.push_str(" hasDecoder=\"true\""); }
        if p.has_signal == Some(true) { xml.push_str(" hasSignal=\"true\""); }
        if let Some(ref cn) = p.custom_name {
            let _ = write!(xml, " customName=\"{}\"", escape_xml(cn));
        }
        xml.push_str("/>\n");
    }
    xml.push_str("  </pieces>\n");

    if let Some(models) = &layout.models {
        if !models.is_empty() {
            xml.push_str("  <models>\n");
            for m in models {
                let _ = write!(
                    xml,
                    "    <model id=\"{}\" name=\"{}\" category=\"{}\" modelType=\"{}\" x=\"{:.2}\" y=\"{:.2}\" rotation=\"{:.2}\" width=\"{:.1}\" depth=\"{:.1}\" height=\"{:.1}\"",
                    escape_xml(&m.id),
                    escape_xml(&m.name),
                    escape_xml(&m.category),
                    escape_xml(&m.model_type),
                    m.transform.x,
                    m.transform.y,
                    m.transform.rotation,
                    m.width,
                    m.depth,
                    m.height
                );
                if let Some(ref bid) = m.builtin_id {
                    let _ = write!(xml, " builtinId=\"{}\"", escape_xml(bid));
                }
                if let Some(ref fnm) = m.file_name {
                    let _ = write!(xml, " fileName=\"{}\"", escape_xml(fnm));
                }
                if let Some(ref lid) = m.layer_id {
                    let _ = write!(xml, " layerId=\"{}\"", escape_xml(lid));
                }
                if let Some(ev) = m.elevation {
                    let _ = write!(xml, " elevation=\"{ev:.2}\"");
                }
                if let Some(sc) = m.scale {
                    let _ = write!(xml, " scale=\"{sc:.3}\"");
                }
                if let Some(ref fd) = m.file_data {
                    let _ = writeln!(xml, ">\n      <fileData>{}</fileData>\n    </model>", escape_xml(fd));
                } else {
                    xml.push_str("/>\n");
                }
            }
            xml.push_str("  </models>\n");
        }
    }

    xml.push_str("</layout>");
    xml
}

fn sample_track_branches_world(piece: &TrackPiece) -> Vec<Vec<Vec2>> {
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

fn model_corners_world(m: &railbench_core::types::Model3D) -> [Vec2; 4] {
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
    let (min_x, max_x, min_y, max_y) = layout_bounds(layout);
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

        for br in sample_track_branches_world(p) {
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
            let _ = writeln!(
                svg,
                r##"  <polyline points="{pts_s}" fill="none" stroke="{rail_col}" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>"##
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

    let _ = writeln!(svg, "</svg>");
    svg
}

fn pdf_escape(s: &str) -> String {
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

fn draw_pdf_plan_page(
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
    let (min_x, max_x, min_y, max_y) = layout_bounds(layout);
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
            let is_road = is_road_id(catalogs, &p.catalog_id);
            if ghost {
                let _ = writeln!(stream, "1 J 0.84 0.87 0.90 RG {ballast_w:.2} w");
            } else if opts.monochrome {
                let _ = writeln!(stream, "1 J 0.72 0.75 0.80 RG {ballast_w:.2} w");
            } else if is_road {
                let _ = writeln!(stream, "1 J 0.32 0.37 0.44 RG {ballast_w:.2} w");
            } else {
                let _ = writeln!(stream, "1 J 0.45 0.52 0.60 RG {ballast_w:.2} w");
            }
            for br in sample_track_branches_world(p) {
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
                for br in sample_track_branches_world(p) {
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

fn draw_pdf_bom_page(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece};
    use railbench_core::types::{BaseboardConfig, Layer, Model3D, Transform};

    fn sample_export_layout() -> (Layout, Vec<Catalog>) {
        let catalogs = builtin_catalogs();
        let straight = find_entry(&catalogs, "24188").unwrap();
        let curve = find_entry(&catalogs, "24130").unwrap();
        let p1 = create_piece("p1".into(), straight, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
        let p2 = create_piece("p2".into(), curve, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, Some("layer-2".into()));
        let m1 = Model3D {
            id: "m1".into(),
            name: "Bahnhof".into(),
            category: "buildings".into(),
            transform: Transform { x: 100.0, y: -120.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: Some("layer-1".into()),
            width: 320.0,
            depth: 140.0,
            height: 120.0,
            model_type: "builtin".into(),
            builtin_id: Some("station".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let bb = BaseboardConfig {
            id: Some("board-1".into()),
            name: Some("Tisch".into()),
            enabled: true,
            shape: Some("l-shape".into()),
            custom_polygon: None,
            triangle_type: None,
            width: 2400.0,
            depth: 1200.0,
            rotation: Some(0.0),
            l_wing_width: Some(800.0),
            l_wing_depth: Some(600.0),
            l_side: Some("left".into()),
            u_left_wing_width: None,
            u_right_wing_width: None,
            u_back_depth: None,
            t_bar_depth: None,
            t_stem_width: None,
            position_mode: "center".into(),
            custom_x: None,
            custom_y: None,
            style: "wood".into(),
            corner_radius: Some(12.0),
            show_dimensions: Some(true),
        };
        let layout = Layout {
            name: "ExportTest".into(),
            scale: "H0".into(),
            pieces: vec![p1, p2],
            models: Some(vec![m1]),
            feed_voltage: Some(16.0),
            catenary_feed_voltage: Some(15.0),
            catenary_circuit_mode: Some("separate".into()),
            loco_current_ma: Some(450.0),
            loco_count: Some(2),
            train_current_amps: None,
            turnout_lantern_current_ma: Some(20.0),
            turnout_decoder_current_ma: Some(15.0),
            layers: Some(vec![
                Layer {
                    id: "layer-1".into(),
                    name: "Ebene 0".into(),
                    color: Some("#38bdf8".into()),
                    visible: true,
                    locked: false,
                    height_mm: Some(0.0),
                    wood_plate_mode: Some("table".into()),
                    wood_plate_thickness_mm: Some(20.0),
                    wood_plate_style: None,
                    plates: None,
                },
                Layer {
                    id: "layer-2".into(),
                    name: "Ebene 1".into(),
                    color: Some("#f59e0b".into()),
                    visible: true,
                    locked: false,
                    height_mm: Some(85.0),
                    wood_plate_mode: None,
                    wood_plate_thickness_mm: None,
                    wood_plate_style: None,
                    plates: None,
                },
            ]),
            active_layer_id: Some("layer-1".into()),
            baseboard: Some(bb.clone()),
            baseboards: Some(vec![bb]),
        };
        (layout, catalogs)
    }

    #[test]
    fn bom_summary_and_csv_separate_tracks_roads_and_models() {
        let (layout, catalogs) = sample_export_layout();
        let sum = get_piece_list_summary(&layout, &catalogs, true);
        assert_eq!(sum.tracks.total_pieces, 2);
        assert_eq!(sum.total_buildings, 1);
        assert_eq!(sum.total_objects, 3);
        let csv = export_bom_csv(&layout, &catalogs, false);
        assert!(csv.contains("24188"));
        assert!(csv.contains("24130"));
        assert!(csv.contains("Bahnhof"));
        assert!(csv.contains("--- 1. Railway Tracks ---"));
    }

    #[test]
    fn xml_export_includes_multi_table_layers_pieces_and_models() {
        let (layout, _) = sample_export_layout();
        let xml = export_layout_xml(&layout);
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("<baseboards>"));
        assert!(xml.contains("shape=\"l-shape\""));
        assert!(xml.contains("<layers activeLayerId=\"layer-1\">"));
        assert!(xml.contains("catalogId=\"24188\""));
        assert!(xml.contains("<model id=\"m1\""));
    }

    #[test]
    fn svg_and_multipage_pdf_export_valid_structure() {
        let (layout, catalogs) = sample_export_layout();
        let opts = BlueprintExportOptions {
            include_overview: true,
            include_layer_pages: true,
            include_bom_page: true,
            ..Default::default()
        };
        let svg = export_layout_svg_with_options(&layout, &catalogs, &opts);
        assert!(svg.contains("<polygon points="));
        assert!(svg.contains("<polyline points="));

        let pdf_bytes = export_layout_pdf_with_options(&layout, &catalogs, &opts, false);
        let pdf_text = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_text.starts_with("%PDF-1.4"));
        assert!(pdf_text.contains("/Count 4"));
        assert!(pdf_text.contains("%%EOF"));
    }
}
