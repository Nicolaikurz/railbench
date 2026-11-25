use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke};
use railbench_core::geometry::rotate;
use railbench_core::types::{TrackPiece, Vec2};

use crate::canvas2d::parse_hex_color;
use crate::catalog::{find_entry, is_road_id};
use crate::export::sample_track_branches_world;
use crate::i18n::Lang;
use crate::vehicle_sim::VehicleDomain;
use crate::RailbenchApp;

const LAYER_FALLBACK_COLORS: [Color32; 5] = [
    Color32::from_rgb(56, 189, 248),
    Color32::from_rgb(52, 211, 153),
    Color32::from_rgb(251, 191, 36),
    Color32::from_rgb(192, 132, 252),
    Color32::from_rgb(248, 113, 113),
];

#[derive(Debug, Clone)]
struct SignalBoxLayer {
    id: String,
    name: String,
    color: Color32,
    height_mm: f64,
    rail_count: usize,
    train_count: usize,
}

fn layer_color(hex: Option<&str>, fallback_idx: usize) -> Color32 {
    parse_hex_color(hex)
        .unwrap_or_else(|| LAYER_FALLBACK_COLORS[fallback_idx % LAYER_FALLBACK_COLORS.len()])
}

fn piece_layer_id(piece: &TrackPiece) -> &str {
    piece.layer_id.as_deref().unwrap_or("layer-1")
}

fn build_piece_layer_map(app: &RailbenchApp) -> HashMap<String, String> {
    app.layout
        .pieces
        .iter()
        .map(|p| (p.id.clone(), piece_layer_id(p).to_string()))
        .collect()
}

/// Collects Signal Box layers sorted by elevation ascending (`height_mm`),
/// counting rail pieces and active rail vehicles on each layer.
fn collect_signal_box_layers(
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
fn rail_bounds(app: &RailbenchApp, layer_filter: Option<&str>) -> Option<(f64, f64, f64, f64)> {
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

fn draw_legend_strip(painter: &egui::Painter, leg_rect: Rect, is_de: bool) {
    let legend = if is_de {
        "Stellwerk: Klick auf Weiche = Stellen  |  Gelb = Fahrweg  |  Rot = Besetzt  |  Rechtsklick+Ziehen = Verschieben"
    } else {
        "Signal Box: Click Turnout = Switch  |  Yellow = Active Route  |  Red = Occupied  |  Right-Drag = Pan"
    };
    painter.rect_filled(leg_rect, 4.0, Color32::from_rgba_unmultiplied(15, 23, 42, 210));
    painter.text(
        Pos2::new(leg_rect.min.x + 8.0, leg_rect.center().y),
        egui::Align2::LEFT_CENTER,
        legend,
        FontId::proportional(10.5),
        Color32::from_rgb(203, 213, 225),
    );
}

fn draw_pane_legend(painter: &egui::Painter, rect: Rect, is_de: bool) {
    let leg_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 10.0, rect.max.y - 26.0),
        egui::vec2((rect.width() - 20.0).min(640.0), 20.0),
    );
    draw_legend_strip(painter, leg_rect, is_de);
}

fn draw_domino_grid(painter: &egui::Painter, rect: Rect, border_col: Color32) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(14, 21, 29));
    let tile_sz = 28.0_f32;
    let grid_col = Color32::from_rgba_unmultiplied(51, 65, 85, 38);
    let mut gx = rect.min.x;
    while gx <= rect.max.x {
        painter.line_segment([Pos2::new(gx, rect.min.y), Pos2::new(gx, rect.max.y)], Stroke::new(1.0, grid_col));
        gx += tile_sz;
    }
    let mut gy = rect.min.y;
    while gy <= rect.max.y {
        painter.line_segment([Pos2::new(rect.min.x, gy), Pos2::new(rect.max.x, gy)], Stroke::new(1.0, grid_col));
        gy += tile_sz;
    }
    painter.rect_stroke(rect, 0.0, Stroke::new(1.5, border_col), egui::StrokeKind::Inside);
}

