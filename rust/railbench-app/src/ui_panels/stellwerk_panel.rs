use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke};
use railbench_core::geometry::rotate;
use railbench_core::types::Vec2;

use crate::catalog::{find_entry, is_road_id};
use crate::export::sample_track_branches_world;
use crate::i18n::Lang;
use crate::vehicle_sim::VehicleDomain;
use crate::RailbenchApp;


use super::stellwerk_theme::*;

/// Interactive DB SpDrS60-style **Stellwerk** (Signal Box Mimic Panel) with separate per-layer views.
pub(super) fn draw_stellwerk_canvas(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let th = stellwerk_theme(app.dark_mode);
    let piece_layers = build_piece_layer_map(app);
    let layers = collect_signal_box_layers(app, &piece_layers);

    if let Some(ref sel) = app.stellwerk_selected_layer {
        if !layers.iter().any(|l| l.id == *sel) {
            app.stellwerk_selected_layer = None;
        }
    }

    let turnout_count = app
        .layout
        .pieces
        .iter()
        .filter(|pc| {
            find_entry(&app.catalogs, &pc.catalog_id)
                .map_or(false, |e| crate::is_turnout_kind(&e.kind) && pc.connectors.len() >= 3)
        })
        .count();
    let train_count = app
        .vehicle_sim
        .vehicles
        .iter()
        .filter(|v| v.domain == VehicleDomain::Rail)
        .count();

    // Top Layer View Selector bar styled cleanly for Dark or Light Mode
    egui::Frame::new()
        .fill(th.header_bg)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .stroke(Stroke::new(1.0_f32, th.header_border))
        .show(ui, |ui| {
            if th.dark_mode {
                crate::theme::style_toolbar_ui(ui, crate::theme::palette(true));
            }
            ui.set_width(ui.available_width());
            ui.spacing_mut().interact_size.y = 22.0;
            ui.spacing_mut().button_padding = egui::vec2(8.0, 2.0);
            ui.horizontal_wrapped(|ui| {
                if layers.len() >= 2 {
                    let split_lbl = if is_de { "Getrennte Ebenen" } else { "Split Layers" };
                    let active = app.stellwerk_split_layers;
                    let fill = if active { th.active_fill } else { th.btn_bg };
                    let border = if active { th.active_border } else { th.btn_border };
                    let txt_col = if active { Color32::WHITE } else { th.header_text };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(split_lbl).strong().size(11.5).color(txt_col))
                                .fill(fill)
                                .stroke(Stroke::new(1.0_f32, border))
                                .min_size(egui::vec2(0.0, 22.0))
                                .corner_radius(egui::CornerRadius::same(5)),
                        )
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
                    let text = if layer.train_count > 0 {
                        format!("{} ({:.0} mm) [{}]", layer.name, layer.height_mm, layer.train_count)
                    } else {
                        format!("{} ({:.0} mm)", layer.name, layer.height_mm)
                    };
                    let adapted = adapt_layer_color(layer.color, th.dark_mode);
                    let fill = if active { th.active_fill } else { th.btn_bg };
                    let border = if active { th.active_border } else { th.btn_border };
                    let txt_col = if active { Color32::WHITE } else { adapted };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(text).strong().size(11.5).color(txt_col))
                                .fill(fill)
                                .stroke(Stroke::new(1.0_f32, border))
                                .min_size(egui::vec2(0.0, 22.0))
                                .corner_radius(egui::CornerRadius::same(5)),
                        )
                        .clicked()
                    {
                        app.stellwerk_split_layers = false;
                        app.stellwerk_selected_layer = Some(layer.id.clone());
                    }
                }

                if layers.len() >= 2 {
                    ui.separator();
                    let all_active = !app.stellwerk_split_layers && app.stellwerk_selected_layer.is_none();
                    let all_lbl = if is_de { "Alle" } else { "All" };
                    let fill = if all_active { th.active_fill } else { th.btn_bg };
                    let border = if all_active { th.active_border } else { th.btn_border };
                    let txt_col = if all_active { Color32::WHITE } else { th.header_text };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(all_lbl).size(11.5).color(txt_col))
                                .fill(fill)
                                .stroke(Stroke::new(1.0_f32, border))
                                .min_size(egui::vec2(0.0, 22.0))
                                .corner_radius(egui::CornerRadius::same(5)),
                        )
                        .clicked()
                    {
                        app.stellwerk_split_layers = false;
                        app.stellwerk_selected_layer = None;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !app.vehicle_sim.turnout_states.is_empty()
                        && ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if is_de { "Weichen gerade" } else { "Reset Turnouts" })
                                        .size(11.0)
                                        .color(th.header_text),
                                )
                                .fill(th.btn_bg)
                                .stroke(Stroke::new(1.0_f32, th.btn_border))
                                .min_size(egui::vec2(0.0, 22.0))
                                .corner_radius(egui::CornerRadius::same(5)),
                            )
                            .clicked()
                    {
                        app.vehicle_sim.turnout_states.clear();
                    }
                    let summary = if is_de {
                        format!("Weichen: {turnout_count}  ·  Zuege: {train_count}")
                    } else {
                        format!("Turnouts: {turnout_count}  ·  Trains: {train_count}")
                    };
                    ui.label(
                        RichText::new(summary)
                            .size(11.0)
                            .color(th.header_muted),
                    );
                });
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
                            &th,
                        ) {
                            focus_layer = Some(lid);
                        }
                    }
                });
            }
            let (leg_rect, _) = ui.allocate_exact_size(egui::vec2(grid_w, legend_h), Sense::hover());
            let leg_painter = ui.painter_at(leg_rect);
            leg_painter.rect_filled(leg_rect, 0.0, th.grid_bg);
            draw_legend_strip(&leg_painter, leg_rect, is_de, &th);
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
        draw_stellwerk_pane(ui, app, avail, layer_filter, active_layer_ref, &piece_layers, true, &th);
    }
}

