use super::*;

pub fn draw_right_sidebar(ctx: &egui::Context, app: &mut RailbenchApp, viewport_rect: egui::Rect) {
    if !app.show_right_sidebar {
        return;
    }
    let t = tr(app.lang);
    let p = palette(app.dark_mode);
    let bg_alpha = if app.dark_mode { 194 } else { 212 };
    let card_alpha = if app.dark_mode { 180 } else { 198 };
    let card_fill = Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), card_alpha);
    let win_fill = Color32::from_rgba_unmultiplied(p.bg_sidebar.r(), p.bg_sidebar.g(), p.bg_sidebar.b(), bg_alpha);
    let win_border = Color32::from_rgba_unmultiplied(p.border_main.r(), p.border_main.g(), p.border_main.b(), 215);

    let mut visibility_or_layers_changed = false;
    let max_w = (viewport_rect.width() * 0.45).max(260.0);
    let max_h = (viewport_rect.height() - 56.0).max(160.0);

    let init_w = app
        .app_settings
        .right_sidebar_width
        .unwrap_or(256.0)
        .clamp(215.0, max_w.max(215.0));

    let win_resp = egui::Window::new(t.layers_title)
        .id(egui::Id::new("floating_layers_panel"))
        .title_bar(false)
        .collapsible(false)
        .resizable([!app.bars_locked, !app.bars_locked])
        .default_width(init_w)
        .min_width(215.0)
        .max_width(max_w)
        .max_height(max_h)
        .constrain_to(viewport_rect.shrink(6.0))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-6.0, 6.0))
        .frame(
            Frame::new()
                .fill(win_fill)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, win_border))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_black_alpha(if app.dark_mode { 115 } else { 55 }),
                }),
        )
        .show(ctx, |ui| {
            let text_col = if app.dark_mode { p.text_main } else { Color32::BLACK };
            let sub_col = if app.dark_mode { p.text_muted } else { Color32::from_rgb(35, 35, 35) };
            let dim_icon_col = if app.dark_mode { p.text_muted } else { Color32::from_rgb(95, 95, 95) };
            let icon_col = if app.dark_mode { p.text_sub } else { text_col };
            ui.visuals_mut().override_text_color = Some(text_col);
            ui.visuals_mut().widgets.inactive.fg_stroke.color = text_col;
            ui.visuals_mut().widgets.hovered.fg_stroke.color = text_col;
            ui.visuals_mut().selection.stroke = Stroke::new(
                1.2_f32,
                if app.dark_mode { p.primary } else { Color32::BLACK },
            );

            // Count pieces per layer
            let mut layer_counts: BTreeMap<String, usize> = BTreeMap::new();
            for piece in &app.layout.pieces {
                let lid = piece.layer_id.clone().unwrap_or_else(|| "layer-1".into());
                *layer_counts.entry(lid).or_insert(0) += 1;
            }

            let active_id = app.layout.active_layer_id.clone().unwrap_or_else(|| "layer-1".into());
            let mut activate_layer_id: Option<String> = None;
            let mut delete_layer_id: Option<String> = None;
            let mut edit_layer_id: Option<String> = None;
            let mut move_sel_to_layer: Option<String> = None;
            let has_selection = !app.selected.is_empty();
            let total_layers = app.layout.layers.as_ref().map(|l| l.len()).unwrap_or(0);

            egui::ScrollArea::vertical()
                .id_salt("floating_layers_scroll")
                .auto_shrink([false, true])
                .max_height((viewport_rect.height() - 190.0).max(96.0))
                .show(ui, |ui| {
                    if let Some(layers) = &mut app.layout.layers {
                        let mut order: Vec<usize> = (0..layers.len()).collect();
                        order.sort_by(|&a, &b| {
                            layers[b]
                                .height_mm
                                .unwrap_or(0.0)
                                .partial_cmp(&layers[a].height_mm.unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        for idx in order {
                            let layer = &mut layers[idx];
                            let layer_id = layer.id.clone();
                            let is_act = layer_id == active_id;
                            let swatch = parse_hex_color(layer.color.as_deref()).unwrap_or(p.primary);
                            let card_bg = if is_act {
                                Color32::from_rgba_unmultiplied(
                                    ((u16::from(card_fill.r()) * 4 + u16::from(p.primary.r())) / 5) as u8,
                                    ((u16::from(card_fill.g()) * 4 + u16::from(p.primary.g())) / 5) as u8,
                                    ((u16::from(card_fill.b()) * 4 + u16::from(p.primary.b())) / 5) as u8,
                                    card_fill.a().max(210),
                                )
                            } else {
                                card_fill
                            };
                            let stroke = if is_act {
                                Stroke::new(1.4_f32, p.primary)
                            } else {
                                Stroke::new(1.0_f32, p.border_main)
                            };
                            let cnt = layer_counts.get(&layer_id).copied().unwrap_or(0);
                            let mut action_clicked = false;
                            let mut action_hovered = false;
                            let mut action_rects: Vec<egui::Rect> = Vec::with_capacity(5);
                            #[cfg(test)]
                            let mut test_btn_rects: [Option<egui::Rect>; 4] = [None; 4];

                            let card_resp = Frame::new()
                                .fill(card_bg)
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(8, 6))
                                .stroke(stroke)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (bar_r, _) = ui.allocate_exact_size(egui::vec2(4.0, 26.0), egui::Sense::hover());
                                        ui.painter().rect_filled(bar_r, 2.0, swatch);

                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 2.0;
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(&layer.name).strong().size(12.0).color(text_col),
                                                    )
                                                    .selectable(false),
                                                );
                                                if is_act {
                                                    Frame::new()
                                                        .fill(p.primary_hover)
                                                        .corner_radius(CornerRadius::same(3))
                                                        .inner_margin(Margin::symmetric(4, 1))
                                                        .show(ui, |ui| {
                                                            ui.add(
                                                                egui::Label::new(
                                                                    RichText::new(t.active_badge)
                                                                        .strong()
                                                                        .size(8.5)
                                                                        .color(Color32::WHITE),
                                                                )
                                                                .selectable(false),
                                                            );
                                                        });
                                                }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(format!(
                                                            "{:.0} mm  •  {} {}",
                                                            layer.height_mm.unwrap_or(0.0),
                                                            cnt,
                                                            t.status_pieces
                                                        ))
                                                        .size(10.0)
                                                        .color(sub_col),
                                                    )
                                                    .selectable(false),
                                                );
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    let sz = egui::vec2(20.0, 20.0);
                                                    if total_layers > 1 {
                                                        let r = icon_button_sized(
                                                            ui,
                                                            Icon::Trash,
                                                            sz,
                                                            false,
                                                            p.bg_input,
                                                            Color32::from_rgb(220, 38, 38),
                                                            Color32::from_rgb(220, 38, 38),
                                                            icon_col,
                                                            if app.lang == Lang::De { "Ebene loeschen" } else { "Delete layer" },
                                                        );
                                                        action_rects.push(r.rect);
                                                        #[cfg(test)]
                                                        {
                                                            test_btn_rects[3] = Some(r.rect);
                                                        }
                                                        action_hovered |= r.hovered();
                                                        if r.clicked() {
                                                            action_clicked = true;
                                                            delete_layer_id = Some(layer_id.clone());
                                                        }
                                                    }
                                                    let edit_r = ui
                                                        .small_button(RichText::new("✏").color(text_col))
                                                        .on_hover_text(if app.lang == Lang::De { "Ebene bearbeiten" } else { "Edit layer" });
                                                    action_rects.push(edit_r.rect);
                                                    #[cfg(test)]
                                                    {
                                                        test_btn_rects[2] = Some(edit_r.rect);
                                                    }
                                                    action_hovered |= edit_r.hovered();
                                                    if edit_r.clicked() {
                                                        action_clicked = true;
                                                        edit_layer_id = Some(layer_id.clone());
                                                    }
                                                    if has_selection {
                                                        let mv_r = ui
                                                            .small_button(RichText::new("⤵").color(text_col))
                                                            .on_hover_text(if app.lang == Lang::De { "Auswahl hierher verschieben" } else { "Move selection to this layer" });
                                                        action_rects.push(mv_r.rect);
                                                        action_hovered |= mv_r.hovered();
                                                        if mv_r.clicked() {
                                                            action_clicked = true;
                                                            move_sel_to_layer = Some(layer_id.clone());
                                                        }
                                                    }
                                                    let lock_icon = if layer.locked { Icon::Lock } else { Icon::Unlock };
                                                    let lock_r = icon_button_sized(
                                                        ui,
                                                        lock_icon,
                                                        sz,
                                                        layer.locked,
                                                        p.bg_input,
                                                        p.primary_hover,
                                                        p.primary,
                                                        icon_col,
                                                        if app.lang == Lang::De { "Ebene sperren" } else { "Lock layer" },
                                                    );
                                                    action_rects.push(lock_r.rect);
                                                    #[cfg(test)]
                                                    {
                                                        test_btn_rects[0] = Some(lock_r.rect);
                                                    }
                                                    action_hovered |= lock_r.hovered();
                                                    if lock_r.clicked() {
                                                        action_clicked = true;
                                                        layer.locked = !layer.locked;
                                                    }
                                                    let vis_icon = if layer.visible { Icon::Eye } else { Icon::EyeOff };
                                                    let vis_r = icon_button_sized(
                                                        ui,
                                                        vis_icon,
                                                        sz,
                                                        false,
                                                        p.bg_input,
                                                        p.primary_hover,
                                                        p.primary,
                                                        if layer.visible { text_col } else { dim_icon_col },
                                                        if app.lang == Lang::De { "Sichtbarkeit" } else { "Toggle visibility" },
                                                    );
                                                    action_rects.push(vis_r.rect);
                                                    #[cfg(test)]
                                                    {
                                                        test_btn_rects[1] = Some(vis_r.rect);
                                                    }
                                                    action_hovered |= vis_r.hovered();
                                                    if vis_r.clicked() {
                                                        action_clicked = true;
                                                        layer.visible = !layer.visible;
                                                        visibility_or_layers_changed = true;
                                                    }
                                                });
                                            });
                                        });
                                    });
                                })
                                .response;

                            #[cfg(test)]
                            {
                                let tid = egui::Id::new(format!("test_layer_rects_{layer_id}"));
                                ui.ctx().data_mut(|d| d.insert_temp(tid, (card_resp.rect, test_btn_rects)));
                            }

                            let pointer_pos =
                                ui.input(|i| i.pointer.hover_pos().or_else(|| i.pointer.interact_pos()));
                            let pointer_in_action =
                                pointer_pos.is_some_and(|pos| action_rects.iter().any(|r| r.contains(pos)));
                            let card_hovered = ui.rect_contains_pointer(card_resp.rect)
                                && !action_hovered
                                && !pointer_in_action;
                            let card_clicked = card_hovered
                                && !action_clicked
                                && ui.input(|i| i.pointer.primary_clicked());

                            if card_hovered {
                                ui.painter().rect_filled(
                                    card_resp.rect,
                                    CornerRadius::same(8),
                                    Color32::from_rgba_unmultiplied(
                                        p.primary.r(),
                                        p.primary.g(),
                                        p.primary.b(),
                                        if app.dark_mode { 24 } else { 18 },
                                    ),
                                );
                                ui.painter().rect_stroke(
                                    card_resp.rect,
                                    CornerRadius::same(8),
                                    Stroke::new(1.3_f32, p.primary),
                                    egui::StrokeKind::Inside,
                                );
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if card_clicked {
                                activate_layer_id = Some(layer_id);
                            }
                        }
                    }
                });

            if let Some(aid) = activate_layer_id {
                app.layout.active_layer_id = Some(aid);
            }

            ui.add_space(6.0);
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(RichText::new(t.add_layer_full).strong().size(12.0).color(text_col))
                        .fill(card_fill)
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.open_layer_dialog_new();
            }

            if let Some(eid) = edit_layer_id {
                app.open_layer_dialog_edit(&eid);
            }
            if let Some(tid) = move_sel_to_layer {
                app.move_selected_to_layer(&tid);
            }

            ui.add_space(6.0);
            let orphaned_count = app.count_orphaned_items();
            if orphaned_count > 0 {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(239, 68, 68, 30))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(8))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(239, 68, 68, 110)))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let warn_txt = if app.lang == Lang::De {
                            format!("{orphaned_count} Element(e) auf gelöschter Ebene")
                        } else {
                            format!("{orphaned_count} item(s) on deleted layer")
                        };
                        ui.label(RichText::new(warn_txt).strong().size(11.0).color(Color32::from_rgb(248, 113, 113)));
                        let btn_lbl = if app.lang == Lang::De {
                            "Aktiver Ebene zuweisen"
                        } else {
                            "Reassign to active layer"
                        };
                        if ui
                            .add(
                                egui::Button::new(RichText::new(btn_lbl).strong().size(11.0).color(Color32::WHITE))
                                    .fill(p.primary_hover)
                                    .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            app.reassign_orphaned_items();
                        }
                    });
                ui.add_space(6.0);
            }

            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(RichText::new(t.heights_ramp_btn).strong().size(12.0).color(Color32::WHITE))
                        .fill(p.primary_hover)
                        .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.show_height_dialog = true;
            }

            if let Some(del_id) = delete_layer_id {
                if app.count_items_on_layer(&del_id) > 0 {
                    app.confirm_delete_layer_id = Some(del_id);
                } else if app.safe_delete_layer(&del_id) {
                    visibility_or_layers_changed = true;
                }
            }
        });

    if let Some(ir) = win_resp {
        let rw = ir.response.rect.width().clamp(215.0, 480.0);
        if (rw - app.app_settings.right_sidebar_width.unwrap_or(256.0)).abs() > 2.0 {
            app.app_settings.right_sidebar_width = Some(rw);
            app.save_current_settings();
        }
    }

    if visibility_or_layers_changed {
        app.mark_layout_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, PointerButton, Pos2, RawInput, Rect};

    fn click_pos(ctx: &egui::Context, app: &mut RailbenchApp, vp: Rect, pos: Pos2) {
        for events in [
            vec![Event::PointerMoved(pos)],
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        ] {
            let mut input = RawInput::default();
            input.screen_rect = Some(vp);
            input.events = events;
            let _ = ctx.run(input, |c| {
                draw_right_sidebar(c, app, vp);
            });
        }
    }

    #[test]
    fn layer_card_buttons_lock_visible_edit_delete_and_card_body_switches_layer() {
        let mut app = RailbenchApp::default();
        app.layout = crate::make_empty_layout();
        app.layout.layers = Some(vec![
            Layer {
                id: "layer-1".into(),
                name: "Ebene 0".into(),
                color: Some("#38bdf8".into()),
                visible: true,
                locked: false,
                height_mm: Some(0.0),
                wood_plate_mode: None,
                wood_plate_thickness_mm: None,
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            },
            Layer {
                id: "layer-2".into(),
                name: "Ebene 1".into(),
                color: Some("#f59e0b".into()),
                visible: true,
                locked: false,
                height_mm: Some(80.0),
                wood_plate_mode: None,
                wood_plate_thickness_mm: None,
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            },
        ]);
        app.layout.active_layer_id = Some("layer-1".into());
        app.show_right_sidebar = true;

        let ctx = egui::Context::default();
        let vp = Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0));
        // Initial layout pass so widgets record their exact screen rects
        let mut init_in = RawInput::default();
        init_in.screen_rect = Some(vp);
        let _ = ctx.run(init_in, |c| draw_right_sidebar(c, &mut app, vp));
        let mut init_in2 = RawInput::default();
        init_in2.screen_rect = Some(vp);
        let _ = ctx.run(init_in2, |c| draw_right_sidebar(c, &mut app, vp));

        let (card_rect, btn_rects): (Rect, [Option<Rect>; 4]) = ctx
            .data(|d| d.get_temp(egui::Id::new("test_layer_rects_layer-2")))
            .expect("layer-2 card & action rects recorded");
        let lock_c = btn_rects[0].expect("lock rect").center();
        let vis_c = btn_rects[1].expect("vis rect").center();
        let edit_c = btn_rects[2].expect("edit rect").center();
        let del_c = btn_rects[3].expect("delete rect").center();

        // 1. Lock button toggles layer-2.locked without stealing active_layer_id
        click_pos(&ctx, &mut app, vp, lock_c);
        assert!(app.layout.layers.as_ref().unwrap()[1].locked);
        assert_eq!(app.layout.active_layer_id.as_deref(), Some("layer-1"));

        // 2. Visible button toggles layer-2.visible without stealing active_layer_id
        click_pos(&ctx, &mut app, vp, vis_c);
        assert!(!app.layout.layers.as_ref().unwrap()[1].visible);
        assert_eq!(app.layout.active_layer_id.as_deref(), Some("layer-1"));

        // 3. Edit button opens layer edit dialog for layer-2
        click_pos(&ctx, &mut app, vp, edit_c);
        assert!(app.show_layer_dialog);
        assert_eq!(app.layer_dialog_edit_id.as_deref(), Some("layer-2"));
        app.show_layer_dialog = false;

        // 4. Clicking card body (top-left of card away from buttons) switches active_layer_id
        let card_body = Pos2::new(card_rect.min.x + 28.0, card_rect.min.y + 12.0);
        click_pos(&ctx, &mut app, vp, card_body);
        assert_eq!(app.layout.active_layer_id.as_deref(), Some("layer-2"));

        // 5. Delete button deletes empty layer-2 cleanly
        click_pos(&ctx, &mut app, vp, del_c);
        assert_eq!(app.layout.layers.as_ref().unwrap().len(), 1);
        assert_eq!(app.layout.layers.as_ref().unwrap()[0].id, "layer-1");
    }
}