fn draw_layer_nameplate(
    painter: &egui::Painter,
    rect: Rect,
    layer: &SignalBoxLayer,
    is_de: bool,
    expandable: bool,
) -> Rect {
    let track_unit = if is_de { "Gl." } else { "tr." };
    let mut label = format!(
        "● {}  ·  {:.0} mm  ·  {} {}",
        layer.name, layer.height_mm, layer.rail_count, track_unit
    );
    if layer.train_count > 0 {
        label.push_str(&format!("  ·  🚂 {}", layer.train_count));
    }
    if expandable {
        label.push_str("   ⤢");
    }
    let width = (label.len() as f32 * 6.3 + 22.0).min((rect.width() - 16.0).max(80.0));
    let pill_rect = Rect::from_min_size(Pos2::new(rect.min.x + 8.0, rect.min.y + 8.0), egui::vec2(width, 22.0));
    painter.rect_filled(pill_rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 232));
    painter.rect_stroke(pill_rect, 5.0, Stroke::new(1.2, layer.color), egui::StrokeKind::Inside);
    painter.text(
        Pos2::new(pill_rect.min.x + 9.0, pill_rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        layer.color,
    );
    pill_rect
}

/// Interactive DB SpDrS60-style **Stellwerk** (Signal Box Mimic Panel) with separate per-layer views.
pub(super) fn draw_stellwerk_canvas(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let piece_layers = build_piece_layer_map(app);
    let layers = collect_signal_box_layers(app, &piece_layers);

    if let Some(ref sel) = app.stellwerk_selected_layer {
        if !layers.iter().any(|l| l.id == *sel) {
            app.stellwerk_selected_layer = None;
        }
    }

    // Top Layer View Selector bar
    egui::Frame::new()
        .fill(Color32::from_rgb(18, 26, 36))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 58, 74)))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                if layers.len() >= 2 {
                    let split_lbl = if is_de { "◫ Getrennte Ebenen" } else { "◫ Split Layers" };
                    if ui
                        .selectable_label(app.stellwerk_split_layers, RichText::new(split_lbl).strong().size(11.5))
                        .clicked()
                    {
                        app.stellwerk_split_layers = true;
                    }
                    ui.separator();
                }

                for layer in &layers {
                    let active = (!app.stellwerk_split_layers
                        && app.stellwerk_selected_layer.as_deref() == Some(layer.id.as_str()))
                        || layers.len() == 1;
                    let mut text = format!("● {} ({:.0} mm)", layer.name, layer.height_mm);
                    if layer.train_count > 0 {
                        text.push_str(&format!(" · 🚂 {}", layer.train_count));
                    }
                    let rich = RichText::new(text).size(11.5).color(if active { Color32::WHITE } else { layer.color });
                    if ui.selectable_label(active, rich).clicked() {
                        app.stellwerk_split_layers = false;
                        app.stellwerk_selected_layer = Some(layer.id.clone());
                    }
                }

                if layers.len() >= 2 {
                    ui.separator();
                    let all_active = !app.stellwerk_split_layers && app.stellwerk_selected_layer.is_none();
                    let all_lbl = if is_de { "Alle" } else { "All" };
                    if ui.selectable_label(all_active, RichText::new(all_lbl).size(11.5)).clicked() {
                        app.stellwerk_split_layers = false;
                        app.stellwerk_selected_layer = None;
                    }
                }
            });
        });

    let avail = ui.available_size();
    if app.stellwerk_split_layers && layers.len() >= 2 {
        let gap = 4.0_f32;
        let legend_h = 22.0_f32;
        let cols = if layers.len() <= 3 { layers.len() } else { 2 };
        let rows = layers.len().div_ceil(cols);
        let grid_w = avail.x.max(80.0);
        let grid_h = (avail.y - legend_h - gap).max(80.0);
        let cell_w = ((grid_w - gap * (cols.saturating_sub(1) as f32)) / cols as f32).max(40.0);
        let cell_h = ((grid_h - gap * (rows.saturating_sub(1) as f32)) / rows as f32).max(40.0);

        let mut focus_layer: Option<String> = None;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
            for row_layers in layers.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                    for layer in row_layers {
                        if let Some(lid) = draw_stellwerk_pane(
                            ui,
                            app,
                            egui::vec2(cell_w, cell_h),
                            Some(&layer.id),
                            Some(layer),
                            &piece_layers,
                            false,
                        ) {
                            focus_layer = Some(lid);
                        }
                    }
                });
            }
            let (leg_rect, _) = ui.allocate_exact_size(egui::vec2(grid_w, legend_h), Sense::hover());
            let leg_painter = ui.painter_at(leg_rect);
            leg_painter.rect_filled(leg_rect, 0.0, Color32::from_rgb(14, 21, 29));
            draw_legend_strip(&leg_painter, leg_rect, is_de);
        });

        if let Some(lid) = focus_layer {
            app.stellwerk_split_layers = false;
            app.stellwerk_selected_layer = Some(lid);
        }
    } else {
        let selected_layer = app.stellwerk_selected_layer.clone();
        let layer_filter = if layers.len() == 1 {
            layers.first().map(|l| l.id.as_str())
        } else {
            selected_layer.as_deref()
        };
        let active_layer_ref = layer_filter.and_then(|id| layers.iter().find(|l| l.id == id));
        draw_stellwerk_pane(ui, app, avail, layer_filter, active_layer_ref, &piece_layers, true);
    }
}

