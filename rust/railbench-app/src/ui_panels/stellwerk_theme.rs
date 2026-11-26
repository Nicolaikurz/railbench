use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};
use railbench_core::types::TrackPiece;

use crate::canvas2d::parse_hex_color;
use crate::catalog::is_road_id;
use crate::i18n::Lang;
use crate::vehicle_sim::VehicleDomain;
use crate::RailbenchApp;


pub(super) const LAYER_FALLBACK_COLORS: [Color32; 5] = [
    Color32::from_rgb(56, 189, 248),
    Color32::from_rgb(52, 211, 153),
    Color32::from_rgb(251, 191, 36),
    Color32::from_rgb(192, 132, 252),
    Color32::from_rgb(248, 113, 113),
];

#[derive(Debug, Clone)]
pub(super) struct SignalBoxLayer {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) color: Color32,
    pub(super) height_mm: f64,
    pub(super) rail_count: usize,
    pub(super) train_count: usize,
}

pub(super) fn layer_color(hex: Option<&str>, fallback_idx: usize) -> Color32 {
    parse_hex_color(hex)
        .unwrap_or_else(|| LAYER_FALLBACK_COLORS[fallback_idx % LAYER_FALLBACK_COLORS.len()])
}

pub(super) fn piece_layer_id(piece: &TrackPiece) -> &str {
    piece.layer_id.as_deref().unwrap_or("layer-1")
}

pub(super) fn build_piece_layer_map(app: &RailbenchApp) -> HashMap<String, String> {
    app.layout
        .pieces
        .iter()
        .map(|p| (p.id.clone(), piece_layer_id(p).to_string()))
        .collect()
}

/// Collects Signal Box layers sorted by elevation ascending (`height_mm`),
/// counting rail pieces and active rail vehicles on each layer.
pub(super) fn collect_signal_box_layers(
    app: &RailbenchApp,
    piece_layers: &HashMap<String, String>,
) -> Vec<SignalBoxLayer> {
    let mut rail_counts: HashMap<String, usize> = HashMap::new();
    for p in &app.layout.pieces {
        if !is_road_id(&app.catalogs, &p.catalog_id) {
            *rail_counts
                .entry(piece_layer_id(p).to_string())
                .or_insert(0) += 1;
        }
    }

    let mut train_counts: HashMap<String, usize> = HashMap::new();
    for v in &app.vehicle_sim.vehicles {
        if v.domain == VehicleDomain::Rail {
            let lid = piece_layers
                .get(&v.piece_id)
                .map(String::as_str)
                .unwrap_or("layer-1")
                .to_string();
            *train_counts.entry(lid).or_insert(0) += 1;
        }
    }

    let mut layers = Vec::new();
    let mut seen = HashSet::new();
    if let Some(ref defs) = app.layout.layers {
        for (idx, def) in defs.iter().enumerate() {
            seen.insert(def.id.clone());
            layers.push(SignalBoxLayer {
                id: def.id.clone(),
                name: def.name.clone(),
                color: layer_color(def.color.as_deref(), idx),
                height_mm: def.height_mm.unwrap_or(0.0),
                rail_count: rail_counts.get(&def.id).copied().unwrap_or(0),
                train_count: train_counts.get(&def.id).copied().unwrap_or(0),
            });
        }
    }

    let mut extra_ids: Vec<String> = rail_counts
        .keys()
        .filter(|id| !seen.contains(*id))
        .cloned()
        .collect();
    extra_ids.sort();
    for lid in extra_ids {
        let idx = layers.len();
        let name = if lid == "layer-1" {
            if app.lang == Lang::De { "Ebene 1" } else { "Layer 1" }.to_string()
        } else {
            lid.clone()
        };
        layers.push(SignalBoxLayer {
            name,
            color: layer_color(None, idx),
            height_mm: 0.0,
            rail_count: rail_counts.get(&lid).copied().unwrap_or(0),
            train_count: train_counts.get(&lid).copied().unwrap_or(0),
            id: lid,
        });
    }

    if layers.is_empty() {
        layers.push(SignalBoxLayer {
            id: "layer-1".to_string(),
            name: if app.lang == Lang::De { "Ebene 1" } else { "Layer 1" }.to_string(),
            color: LAYER_FALLBACK_COLORS[0],
            height_mm: 0.0,
            rail_count: 0,
            train_count: train_counts.get("layer-1").copied().unwrap_or(0),
        });
    }

    layers.sort_by(|a, b| a.height_mm.total_cmp(&b.height_mm));
    layers
}

