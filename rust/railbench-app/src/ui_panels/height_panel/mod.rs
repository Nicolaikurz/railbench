mod auto_tab;
mod level_tab;
mod slope_tab;

use eframe::egui::{
    self, Color32, CornerRadius, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use railbench_core::types::{CatalogEntry, Elevation};

use crate::i18n::{tr, Lang};
use crate::icons::{paint_chevron, ChevronDir};
use crate::ramp_tools::{
    apply_auto_ramp_for_chain, available_curve_radii_dedup, build_ramp_breakdown,
    calculate_auto_ramp, collect_open_connector_options, compute_auto_fit_end_height,
    generate_ramp_or_helix_ext, min_clearance_for_scale, preview_auto_ramp_for_chain,
    preview_generate_ramp_or_helix, resolve_ramp_chain, RampProfile, RampShape,
};
use crate::theme::{draw_window_header_bar, palette, RailbenchPalette};
use crate::RailbenchApp;

const PANEL_WIDTH: f32 = 356.0;
pub(super) const LAYER_NEW_SENTINEL: &str = "__new_layer__";
pub(super) const LAYER_ABOVE_SENTINEL: &str = "__create_above__";
pub(super) const LAYER_BELOW_SENTINEL: &str = "__create_below__";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusKind {
    Ok,
    Warn,
    Danger,
}

fn status_palette(dark: bool, kind: StatusKind) -> (Color32, Color32, Color32) {
    match (dark, kind) {
        (true, StatusKind::Ok) => (
            Color32::from_rgba_unmultiplied(16, 185, 129, 32),
            Color32::from_rgba_unmultiplied(16, 185, 129, 95),
            Color32::from_rgb(52, 211, 153),
        ),
        (false, StatusKind::Ok) => (
            Color32::from_rgba_unmultiplied(16, 185, 129, 28),
            Color32::from_rgba_unmultiplied(5, 150, 105, 100),
            Color32::from_rgb(4, 120, 87),
        ),
        (true, StatusKind::Warn) => (
            Color32::from_rgba_unmultiplied(245, 158, 11, 32),
            Color32::from_rgba_unmultiplied(245, 158, 11, 95),
            Color32::from_rgb(251, 191, 36),
        ),
        (false, StatusKind::Warn) => (
            Color32::from_rgba_unmultiplied(245, 158, 11, 28),
            Color32::from_rgba_unmultiplied(217, 119, 6, 105),
            Color32::from_rgb(180, 83, 9),
        ),
        (true, StatusKind::Danger) => (
            Color32::from_rgba_unmultiplied(239, 68, 68, 34),
            Color32::from_rgba_unmultiplied(239, 68, 68, 105),
            Color32::from_rgb(248, 113, 113),
        ),
        (false, StatusKind::Danger) => (
            Color32::from_rgba_unmultiplied(239, 68, 68, 26),
            Color32::from_rgba_unmultiplied(220, 38, 38, 110),
            Color32::from_rgb(185, 28, 28),
        ),
    }
}

fn paint_pin_icon(painter: &egui::Painter, center: Pos2, radius: f32, col: Color32) {
    let s = Stroke::new(1.35_f32, col);
    let head = Pos2::new(center.x, center.y - radius * 0.28);
    painter.circle_stroke(head, radius * 0.45, s);
    painter.line_segment(
        [
            Pos2::new(center.x - radius * 0.34, center.y - radius * 0.05),
            Pos2::new(center.x, center.y + radius * 0.85),
        ],
        s,
    );
    painter.line_segment(
        [
            Pos2::new(center.x + radius * 0.34, center.y - radius * 0.05),
            Pos2::new(center.x, center.y + radius * 0.85),
        ],
        s,
    );
}

fn paint_swap_arrows_icon(painter: &egui::Painter, center: Pos2, col: Color32) {
    let s = Stroke::new(1.4_f32, col);
    // Up arrow on left
    let lx = center.x - 2.6;
    painter.line_segment([Pos2::new(lx, center.y - 4.0), Pos2::new(lx, center.y + 4.0)], s);
    painter.line_segment([Pos2::new(lx - 2.2, center.y - 1.6), Pos2::new(lx, center.y - 4.0)], s);
    painter.line_segment([Pos2::new(lx + 2.2, center.y - 1.6), Pos2::new(lx, center.y - 4.0)], s);
    // Down arrow on right
    let rx = center.x + 2.6;
    painter.line_segment([Pos2::new(rx, center.y - 4.0), Pos2::new(rx, center.y + 4.0)], s);
    painter.line_segment([Pos2::new(rx - 2.2, center.y + 1.6), Pos2::new(rx, center.y + 4.0)], s);
    painter.line_segment([Pos2::new(rx + 2.2, center.y + 1.6), Pos2::new(rx, center.y + 4.0)], s);
}

fn paint_trend_arrow_icon(painter: &egui::Painter, center: Pos2, up: bool, col: Color32) {
    let s = Stroke::new(1.5_f32, col);
    if up {
        let a = Pos2::new(center.x - 4.0, center.y + 3.0);
        let b = Pos2::new(center.x + 4.0, center.y - 3.0);
        painter.line_segment([a, b], s);
        painter.line_segment([b, Pos2::new(b.x - 4.2, b.y)], s);
        painter.line_segment([b, Pos2::new(b.x, b.y + 4.0)], s);
    } else {
        let a = Pos2::new(center.x - 4.0, center.y - 3.0);
        let b = Pos2::new(center.x + 4.0, center.y + 3.0);
        painter.line_segment([a, b], s);
        painter.line_segment([b, Pos2::new(b.x - 4.2, b.y)], s);
        painter.line_segment([b, Pos2::new(b.x, b.y - 4.0)], s);
    }
}

fn paint_status_icon(painter: &egui::Painter, center: Pos2, kind: StatusKind, col: Color32) {
    let s = Stroke::new(1.6_f32, col);
    match kind {
        StatusKind::Ok => {
            painter.circle_stroke(center, 6.2, s);
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(center.x - 3.0, center.y + 0.3),
                    Pos2::new(center.x - 0.8, center.y + 2.6),
                    Pos2::new(center.x + 3.2, center.y - 2.4),
                ],
                s,
            ));
        }
        StatusKind::Warn => {
            painter.add(egui::Shape::closed_line(
                vec![
                    Pos2::new(center.x, center.y - 5.8),
                    Pos2::new(center.x + 6.0, center.y + 5.0),
                    Pos2::new(center.x - 6.0, center.y + 5.0),
                ],
                s,
            ));
            painter.line_segment(
                [Pos2::new(center.x, center.y - 2.0), Pos2::new(center.x, center.y + 1.5)],
                s,
            );
            painter.circle_filled(Pos2::new(center.x, center.y + 3.3), 0.9, col);
        }
        StatusKind::Danger => {
            painter.circle_stroke(center, 6.2, s);
            painter.line_segment(
                [Pos2::new(center.x - 2.5, center.y - 2.5), Pos2::new(center.x + 2.5, center.y + 2.5)],
                s,
            );
            painter.line_segment(
                [Pos2::new(center.x + 2.5, center.y - 2.5), Pos2::new(center.x - 2.5, center.y + 2.5)],
                s,
            );
        }
    }
}

