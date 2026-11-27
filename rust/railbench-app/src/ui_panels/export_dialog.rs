use super::*;
use crate::board_tools::{get_baseboard_polygon, get_baseboards_slice};
use crate::export::{get_piece_list_summary, layout_bounds, sample_track_branches_world};

fn parse_layer_swatch(hex: Option<&str>) -> Color32 {
    let Some(raw) = hex else {
        return Color32::from_rgb(56, 189, 248);
    };
    let h = raw.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    Color32::from_rgb(56, 189, 248)
}

fn draw_mini_plan_preview(
    ui: &mut egui::Ui,
    app: &RailbenchApp,
    filter_layer_id: Option<&str>,
    aspect_w_over_h: f32,
) {
    let avail_w = ui.available_width().clamp(160.0, 360.0);
    let preview_h = (avail_w / aspect_w_over_h).clamp(110.0, 260.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, preview_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Crisp white paper background
    painter.rect_filled(rect, CornerRadius::same(4), Color32::WHITE);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0_f32, Color32::from_rgb(203, 213, 225)),
        egui::StrokeKind::Inside,
    );

    let (min_x, max_x, min_y, max_y) = layout_bounds(&app.layout, &app.catalogs);
    let span_x = (max_x - min_x).max(100.0);
    let span_y = (max_y - min_y).max(100.0);
    let pad = 10.0_f32;
    let sx = (rect.width() - pad * 2.0) as f64 / span_x;
    let sy = (rect.height() - pad * 2.0) as f64 / span_y;
    let scale = sx.min(sy);
    let ox = rect.min.x as f64 + (rect.width() as f64 - span_x * scale) * 0.5;
    let oy = rect.min.y as f64 + (rect.height() as f64 - span_y * scale) * 0.5;

    let to_scr = |wx: f64, wy: f64| -> egui::Pos2 {
        egui::pos2(
            (ox + (wx - min_x) * scale) as f32,
            (oy + (wy - min_y) * scale) as f32,
        )
    };

    if app.export_opts.show_grid {
        let grid_col = Color32::from_rgba_unmultiplied(148, 163, 184, 70);
        let sx = (min_x / 500.0).ceil() as i64;
        let ex = (max_x / 500.0).floor() as i64;
        let sy = (min_y / 500.0).ceil() as i64;
        let ey = (max_y / 500.0).floor() as i64;
        if (ex - sx) <= 40 && (ey - sy) <= 40 {
            for ix in sx..=ex {
                let a = to_scr(ix as f64 * 500.0, min_y);
                let b = to_scr(ix as f64 * 500.0, max_y);
                painter.line_segment([a, b], Stroke::new(0.6_f32, grid_col));
            }
            for iy in sy..=ey {
                let a = to_scr(min_x, iy as f64 * 500.0);
                let b = to_scr(max_x, iy as f64 * 500.0);
                painter.line_segment([a, b], Stroke::new(0.6_f32, grid_col));
            }
        }
    }

    if app.export_opts.show_baseboards {
        for bb in get_baseboards_slice(&app.layout) {
            if !bb.enabled {
                continue;
            }
            let poly = get_baseboard_polygon(&bb);
            if poly.len() >= 3 {
                let pts: Vec<egui::Pos2> = poly.iter().map(|p| to_scr(p.x, p.y)).collect();
                painter.add(egui::Shape::closed_line(
                    pts,
                    Stroke::new(1.0_f32, Color32::from_rgb(148, 163, 184)),
                ));
            }
        }
    }

    // Soft mountain & ground-terrain topographic underlay BEFORE tracks so tracks underneath stay visible
    if let Some(models) = &app.layout.models {
        for m in models {
            let m_lid = m.layer_id.as_deref().unwrap_or("layer-1");
            if let Some(flid) = filter_layer_id {
                if flid != m_lid {
                    continue;
                }
            }
            let is_mtn = m.builtin_id.as_deref() == Some("mountain");
            let is_gnd = m.builtin_id.as_deref() == Some("ground-terrain");
            if !is_mtn && !is_gnd {
                continue;
            }
            let fill = if app.export_opts.monochrome {
                Color32::from_rgba_unmultiplied(148, 163, 184, 42)
            } else if is_mtn {
                Color32::from_rgba_unmultiplied(34, 139, 64, 34)
            } else {
                Color32::from_rgba_unmultiplied(101, 163, 13, 30)
            };
            let stroke = if app.export_opts.monochrome {
                Stroke::new(0.8_f32, Color32::from_rgb(100, 116, 139))
            } else {
                Stroke::new(0.8_f32, Color32::from_rgb(74, 135, 80))
            };
            if let Some(nodes) = &m.mountain_path {
                if !nodes.is_empty() {
                    for nd in nodes {
                        let rp = railbench_core::geometry::rotate(
                            railbench_core::types::Vec2 { x: nd.x, y: nd.y },
                            m.transform.rotation,
                        );
                        let c = to_scr(m.transform.x + rp.x, m.transform.y + rp.y);
                        let r_scr = ((nd.radius.max(10.0) * scale) as f32).max(2.5);
                        painter.circle_filled(c, r_scr, fill);
                        if is_mtn {
                            painter.circle_stroke(c, r_scr, stroke);
                        }
                    }
                    continue;
                }
            }
            let sc = m.scale.unwrap_or(1.0);
            let rx = (m.width * sc * 0.5).max(12.0);
            let ry = (m.depth * sc * 0.5).max(12.0);
            let pts: Vec<egui::Pos2> = (0..24)
                .map(|k| {
                    let a = (k as f64 / 24.0) * std::f64::consts::TAU;
                    let rp = railbench_core::geometry::rotate(
                        railbench_core::types::Vec2 { x: rx * a.cos(), y: ry * a.sin() },
                        m.transform.rotation,
                    );
                    to_scr(m.transform.x + rp.x, m.transform.y + rp.y)
                })
                .collect();
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
            if is_mtn {
                painter.add(egui::Shape::closed_line(pts, stroke));
            }
        }
    }

    let draw_pass = |ghost: bool| {
        for p in &app.layout.pieces {
            let p_lid = p.layer_id.as_deref().unwrap_or("layer-1");
            let matches = filter_layer_id.map_or(true, |lid| lid == p_lid);
            if ghost && matches {
                continue;
            }
            if !ghost && !matches {
                continue;
            }
            let entry = crate::catalog::find_entry(&app.catalogs, &p.catalog_id);
            let kind = entry.map(|e| e.kind.as_str()).unwrap_or("");
            let is_road = crate::catalog::is_road_id(&app.catalogs, &p.catalog_id);
            let col = if ghost {
                Color32::from_rgb(203, 213, 225)
            } else if app.export_opts.monochrome {
                Color32::from_rgb(30, 41, 59)
            } else if is_road {
                Color32::from_rgb(71, 85, 105)
            } else {
                match kind {
                    "curve" => Color32::from_rgb(37, 99, 235),
                    "turnout-left" | "turnout-right" | "curved-turnout-left" | "curved-turnout-right" | "three-way" | "double-slip" => {
                        Color32::from_rgb(217, 119, 6)
                    }
                    "flex" => Color32::from_rgb(8, 145, 178),
                    _ => Color32::from_rgb(2, 132, 199),
                }
            };
            let w = if ghost { 1.0_f32 } else { 1.8_f32 };
            for br in sample_track_branches_world(p, entry) {
                if br.len() < 2 {
                    continue;
                }
                let pts: Vec<egui::Pos2> = br.iter().map(|pt| to_scr(pt.x, pt.y)).collect();
                painter.add(egui::Shape::line(pts, Stroke::new(w, col)));
            }
        }
    };

    if filter_layer_id.is_some() && app.export_opts.ghost_other_layers {
        draw_pass(true);
    }
    draw_pass(false);

    if app.export_opts.show_labels && app.layout.pieces.len() <= 160 {
        let lbl_col = if app.export_opts.monochrome {
            Color32::from_rgb(30, 41, 59)
        } else {
            Color32::from_rgb(15, 23, 42)
        };
        for p in &app.layout.pieces {
            let p_lid = p.layer_id.as_deref().unwrap_or("layer-1");
            if let Some(flid) = filter_layer_id {
                if flid != p_lid {
                    continue;
                }
            }
            let pos = to_scr(p.transform.x, p.transform.y);
            painter.text(
                pos + egui::vec2(0.0, -5.0),
                egui::Align2::CENTER_CENTER,
                &p.catalog_id,
                egui::FontId::proportional(7.5),
                lbl_col,
            );
        }
    }

    // Real 2D top-view projected buildings in live Print Preview (toggleable via show_models)
    if app.export_opts.show_models {
        if let Some(models) = &app.layout.models {
            for (mi, m) in models.iter().enumerate() {
                let m_lid = m.layer_id.as_deref().unwrap_or("layer-1");
                if let Some(flid) = filter_layer_id {
                    if flid != m_lid {
                        continue;
                    }
                }
                if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                    continue;
                }
                let mesh_opt = app.canvas_2d_cache.model_top_mesh(mi);
                let owned_mesh;
                let mesh_ref = if let Some(cached) = mesh_opt {
                    cached
                } else {
                    owned_mesh = crate::canvas2d::build_model_top_view_mesh_2d(m);
                    &owned_mesh
                };
                if !mesh_ref.tris.is_empty() {
                    let rad = m.transform.rotation.to_radians();
                    let (sin_r, cos_r) = rad.sin_cos();
                    let mut egui_mesh = egui::Mesh::default();
                    egui_mesh.vertices.reserve(mesh_ref.tris.len() * 3);
                    egui_mesh.indices.reserve(mesh_ref.tris.len() * 3);
                    for (pts, cols) in &mesh_ref.tris {
                        let base = egui_mesh.vertices.len() as u32;
                        for k in 0..3 {
                            let lx = pts[k][0] as f64;
                            let ly = pts[k][1] as f64;
                            let wx = m.transform.x + lx * cos_r - ly * sin_r;
                            let wy = m.transform.y + lx * sin_r + ly * cos_r;
                            let sp = to_scr(wx, wy);
                            let c = if app.export_opts.monochrome {
                                let lum = ((cols[k].r() as u16 * 77
                                    + cols[k].g() as u16 * 150
                                    + cols[k].b() as u16 * 29)
                                    >> 8) as u8;
                                Color32::from_rgb(lum, lum, lum)
                            } else {
                                cols[k]
                            };
                            egui_mesh.vertices.push(egui::epaint::Vertex {
                                pos: sp,
                                uv: egui::epaint::WHITE_UV,
                                color: c,
                            });
                        }
                        egui_mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
                    }
                    painter.add(egui::Shape::mesh(egui_mesh));
                }
            }
        }
    }
}

