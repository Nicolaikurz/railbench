use super::*;

#[allow(clippy::too_many_arguments)]
fn draw_welcome_action_card(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    dark_mode: bool,
    icon: Icon,
    title: &str,
    subtitle: &str,
    is_primary: bool,
) -> bool {
    let w = ui.available_width();
    let h = 60.0_f32;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let fill = if is_primary {
            Color32::from_rgba_unmultiplied(
                2,
                132,
                199,
                if dark_mode {
                    if hovered { 46 } else { 28 }
                } else if hovered {
                    36
                } else {
                    20
                },
            )
        } else if hovered {
            p.bg_card_hover
        } else {
            p.bg_sidebar
        };
        let border = if is_primary {
            Stroke::new(1.25_f32, p.primary)
        } else if hovered {
            Stroke::new(1.0_f32, p.primary)
        } else {
            Stroke::new(1.0_f32, p.border_main)
        };
        ui.painter()
            .rect(rect, CornerRadius::same(10), fill, border, egui::StrokeKind::Inside);

        let icon_box = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + 14.0, rect.center().y - 18.0),
            egui::vec2(36.0, 36.0),
        );
        ui.painter().rect(
            icon_box,
            CornerRadius::same(8),
            p.bg_card,
            Stroke::new(1.0_f32, p.border_main),
            egui::StrokeKind::Inside,
        );
        crate::icons::paint_icon(
            ui.painter(),
            icon_box.shrink(9.0),
            icon,
            p.text_main,
        );

        let text_left = icon_box.max.x + 12.0;
        let text_clip = egui::Rect::from_min_max(
            egui::pos2(text_left, rect.min.y + 8.0),
            egui::pos2((rect.max.x - 36.0).max(text_left + 40.0), rect.max.y - 8.0),
        );
        let tp = ui.painter_at(text_clip);
        tp.text(
            egui::pos2(text_clip.min.x, rect.center().y - 9.0),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(13.5),
            p.text_main,
        );
        tp.text(
            egui::pos2(text_clip.min.x, rect.center().y + 10.0),
            egui::Align2::LEFT_CENTER,
            subtitle,
            egui::FontId::proportional(11.5),
            p.text_muted,
        );

        let arrow_col = if hovered { p.primary } else { p.text_muted };
        let arrow_x = rect.max.x - 18.0 + if hovered { 2.0 } else { 0.0 };
        crate::icons::paint_arrow_right(
            ui.painter(),
            egui::pos2(arrow_x, rect.center().y),
            4.5,
            arrow_col,
        );
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