pub(super) fn segmented_control<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    p: RailbenchPalette,
    dark_mode: bool,
    value: &mut T,
    items: &[(T, &str)],
    height: f32,
    text_size: f32,
) -> bool {
    let mut changed = false;
    let total_w = ui.available_width();
    Frame::new()
        .fill(p.bg_input)
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            let gap = 3.0_f32;
            let count = items.len().max(1) as f32;
            let inner_w = (total_w - 8.0).max(60.0);
            let btn_w = ((inner_w - gap * (count - 1.0)) / count).floor();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &(item_val, label) in items {
                    let active = *value == item_val;
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(btn_w, height), Sense::click());
                    let bg = if active {
                        if dark_mode {
                            Color32::from_rgb(2, 132, 199)
                        } else {
                            p.primary
                        }
                    } else if resp.hovered() {
                        p.bg_card_hover
                    } else {
                        Color32::TRANSPARENT
                    };
                    let fg = if active {
                        Color32::WHITE
                    } else if resp.hovered() {
                        p.text_main
                    } else {
                        p.text_sub
                    };
                    ui.painter().rect_filled(rect, CornerRadius::same(5), bg);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(text_size),
                        fg,
                    );
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.clicked() && !active {
                        *value = item_val;
                        changed = true;
                    }
                }
            });
        });
    changed
}