/// Computes world bounding box of rail pieces matching `layer_filter` (`None` = all layers).
pub(super) fn rail_bounds(app: &RailbenchApp, layer_filter: Option<&str>) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in &app.layout.pieces {
        if is_road_id(&app.catalogs, &p.catalog_id) {
            continue;
        }
        if layer_filter.is_some_and(|lid| piece_layer_id(p) != lid) {
            continue;
        }
        if p.connectors.is_empty() {
            min_x = min_x.min(p.transform.x);
            max_x = max_x.max(p.transform.x);
            min_y = min_y.min(p.transform.y);
            max_y = max_y.max(p.transform.y);
        } else {
            for c in &p.connectors {
                min_x = min_x.min(c.x);
                max_x = max_x.max(c.x);
                min_y = min_y.min(c.y);
                max_y = max_y.max(c.y);
            }
        }
    }
    min_x
        .is_finite()
        .then_some((min_x - 80.0, min_y - 80.0, max_x + 80.0, max_y + 80.0))
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StellwerkTheme {
    pub dark_mode: bool,
    pub header_bg: Color32,
    pub header_border: Color32,
    pub header_text: Color32,
    pub header_muted: Color32,
    pub btn_bg: Color32,
    pub btn_border: Color32,
    pub active_fill: Color32,
    pub active_border: Color32,
    pub grid_bg: Color32,
    pub grid_line: Color32,
    pub pane_border: Color32,
    pub track_casing: Color32,
    pub track_idle: Color32,
    pub turnout_inactive: Color32,
    pub route_active: Color32,
    pub occupied: Color32,
    pub occupied_glow: Color32,
    pub clear_dot: Color32,
    pub pit_bg: Color32,
    pub pit_border: Color32,
    pub pit_text: Color32,
    pub signal_housing: Color32,
    pub signal_border: Color32,
    pub signal_mast: Color32,
    pub turnout_straight_fill: Color32,
    pub turnout_diverge_fill: Color32,
    pub turnout_ring: Color32,
    pub turnout_label: Color32,
    pub nameplate_bg: Color32,
    pub legend_bg: Color32,
    pub legend_text: Color32,
    pub legend_hint: Color32,
    pub train_tag_bg: Color32,
    pub train_tag_text_ok: Color32,
    pub train_tag_text_halt: Color32,
    pub train_selected: Color32,
    pub train_idle: Color32,
}