struct StellwerkHits {
    toggle_turnout: Option<(String, usize)>,
    step_tt: Option<(String, String, isize)>,
    step_tr: Option<(String, String, isize)>,
    select_veh: Option<String>,
}

type TurnoutNode = (String, String, usize, usize, Pos2, bool);

#[allow(clippy::too_many_arguments)]
fn draw_stellwerk_tracks_and_specials(
    painter: &egui::Painter,
    app: &RailbenchApp,
    layer_filter: Option<&str>,
    occupied: &HashSet<String>,
    to_scr: &impl Fn(f64, f64) -> Pos2,
    active_click: Option<Pos2>,
    rclick_pos: Option<Pos2>,
    scale: f32,
    hits: &mut StellwerkHits,
) -> Vec<TurnoutNode> {
    let mut turnout_nodes = Vec::new();
    for piece in &app.layout.pieces {
        if is_road_id(&app.catalogs, &piece.catalog_id) {
            continue;
        }
        if layer_filter.is_some_and(|lid| piece_layer_id(piece) != lid) {
            continue;
        }
        let entry = find_entry(&app.catalogs, &piece.catalog_id);
        let kind = entry.map(|e| e.kind.as_str()).unwrap_or("straight");
        let is_occ = occupied.contains(&piece.id);

        if kind == "turntable" {
            let outer_r = crate::turntables_3d::turntable_outer_radius(piece, entry);
            let pit_r = 139.0 * (outer_r / 189.0).clamp(0.4, 2.5);
            let cp = to_scr(piece.transform.x, piece.transform.y);
            let r_scr = (pit_r as f32 * scale).max(12.0);
            painter.circle_filled(cp, r_scr, Color32::from_rgb(20, 30, 42));
            painter.circle_stroke(
                cp,
                r_scr,
                Stroke::new(2.0, if is_occ { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgb(100, 116, 139) }),
            );
            let ang = app.vehicle_sim.turntable_angles_deg.get(&piece.id).copied().unwrap_or(0.0);
            let rad = (piece.transform.rotation + ang).to_radians();
            let (ca, sa) = (rad.cos(), rad.sin());
            let b0 = to_scr(piece.transform.x - pit_r * ca, piece.transform.y - pit_r * sa);
            let b1 = to_scr(piece.transform.x + pit_r * ca, piece.transform.y + pit_r * sa);
            let b_col = if is_occ { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgb(250, 204, 21) };
            painter.line_segment([b0, b1], Stroke::new(5.0, b_col));
            painter.text(cp, egui::Align2::CENTER_CENTER, format!("⟳ {:.0}°", ang), FontId::monospace(9.5), Color32::WHITE);
            if active_click.is_some_and(|p| p.distance(cp) <= r_scr + 6.0) {
                hits.step_tt = Some((piece.id.clone(), piece.catalog_id.clone(), 1));
            } else if rclick_pos.is_some_and(|p| p.distance(cp) <= r_scr + 6.0) {
                hits.step_tt = Some((piece.id.clone(), piece.catalog_id.clone(), -1));
            }
            continue;
        }

        if kind == "transfer-table" {
            let l = entry.and_then(|e| e.length).unwrap_or(360.0);
            let zs = crate::turntables_3d::extract_transfer_track_zs(entry);
            let min_z = zs[0] as f64 - 28.0;
            let max_z = *zs.last().unwrap_or(&240.0) as f64 + 28.0;
            let corners: Vec<Pos2> = [(0.0, min_z), (l, min_z), (l, max_z), (0.0, max_z)]
                .iter()
                .map(|&(lx, ly)| {
                    let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
                    to_scr(piece.transform.x + rp.x, piece.transform.y + rp.y)
                })
                .collect();
            painter.add(egui::Shape::convex_polygon(corners, Color32::from_rgb(20, 30, 42), Stroke::new(1.8, Color32::from_rgb(100, 116, 139))));
            let bz = app.vehicle_sim.transfer_offsets_mm.get(&piece.id).copied().unwrap_or(zs[zs.len() / 2] as f64);
            let r0 = rotate(Vec2 { x: 0.0, y: bz }, piece.transform.rotation);
            let r1 = rotate(Vec2 { x: l, y: bz }, piece.transform.rotation);
            let b0 = to_scr(piece.transform.x + r0.x, piece.transform.y + r0.y);
            let b1 = to_scr(piece.transform.x + r1.x, piece.transform.y + r1.y);
            let b_col = if is_occ { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgb(250, 204, 21) };
            painter.line_segment([b0, b1], Stroke::new(5.0, b_col));
            let mid = Pos2::new((b0.x + b1.x) * 0.5, (b0.y + b1.y) * 0.5);
            if active_click.is_some_and(|p| p.distance(mid) <= 36.0) {
                hits.step_tr = Some((piece.id.clone(), piece.catalog_id.clone(), 1));
            } else if rclick_pos.is_some_and(|p| p.distance(mid) <= 36.0) {
                hits.step_tr = Some((piece.id.clone(), piece.catalog_id.clone(), -1));
            }
            continue;
        }

        let branches = sample_track_branches_world(piece, entry);
        let is_sw = crate::is_turnout_kind(kind) && piece.connectors.len() >= 3;
        let is_dkw = kind == "double-slip";
        let state_cnt = crate::turnout_state_count(kind, piece.connectors.len());
        let active_br = app.vehicle_sim.turnout_states.get(&piece.id).copied().unwrap_or(0);
        let is_branch_active = |bi: usize| -> bool {
            if !is_sw {
                true
            } else if is_dkw {
                if active_br % 2 == 0 {
                    bi == 0 || bi == 1
                } else {
                    bi == 2 || bi == 3
                }
            } else {
                bi == active_br.min(branches.len().saturating_sub(1))
            }
        };

        for draw_active_pass in [false, true] {
            for (bi, br) in branches.iter().enumerate() {
                if br.len() < 2 {
                    continue;
                }
                let is_active_branch = is_branch_active(bi);
                if is_active_branch != draw_active_pass {
                    continue;
                }
                let spts: Vec<Pos2> = br.iter().map(|pt| to_scr(pt.x, pt.y)).collect();
                let (col, width) = if is_occ && is_active_branch {
                    (Color32::from_rgb(239, 68, 68), 6.2_f32)
                } else if is_sw && is_active_branch {
                    (Color32::from_rgb(250, 204, 21), 5.0_f32)
                } else if is_sw {
                    (Color32::from_rgb(45, 58, 74), 2.8_f32)
                } else {
                    (Color32::from_rgb(100, 116, 139), 4.0_f32)
                };
                if is_occ && is_active_branch {
                    painter.add(egui::Shape::line(spts.clone(), Stroke::new(11.0, Color32::from_rgba_unmultiplied(239, 68, 68, 52))));
                }
                painter.add(egui::Shape::line(spts, Stroke::new(width, col)));
            }
        }

        if piece.has_signal == Some(true) {
            if let Some(c0) = piece.connectors.first() {
                let sp = to_scr(c0.x, c0.y);
                let sig_col = if is_occ { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgb(34, 197, 94) };
                painter.circle_filled(Pos2::new(sp.x, sp.y - 9.0), 4.2, sig_col);
                painter.circle_stroke(Pos2::new(sp.x, sp.y - 9.0), 4.8, Stroke::new(1.0, Color32::from_rgb(15, 23, 42)));
            }
        }

        if is_sw {
            let frog_pt = branches
                .first()
                .filter(|b| b.len() >= 2)
                .map(|b0| {
                    let mid_i = if is_dkw {
                        b0.len() / 2
                    } else {
                        (b0.len() / 3).min(b0.len() - 1)
                    };
                    to_scr(b0[mid_i].x, b0[mid_i].y)
                })
                .unwrap_or_else(|| to_scr(piece.transform.x, piece.transform.y));
            turnout_nodes.push((piece.id.clone(), piece.catalog_id.clone(), state_cnt, active_br % state_cnt, frog_pt, is_dkw));
        }
    }
    turnout_nodes
}