pub(super) fn draw_export_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_export_dialog;
    let mut trigger: Option<u8> = None;

    let title = if is_de {
        "Bauplan-Export & PDF Druckvorschau"
    } else {
        "Blueprint Export & PDF Print Preview"
    };
    egui::Window::new(title)
        .id(egui::Id::new("railbench_export_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_size([740.0, 520.0])
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, title, Some(&mut open), 12, |ui| {
        // Compute selected layers in elevation order (matching Electron print.ts)
        let all_layers: Vec<(String, String, f64, Option<String>, usize)> = app
            .layout
            .layers
            .as_ref()
            .map(|ls| {
                ls.iter()
                    .map(|l| {
                        let cnt = app
                            .layout
                            .pieces
                            .iter()
                            .filter(|pc| pc.layer_id.as_deref().unwrap_or("layer-1") == l.id)
                            .count();
                        (l.id.clone(), l.name.clone(), l.height_mm.unwrap_or(0.0), l.color.clone(), cnt)
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![(
                    "layer-1".into(),
                    if is_de { "Ebene 1" } else { "Layer 1" }.into(),
                    0.0,
                    None,
                    app.layout.pieces.len(),
                )]
            });

        let active_layer_pages: Vec<_> = if app.export_opts.include_layer_pages {
            let mut v: Vec<_> = all_layers
                .iter()
                .filter(|(id, _, _, _, _)| {
                    app.export_opts.selected_layer_ids.is_empty()
                        || app.export_opts.selected_layer_ids.contains(id)
                })
                .cloned()
                .collect();
            v.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            v
        } else {
            Vec::new()
        };

        let total_pages = (app.export_opts.include_overview as usize)
            + active_layer_pages.len()
            + (app.export_opts.include_bom_page as usize);

        ui.columns(2, |cols| {
            // LEFT COLUMN: Configuration Pane (matching Electron print-config-pane)
            egui::ScrollArea::vertical().id_salt("export_cfg_scroll").show(&mut cols[0], |ui| {
                ui.label(RichText::new(if is_de { "Ebenen auswaehlen" } else { "Layers to Print" }).strong());
                ui.checkbox(
                    &mut app.export_opts.include_layer_pages,
                    if is_de { "Separate Seiten je Ebene" } else { "Separate Page per Layer" },
                );
                if app.export_opts.include_layer_pages {
                    Frame::new()
                        .fill(p.bg_card)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::same(6))
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for (lid, lname, lh, lcol, cnt) in &all_layers {
                                let all_mode = app.export_opts.selected_layer_ids.is_empty();
                                let mut checked = all_mode || app.export_opts.selected_layer_ids.contains(lid);
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut checked, "").changed() {
                                        if all_mode {
                                            app.export_opts.selected_layer_ids = all_layers
                                                .iter()
                                                .filter(|(id, _, _, _, _)| id != lid)
                                                .map(|(id, _, _, _, _)| id.clone())
                                                .collect();
                                        } else if checked {
                                            app.export_opts.selected_layer_ids.push(lid.clone());
                                            if app.export_opts.selected_layer_ids.len() >= all_layers.len() {
                                                app.export_opts.selected_layer_ids.clear();
                                            }
                                        } else {
                                            app.export_opts.selected_layer_ids.retain(|id| id != lid);
                                        }
                                    }
                                    let (dot_r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                                    ui.painter().circle_filled(dot_r.center(), 4.5, parse_layer_swatch(lcol.as_deref()));
                                    ui.label(RichText::new(lname).strong().size(12.0));
                                    let h_sign = if *lh > 0.0 { format!("+{lh:.0}") } else { format!("{lh:.0}") };
                                    ui.label(
                                        RichText::new(format!("{h_sign} mm · {cnt}"))
                                            .size(11.0)
                                            .color(p.text_muted),
                                    );
                                });
                            }
                        });
                }

                ui.add_space(8.0);
                ui.label(RichText::new(if is_de { "Zusaetzliche Seiten" } else { "Additional Pages" }).strong());
                ui.checkbox(
                    &mut app.export_opts.include_overview,
                    if is_de { "Gesamtuebersicht einschliessen" } else { "Include Full Overview Page" },
                );
                ui.checkbox(
                    &mut app.export_opts.include_bom_page,
                    if is_de { "Teileliste anhängen" } else { "Include Part List" },
                );

                ui.add_space(8.0);
                ui.label(RichText::new(if is_de { "Darstellung (PDF & SVG)" } else { "Visual Options" }).strong());
                ui.checkbox(
                    &mut app.export_opts.ghost_other_layers,
                    if is_de { "Andere Ebenen ausgegraut hinterlegen" } else { "Ghost Other Layers" },
                );
                ui.checkbox(
                    &mut app.export_opts.show_labels,
                    if is_de { "Gleis-Artikelnummern beschriften" } else { "Show Track Part Numbers" },
                );
                ui.checkbox(
                    &mut app.export_opts.show_baseboards,
                    if is_de { "Grundplatten / Tische zeichnen" } else { "Show Baseboards / Tables" },
                );
                ui.checkbox(
                    &mut app.export_opts.show_models,
                    if is_de { "Gebaeude & 3D-Modelle zeichnen" } else { "Show Buildings & 3D Models" },
                );
                ui.checkbox(
                    &mut app.export_opts.show_grid,
                    if is_de { "CAD-Raster (100 / 500 mm) anzeigen" } else { "Show CAD Grid (100 / 500 mm)" },
                );
                ui.checkbox(
                    &mut app.export_opts.show_electrical,
                    if is_de { "Elektrik- & Hoehen-Markierungen" } else { "Show Electrical & Elevation Markers" },
                );
                ui.checkbox(
                    &mut app.export_opts.monochrome,
                    if is_de { "Schwarz-Weiss-Bauplan (Monochrom)" } else { "Black & White (Monochrome)" },
                );

                ui.add_space(8.0);
                ui.label(RichText::new(if is_de { "Ausrichtung" } else { "Orientation" }).strong());
                ui.horizontal(|ui| {
                    ui.radio_value(&mut app.export_opts.landscape, true, if is_de { "A4 Querformat" } else { "A4 Landscape" });
                    ui.radio_value(&mut app.export_opts.landscape, false, if is_de { "A4 Hochformat" } else { "A4 Portrait" });
                });
            });

            // RIGHT COLUMN: Live Multi-Page Preview Pane (matching Electron print-preview-pane)
            cols[1].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if is_de { "Seitenvorschau" } else { "Page Preview" }).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let badge_txt = if is_de {
                            format!("{} Seiten", total_pages.max(1))
                        } else {
                            format!("{} page(s)", total_pages.max(1))
                        };
                        Frame::new()
                            .fill(p.primary_hover)
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(badge_txt).size(11.0).strong().color(Color32::WHITE));
                            });
                    });
                });
                ui.add_space(4.0);

                let aspect = if app.export_opts.landscape { 842.0_f32 / 595.0 } else { 595.0_f32 / 842.0 };
                let mut page_idx = 1usize;

                egui::ScrollArea::vertical()
                    .id_salt("export_preview_scroll")
                    .max_height(380.0)
                    .show(ui, |ui| {
                        if app.export_opts.include_overview {
                            Frame::new()
                                .fill(p.bg_card)
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(6))
                                .stroke(Stroke::new(1.0_f32, p.border_main))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("#{page_idx}")).strong().color(p.primary_hover));
                                        ui.label(RichText::new(if is_de { "Gesamtuebersicht" } else { "Overall Layout Overview" }).strong().size(12.0));
                                    });
                                    draw_mini_plan_preview(ui, app, None, aspect);
                                });
                            ui.add_space(6.0);
                            page_idx += 1;
                        }

                        for (lid, lname, lh, lcol, cnt) in &active_layer_pages {
                            Frame::new()
                                .fill(p.bg_card)
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(6))
                                .stroke(Stroke::new(1.0_f32, p.border_main))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("#{page_idx}")).strong().color(parse_layer_swatch(lcol.as_deref())));
                                        ui.label(RichText::new(lname).strong().size(12.0));
                                        ui.label(RichText::new(format!("{lh:.0} mm · {cnt}")).size(11.0).color(p.text_muted));
                                    });
                                    draw_mini_plan_preview(ui, app, Some(lid.as_str()), aspect);
                                });
                            ui.add_space(6.0);
                            page_idx += 1;
                        }

                        if app.export_opts.include_bom_page {
                            let sum = get_piece_list_summary(&app.layout, &app.catalogs, is_de);
                            Frame::new()
                                .fill(p.bg_card)
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(6))
                                .stroke(Stroke::new(1.0_f32, p.border_main))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("#{page_idx}")).strong().color(Color32::from_rgb(16, 185, 129)));
                                        ui.label(RichText::new(if is_de { "Teileliste" } else { "Part List" }).strong().size(12.0));
                                    });
                                    Frame::new()
                                        .fill(Color32::WHITE)
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::same(6))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            for it in sum.tracks.items.iter().take(4) {
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new(&it.catalog_id).strong().size(10.5).color(Color32::from_rgb(15, 23, 42)));
                                                    ui.label(RichText::new(it.name.chars().take(26).collect::<String>()).size(10.0).color(Color32::from_rgb(51, 65, 85)));
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.label(RichText::new(format!("{}x", it.count)).strong().size(10.5).color(Color32::from_rgb(15, 23, 42)));
                                                    });
                                                });
                                            }
                                            if sum.tracks.items.len() > 4 {
                                                ui.label(RichText::new(format!("… +{} more items", sum.tracks.items.len() - 4)).size(10.0).color(Color32::from_rgb(100, 116, 139)));
                                            }
                                        });
                                });
                        }
                    });
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(if is_de { "PDF Exportieren..." } else { "Export PDF Blueprint..." })
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(p.primary_hover),
                )
                .clicked()
            {
                trigger = Some(1);
            }
            if ui
                .button(if is_de { "Drucken / PDF oeffnen..." } else { "Print / Open PDF..." })
                .clicked()
            {
                trigger = Some(5);
            }
            if ui.button("SVG (1:1 mm)...").clicked() {
                trigger = Some(2);
            }
            if ui.button("XML (.xml)...").clicked() {
                trigger = Some(3);
            }
            if ui
                .button(if is_de {
                    "Teileliste (CSV)..."
                } else {
                    "Part List (CSV)..."
                })
                .clicked()
            {
                trigger = Some(4);
            }
        });
            });
    });

    app.show_export_dialog = open;
    match trigger {
        Some(1) => app.export_pdf_dialog(),
        Some(2) => app.export_svg_dialog(),
        Some(3) => app.export_xml_dialog(),
        Some(4) => app.export_bom_csv_dialog(),
        Some(5) => app.print_or_open_pdf(),
        _ => {}
    }
}