pub(crate) fn stellwerk_theme(dark_mode: bool) -> StellwerkTheme {
    let p = crate::theme::palette(dark_mode);
    if dark_mode {
        StellwerkTheme {
            dark_mode: true,
            header_bg: Color32::from_rgb(18, 26, 36),
            header_border: Color32::from_rgb(45, 58, 74),
            header_text: Color32::from_rgb(248, 250, 252),
            header_muted: Color32::from_rgb(148, 163, 184),
            btn_bg: Color32::from_rgb(28, 39, 54),
            btn_border: Color32::from_rgb(59, 75, 96),
            active_fill: p.primary_hover,
            active_border: p.primary,
            grid_bg: Color32::from_rgb(14, 21, 29),
            grid_line: Color32::from_rgba_unmultiplied(51, 65, 85, 38),
            pane_border: Color32::from_rgb(45, 58, 74),
            track_casing: Color32::from_rgb(7, 11, 17),
            track_idle: Color32::from_rgb(100, 116, 139),
            turnout_inactive: Color32::from_rgb(45, 58, 74),
            route_active: Color32::from_rgb(250, 204, 21),
            occupied: Color32::from_rgb(239, 68, 68),
            occupied_glow: Color32::from_rgba_unmultiplied(239, 68, 68, 52),
            clear_dot: Color32::from_rgb(34, 197, 94),
            pit_bg: Color32::from_rgb(20, 30, 42),
            pit_border: Color32::from_rgb(100, 116, 139),
            pit_text: Color32::WHITE,
            signal_housing: Color32::from_rgb(9, 13, 20),
            signal_border: Color32::from_rgb(71, 85, 105),
            signal_mast: Color32::from_rgb(148, 163, 184),
            turnout_straight_fill: Color32::from_rgb(2, 132, 199),
            turnout_diverge_fill: Color32::from_rgb(217, 119, 6),
            turnout_ring: Color32::from_rgb(250, 204, 21),
            turnout_label: Color32::from_rgb(203, 213, 225),
            nameplate_bg: Color32::from_rgba_unmultiplied(15, 23, 42, 236),
            legend_bg: Color32::from_rgba_unmultiplied(15, 23, 42, 228),
            legend_text: Color32::from_rgb(203, 213, 225),
            legend_hint: Color32::from_rgb(148, 163, 184),
            train_tag_bg: Color32::from_rgb(9, 13, 20),
            train_tag_text_ok: Color32::from_rgb(74, 222, 128),
            train_tag_text_halt: Color32::from_rgb(251, 191, 36),
            train_selected: Color32::from_rgb(56, 189, 248),
            train_idle: Color32::from_rgb(248, 113, 113),
        }
    } else {
        StellwerkTheme {
            dark_mode: false,
            header_bg: p.bg_sidebar,
            header_border: p.border_main,
            header_text: p.text_main,
            header_muted: p.text_muted,
            btn_bg: p.bg_card,
            btn_border: p.border_main,
            active_fill: p.primary_hover,
            active_border: p.primary,
            grid_bg: p.canvas_bg,
            grid_line: Color32::from_rgba_unmultiplied(138, 126, 108, 44),
            pane_border: Color32::from_rgb(194, 184, 166),
            track_casing: Color32::from_rgb(214, 204, 184),
            track_idle: Color32::from_rgb(71, 85, 105),
            turnout_inactive: Color32::from_rgb(184, 173, 154),
            route_active: Color32::from_rgb(217, 119, 6),
            occupied: Color32::from_rgb(220, 38, 38),
            occupied_glow: Color32::from_rgba_unmultiplied(220, 38, 38, 44),
            clear_dot: Color32::from_rgb(22, 163, 74),
            pit_bg: Color32::from_rgb(223, 215, 199),
            pit_border: Color32::from_rgb(100, 116, 139),
            pit_text: Color32::from_rgb(15, 23, 42),
            signal_housing: Color32::from_rgb(30, 41, 59),
            signal_border: Color32::from_rgb(100, 116, 139),
            signal_mast: Color32::from_rgb(71, 85, 105),
            turnout_straight_fill: Color32::from_rgb(3, 105, 161),
            turnout_diverge_fill: Color32::from_rgb(180, 83, 9),
            turnout_ring: Color32::from_rgb(250, 246, 238),
            turnout_label: Color32::from_rgb(51, 65, 85),
            nameplate_bg: Color32::from_rgba_unmultiplied(250, 246, 238, 244),
            legend_bg: Color32::from_rgba_unmultiplied(245, 240, 230, 244),
            legend_text: p.text_main,
            legend_hint: p.text_sub,
            train_tag_bg: p.bg_card,
            train_tag_text_ok: Color32::from_rgb(21, 128, 61),
            train_tag_text_halt: Color32::from_rgb(180, 83, 9),
            train_selected: Color32::from_rgb(2, 132, 199),
            train_idle: Color32::from_rgb(220, 38, 38),
        }
    }
}

pub(crate) fn adapt_layer_color(col: Color32, dark_mode: bool) -> Color32 {
    if dark_mode {
        return col;
    }
    let lum = (0.2126 * col.r() as f32 + 0.7152 * col.g() as f32 + 0.0722 * col.b() as f32) / 255.0;
    if lum <= 0.36 {
        return col;
    }
    let scale = (0.36 / lum).clamp(0.32, 0.72);
    Color32::from_rgb(
        (col.r() as f32 * scale).round() as u8,
        (col.g() as f32 * scale).round() as u8,
        (col.b() as f32 * scale).round() as u8,
    )
}