fn draw_stellwerk_turnout_buttons(
    painter: &egui::Painter,
    nodes: &[TurnoutNode],
    hover_pos: Option<Pos2>,
    active_click: Option<Pos2>,
    scale: f32,
    hits: &mut StellwerkHits,
) {
    for (tid, cid, br_cnt, st, pos, is_dkw) in nodes {
        let hovered = hover_pos.is_some_and(|hp| hp.distance(*pos) <= 13.5);
        let r = if hovered { 12.5_f32 } else { 10.5_f32 };
        let st_norm = *st % (*br_cnt).max(2);
        let fill = if st_norm == 0 { Color32::from_rgb(2, 132, 199) } else { Color32::from_rgb(217, 119, 6) };
        let border = if hovered { Color32::WHITE } else { Color32::from_rgb(250, 204, 21) };
        painter.circle_filled(*pos, r, fill);
        painter.circle_stroke(*pos, r, Stroke::new(if hovered { 2.2 } else { 1.4 }, border));
        let sym = if *is_dkw {
            if st_norm == 0 { "×" } else { ")(" }
        } else if st_norm == 0 {
            "↑"
        } else if *br_cnt >= 3 && st_norm == 1 {
            "↖"
        } else if *br_cnt >= 3 && st_norm == 2 {
            "↗"
        } else {
            "↱"
        };
        painter.text(*pos, egui::Align2::CENTER_CENTER, sym, FontId::proportional(11.5), Color32::WHITE);
        if scale >= 0.16 {
            painter.text(
                Pos2::new(pos.x, pos.y + r + 8.0),
                egui::Align2::CENTER_CENTER,
                cid,
                FontId::monospace(8.5),
                Color32::from_rgb(203, 213, 225),
            );
        }
        if active_click.is_some_and(|cp| cp.distance(*pos) <= 14.0) {
            hits.toggle_turnout = Some((tid.clone(), *br_cnt));
        }
    }
}