pub(super) fn draw_welcome_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_welcome_dialog;
    let mut action: Option<u8> = None;

    let recent_path = app
        .app_settings
        .recent_files
        .iter()
        .find(|path| std::path::Path::new(path).exists())
        .cloned()
        .or_else(|| {
            app.app_settings
                .last_opened_path
                .clone()
                .filter(|path| std::path::Path::new(path).exists())
        });

    let screen_rect = ctx.screen_rect();
    egui::Area::new(egui::Id::new("railbench_welcome_backdrop"))
        .order(egui::Order::Middle)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let (bg_rect, bg_resp) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                bg_rect,
                0.0,
                Color32::from_rgba_unmultiplied(10, 15, 29, if app.dark_mode { 184 } else { 145 }),
            );
            if bg_resp.clicked() && !app.show_help_dialog {
                open = false;
            }
        });
    if !app.show_help_dialog && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        open = false;
    }

    let card_w = 440.0_f32.min((screen_rect.width() - 32.0).max(300.0));
    let card_frame = Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(0))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .shadow(egui::Shadow {
            offset: [0, 20],
            blur: 45,
            spread: 0,
            color: Color32::from_black_alpha(if app.dark_mode { 160 } else { 85 }),
        });

    egui::Window::new("Railbench")
        .id(egui::Id::new("railbench_welcome_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Middle)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(card_w)
        .max_width(card_w)
        .frame(card_frame)
        .show(ctx, |ui| {
            ui.set_width(card_w);

            // Header bar (Train icon + "Railbench" + version tag left | Guide + Close right)
            let header_resp = Frame::new()
                .fill(p.bg_sidebar)
                .corner_radius(CornerRadius {
                    nw: 12,
                    ne: 12,
                    sw: 0,
                    se: 0,
                })
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (tr_r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                        crate::icons::paint_icon(ui.painter(), tr_r, Icon::Train, p.text_main);
                        ui.label(RichText::new("Railbench").strong().size(15.5).color(p.text_main));
                        Frame::new()
                            .fill(p.bg_card)
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(6, 2))
                            .stroke(Stroke::new(1.0_f32, p.border_main))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(crate::version::APP_VERSION_TAG)
                                        .strong()
                                        .size(11.0)
                                        .color(p.text_muted),
                                );
                            });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (close_r, close_resp) =
                                ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                            if close_resp.hovered() {
                                ui.painter().rect_filled(close_r, CornerRadius::same(6), p.bg_card_hover);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            crate::icons::paint_icon(
                                ui.painter(),
                                close_r.shrink(7.0),
                                Icon::WinClose,
                                if close_resp.hovered() { p.text_main } else { p.text_muted },
                            );
                            if close_resp
                                .on_hover_text(if is_de { "Schliessen (Esc)" } else { "Close (Esc)" })
                                .clicked()
                            {
                                open = false;
                            }

                            ui.add_space(6.0);
                            let guide_lbl = if is_de { "Anleitung" } else { "Guide" };
                            let guide_btn = egui::Button::new(
                                RichText::new(guide_lbl).strong().size(12.0).color(p.text_main),
                            )
                            .fill(p.bg_card)
                            .stroke(Stroke::new(1.0_f32, p.border_main))
                            .min_size(egui::vec2(0.0, 28.0))
                            .corner_radius(CornerRadius::same(6));
                            let guide_resp = ui
                                .add(guide_btn)
                                .on_hover_text(if is_de {
                                    "Bildanleitung & Hilfe oeffnen"
                                } else {
                                    "Open visual guide & help"
                                });
                            if guide_resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if guide_resp.clicked() {
                                app.help_tab = 1;
                                app.show_help_dialog = true;
                            }
                        });
                    });
                });
            let hr = header_resp.response.rect;
            ui.painter()
                .hline(hr.x_range(), hr.bottom(), Stroke::new(1.0_f32, p.border_main));

            // Action cards stack
            Frame::new()
                .inner_margin(Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 8.0;

                    let has_recent = recent_path.is_some();
                    if let Some(ref recent) = recent_path {
                        let fname = std::path::Path::new(recent)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(recent);
                        if draw_welcome_action_card(
                            ui,
                            p,
                            app.dark_mode,
                            Icon::Clock,
                            if is_de { "Letzten Plan fortsetzen" } else { "Continue Last Plan" },
                            fname,
                            true,
                        ) {
                            action = Some(1);
                        }
                    }

                    if draw_welcome_action_card(
                        ui,
                        p,
                        app.dark_mode,
                        Icon::Sparkle,
                        if is_de { "Neuer Plan" } else { "New Plan" },
                        if is_de {
                            "Leere Platte mit Standard-Ebenen"
                        } else {
                            "Empty baseboard with default layers"
                        },
                        !has_recent,
                    ) {
                        action = Some(2);
                    }

                    if draw_welcome_action_card(
                        ui,
                        p,
                        app.dark_mode,
                        Icon::Mountain,
                        if is_de { "Demo-Layout" } else { "Demo Layout" },
                        if is_de {
                            "Musterstrecke zum Testen & Planen"
                        } else {
                            "Sample layout for testing & planning"
                        },
                        false,
                    ) {
                        action = Some(4);
                    }

                    if draw_welcome_action_card(
                        ui,
                        p,
                        app.dark_mode,
                        Icon::FolderOpen,
                        if is_de { "Plan oeffnen..." } else { "Open Plan..." },
                        if is_de {
                            ".railbench Datei von Festplatte laden"
                        } else {
                            "Load .railbench file from disk"
                        },
                        false,
                    ) {
                        action = Some(3);
                    }
                });

            // Footer bar
            let footer_resp = Frame::new()
                .fill(p.bg_sidebar)
                .corner_radius(CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 12,
                    se: 12,
                })
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut app.welcome_remember_choice,
                            RichText::new(if is_de {
                                "Auswahl merken (nicht mehr fragen)"
                            } else {
                                "Remember choice (don't ask again)"
                            })
                            .size(12.0)
                            .color(p.text_muted),
                        );
                    });
                });
            let fr = footer_resp.response.rect;
            ui.painter()
                .hline(fr.x_range(), fr.top(), Stroke::new(1.0_f32, p.border_main));
        });

    app.show_welcome_dialog = open;
    if let Some(act) = action {
        match act {
            1 => {
                if let Some(ref recent) = recent_path {
                    app.open_layout_path(recent);
                }
                if app.welcome_remember_choice {
                    app.app_settings.startup_action = StartupAction::Recent;
                    app.save_current_settings();
                }
                app.show_welcome_dialog = false;
            }
            2 => {
                app.new_empty_layout();
                if app.welcome_remember_choice {
                    app.app_settings.startup_action = StartupAction::New;
                    app.save_current_settings();
                }
                app.show_welcome_dialog = false;
            }
            3 => {
                app.open_layout_dialog();
                app.show_welcome_dialog = false;
            }
            4 => {
                if app.welcome_remember_choice {
                    app.app_settings.startup_action = StartupAction::Demo;
                    app.save_current_settings();
                }
                app.show_welcome_dialog = false;
                app.show_demo_dialog = true;
            }
            _ => {
                app.show_welcome_dialog = false;
            }
        }
    }
}

