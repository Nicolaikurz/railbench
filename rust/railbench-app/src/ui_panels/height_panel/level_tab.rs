use super::*;

#[allow(clippy::ptr_arg)]
pub(super) fn draw_level_tab(
    ui: &mut egui::Ui,
    app: &mut RailbenchApp,
    p: RailbenchPalette,
    is_de: bool,
    nem_h: f64,
    layers_list: &[(String, String, f64)],
    flat_apply_params: &mut Option<(String, bool, f64, f64)>,
) {
        // TAB 0: HEIGHT / LEVEL
        let sel_count = app.selected.len();
        let active_layer_name = app
            .selected
            .iter()
            .next()
            .and_then(|sid| app.layout.pieces.iter().find(|pc| &pc.id == sid))
            .and_then(|pc| pc.layer_id.as_deref())
            .or(app.layout.active_layer_id.as_deref())
            .and_then(|lid| layers_list.iter().find(|(id, _, _)| id == lid))
            .map(|(_, name, _)| name.clone())
            .unwrap_or_else(|| {
                layers_list
                    .first()
                    .map(|(_, n, _)| n.clone())
                    .unwrap_or_else(|| "-".into())
            });

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if is_de {
                    format!("{sel_count} Gleise")
                } else {
                    format!("{sel_count} tracks")
                })
                .size(11.0)
                .color(p.text_muted),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(&active_layer_name).size(11.0).strong().color(p.text_sub));
            });
        });

        section_label(ui, p, if is_de { "Zielebene" } else { "Target level" });
        let chosen_flat_id = app
            .height_flat_layer_id
            .as_deref()
            .filter(|id| layers_list.iter().any(|(lid, _, _)| lid == id))
            .map(|s| s.to_string())
            .unwrap_or_else(|| layers_list[0].0.clone());
        app.height_flat_layer_id = Some(chosen_flat_id.clone());

        let chosen_flat_meta = layers_list
            .iter()
            .find(|(id, _, _)| *id == chosen_flat_id)
            .cloned()
            .unwrap_or_else(|| layers_list[0].clone());

        let mut flat_choice = chosen_flat_id.clone();
        egui::ComboBox::from_id_salt("hp_flat_layer_cb")
            .width(ui.available_width())
            .selected_text(format!(
                "{} ({:.0} mm)",
                chosen_flat_meta.1, chosen_flat_meta.2
            ))
            .show_ui(ui, |ui| {
                for (lid, lname, lh) in layers_list {
                    ui.selectable_value(
                        &mut flat_choice,
                        lid.clone(),
                        format!("{lname} ({lh:.0} mm)"),
                    );
                }
                ui.separator();
                ui.selectable_value(
                    &mut flat_choice,
                    LAYER_NEW_SENTINEL.into(),
                    if is_de {
                        "+ Neue Ebene erstellen..."
                    } else {
                        "+ Create new layer..."
                    },
                );
            });
        if flat_choice == LAYER_NEW_SENTINEL {
            app.open_layer_dialog_new();
        } else if flat_choice != chosen_flat_id {
            app.height_flat_layer_id = Some(flat_choice);
        }

        section_label(ui, p, if is_de { "Höhen-Modus" } else { "Height mode" });
        let modes = [
            (false, if is_de { "Ebenenhöhe" } else { "Level height" }),
            (true, if is_de { "Eigener Wert" } else { "Custom value" }),
        ];
        segmented_control(
            ui,
            p,
            app.dark_mode,
            &mut app.height_flat_use_custom,
            &modes,
            24.0,
            11.2,
        );

        if app.height_flat_use_custom {
            section_label(
                ui,
                p,
                if is_de {
                    "Absolute Höhe (mm)"
                } else {
                    "Absolute height (mm)"
                },
            );
            ui.horizontal(|ui| {
                ui.add_sized(
                    [130.0, 26.0],
                    egui::DragValue::new(&mut app.batch_elevation_mm)
                        .speed(0.5)
                        .fixed_decimals(1)
                        .suffix(" mm"),
                );
                if pill_button(
                    ui,
                    p,
                    app.dark_mode,
                    app.batch_elevation_mm.abs() < 0.1,
                    52.0,
                    24.0,
                    "0 mm",
                )
                .clicked()
                {
                    app.batch_elevation_mm = 0.0;
                }
                if pill_button(
                    ui,
                    p,
                    app.dark_mode,
                    (app.batch_elevation_mm - nem_h).abs() < 0.1,
                    62.0,
                    24.0,
                    &format!("+{nem_h:.0}"),
                )
                .clicked()
                {
                    app.batch_elevation_mm = nem_h;
                }
                if pill_button(
                    ui,
                    p,
                    app.dark_mode,
                    (app.batch_elevation_mm + nem_h).abs() < 0.1,
                    62.0,
                    24.0,
                    &format!("-{nem_h:.0}"),
                )
                .clicked()
                {
                    app.batch_elevation_mm = -nem_h;
                }
            });
        }

        if sel_count == 0 {
            draw_status_card(
                ui,
                app.dark_mode,
                StatusKind::Warn,
                if is_de {
                    "Keine Gleise ausgewählt"
                } else {
                    "No tracks selected"
                },
                if is_de {
                    "Markiere ein oder mehrere Gleise im Plan, um deren Höhe festzulegen."
                } else {
                    "Select one or more tracks on the canvas to set their height."
                },
                None,
            );
        } else if !app.height_flat_use_custom {
            let title = format!("{} ({:.0} mm)", chosen_flat_meta.1, chosen_flat_meta.2);
            let desc = if is_de {
                format!(
                    "Gleise werden auf Ebene '{}' ({:.0} mm) gesetzt.",
                    chosen_flat_meta.1, chosen_flat_meta.2
                )
            } else {
                format!(
                    "Tracks will be set to level '{}' ({:.0} mm).",
                    chosen_flat_meta.1, chosen_flat_meta.2
                )
            };
            draw_status_card(ui, app.dark_mode, StatusKind::Ok, &title, &desc, None);
        } else {
            let abs_h = app.batch_elevation_mm;
            let local_off = abs_h - chosen_flat_meta.2;
            let title = if is_de {
                format!("Feste Höhe: {abs_h:.1} mm")
            } else {
                format!("Fixed height: {abs_h:.1} mm")
            };
            let desc = if is_de {
                format!(
                    "Gleise werden auf Ebene '{}' mit Offset {:+.1} mm gesetzt.",
                    chosen_flat_meta.1, local_off
                )
            } else {
                format!(
                    "Tracks will be set on '{}' with {:+.1} mm offset.",
                    chosen_flat_meta.1, local_off
                )
            };
            draw_status_card(ui, app.dark_mode, StatusKind::Ok, &title, &desc, None);
        }

        if sel_count > 0 {
            *flat_apply_params = Some((
                chosen_flat_meta.0.clone(),
                app.height_flat_use_custom,
                app.batch_elevation_mm,
                chosen_flat_meta.2,
            ));
        }
}