fn draw_stellwerk_trains(
    painter: &egui::Painter,
    app: &RailbenchApp,
    layer_filter: Option<&str>,
    piece_layers: &HashMap<String, String>,
    to_scr: &impl Fn(f64, f64) -> Pos2,
    active_click: Option<Pos2>,
    hits: &mut StellwerkHits,
) {
    for v in &app.vehicle_sim.vehicles {
        if v.domain != VehicleDomain::Rail {
            continue;
        }
        if let Some(lid) = layer_filter {
            let v_lid = piece_layers.get(&v.piece_id).map(String::as_str).unwrap_or("layer-1");
            if v_lid != lid {
                continue;
            }
        }
        let sp = to_scr(v.world_x, v.world_y);
        let is_selected = app.vehicle_sim.selected_vehicle_id.as_deref() == Some(v.id.as_str());
        let kmh = v.scale_speed_kmh(&app.layout.scale);
        let rad = v.heading_deg.to_radians();
        let dir_x = rad.cos() as f32 * 10.0;
        let dir_y = rad.sin() as f32 * 10.0;
        painter.line_segment(
            [Pos2::new(sp.x - dir_x, sp.y - dir_y), Pos2::new(sp.x + dir_x, sp.y + dir_y)],
            Stroke::new(6.0, if is_selected { Color32::from_rgb(56, 189, 248) } else { Color32::from_rgb(248, 113, 113) }),
        );
        let znm_txt = if v.blocked_ahead {
            format!("{} · ⚠ HALT", v.name)
        } else {
            format!("{} · {:.0} km/h", v.name, kmh)
        };
        let box_rect = Rect::from_center_size(
            Pos2::new(sp.x, sp.y - 20.0),
            egui::vec2((znm_txt.len() as f32 * 6.2 + 12.0).max(80.0), 17.0),
        );
        painter.rect_filled(box_rect, 3.0, Color32::from_rgb(9, 13, 20));
        let stroke_col = if v.blocked_ahead {
            Color32::from_rgb(245, 158, 11)
        } else if is_selected {
            Color32::from_rgb(56, 189, 248)
        } else {
            Color32::from_rgb(34, 197, 94)
        };
        painter.rect_stroke(box_rect, 3.0, Stroke::new(if is_selected { 1.6 } else { 1.0 }, stroke_col), egui::StrokeKind::Inside);
        painter.text(
            box_rect.center(),
            egui::Align2::CENTER_CENTER,
            znm_txt,
            FontId::monospace(9.5),
            if v.blocked_ahead { Color32::from_rgb(251, 191, 36) } else { Color32::from_rgb(74, 222, 128) },
        );
        if active_click.is_some_and(|cp| box_rect.expand(4.0).contains(cp) || cp.distance(sp) <= 12.0) {
            hits.select_veh = Some(v.id.clone());
        }
    }
}