struct StellwerkHits {
    toggle_turnout: Option<(String, usize)>,
    step_tt: Option<(String, String, isize)>,
    step_tr: Option<(String, String, isize)>,
    trigger_uncoupler: Option<String>,
    toggle_uncoupler_armed: Option<String>,
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
    th: &StellwerkTheme,
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
            painter.circle_filled(cp, r_scr, th.pit_bg);
            painter.circle_stroke(
                cp,
                r_scr,
                Stroke::new(2.0_f32, if is_occ { th.occupied } else { th.pit_border }),
            );
            let ang = app.vehicle_sim.turntable_angles_deg.get(&piece.id).copied().unwrap_or(0.0);
            let rad = (piece.transform.rotation + ang).to_radians();
            let (ca, sa) = (rad.cos(), rad.sin());
            let b0 = to_scr(piece.transform.x - pit_r * ca, piece.transform.y - pit_r * sa);
            let b1 = to_scr(piece.transform.x + pit_r * ca, piece.transform.y + pit_r * sa);
            let b_col = if is_occ { th.occupied } else { th.route_active };
            painter.line_segment([b0, b1], Stroke::new(5.0_f32, b_col));
            let deg_txt = format!("{:.0}°", ang);
            let badge_rect = Rect::from_center_size(cp, egui::vec2(28.0, 14.0));
            painter.rect_filled(badge_rect, 3.0, th.nameplate_bg);
            painter.text(cp, egui::Align2::CENTER_CENTER, deg_txt, FontId::monospace(9.5), th.pit_text);
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
            painter.add(egui::Shape::convex_polygon(corners, th.pit_bg, Stroke::new(1.8_f32, th.pit_border)));
            let bz = app.vehicle_sim.transfer_offsets_mm.get(&piece.id).copied().unwrap_or(zs[zs.len() / 2] as f64);
            let r0 = rotate(Vec2 { x: 0.0, y: bz }, piece.transform.rotation);
            let r1 = rotate(Vec2 { x: l, y: bz }, piece.transform.rotation);
            let b0 = to_scr(piece.transform.x + r0.x, piece.transform.y + r0.y);
            let b1 = to_scr(piece.transform.x + r1.x, piece.transform.y + r1.y);
            let b_col = if is_occ { th.occupied } else { th.route_active };
            painter.line_segment([b0, b1], Stroke::new(5.0_f32, b_col));
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
                    (th.occupied, 6.2_f32)
                } else if is_sw && is_active_branch {
                    (th.route_active, 5.0_f32)
                } else if is_sw {
                    (th.turnout_inactive, 2.8_f32)
                } else {
                    (th.track_idle, 4.0_f32)
                };
                if is_occ && is_active_branch {
                    painter.add(egui::Shape::line(spts.clone(), Stroke::new(11.5_f32, th.occupied_glow)));
                }
                painter.add(egui::Shape::line(spts.clone(), Stroke::new(width + 2.6, th.track_casing)));
                painter.add(egui::Shape::line(spts, Stroke::new(width, col)));
            }
        }

        if piece.has_signal == Some(true) {
            if let Some(c0) = piece.connectors.first() {
                let sp = to_scr(c0.x, c0.y);
                let head_pos = Pos2::new(sp.x, sp.y - 11.0);
                let head_rect = Rect::from_center_size(head_pos, egui::vec2(9.0, 12.0));
                let sig_col = if is_occ { th.occupied } else { th.clear_dot };
                painter.line_segment([Pos2::new(sp.x, sp.y - 3.0), head_pos], Stroke::new(1.5_f32, th.signal_mast));
                painter.rect_filled(head_rect, 3.0, th.signal_housing);
                painter.rect_stroke(head_rect, 3.0, Stroke::new(1.0_f32, th.signal_border), egui::StrokeKind::Inside);
                painter.circle_filled(head_pos, 4.6, Color32::from_rgba_unmultiplied(sig_col.r(), sig_col.g(), sig_col.b(), 60));
                painter.circle_filled(head_pos, 2.7, sig_col);
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
        } else if entry.is_some_and(crate::catalog::is_uncoupler_entry) {
            if let Some(br0) = branches.first().filter(|b| b.len() >= 2) {
                let mid = &br0[br0.len() / 2];
                let sp = to_scr(mid.x, mid.y);
                let raised = app.vehicle_sim.is_uncoupler_raised(&piece.id);
                let armed = app.vehicle_sim.armed_uncouplers.contains(&piece.id);
                let badge = Rect::from_center_size(sp, egui::vec2(16.0, 12.0));
                let fill = if raised {
                    Color32::from_rgb(217, 119, 6)
                } else if armed {
                    Color32::from_rgb(124, 58, 237)
                } else {
                    Color32::from_rgb(30, 64, 175)
                };
                painter.rect_filled(badge, 3.0, fill);
                painter.rect_stroke(
                    badge,
                    3.0,
                    Stroke::new(1.2_f32, if raised { th.route_active } else { Color32::WHITE }),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    sp,
                    egui::Align2::CENTER_CENTER,
                    "U",
                    FontId::monospace(8.5),
                    Color32::WHITE,
                );
                if active_click.is_some_and(|cp| badge.expand(5.0).contains(cp)) {
                    hits.trigger_uncoupler = Some(piece.id.clone());
                } else if rclick_pos.is_some_and(|cp| badge.expand(5.0).contains(cp)) {
                    hits.toggle_uncoupler_armed = Some(piece.id.clone());
                }
            }
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
    th: &StellwerkTheme,
    hits: &mut StellwerkHits,
) {
    for (tid, cid, br_cnt, st, pos, is_dkw) in nodes {
        let hovered = hover_pos.is_some_and(|hp| hp.distance(*pos) <= 13.5);
        let r = if hovered { 12.5_f32 } else { 10.5_f32 };
        let st_norm = *st % (*br_cnt).max(2);
        let fill = if st_norm == 0 { th.turnout_straight_fill } else { th.turnout_diverge_fill };
        let border = if hovered {
            if th.dark_mode { Color32::WHITE } else { th.header_text }
        } else {
            th.turnout_ring
        };
        painter.circle_filled(*pos, r, fill);
        painter.circle_stroke(*pos, r, Stroke::new(if hovered { 2.2 } else { 1.4_f32 }, border));
        let vec_s = Stroke::new(1.9_f32, Color32::WHITE);
        let d = r * 0.44;
        if *is_dkw {
            if st_norm == 0 {
                painter.line_segment([Pos2::new(pos.x - d, pos.y - d), Pos2::new(pos.x + d, pos.y + d)], vec_s);
                painter.line_segment([Pos2::new(pos.x + d, pos.y - d), Pos2::new(pos.x - d, pos.y + d)], vec_s);
            } else {
                painter.line_segment([Pos2::new(pos.x - d * 0.55, pos.y - d), Pos2::new(pos.x - d * 0.55, pos.y + d)], vec_s);
                painter.line_segment([Pos2::new(pos.x + d * 0.55, pos.y - d), Pos2::new(pos.x + d * 0.55, pos.y + d)], vec_s);
            }
        } else if st_norm == 0 {
            painter.line_segment([Pos2::new(pos.x, pos.y + d), Pos2::new(pos.x, pos.y - d)], vec_s);
        } else if *br_cnt >= 3 && st_norm == 1 {
            painter.line_segment([Pos2::new(pos.x, pos.y + d), Pos2::new(pos.x - d, pos.y - d)], vec_s);
        } else {
            painter.line_segment([Pos2::new(pos.x, pos.y + d), Pos2::new(pos.x + d, pos.y - d)], vec_s);
        }
        if scale >= 0.16 {
            painter.text(
                Pos2::new(pos.x, pos.y + r + 8.0),
                egui::Align2::CENTER_CENTER,
                cid,
                FontId::monospace(8.5),
                th.turnout_label,
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
    th: &StellwerkTheme,
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
            Stroke::new(6.0_f32, if is_selected { th.train_selected } else { th.train_idle }),
        );
        let znm_txt = if !v.kind.is_locomotive() {
            format!("{} · {}", v.name, if app.lang == Lang::De { "ABGESTELLT" } else { "PARKED" })
        } else if v.blocked_ahead {
            format!("{} · HALT", v.name)
        } else {
            format!("{} · {:.0} km/h", v.name, kmh)
        };
        let box_rect = Rect::from_center_size(
            Pos2::new(sp.x, sp.y - 20.0),
            egui::vec2((znm_txt.len() as f32 * 6.2 + 12.0).max(80.0), 17.0),
        );
        painter.rect_filled(box_rect, 3.0, th.train_tag_bg);
        let stroke_col = if !v.kind.is_locomotive() {
            th.header_muted
        } else if v.blocked_ahead {
            th.train_tag_text_halt
        } else if is_selected {
            th.train_selected
        } else {
            th.clear_dot
        };
        painter.rect_stroke(box_rect, 3.0, Stroke::new(if is_selected { 1.6 } else { 1.0_f32 }, stroke_col), egui::StrokeKind::Inside);
        painter.text(
            box_rect.center(),
            egui::Align2::CENTER_CENTER,
            znm_txt,
            FontId::monospace(9.5),
            if !v.kind.is_locomotive() {
                th.legend_text
            } else if v.blocked_ahead {
                th.train_tag_text_halt
            } else {
                th.train_tag_text_ok
            },
        );
        if active_click.is_some_and(|cp| box_rect.expand(4.0).contains(cp) || cp.distance(sp) <= 12.0) {
            hits.select_veh = Some(v.id.clone());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_stellwerk_pane(
    ui: &mut egui::Ui,
    app: &mut RailbenchApp,
    pane_size: egui::Vec2,
    layer_filter: Option<&str>,
    header: Option<&SignalBoxLayer>,
    piece_layers: &HashMap<String, String>,
    show_legend: bool,
    th: &StellwerkTheme,
) -> Option<String> {
    let is_de = app.lang == Lang::De;
    let pane_key = layer_filter.unwrap_or("__all__");
    let expandable = header.is_some() && !show_legend;

    ui.push_id(pane_key, |ui| {
        let (rect, resp) = ui.allocate_exact_size(pane_size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let border_col = header
            .map(|h| {
                let ac = adapt_layer_color(h.color, th.dark_mode);
                Color32::from_rgba_unmultiplied(ac.r(), ac.g(), ac.b(), if th.dark_mode { 145 } else { 185 })
            })
            .unwrap_or(th.pane_border);
        draw_domino_grid(&painter, rect, border_col, th);

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

        let pill_rect = header.map(|h| draw_layer_nameplate(&painter, rect, h, is_de, expandable, th));
        if let (Some(cp), Some(pr), Some(h)) = (click_pos, pill_rect, header) {
            if expandable && pr.contains(cp) {
                return Some(h.id.clone());
            }
        }

        let Some((min_x, min_y, max_x, max_y)) = rail_bounds(app, layer_filter) else {
            let msg = if is_de { "Keine Gleise auf dieser Ebene" } else { "No tracks on this layer" };
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, msg, FontId::proportional(13.0), th.header_muted);
            if show_legend {
                draw_pane_legend(&painter, rect, is_de, th);
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
        let mut hits = StellwerkHits {
            toggle_turnout: None,
            step_tt: None,
            step_tr: None,
            trigger_uncoupler: None,
            toggle_uncoupler_armed: None,
            select_veh: None,
        };

        let nodes = draw_stellwerk_tracks_and_specials(&painter, app, layer_filter, &occupied, &to_scr, active_click, rclick_pos, scale, th, &mut hits);
        draw_stellwerk_turnout_buttons(&painter, &nodes, hover_pos, active_click, scale, th, &mut hits);
        draw_stellwerk_trains(&painter, app, layer_filter, piece_layers, &to_scr, active_click, th, &mut hits);

        if let Some(h) = header {
            draw_layer_nameplate(&painter, rect, h, is_de, expandable, th);
        }
        if show_legend {
            draw_pane_legend(&painter, rect, is_de, th);
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
        if let Some(pid) = hits.trigger_uncoupler {
            app.vehicle_sim.trigger_uncoupler(&pid, &app.layout, &app.catalogs);
        }
        if let Some(pid) = hits.toggle_uncoupler_armed {
            app.vehicle_sim.toggle_uncoupler_armed(&pid);
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

    #[test]
    fn stellwerk_theme_light_and_dark_have_distinct_high_contrast_palettes() {
        let dark = stellwerk_theme(true);
        let light = stellwerk_theme(false);

        assert!(dark.grid_bg.r() < 30 && dark.header_text.r() > 220);
        assert!(light.grid_bg.r() > 210 && light.header_text.r() < 60);
        assert_ne!(dark.grid_bg, light.grid_bg);

        let bright_cyan = Color32::from_rgb(56, 189, 248);
        assert_eq!(adapt_layer_color(bright_cyan, true), bright_cyan);
        let light_cyan = adapt_layer_color(bright_cyan, false);
        assert!(
            light_cyan.r() < bright_cyan.r()
                && light_cyan.g() < bright_cyan.g()
                && light_cyan.b() < bright_cyan.b(),
            "light mode should darken bright pastel layer colors for contrast"
        );
    }
}