pub(super) fn draw_legend_strip(painter: &egui::Painter, leg_rect: Rect, is_de: bool, th: &StellwerkTheme) {
    painter.rect_filled(leg_rect, 4.0, th.legend_bg);
    if !th.dark_mode {
        painter.rect_stroke(leg_rect, 4.0, Stroke::new(1.0, th.header_border), egui::StrokeKind::Inside);
    }
    let mut x = leg_rect.min.x + 10.0;
    let cy = leg_rect.center().y;
    for (dot_col, lbl) in [
        (th.route_active, if is_de { "Fahrweg" } else { "Route" }),
        (th.occupied, if is_de { "Besetzt" } else { "Occupied" }),
        (th.clear_dot, if is_de { "Frei" } else { "Clear" }),
    ] {
        painter.circle_filled(Pos2::new(x + 3.5, cy), 3.5, dot_col);
        let text_w = lbl.len() as f32 * 6.0;
        painter.text(
            Pos2::new(x + 10.0, cy),
            egui::Align2::LEFT_CENTER,
            lbl,
            FontId::proportional(10.5),
            th.legend_text,
        );
        x += 18.0 + text_w;
    }
    let hint = if is_de {
        "|  Klick: Weiche stellen / Zug waehlen  ·  Rechts-Ziehen: Verschieben  ·  Rad: Zoom"
    } else {
        "|  Click: Switch Turnout / Select Train  ·  Right-Drag: Pan  ·  Scroll: Zoom"
    };
    painter.text(
        Pos2::new(x + 2.0, cy),
        egui::Align2::LEFT_CENTER,
        hint,
        FontId::proportional(10.5),
        th.legend_hint,
    );
}

pub(super) fn draw_pane_legend(painter: &egui::Painter, rect: Rect, is_de: bool, th: &StellwerkTheme) {
    let leg_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 10.0, rect.max.y - 26.0),
        egui::vec2((rect.width() - 20.0).min(700.0), 20.0),
    );
    draw_legend_strip(painter, leg_rect, is_de, th);
}

pub(super) fn draw_domino_grid(painter: &egui::Painter, rect: Rect, border_col: Color32, th: &StellwerkTheme) {
    painter.rect_filled(rect, 0.0, th.grid_bg);
    let tile_sz = 28.0_f32;
    let mut gx = rect.min.x;
    while gx <= rect.max.x {
        painter.line_segment([Pos2::new(gx, rect.min.y), Pos2::new(gx, rect.max.y)], Stroke::new(1.0, th.grid_line));
        gx += tile_sz;
    }
    let mut gy = rect.min.y;
    while gy <= rect.max.y {
        painter.line_segment([Pos2::new(rect.min.x, gy), Pos2::new(rect.max.x, gy)], Stroke::new(1.0, th.grid_line));
        gy += tile_sz;
    }
    painter.rect_stroke(rect, 0.0, Stroke::new(1.5, border_col), egui::StrokeKind::Inside);
}

pub(super) fn draw_layer_nameplate(
    painter: &egui::Painter,
    rect: Rect,
    layer: &SignalBoxLayer,
    is_de: bool,
    expandable: bool,
    th: &StellwerkTheme,
) -> Rect {
    let accent = adapt_layer_color(layer.color, th.dark_mode);
    let track_unit = if is_de { "Gl." } else { "tr." };
    let mut label = format!(
        "{}  ·  {:.0} mm  ·  {} {}",
        layer.name, layer.height_mm, layer.rail_count, track_unit
    );
    if layer.train_count > 0 {
        let train_unit = if is_de { "Zug" } else { "tr" };
        label.push_str(&format!("  ·  {} {}", layer.train_count, train_unit));
    }
    if expandable {
        label.push_str("  [+]");
    }
    let width = (label.len() as f32 * 6.3 + 32.0).min((rect.width() - 16.0).max(90.0));
    let pill_rect = Rect::from_min_size(Pos2::new(rect.min.x + 8.0, rect.min.y + 8.0), egui::vec2(width, 22.0));
    painter.rect_filled(pill_rect, 5.0, th.nameplate_bg);
    painter.rect_stroke(pill_rect, 5.0, Stroke::new(1.2, accent), egui::StrokeKind::Inside);
    painter.circle_filled(Pos2::new(pill_rect.min.x + 11.0, pill_rect.center().y), 3.6, accent);
    painter.text(
        Pos2::new(pill_rect.min.x + 19.0, pill_rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        accent,
    );
    pill_rect
}