pub(super) fn pill_button(
    ui: &mut egui::Ui,
    p: RailbenchPalette,
    dark_mode: bool,
    selected: bool,
    width: f32,
    height: f32,
    label: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let active_bg = if dark_mode {
        Color32::from_rgb(2, 132, 199)
    } else {
        p.primary
    };
    let bg = if selected {
        active_bg
    } else if resp.hovered() {
        p.bg_card_hover
    } else {
        p.bg_input
    };
    let border = if selected {
        Stroke::new(1.0_f32, active_bg)
    } else if resp.hovered() {
        Stroke::new(1.0_f32, p.primary)
    } else {
        Stroke::new(1.0_f32, p.border_main)
    };
    let fg = if selected { Color32::WHITE } else { p.text_main };
    ui.painter()
        .rect(rect, CornerRadius::same(6), bg, border, egui::StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.5),
        fg,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub(super) fn draw_status_card(
    ui: &mut egui::Ui,
    dark_mode: bool,
    kind: StatusKind,
    title: &str,
    desc: &str,
    extra_warn: Option<&str>,
) {
    let (fill, border, fg) = status_palette(dark_mode, kind);
    let warn_col = status_palette(dark_mode, StatusKind::Warn).2;
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
                paint_status_icon(ui.painter(), icon_rect.center(), kind, fg);
                ui.label(RichText::new(title).strong().size(11.5).color(fg));
            });
            if !desc.is_empty() {
                ui.add_space(2.0);
                ui.label(RichText::new(desc).size(10.8).color(fg));
            }
            if let Some(w) = extra_warn {
                ui.add_space(2.0);
                ui.label(RichText::new(w).size(10.5).color(warn_col));
            }
        });
}

pub(super) fn section_label(ui: &mut egui::Ui, p: RailbenchPalette, text: &str) {
    ui.label(RichText::new(text).size(11.0).strong().color(p.text_sub));
}

pub(super) fn format_connector_name(ci: usize, is_de: bool) -> &'static str {
    match (ci, is_de) {
        (0, true) => "Ende A",
        (0, false) => "End A",
        (1, true) => "Ende B",
        (1, false) => "End B",
        (_, true) => "Ende C",
        (_, false) => "End C",
    }
}

pub(super) fn truncate_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let head: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{head}...")
    }
}

