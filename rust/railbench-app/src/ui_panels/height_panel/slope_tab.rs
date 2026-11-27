use super::*;

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub(super) fn draw_slope_tab(
    ui: &mut egui::Ui,
    app: &mut RailbenchApp,
    p: RailbenchPalette,
    is_de: bool,
    layers_list: &[(String, String, f64)],
    slope_chain_for_footer: &mut Vec<(String, bool)>,
) {
        // TAB 1: SLOPE
        let chain = resolve_ramp_chain(
            &app.layout,
            &app.selected,
            app.ramp_sel.start_piece.as_deref(),
            app.ramp_sel.end_piece.as_deref(),
        );
        let summary = preview_auto_ramp_for_chain(
            &app.layout,
            &app.catalogs,
            &chain,
            app.ramp_start_h,
            app.ramp_end_h,
            app.ramp_profile,
        );
        let total_len_mm = summary.as_ref().map(|s| s.total_length_mm).unwrap_or(0.0);
        let track_cnt = summary.as_ref().map(|s| s.piece_count).unwrap_or(chain.len());

        // Meta strip: "{N} tracks" | "{len_cm:.1} cm"
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if is_de {
                    format!("{track_cnt} Gleise")
                } else {
                    format!("{track_cnt} tracks")
                })
                .size(11.0)
                .color(p.text_muted),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{:.1} cm", total_len_mm / 10.0))
                        .size(11.0)
                        .strong()
                        .color(p.text_sub),
                );
            });
        });

        // Unified Endpoints Box (A Start / Swap+Status / B End)
        let resolve_endpoint_label = |explicit_id: Option<&str>, chain_idx: Option<usize>| -> String {
            let target_id = explicit_id.map(|s| s.to_string()).or_else(|| {
                chain_idx.and_then(|idx| chain.get(idx).map(|(id, _)| id.clone()))
            });
            if let Some(pid) = target_id {
                if let Some(pc) = app.layout.pieces.iter().find(|p| p.id == pid) {
                    return format!(
                        "#{} ({:.0}, {:.0})",
                        pc.catalog_id, pc.transform.x, pc.transform.y
                    );
                }
            }
            if is_de { "Gleis wählen..." } else { "Pick track..." }.into()
        };

        let label_a = resolve_endpoint_label(
            app.ramp_sel.start_piece.as_deref(),
            if !chain.is_empty() { Some(0) } else { None },
        );
        let label_b = resolve_endpoint_label(
            app.ramp_sel.end_piece.as_deref(),
            if chain.len() >= 2 { Some(chain.len() - 1) } else { None },
        );

        Frame::new()
            .fill(p.bg_input)
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(8, 8))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 5.0;

                let draw_endpoint_row = |ui: &mut egui::Ui,
                                         badge: &str,
                                         row_lbl: &str,
                                         badge_fill: Color32,
                                         accent_text: Color32,
                                         pick_txt: &str,
                                         picking: bool,
                                         height_val: &mut f64,
                                         on_pick: &mut bool,
                                         h_changed: &mut bool| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (dot_r, _) =
                            ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
                        ui.painter().circle_filled(dot_r.center(), 7.5, badge_fill);
                        ui.painter().text(
                            dot_r.center(),
                            egui::Align2::CENTER_CENTER,
                            badge,
                            egui::FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                        ui.add_sized(
                            [34.0, 22.0],
                            egui::Label::new(RichText::new(row_lbl).size(11.0).strong().color(p.text_sub)),
                        );

                        let pick_w = (ui.available_width() - 84.0).max(96.0);
                        let (pick_rect, pick_resp) =
                            ui.allocate_exact_size(Vec2::new(pick_w, 24.0), Sense::click());
                        let pick_bg = if picking {
                            Color32::from_rgba_unmultiplied(accent_text.r(), accent_text.g(), accent_text.b(), 42)
                        } else if pick_resp.hovered() {
                            Color32::from_rgba_unmultiplied(p.primary.r(), p.primary.g(), p.primary.b(), 26)
                        } else {
                            p.bg_card
                        };
                        let pick_border = if picking {
                            Stroke::new(1.3_f32, accent_text)
                        } else if pick_resp.hovered() {
                            Stroke::new(1.0_f32, p.primary)
                        } else {
                            Stroke::new(1.0_f32, p.border_main)
                        };
                        ui.painter().rect(
                            pick_rect,
                            CornerRadius::same(6),
                            pick_bg,
                            pick_border,
                            egui::StrokeKind::Inside,
                        );
                        paint_pin_icon(
                            ui.painter(),
                            Pos2::new(pick_rect.min.x + 11.0, pick_rect.center().y),
                            5.0,
                            if picking { accent_text } else { p.text_muted },
                        );
                        let txt_col = if picking { accent_text } else { p.text_main };
                        ui.painter().text(
                            Pos2::new(pick_rect.min.x + 21.0, pick_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            truncate_text(pick_txt, 18),
                            egui::FontId::proportional(10.8),
                            txt_col,
                        );
                        if pick_resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if pick_resp.clicked() {
                            *on_pick = true;
                        }

                        let dv = ui.add_sized(
                            [52.0, 24.0],
                            egui::DragValue::new(height_val)
                                .speed(0.5)
                                .fixed_decimals(1),
                        );
                        if dv.changed() {
                            *h_changed = true;
                        }
                        ui.label(RichText::new("mm").size(10.5).color(p.text_muted));
                    });
                };

                let mut clicked_pick_a = false;
                let mut start_h_changed = false;
                draw_endpoint_row(
                    ui,
                    "A",
                    "Start",
                    Color32::from_rgb(4, 120, 87),
                    p.text_success,
                    &label_a,
                    app.ramp_sel.pick_mode == Some(true),
                    &mut app.ramp_start_h,
                    &mut clicked_pick_a,
                    &mut start_h_changed,
                );
                if clicked_pick_a {
                    app.ramp_sel.pick_mode = if app.ramp_sel.pick_mode == Some(true) {
                        None
                    } else {
                        Some(true)
                    };
                }
                if start_h_changed {
                    app.ramp_target_warning = None;
                }

                // Center Swap & Connection Status Row
                ui.horizontal(|ui| {
                    let row_w = ui.available_width();
                    let (row_rect, _) =
                        ui.allocate_exact_size(Vec2::new(row_w, 22.0), Sense::hover());
                    let swap_rect = Rect::from_center_size(
                        row_rect.center(),
                        Vec2::new(22.0, 22.0),
                    );
                    let swap_resp = ui.interact(
                        swap_rect,
                        ui.make_persistent_id("hp_swap_btn"),
                        Sense::click(),
                    );
                    let can_swap = chain.len() >= 2
                        || app.ramp_sel.start_piece.is_some()
                        || app.ramp_sel.end_piece.is_some();
                    let swap_bg = if swap_resp.hovered() && can_swap {
                        p.primary
                    } else {
                        p.bg_card
                    };
                    let swap_fg = if swap_resp.hovered() && can_swap {
                        Color32::WHITE
                    } else {
                        p.text_muted
                    };
                    ui.painter().circle(
                        swap_rect.center(),
                        10.5,
                        swap_bg,
                        Stroke::new(1.0_f32, p.border_main),
                    );
                    paint_swap_arrows_icon(ui.painter(), swap_rect.center(), swap_fg);
                    if swap_resp.hovered() && can_swap {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if swap_resp.clicked() && can_swap {
                        if app.ramp_sel.start_piece.is_some()
                            || app.ramp_sel.end_piece.is_some()
                        {
                            std::mem::swap(
                                &mut app.ramp_sel.start_piece,
                                &mut app.ramp_sel.end_piece,
                            );
                        } else if chain.len() >= 2 {
                            app.ramp_sel.start_piece =
                                chain.last().map(|(id, _)| id.clone());
                            app.ramp_sel.end_piece =
                                chain.first().map(|(id, _)| id.clone());
                        }
                        std::mem::swap(&mut app.ramp_start_h, &mut app.ramp_end_h);
                        app.ramp_target_warning = None;
                    }

                    let is_cont = summary.as_ref().is_none_or(|s| s.is_continuous);
                    let (conn_txt, conn_col) = if is_cont {
                        (
                            if is_de { "Durchgehend" } else { "Continuous" },
                            p.text_success,
                        )
                    } else {
                        (
                            if is_de { "Unterbrochen" } else { "Split" },
                            p.text_warning,
                        )
                    };
                    ui.painter().text(
                        Pos2::new(row_rect.max.x - 2.0, row_rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        conn_txt,
                        egui::FontId::proportional(10.5),
                        conn_col,
                    );
                });

                let mut clicked_pick_b = false;
                let mut end_h_changed = false;
                draw_endpoint_row(
                    ui,
                    "B",
                    if is_de { "Ende" } else { "End" },
                    Color32::from_rgb(161, 72, 6),
                    p.text_warning,
                    &label_b,
                    app.ramp_sel.pick_mode == Some(false),
                    &mut app.ramp_end_h,
                    &mut clicked_pick_b,
                    &mut end_h_changed,
                );
                if clicked_pick_b {
                    app.ramp_sel.pick_mode = if app.ramp_sel.pick_mode == Some(false) {
                        None
                    } else {
                        Some(false)
                    };
                }
                if end_h_changed {
                    app.ramp_target_warning = None;
                }
            });

        // Maximum Slope + Auto-Fit
        let mut trigger_autofit = false;
        ui.horizontal(|ui| {
            section_label(
                ui,
                p,
                if is_de {
                    "Maximale Steigung"
                } else {
                    "Maximum slope"
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.checkbox(&mut app.ramp_auto_fit, "Auto-Fit").changed()
                    && app.ramp_auto_fit
                {
                    trigger_autofit = true;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let pill_w = ((ui.available_width() - 78.0) / 3.0).max(52.0);
            for preset in [2.0_f64, 2.5, 3.0] {
                let active = (app.ramp_max_grade_limit - preset).abs() < 0.05;
                if pill_button(
                    ui,
                    p,
                    app.dark_mode,
                    active,
                    pill_w,
                    26.0,
                    &format!("{preset:.1}%"),
                )
                .clicked()
                {
                    app.ramp_max_grade_limit = preset;
                    if app.ramp_auto_fit {
                        trigger_autofit = true;
                    }
                }
            }
            let dv = ui.add_sized(
                [58.0, 26.0],
                egui::DragValue::new(&mut app.ramp_max_grade_limit)
                    .range(0.5..=12.0)
                    .speed(0.1)
                    .fixed_decimals(1)
                    .suffix(" %"),
            );
            if dv.changed() && app.ramp_auto_fit {
                trigger_autofit = true;
            }
        });

        if trigger_autofit && total_len_mm > 1.0 {
            let clamped = compute_auto_fit_end_height(
                app.ramp_start_h,
                app.ramp_end_h,
                total_len_mm,
                app.ramp_max_grade_limit,
            );
            if (clamped - app.ramp_end_h).abs() > 0.05 {
                app.ramp_end_h = clamped;
            }
            app.ramp_target_warning = None;
        }

        // Slope Profile 2-way segmented bar
        section_label(
            ui,
            p,
            if is_de {
                "Steigungsprofil"
            } else {
                "Slope profile"
            },
        );
        let profiles = [
            (
                RampProfile::Linear,
                if is_de {
                    "Linear (Gleichmäßig)"
                } else {
                    "Linear (uniform)"
                },
            ),
            (
                RampProfile::Eased,
                if is_de {
                    "S-Kurve (Ausgerundet)"
                } else {
                    "S-curve (eased)"
                },
            ),
        ];
        segmented_control(
            ui,
            p,
            app.dark_mode,
            &mut app.ramp_profile,
            &profiles,
            25.0,
            11.0,
        );

        // Target Level for Tracks
        section_label(
            ui,
            p,
            if is_de {
                "Zielebene für Gleise"
            } else {
                "Target level for tracks"
            },
        );
        let cur_tl_choice = app
            .ramp_target_layer_id
            .as_deref()
            .filter(|id| layers_list.iter().any(|(lid, _, _)| lid == id))
            .map(|s| s.to_string())
            .unwrap_or_default();
        let cur_tl_lbl = if cur_tl_choice.is_empty() {
            if is_de { "Unverändert" } else { "Unchanged" }.into()
        } else {
            layers_list
                .iter()
                .find(|(lid, _, _)| *lid == cur_tl_choice)
                .map(|(_, n, lh)| format!("{n} ({lh:.0} mm)"))
                .unwrap_or_else(|| "-".into())
        };
        let mut next_tl_choice = cur_tl_choice.clone();
        egui::ComboBox::from_id_salt("hp_slope_target_layer_cb")
            .width(ui.available_width())
            .selected_text(cur_tl_lbl)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut next_tl_choice,
                    String::new(),
                    if is_de { "Unverändert" } else { "Unchanged" },
                );
                for (lid, lname, lh) in layers_list {
                    ui.selectable_value(
                        &mut next_tl_choice,
                        lid.clone(),
                        format!("{lname} ({lh:.0} mm)"),
                    );
                }
                ui.separator();
                ui.selectable_value(
                    &mut next_tl_choice,
                    LAYER_NEW_SENTINEL.into(),
                    if is_de {
                        "+ Neue Ebene erstellen..."
                    } else {
                        "+ Create new layer..."
                    },
                );
            });
        if next_tl_choice == LAYER_NEW_SENTINEL {
            app.open_layer_dialog_new();
        } else if next_tl_choice != cur_tl_choice {
            if next_tl_choice.is_empty() {
                app.ramp_target_layer_id = None;
                app.ramp_target_warning = None;
            } else if let Some((_, _, lh)) =
                layers_list.iter().find(|(lid, _, _)| *lid == next_tl_choice)
            {
                app.ramp_target_layer_id = Some(next_tl_choice);
                let target_h = *lh;
                if app.ramp_auto_fit && total_len_mm > 1.0 {
                    let clamped = compute_auto_fit_end_height(
                        app.ramp_start_h,
                        target_h,
                        total_len_mm,
                        app.ramp_max_grade_limit,
                    );
                    app.ramp_end_h = clamped;
                    if (clamped - target_h).abs() > 0.2 {
                        app.ramp_target_warning = Some((
                            format!(
                                "Gleislänge zu kurz für Zielhöhe {target_h:.0} mm (begrenzt auf {clamped:.1} mm)"
                            ),
                            format!(
                                "Track length too short for target height {target_h:.0} mm (capped at {clamped:.1} mm)"
                            ),
                        ));
                    } else {
                        app.ramp_target_warning = None;
                    }
                } else {
                    app.ramp_end_h = target_h;
                    app.ramp_target_warning = None;
                }
            }
        }

        // Colored Slope Status Box
        if let Some(ref sum) = summary {
            let eff = sum.effective_max_pct;
            let kind = if eff > 4.5 + 0.05 {
                StatusKind::Danger
            } else if eff > 3.0 + 0.05
                || app.ramp_target_warning.is_some()
                || sum.has_s_curve
            {
                StatusKind::Warn
            } else {
                StatusKind::Ok
            };
            let headline_prefix = match (kind, is_de) {
                (StatusKind::Ok, true) => "Sichere Steigung",
                (StatusKind::Ok, false) => "Safe Slope",
                (StatusKind::Warn, true) => "Mittlere Steigung",
                (StatusKind::Warn, false) => "Moderate Slope",
                (StatusKind::Danger, true) => "Steile Rampe",
                (StatusKind::Danger, false) => "Steep Ramp",
            };
            let title = format!(
                "{headline_prefix}: {:.1}% (Δ {:.0} mm)",
                sum.max_gradient_pct,
                sum.total_rise_mm.abs()
            );
            let desc = if eff > 4.5 + 0.05 {
                if is_de {
                    "Gefahr von Durchrutschen oder Zugtrennung! Länge erhöhen."
                } else {
                    "Risk of slipping or uncoupling! Increase length."
                }
            } else if eff > 3.0 + 0.05 {
                if is_de {
                    "Geeignet für kurze bis mittlere Züge."
                } else {
                    "Suitable for short to medium trains."
                }
            } else if is_de {
                "Geeignet für alle Zuglängen."
            } else {
                "Safe for all train formations."
            };
            let mut extra = app
                .ramp_target_warning
                .as_ref()
                .map(|(de, en)| if is_de { de.clone() } else { en.clone() });
            if sum.has_s_curve {
                let s_msg = if is_de {
                    format!("Gegenbogen/S-Kurve: eff. Reibung {:.1}% (1.5x)", sum.effective_max_pct)
                } else {
                    format!("S-curve detected: eff. grade {:.1}% (1.5x)", sum.effective_max_pct)
                };
                extra = Some(match extra {
                    Some(prev) => format!("{prev} · {s_msg}"),
                    None => s_msg,
                });
            }
            draw_status_card(
                ui,
                app.dark_mode,
                kind,
                &title,
                desc,
                extra.as_deref(),
            );

            // Collapsible Track Details (N tracks)
            let rows = build_ramp_breakdown(
                &app.layout,
                &app.catalogs,
                &chain,
                app.ramp_start_h,
                app.ramp_end_h,
                app.ramp_profile,
            );
            if !rows.is_empty() {
                let details_id = ui.make_persistent_id("hp_track_details_open");
                let mut details_open = ui
                    .ctx()
                    .data(|d| d.get_temp::<bool>(details_id))
                    .unwrap_or(false);
                let hdr_resp = ui.horizontal(|ui| {
                    let (chev_r, _) =
                        ui.allocate_exact_size(Vec2::new(12.0, 16.0), Sense::hover());
                    paint_chevron(
                        ui.painter(),
                        chev_r.center(),
                        if details_open { ChevronDir::Down } else { ChevronDir::Right },
                        4.0,
                        p.text_muted,
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(if is_de {
                                format!("Gleisdetails ({} Gleise)", rows.len())
                            } else {
                                format!("Track Details ({} tracks)", rows.len())
                            })
                            .size(11.0)
                            .color(p.text_muted),
                        )
                        .sense(Sense::click()),
                    )
                });
                if hdr_resp.response.interact(Sense::click()).clicked()
                    || hdr_resp.inner.clicked()
                {
                    details_open = !details_open;
                    ui.ctx().data_mut(|d| d.insert_temp(details_id, details_open));
                }
                if details_open {
                    Frame::new()
                        .fill(p.bg_input)
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::same(4))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(110.0)
                                .show(ui, |ui| {
                                    for (idx, r) in rows.iter().enumerate() {
                                        let row_bg = if idx % 2 == 1 {
                                            if app.dark_mode {
                                                Color32::from_rgba_unmultiplied(255, 255, 255, 10)
                                            } else {
                                                Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                                            }
                                        } else {
                                            Color32::TRANSPARENT
                                        };
                                        Frame::new()
                                            .fill(row_bg)
                                            .corner_radius(CornerRadius::same(4))
                                            .inner_margin(Margin::symmetric(6, 2))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{}. #{} ({:.0} mm)",
                                                            idx + 1,
                                                            r.catalog_id,
                                                            r.length_mm
                                                        ))
                                                        .size(10.5)
                                                        .color(p.text_main),
                                                    )
                                                    .on_hover_text(format!("ID: {}", r.piece_id));
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(egui::Align::Center),
                                                        |ui| {
                                                            let gtxt = if r.is_s_curve {
                                                                format!(
                                                                    "{:+.0} -> {:+.0} mm ({:.1}% S)",
                                                                    r.start_h_mm,
                                                                    r.end_h_mm,
                                                                    r.effective_grade_pct
                                                                )
                                                            } else {
                                                                format!(
                                                                    "{:+.0} -> {:+.0} mm ({:.1}%)",
                                                                    r.start_h_mm,
                                                                    r.end_h_mm,
                                                                    r.grade_pct
                                                                )
                                                            };
                                                            ui.label(
                                                                RichText::new(gtxt)
                                                                    .size(10.5)
                                                                    .color(p.text_sub),
                                                            );
                                                        },
                                                    );
                                                });
                                            });
                                    }
                                });
                        });
                }
            }
        } else if app.ramp_sel.start_piece.is_some()
            && app.ramp_sel.end_piece.is_some()
        {
            draw_status_card(
                ui,
                app.dark_mode,
                StatusKind::Danger,
                if is_de {
                    "Kein verbundener Gleisweg"
                } else {
                    "No connected track path"
                },
                if is_de {
                    "Zwischen Start (A) und Ende (B) besteht keine Gleisverbindung."
                } else {
                    "No connected path found between Start (A) and End (B)."
                },
                None,
            );
        } else {
            draw_status_card(
                ui,
                app.dark_mode,
                StatusKind::Warn,
                if is_de {
                    "Gleiskette festlegen"
                } else {
                    "Select ramp tracks"
                },
                if is_de {
                    "Startpunkt (A) & Endpunkt (B) im Plan wählen oder Gleiskette markieren."
                } else {
                    "Pick Start (A) & End (B) tracks on canvas, or select a track chain."
                },
                None,
            );
        }

        *slope_chain_for_footer = chain;
}