pub(super) fn draw_demo_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_demo_dialog;
    let mut chosen_path: Option<&'static str> = None;

    const DEMOS: &[(&str, &str, &str, &str, &str)] = &[
        (
            "H0",
            "Alpenland H0 Demo",
            "demotracks/alpenland-h0.railbench",
            "Gebirgsanlage mit Tunneln, Bruecken, Bahnhof und Landschaft",
            "Mountain layout with tunnels, bridges, station & terrain",
        ),
        (
            "H0",
            "Mainline H0",
            "demotracks/mainline-h0.railbench",
            "Zweigleisiges Hauptbahn-Oval mit Ausweichgleisen & Stromkreisen",
            "Double-track mainline oval with sidings & power feeders",
        ),
        (
            "0",
            "Shunting Yard 0",
            "demotracks/shunting-0.railbench",
            "Rangierbahnhof im Massstab 1:45 (Lenz Spur 0)",
            "Industrial switching yard in 1:45 (Scale 0)",
        ),
        (
            "N",
            "Switching N",
            "demotracks/switching-n.railbench",
            "Kompakte Spur-N Timesaver-Rangieranlage",
            "Compact Scale N timesaver shunting layout",
        ),
        (
            "G",
            "Garden G",
            "demotracks/garden-g.railbench",
            "Gartenbahn mit R1-Boegen & Abstellgleisen (LGB Spur G)",
            "Outdoor garden railway with R1 curves & spurs (LGB G)",
        ),
        (
            "Z",
            "Suitcase Z",
            "demotracks/suitcase-z.railbench",
            "Kompakte Kofferanlage in 1:220 (Maerklin mini-club Spur Z)",
            "Compact suitcase layout in 1:220 (Scale Z)",
        ),
    ];

    let demo_title = if is_de {
        "Demo-Anlagen"
    } else {
        "Demo Layouts"
    };
    egui::Window::new(demo_title)
        .id(egui::Id::new("railbench_demo_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_width(520.0)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, demo_title, Some(&mut open), 12, |ui| {
        ui.label(
            RichText::new(if is_de {
                "Waehlen Sie eine vorkonfigurierte Beispielanlage zum Laden:"
            } else {
                "Choose a ready-to-run sample layout to load:"
            })
            .size(12.0)
            .color(p.text_muted),
        );
        ui.add_space(6.0);

        for (scale, title, path, desc_de, desc_en) in DEMOS {
            let demo_meta = app.demo_preview_meta(path);
            let available = std::path::Path::new(path).exists() || crate::RailbenchApp::embedded_demo_json(path).is_some();
            Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::same(9))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        Frame::new()
                            .fill(p.primary_hover)
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.label(RichText::new(*scale).strong().size(12.0).color(Color32::WHITE));
                            });
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new(*title).strong().size(13.0).color(p.text_main));
                            ui.label(
                                RichText::new(if is_de { *desc_de } else { *desc_en })
                                    .size(11.0)
                                    .color(p.text_muted),
                            );
                            if let Some(ref meta) = demo_meta {
                                ui.label(
                                    RichText::new(meta)
                                        .size(10.0)
                                        .color(p.primary),
                                );
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    available,
                                    egui::Button::new(if is_de { "Laden" } else { "Load" }),
                                )
                                .clicked()
                            {
                                chosen_path = Some(*path);
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }
            });
    });

    app.show_demo_dialog = open;
    if let Some(path) = chosen_path {
        app.request_open_demo(path);
        app.show_demo_dialog = false;
        app.show_welcome_dialog = false;
    }
}