pub fn draw_height_ramp_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    if !app.show_height_dialog {
        app.height_dialog_was_open = false;
        return;
    }

    let t = tr(app.lang);
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let nem_h = min_clearance_for_scale(&app.layout.scale);
    let scale_label = app.layout.scale.clone();

    let layers_list: Vec<(String, String, f64)> = app
        .layout
        .layers
        .as_ref()
        .map(|ls| {
            ls.iter()
                .map(|l| (l.id.clone(), l.name.clone(), l.height_mm.unwrap_or(0.0)))
                .collect()
        })
        .unwrap_or_else(|| vec![("layer-1".into(), "Ebene 0".into(), 0.0)]);

    // One-shot initialization when opening the Height & Ramp panel
    if !app.height_dialog_was_open {
        app.height_dialog_was_open = true;
        app.ramp_target_warning = None;
        if app.height_flat_layer_id.is_none() {
            app.height_flat_layer_id = app
                .selected
                .iter()
                .next()
                .and_then(|sid| app.layout.pieces.iter().find(|pc| &pc.id == sid))
                .and_then(|pc| pc.layer_id.clone())
                .or_else(|| app.layout.active_layer_id.clone())
                .or_else(|| layers_list.first().map(|(id, _, _)| id.clone()));
        }
        if app.ramp_sel.start_piece.is_none()
            && app.ramp_sel.end_piece.is_none()
            && !app.selected.is_empty()
        {
            if let Some((_, sh, eh)) =
                calculate_auto_ramp(&app.layout, &app.selected, &app.catalogs)
            {
                app.ramp_start_h = (sh * 10.0).round() / 10.0;
                app.ramp_end_h = (eh * 10.0).round() / 10.0;
            }
        }
        if app.gen_target_layer_choice.is_empty() {
            app.gen_target_layer_choice = LAYER_ABOVE_SENTINEL.into();
        }
    }

    let mut open = app.show_height_dialog;
    let screen = ctx.screen_rect();
    let default_pos = Pos2::new((screen.max.x - PANEL_WIDTH - 24.0).max(24.0), 88.0);

    let mut flat_apply_params: Option<(String, bool, f64, f64)> = None;
    let mut slope_chain_for_footer: Vec<(String, bool)> = Vec::new();
    let mut auto_apply_params: Option<(String, usize)> = None;
    let mut trigger_apply = false;
    let mut action_cancel = false;

    egui::Window::new(t.height_dialog)
        .id(egui::Id::new("railbench_height_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .min_width(PANEL_WIDTH)
        .max_width(PANEL_WIDTH)
        .default_pos(default_pos)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            draw_window_header_bar(ui, p, t.height_dialog, Some(&mut open), None);

            Frame::new()
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 7.0;

                    // 3-Column Equal-Width Segmented Tabs
                    let prev_tab = app.height_tab;
                    let tabs = [
                        (0_u8, if is_de { "Höhe / Ebene" } else { "Height / Level" }),
                        (1_u8, if is_de { "Steigung" } else { "Slope" }),
                        (2_u8, if is_de { "Automatisch" } else { "Automatic" }),
                    ];
                    if segmented_control(ui, p, app.dark_mode, &mut app.height_tab, &tabs, 26.0, 11.5)
                        && prev_tab != app.height_tab
                    {
                        if app.height_tab != 1 {
                            app.ramp_sel.pick_mode = None;
                        }
                        if app.height_tab != 2 {
                            app.gen_pick_mode = false;
                            app.gen_preview_pieces.clear();
                        }
                    }

                    match app.height_tab {
                        0 => level_tab::draw_level_tab(
                            ui, app, p, is_de, nem_h, &layers_list, &mut flat_apply_params,
                        ),
                        1 => slope_tab::draw_slope_tab(
                            ui, app, p, is_de, &layers_list, &mut slope_chain_for_footer,
                        ),
                        _ => auto_tab::draw_auto_tab(
                            ui, app, p, is_de, nem_h, &scale_label, &layers_list, &mut auto_apply_params,
                        ),
                    }
                });

            // Shared Bottom Footer Bar (Cancel + Dynamic Primary Apply Button)
            ui.painter().line_segment(
                [
                    Pos2::new(ui.max_rect().min.x, ui.cursor().min.y),
                    Pos2::new(ui.max_rect().max.x, ui.cursor().min.y),
                ],
                Stroke::new(1.0_f32, p.border_main),
            );
            Frame::new()
                .fill(p.bg_sidebar)
                .corner_radius(CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 10,
                    se: 10,
                })
                .inner_margin(Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    let (apply_lbl, can_apply) = match app.height_tab {
                        0 => (
                            if is_de { "Höhe anwenden" } else { "Apply Height" },
                            flat_apply_params.is_some(),
                        ),
                        1 => (
                            if is_de { "Steigung anwenden" } else { "Apply Ramp" },
                            !slope_chain_for_footer.is_empty(),
                        ),
                        _ => (
                            if is_de { "Rampe erstellen" } else { "Create Ramp" },
                            auto_apply_params.is_some(),
                        ),
                    };

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        let primary_fill = if can_apply {
                            Color32::from_rgb(2, 132, 199)
                        } else {
                            p.bg_card
                        };
                        let apply_btn = egui::Button::new(
                            RichText::new(apply_lbl)
                                .strong()
                                .size(12.0)
                                .color(if can_apply { Color32::WHITE } else { p.text_muted }),
                        )
                        .fill(primary_fill)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(112.0, 28.0));

                        if ui.add_enabled(can_apply, apply_btn).clicked() {
                            trigger_apply = true;
                        }

                        let cancel_btn = egui::Button::new(
                            RichText::new(if is_de { "Abbrechen" } else { "Cancel" })
                                .size(12.0)
                                .color(p.text_sub),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(82.0, 28.0));
                        if ui.add(cancel_btn).clicked() {
                            action_cancel = true;
                        }
                    });
                });
        });

    if trigger_apply && !action_cancel {
        match app.height_tab {
            0 => {
                if let Some((target_lid, use_custom, abs_h, layer_h)) = flat_apply_params {
                    app.push_undo();
                    let count = app.selected.len();
                    for piece in &mut app.layout.pieces {
                        if app.selected.contains(&piece.id) {
                            piece.layer_id = Some(target_lid.clone());
                            piece.elevation = if use_custom {
                                let off = ((abs_h - layer_h) * 100.0).round() / 100.0;
                                Some(Elevation {
                                    start: off,
                                    end: off,
                                })
                            } else {
                                None
                            };
                        }
                    }
                    railbench_core::snapping::update_layout_connections(
                        &mut app.layout,
                        4.0,
                        35.0,
                        1.0,
                    );
                    crate::catalog::disconnect_cross_type_connections(
                        &mut app.layout,
                        &app.catalogs,
                    );
                    app.mark_layout_dirty();
                    app.show_toast(if is_de {
                        format!("Höhe für {count} Gleise aktualisiert.")
                    } else {
                        format!("Updated height for {count} tracks.")
                    });
                    open = false;
                }
            }
            1 => {
                if !slope_chain_for_footer.is_empty() {
                    app.push_undo();
                    if let Some(ref tl) = app.ramp_target_layer_id {
                        for (pid, _) in &slope_chain_for_footer {
                            if let Some(pc) = app.layout.pieces.iter_mut().find(|p| &p.id == pid) {
                                pc.layer_id = Some(tl.clone());
                            }
                        }
                    }
                    match apply_auto_ramp_for_chain(
                        &mut app.layout,
                        &app.catalogs,
                        &slope_chain_for_footer,
                        app.ramp_start_h,
                        app.ramp_end_h,
                        app.ramp_profile,
                    ) {
                        Ok(sum) => {
                            app.ramp_sel.pick_mode = None;
                            app.ramp_target_warning = None;
                            app.mark_layout_dirty();
                            app.show_toast(if is_de {
                                format!(
                                    "Rampe auf {} Gleise angewendet ({:.1}%)",
                                    sum.piece_count, sum.max_gradient_pct
                                )
                            } else {
                                format!(
                                    "Applied ramp over {} pieces ({:.1}% peak grade)",
                                    sum.piece_count, sum.max_gradient_pct
                                )
                            });
                            open = false;
                        }
                        Err(e) => app.status_message = Some(e),
                    }
                }
            }
            _ => {
                if let Some((anchor_id, conn_idx)) = auto_apply_params {
                    app.push_undo();
                    let final_layer_id = if app.gen_target_layer_choice == LAYER_ABOVE_SENTINEL {
                        let new_id = app.create_layer_at_height(app.gen_target_h, true);
                        app.gen_target_layer_choice = new_id.clone();
                        app.gen_target_layer_id = Some(new_id.clone());
                        Some(new_id)
                    } else if app.gen_target_layer_choice == LAYER_BELOW_SENTINEL {
                        let new_id = app.create_layer_at_height(app.gen_target_h, false);
                        app.gen_target_layer_choice = new_id.clone();
                        app.gen_target_layer_id = Some(new_id.clone());
                        Some(new_id)
                    } else if !app.gen_target_layer_choice.is_empty() {
                        Some(app.gen_target_layer_choice.clone())
                    } else {
                        None
                    };

                    match generate_ramp_or_helix_ext(
                        &mut app.layout,
                        &app.catalogs,
                        app.selected_catalog_idx,
                        &mut app.next_piece_seq,
                        &anchor_id,
                        Some(conn_idx),
                        final_layer_id.as_deref(),
                        app.gen_ramp_shape,
                        app.gen_curve_left,
                        app.gen_arc_angle_deg,
                        app.gen_target_h,
                        app.gen_max_grade,
                        app.gen_helix_turns,
                        app.gen_curve_id.as_deref(),
                    ) {
                        Ok((new_ids, sum)) => {
                            app.selected.clear();
                            for id in new_ids {
                                app.selected.insert(id);
                            }
                            app.gen_pick_mode = false;
                            app.gen_preview_pieces.clear();
                            app.mark_layout_dirty();
                            app.show_toast(if is_de {
                                format!(
                                    "{} Gleise erzeugt ({:.1}%, {:.1} mm Hub)",
                                    sum.piece_count, sum.avg_gradient_pct, sum.total_rise_mm
                                )
                            } else {
                                format!(
                                    "Generated {} pieces ({:.1}% grade, {:.1} mm rise)",
                                    sum.piece_count, sum.avg_gradient_pct, sum.total_rise_mm
                                )
                            });
                            open = false;
                        }
                        Err(e) => app.status_message = Some(e),
                    }
                }
            }
        }
    }

    if !open || action_cancel {
        app.show_height_dialog = false;
        app.ramp_sel.pick_mode = None;
        app.gen_pick_mode = false;
        app.gen_preview_pieces.clear();
        app.ramp_target_warning = None;
    } else {
        app.show_height_dialog = true;
    }
}