fn draw_stellwerk_pane(
    ui: &mut egui::Ui,
    app: &mut RailbenchApp,
    pane_size: egui::Vec2,
    layer_filter: Option<&str>,
    header: Option<&SignalBoxLayer>,
    piece_layers: &HashMap<String, String>,
    show_legend: bool,
) -> Option<String> {
    let is_de = app.lang == Lang::De;
    let pane_key = layer_filter.unwrap_or("__all__");
    let expandable = header.is_some() && !show_legend;

    ui.push_id(pane_key, |ui| {
        let (rect, resp) = ui.allocate_exact_size(pane_size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let border_col = header
            .map(|h| Color32::from_rgba_unmultiplied(h.color.r(), h.color.g(), h.color.b(), 145))
            .unwrap_or_else(|| Color32::from_rgb(45, 58, 74));
        draw_domino_grid(&painter, rect, border_col);

        let view = app
            .stellwerk_layer_views
            .entry(pane_key.to_string())
            .or_insert((app.stellwerk_zoom, app.stellwerk_pan));
        if resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) {
            let d = resp.drag_delta();
            view.1[0] += d.x;
            view.1[1] += d.y;
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                view.0 = (view.0 * (1.0 + scroll * 0.0018).clamp(0.75, 1.35)).clamp(0.25, 6.0);
            }
        }
        let (pane_zoom, pane_pan) = *view;

        let click_pos = resp.clicked_by(egui::PointerButton::Primary).then(|| resp.interact_pointer_pos()).flatten();
        let rclick_pos = resp.clicked_by(egui::PointerButton::Secondary).then(|| resp.interact_pointer_pos()).flatten();
        let hover_pos = resp.hover_pos();

        let pill_rect = header.map(|h| draw_layer_nameplate(&painter, rect, h, is_de, expandable));
        if let (Some(cp), Some(pr), Some(h)) = (click_pos, pill_rect, header) {
            if expandable && pr.contains(cp) {
                return Some(h.id.clone());
            }
        }

        let Some((min_x, min_y, max_x, max_y)) = rail_bounds(app, layer_filter) else {
            let msg = if is_de { "Keine Gleise auf dieser Ebene" } else { "No tracks on this layer" };
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, msg, FontId::proportional(13.0), Color32::from_rgb(148, 163, 184));
            if show_legend {
                draw_pane_legend(&painter, rect, is_de);
            }
            return None;
        };

        let world_w = (max_x - min_x).max(100.0);
        let world_h = (max_y - min_y).max(100.0);
        let pad_top = if header.is_some() { 36.0_f32 } else { 24.0_f32 };
        let usable_w = (rect.width() - 48.0).max(40.0);
        let usable_h = (rect.height() - pad_top - if show_legend { 32.0 } else { 18.0 }).max(40.0);
        let scale = (usable_w / world_w as f32).min(usable_h / world_h as f32) * pane_zoom;
        let (cx_w, cy_w) = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        let (cx_s, cy_s) = (rect.center().x + pane_pan[0], rect.min.y + pad_top + usable_h * 0.5 + pane_pan[1]);
        let to_scr = |wx: f64, wy: f64| Pos2::new(cx_s + ((wx - cx_w) as f32) * scale, cy_s + ((wy - cy_w) as f32) * scale);

        let occupied: HashSet<String> = app
            .vehicle_sim
            .vehicles
            .iter()
            .filter(|v| v.domain == VehicleDomain::Rail)
            .map(|v| v.piece_id.clone())
            .collect();
        let active_click = click_pos.filter(|cp| !pill_rect.is_some_and(|pr| pr.contains(*cp)));
        let mut hits = StellwerkHits { toggle_turnout: None, step_tt: None, step_tr: None, select_veh: None };

        let nodes = draw_stellwerk_tracks_and_specials(&painter, app, layer_filter, &occupied, &to_scr, active_click, rclick_pos, scale, &mut hits);
        draw_stellwerk_turnout_buttons(&painter, &nodes, hover_pos, active_click, scale, &mut hits);
        draw_stellwerk_trains(&painter, app, layer_filter, piece_layers, &to_scr, active_click, &mut hits);

        if let Some(h) = header {
            draw_layer_nameplate(&painter, rect, h, is_de, expandable);
        }
        if show_legend {
            draw_pane_legend(&painter, rect, is_de);
        }

        if let Some((tid, cnt)) = hits.toggle_turnout {
            app.vehicle_sim.toggle_turnout(&tid, cnt);
        }
        if let Some((pid, cid, delta)) = hits.step_tt {
            let entry = find_entry(&app.catalogs, &cid).cloned();
            app.vehicle_sim.step_turntable(&pid, entry.as_ref(), delta);
        }
        if let Some((pid, cid, delta)) = hits.step_tr {
            let entry = find_entry(&app.catalogs, &cid).cloned();
            app.vehicle_sim.step_transfer_table(&pid, entry.as_ref(), delta);
        }
        if let Some(vid) = hits.select_veh {
            app.vehicle_sim.selected_vehicle_id = Some(vid);
        }
        None
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{create_piece, find_entry};
    use railbench_core::types::{Layer, Transform};

    fn make_layer(id: &str, name: &str, height_mm: f64, color: Option<&str>) -> Layer {
        Layer {
            id: id.to_string(),
            name: name.to_string(),
            color: color.map(str::to_string),
            visible: true,
            locked: false,
            height_mm: Some(height_mm),
            wood_plate_mode: None,
            wood_plate_thickness_mm: None,
            wood_plate_style: None,
            plates: None,
            terrain_field: None,
        }
    }

    #[test]
    fn collect_layers_sorts_by_height_and_separates_rail_from_road() {
        let mut app = RailbenchApp::default();
        app.layout.pieces.clear();
        app.layout.layers = Some(vec![
            make_layer("layer-top", "Main Station", 120.0, Some("#22c55e")),
            make_layer("layer-low", "Shadow Yard", 0.0, Some("#38bdf8")),
        ]);

        let rail_entry = app
            .catalogs
            .iter()
            .find(|c| c.is_road != Some(true) && !c.entries.is_empty())
            .and_then(|c| find_entry(&app.catalogs, &c.entries[0].id))
            .expect("rail catalog entry")
            .clone();
        let road_entry = app
            .catalogs
            .iter()
            .find(|c| c.is_road == Some(true) && !c.entries.is_empty())
            .and_then(|c| find_entry(&app.catalogs, &c.entries[0].id))
            .expect("road catalog entry")
            .clone();

        app.layout.pieces = vec![
            create_piece("p1".into(), &rail_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-low".into())),
            create_piece("p2".into(), &rail_entry, Transform { x: 1000.0, y: 500.0, rotation: 0.0 }, Some("layer-top".into())),
            create_piece("p3".into(), &road_entry, Transform { x: 500.0, y: 500.0, rotation: 0.0 }, Some("layer-low".into())),
            create_piece("p4".into(), &rail_entry, Transform { x: -300.0, y: -200.0, rotation: 0.0 }, Some("layer-extra".into())),
        ];

        let piece_layers = build_piece_layer_map(&app);
        let layers = collect_signal_box_layers(&app, &piece_layers);

        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].id, "layer-low");
        assert_eq!(layers[0].rail_count, 1);
        assert_eq!(layers[1].id, "layer-extra");
        assert_eq!(layers[1].height_mm, 0.0);
        assert_eq!(layers[2].id, "layer-top");
        assert_eq!(layers[2].color, Color32::from_rgb(34, 197, 94));
    }

    #[test]
    fn rail_bounds_filters_strictly_by_layer() {
        let mut app = RailbenchApp::default();
        app.layout.pieces.clear();
        let rail_entry = app
            .catalogs
            .iter()
            .find(|c| c.is_road != Some(true) && !c.entries.is_empty())
            .and_then(|c| find_entry(&app.catalogs, &c.entries[0].id))
            .expect("rail catalog entry")
            .clone();

        app.layout.pieces = vec![
            create_piece("p1".into(), &rail_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("l1".into())),
            create_piece("p2".into(), &rail_entry, Transform { x: 2000.0, y: 1500.0, rotation: 0.0 }, Some("l2".into())),
        ];

        let b1 = rail_bounds(&app, Some("l1")).expect("l1 bounds");
        let b2 = rail_bounds(&app, Some("l2")).expect("l2 bounds");
        assert!(b1.2 < 1000.0);
        assert!(b2.0 > 1000.0);
        assert!(rail_bounds(&app, Some("empty-layer")).is_none());
    }
}
