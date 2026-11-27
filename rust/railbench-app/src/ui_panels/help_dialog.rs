use super::*;
use crate::theme::RailbenchPalette;
use egui::text::LayoutJob;

#[derive(Clone, Copy)]
struct HelpCard {
    title: &'static str,
    body: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GuideImage {
    Layout,
    Toolbar,
    Selection,
    ContextMenu,
    HeightRamp,
}

impl GuideImage {
    #[cfg(test)]
    const ALL: [Self; 5] = [
        Self::Layout,
        Self::Toolbar,
        Self::Selection,
        Self::ContextMenu,
        Self::HeightRamp,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Layout => "help_guide_layout",
            Self::Toolbar => "help_guide_toolbar",
            Self::Selection => "help_guide_selection",
            Self::ContextMenu => "help_guide_context_menu",
            Self::HeightRamp => "help_guide_height_ramp",
        }
    }

    fn png_bytes(self) -> &'static [u8] {
        match self {
            Self::Layout => include_bytes!("../../../../src/assets/help/guide-layout.png"),
            Self::Toolbar => include_bytes!("../../../../src/assets/help/guide-toolbar.png"),
            Self::Selection => include_bytes!("../../../../src/assets/help/guide-selection.png"),
            Self::ContextMenu => include_bytes!("../../../../src/assets/help/guide-context-menu.png"),
            Self::HeightRamp => include_bytes!("../../../../src/assets/help/height-ramp-panel.png"),
        }
    }

    fn max_display_width(self) -> f32 {
        match self {
            Self::Layout | Self::Toolbar => f32::INFINITY,
            Self::Selection => 620.0,
            Self::ContextMenu => 290.0,
            Self::HeightRamp => 340.0,
        }
    }
}

const HELP_TAB_LABELS_DE: [&str; 11] = [
    "Erste Schritte",
    "Bildanleitung",
    "Ebenen & Hoehen",
    "Ebenen-Druck",
    "Spannungs-Simulation",
    "Tisch & Platte",
    "3D-Ansicht",
    "Multi-User & Server",
    "Tastaturkuerzel",
    "Gleiskatalog (JSON)",
    "Ueber Railbench & Gleise",
];

const HELP_TAB_LABELS_EN: [&str; 11] = [
    "Getting Started",
    "Visual Guide",
    "Layers & Heights",
    "Layer Printing",
    "Voltage Simulation",
    "Table & Baseboard",
    "3D View",
    "Multi-User & Server",
    "Keyboard Shortcuts",
    "Track Catalog (JSON)",
    "About & Systems",
];

const EXAMPLE_H0_CATALOG_JSON: &str = r#"{
  "id": "custom-h0-example",
  "name": "Muster-Gleiskatalog H0 (Beispiel)",
  "manufacturer": "Railbench",
  "system": "H0-Universal",
  "scale": "H0",
  "gauge": 16.5,
  "hasRoadbed": true,
  "conductors": 2,
  "description": "Vollstaendiger Beispiel-Gleiskatalog fuer Spur H0 (16,5 mm) mit allen 17 unterstuetzten Gleistypen.",
  "entries": [
    { "id": "EX-101", "name": "Gerades Gleis 188,3 mm", "nameEn": "Straight Track 188.3 mm", "kind": "straight", "category": "Geraden", "length": 188.3, "gauge": 16.5 },
    { "id": "EX-201", "name": "Gebogenes Gleis R1 / 30°", "nameEn": "Curved Track R1 / 30 deg", "kind": "curve", "category": "Boegen", "radius": 360, "angle": 30, "gauge": 16.5 },
    { "id": "EX-301", "name": "Weiche links 24,3°", "nameEn": "Turnout Left 24.3 deg", "kind": "turnout-left", "category": "Weichen", "length": 188.3, "branchRadius": 437.5, "branchAngle": 24.3, "gauge": 16.5 },
    { "id": "EX-302", "name": "Weiche rechts 24,3°", "nameEn": "Turnout Right 24.3 deg", "kind": "turnout-right", "category": "Weichen", "length": 188.3, "branchRadius": 437.5, "branchAngle": 24.3, "gauge": 16.5 },
    { "id": "EX-311", "name": "Bogenweiche links R1/R2", "nameEn": "Curved Turnout Left R1/R2", "kind": "curved-turnout-left", "category": "Weichen", "radius": 360, "angle": 30, "branchRadius": 437.5, "branchAngle": 30, "gauge": 16.5 },
    { "id": "EX-312", "name": "Bogenweiche rechts R1/R2", "nameEn": "Curved Turnout Right R1/R2", "kind": "curved-turnout-right", "category": "Weichen", "radius": 360, "angle": 30, "branchRadius": 437.5, "branchAngle": 30, "gauge": 16.5 },
    { "id": "EX-321", "name": "Dreiwegweiche 24,3°", "nameEn": "Three-Way Turnout 24.3 deg", "kind": "three-way", "category": "Weichen", "length": 188.3, "branchRadius": 437.5, "branchAngle": 24.3, "gauge": 16.5 },
    { "id": "EX-331", "name": "Doppelkreuzungsweiche (DKW) 12,1°", "nameEn": "Double Slip Switch 12.1 deg", "kind": "double-slip", "category": "Weichen", "length": 188.3, "branchRadius": 1114.6, "branchAngle": 12.1, "angle": 12.1, "gauge": 16.5 },
    { "id": "EX-401", "name": "Kreuzung 24,3°", "nameEn": "Crossing 24.3 deg", "kind": "crossing", "category": "Kreuzungen", "length": 188.3, "angle": 24.3, "gauge": 16.5 },
    { "id": "EX-501", "name": "Elektrisches Entkupplungsgleis 94,2 mm", "nameEn": "Electric Uncoupler Track 94.2 mm", "kind": "uncoupler", "category": "Zubehoer", "length": 94.2, "gauge": 16.5 },
    { "id": "EX-502", "name": "Prellbockgleis 77,5 mm", "nameEn": "Buffer Stop Track 77.5 mm", "kind": "buffer-stop", "category": "Zubehoer", "length": 77.5, "gauge": 16.5, "localConnectors": [{ "x": 0, "y": 0, "angle": 180 }] },
    { "id": "EX-503", "name": "Stromeinspeisegleis 188,3 mm", "nameEn": "Power Feeder Track 188.3 mm", "kind": "feeder", "category": "Zubehoer", "length": 188.3, "gauge": 16.5 },
    { "id": "EX-504", "name": "Isolier- / Trenngleis 188,3 mm", "nameEn": "Isolating Track 188.3 mm", "kind": "isolating", "category": "Zubehoer", "length": 188.3, "gauge": 16.5 },
    { "id": "EX-601", "name": "Flexgleis 900 mm", "nameEn": "Flex Track 900 mm", "kind": "flex", "category": "Geraden", "length": 900, "gauge": 16.5 },
    { "id": "EX-701", "name": "Drehscheibe 23 m (378 mm)", "nameEn": "Turntable 23 m (378 mm)", "kind": "turntable", "category": "Zubehoer", "length": 378, "radius": 189, "gauge": 16.5 },
    { "id": "EX-702", "name": "Schiebebuehne 360 mm", "nameEn": "Transfer Table 360 mm", "kind": "transfer-table", "category": "Zubehoer", "length": 360, "gauge": 16.5 },
    { "id": "EX-801", "name": "Boeschungsendstueck 16,5 mm", "nameEn": "Roadbed End Piece 16.5 mm", "kind": "accessory", "category": "Zubehoer", "length": 16.5, "gauge": 16.5, "localConnectors": [{ "x": 0, "y": 0, "angle": 180 }] }
  ]
}"#;

fn surface_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(21, 31, 50) // #151f32
    } else {
        Color32::from_rgb(241, 245, 249) // #f1f5f9
    }
}

/// Renders inline **bold** and `code` spans cleanly without HTML noise.
fn build_rich_job(
    text: &str,
    font_size: f32,
    base_color: Color32,
    strong_color: Color32,
    code_bg: Color32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let prop_font = egui::FontId::proportional(font_size);
    let mono_font = egui::FontId::monospace((font_size - 0.5).max(10.0));
    let mut rest = text;

    while !rest.is_empty() {
        let next_bold = rest.find("**");
        let next_code = rest.find('`');
        let (kind, idx) = match (next_bold, next_code) {
            (Some(b), Some(c)) => {
                if b <= c {
                    (0u8, b)
                } else {
                    (1u8, c)
                }
            }
            (Some(b), None) => (0u8, b),
            (None, Some(c)) => (1u8, c),
            (None, None) => {
                job.append(
                    rest,
                    0.0,
                    egui::TextFormat {
                        font_id: prop_font.clone(),
                        color: base_color,
                        ..Default::default()
                    },
                );
                break;
            }
        };

        if idx > 0 {
            job.append(
                &rest[..idx],
                0.0,
                egui::TextFormat {
                    font_id: prop_font.clone(),
                    color: base_color,
                    ..Default::default()
                },
            );
        }

        if kind == 0 {
            let after = &rest[idx + 2..];
            if let Some(end) = after.find("**") {
                job.append(
                    &after[..end],
                    0.0,
                    egui::TextFormat {
                        font_id: prop_font.clone(),
                        color: strong_color,
                        ..Default::default()
                    },
                );
                rest = &after[end + 2..];
            } else {
                job.append(
                    &rest[idx..idx + 2],
                    0.0,
                    egui::TextFormat {
                        font_id: prop_font.clone(),
                        color: base_color,
                        ..Default::default()
                    },
                );
                rest = after;
            }
        } else {
            let after = &rest[idx + 1..];
            if let Some(end) = after.find('`') {
                job.append(
                    &format!(" {} ", &after[..end]),
                    0.0,
                    egui::TextFormat {
                        font_id: mono_font.clone(),
                        color: strong_color,
                        background: code_bg,
                        ..Default::default()
                    },
                );
                rest = &after[end + 1..];
            } else {
                job.append(
                    &rest[idx..idx + 1],
                    0.0,
                    egui::TextFormat {
                        font_id: prop_font.clone(),
                        color: base_color,
                        ..Default::default()
                    },
                );
                rest = after;
            }
        }
    }

    job
}

fn rich_label_colored(
    ui: &mut egui::Ui,
    base_color: Color32,
    strong_color: Color32,
    code_bg: Color32,
    text: &str,
    font_size: f32,
) {
    let job = build_rich_job(text, font_size, base_color, strong_color, code_bg);
    ui.add(egui::Label::new(job).wrap());
}

fn rich_label(ui: &mut egui::Ui, p: RailbenchPalette, text: &str, font_size: f32) {
    rich_label_colored(ui, p.text_sub, p.text_main, p.bg_card_hover, text, font_size);
}

fn render_section_header(ui: &mut egui::Ui, p: RailbenchPalette, title: &str, intro: &str) {
    ui.label(RichText::new(title).strong().size(18.0).color(p.text_main));
    ui.add_space(4.0);
    let (line_r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 2.0), egui::Sense::hover());
    ui.painter().rect_filled(line_r, 0.0, p.border_main);
    ui.add_space(8.0);
    rich_label(ui, p, intro, 13.0);
    ui.add_space(12.0);
}

fn render_single_card(ui: &mut egui::Ui, p: RailbenchPalette, surface: Color32, card: HelpCard, width: f32) {
    Frame::new()
        .fill(surface)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(13))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width((width - 26.0).max(120.0));
            ui.label(RichText::new(card.title).strong().size(13.5).color(p.text_main));
            ui.add_space(5.0);
            rich_label(ui, p, card.body, 12.0);
        });
}

fn render_card_grid(ui: &mut egui::Ui, p: RailbenchPalette, dark: bool, cards: &[HelpCard]) {
    let surface = surface_color(dark);
    let gap = 12.0_f32;
    let avail = ui.available_width();
    let col_w = ((avail - gap) * 0.5).max(200.0);

    for pair in cards.chunks(2) {
        if pair.len() == 2 && avail >= 420.0 {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.allocate_ui_with_layout(
                    egui::vec2(col_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| render_single_card(ui, p, surface, pair[0], col_w),
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(col_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| render_single_card(ui, p, surface, pair[1], col_w),
                );
            });
        } else {
            for card in pair {
                render_single_card(ui, p, surface, *card, avail);
                ui.add_space(10.0);
            }
        }
        ui.add_space(10.0);
    }
}

fn render_callout(ui: &mut egui::Ui, p: RailbenchPalette, dark: bool, text: &str) {
    let surface = surface_color(dark);
    let outer = Frame::new()
        .fill(surface)
        .corner_radius(CornerRadius {
            nw: 0,
            ne: 6,
            sw: 0,
            se: 6,
        })
        .inner_margin(Margin {
            left: 16,
            right: 14,
            top: 11,
            bottom: 11,
        })
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            rich_label(ui, p, text, 12.5);
        });
    let r = outer.response.rect;
    let bar = egui::Rect::from_min_max(r.min, egui::pos2(r.min.x + 4.0, r.max.y));
    ui.painter().rect_filled(bar, 0.0, p.primary);
    ui.add_space(8.0);
}

fn render_kbd_line(ui: &mut egui::Ui, p: RailbenchPalette, shortcut: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
        for (i, group) in shortcut.split(" / ").enumerate() {
            if i > 0 {
                ui.label(RichText::new("/").size(11.0).color(p.text_muted));
            }
            let parts: Vec<&str> = if group.trim() == "+" {
                vec!["+"]
            } else {
                group.split('+').map(|s| s.trim()).collect()
            };
            for (pi, part) in parts.iter().enumerate() {
                if pi > 0 {
                    ui.label(RichText::new("+").size(11.0).color(p.text_muted));
                }
                Frame::new()
                    .fill(p.bg_card)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(6, 2))
                    .stroke(Stroke::new(1.0_f32, p.border_main))
                    .show(ui, |ui| {
                        ui.label(RichText::new(*part).strong().size(11.0).color(p.text_main));
                    });
            }
        }
    });
}

fn render_three_col_table(
    ui: &mut egui::Ui,
    p: RailbenchPalette,
    dark: bool,
    headers: [&str; 3],
    rows: &[(String, String, String)],
    col_ratios: [f32; 3],
    first_col_kbd: bool,
) {
    let surface = surface_color(dark);
    let total_w = ui.available_width().max(320.0);
    let inner_w = (total_w - 16.0).max(240.0);
    let col_gap = 8.0_f32;
    let usable = (inner_w - col_gap * 2.0).max(200.0);
    let w0 = (usable * col_ratios[0]).floor();
    let w1 = (usable * col_ratios[1]).floor();
    let w2 = (usable - w0 - w1).max(60.0);

    Frame::new()
        .fill(surface)
        .inner_margin(Margin::symmetric(8, 7))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(inner_w);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = col_gap;
                ui.allocate_ui_with_layout(egui::vec2(w0, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(RichText::new(headers[0].to_uppercase()).strong().size(11.0).color(p.text_muted));
                });
                ui.allocate_ui_with_layout(egui::vec2(w1, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(RichText::new(headers[1].to_uppercase()).strong().size(11.0).color(p.text_muted));
                });
                ui.allocate_ui_with_layout(egui::vec2(w2, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(RichText::new(headers[2].to_uppercase()).strong().size(11.0).color(p.text_muted));
                });
            });
        });

    for (c0, c1, c2) in rows {
        let row_resp = Frame::new()
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.set_width(inner_w);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = col_gap;
                    ui.vertical(|ui| {
                        ui.set_width(w0);
                        if first_col_kbd {
                            render_kbd_line(ui, p, c0);
                        } else {
                            rich_label(ui, p, c0, 12.0);
                        }
                    });
                    ui.vertical(|ui| {
                        ui.set_width(w1);
                        rich_label(ui, p, c1, 12.0);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(w2);
                        rich_label_colored(ui, p.text_muted, p.text_main, p.bg_card_hover, c2, 11.5);
                    });
                });
            });
        let rr = row_resp.response.rect;
        ui.painter().hline(rr.x_range(), rr.bottom(), Stroke::new(1.0_f32, p.border_main));
    }
}

fn decode_guide_color_image(img_kind: GuideImage) -> Option<egui::ColorImage> {
    let decoded = image::load_from_memory(img_kind.png_bytes()).ok()?.to_rgba8();
    let (w, h) = (decoded.width() as usize, decoded.height() as usize);
    Some(egui::ColorImage::from_rgba_unmultiplied([w, h], decoded.as_raw()))
}

fn get_or_load_guide_texture(ctx: &egui::Context, img_kind: GuideImage) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(img_kind.key());
    if let Some(existing) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(existing);
    }
    let color_img = decode_guide_color_image(img_kind)?;
    let tex = ctx.load_texture(img_kind.key(), color_img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

fn render_guide_step_card(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: RailbenchPalette,
    dark: bool,
    title: &str,
    img_kind: GuideImage,
    caption: &str,
    body: &str,
) {
    let surface = surface_color(dark);
    Frame::new()
        .fill(surface)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).strong().size(14.0).color(p.text_main));
            ui.add_space(8.0);
            if let Some(tex) = get_or_load_guide_texture(ctx, img_kind) {
                let ts = tex.size_vec2();
                let max_w = (ui.available_width() - 2.0).min(img_kind.max_display_width()).max(120.0);
                let scale = (max_w / ts.x.max(1.0)).min(1.0);
                let disp = ts * scale;
                ui.vertical_centered(|ui| {
                    Frame::new()
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .inner_margin(Margin::same(0))
                        .show(ui, |ui| {
                            ui.image((tex.id(), disp));
                        });
                    ui.add_space(4.0);
                    ui.label(RichText::new(caption).italics().size(11.0).color(p.text_muted));
                });
                ui.add_space(8.0);
            }
            rich_label(ui, p, body, 12.2);
        });
    ui.add_space(12.0);
}

pub(super) fn draw_help_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let surface = surface_color(app.dark_mode);
    let mut open = app.show_help_dialog;
    let mut open_keymap = false;
    let screen_rect = ctx.screen_rect();

    // Dark translucent overlay matching #help-modal (.rb-modal-overlay)
    egui::Area::new(egui::Id::new("railbench_help_backdrop"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let (bg_rect, bg_resp) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                bg_rect,
                0.0,
                Color32::from_rgba_unmultiplied(15, 23, 42, if app.dark_mode { 175 } else { 150 }),
            );
            if bg_resp.clicked() {
                open = false;
            }
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        open = false;
    }

    let modal_w = 900.0_f32.min((screen_rect.width() * 0.96).max(460.0));
    let modal_h = 640.0_f32.min((screen_rect.height() * 0.90).max(380.0));
    let modal_frame = Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(0))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .shadow(egui::Shadow {
            offset: [0, 14],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(if app.dark_mode { 165 } else { 85 }),
        });

    let header_h = 46.0_f32;
    let footer_h = 48.0_f32;
    let body_h = (modal_h - header_h - footer_h).max(220.0);

    let win_resp = egui::Window::new("Railbench Help")
        .id(egui::Id::new("railbench_help_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([modal_w, modal_h])
        .frame(modal_frame)
        .show(ctx, |ui| {
            ui.set_width(modal_w);
            ui.set_height(modal_h);
            ui.spacing_mut().item_spacing.y = 0.0;

            // --- Modal Header (.rb-modal-header) ---
            let modal_title = if is_de {
                "Railbench - Benutzerhandbuch & Funktionsuebersicht"
            } else {
                "Railbench - User & Feature Guide"
            };
            let hdr_inner_w = (modal_w - 36.0).max(200.0);
            let close_w = 26.0_f32;
            let title_w = (hdr_inner_w - close_w - 8.0).max(120.0);
            let hdr_resp = Frame::new()
                .fill(surface)
                .corner_radius(CornerRadius {
                    nw: 10,
                    ne: 10,
                    sw: 0,
                    se: 0,
                })
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_width(hdr_inner_w);
                    ui.allocate_ui_with_layout(
                        egui::vec2(hdr_inner_w, 26.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.allocate_ui_with_layout(
                                egui::vec2(title_w, 26.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(modal_title)
                                                .strong()
                                                .size(15.0)
                                                .color(p.text_main),
                                        )
                                        .truncate(),
                                    );
                                    ui.add_space(4.0);
                                    Frame::new()
                                        .fill(p.primary_hover)
                                        .corner_radius(CornerRadius::same(5))
                                        .inner_margin(Margin::symmetric(7, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(crate::version::APP_VERSION_TAG)
                                                    .strong()
                                                    .size(11.0)
                                                    .color(Color32::WHITE),
                                            );
                                        });
                                },
                            );
                            let (close_r, close_resp) =
                                ui.allocate_exact_size(egui::vec2(close_w, 26.0), egui::Sense::click());
                            if close_resp.hovered() {
                                ui.painter().rect_filled(close_r, CornerRadius::same(4), p.border_main);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            crate::icons::paint_icon(
                                ui.painter(),
                                close_r.shrink(7.0),
                                Icon::WinClose,
                                if close_resp.hovered() { p.text_main } else { p.text_muted },
                            );
                            if close_resp
                                .on_hover_text(if is_de { "Hilfe schliessen (Esc)" } else { "Close Help (Esc)" })
                                .clicked()
                            {
                                open = false;
                            }
                        },
                    );
                });
            let hr = hdr_resp.response.rect;
            ui.painter()
                .hline(hr.x_range(), hr.bottom(), Stroke::new(1.0_f32, p.border_main));

            // --- Split Layout (.help-modal-layout): Left Nav Sidebar + Right Content Area ---
            let nav_w = 220.0_f32.min((modal_w * 0.30).max(150.0));
            let content_w = (modal_w - nav_w).max(260.0);
            let nav_inner_w = (nav_w - 16.0).max(120.0);
            let nav_scroll_h = (body_h - 24.0).max(120.0);
            let content_inner_w = (content_w - 48.0).max(180.0);
            let content_scroll_h = (body_h - 40.0).max(120.0);
            ui.allocate_ui_with_layout(
                egui::vec2(modal_w, body_h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    // Left Nav Sidebar (.help-nav-sidebar)
                    let nav_resp = ui.allocate_ui_with_layout(
                        egui::vec2(nav_w, body_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            Frame::new()
                                .fill(surface)
                                .inner_margin(Margin::symmetric(8, 12))
                                .show(ui, |ui| {
                                    ui.set_width(nav_inner_w);
                                    ui.set_height(nav_scroll_h);
                                    ui.spacing_mut().item_spacing.y = 4.0;
                                    let labels = if is_de { &HELP_TAB_LABELS_DE } else { &HELP_TAB_LABELS_EN };
                                    egui::ScrollArea::vertical()
                                        .id_salt("help_nav_sidebar_scroll")
                                        .max_width(nav_inner_w)
                                        .max_height(nav_scroll_h)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.with_layout(
                                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                                |ui| {
                                                    ui.visuals_mut().widgets.hovered.weak_bg_fill = p.bg_card;
                                                    ui.visuals_mut().widgets.hovered.bg_fill = p.bg_card;
                                                    for (idx, lbl) in labels.iter().enumerate() {
                                                        let active = app.help_tab as usize == idx;
                                                        let btn = egui::Button::new(
                                                            RichText::new(*lbl)
                                                                .strong()
                                                                .size(12.5)
                                                                .color(if active { Color32::WHITE } else { p.text_sub }),
                                                        )
                                                        .fill(if active { p.primary_hover } else { Color32::TRANSPARENT })
                                                        .stroke(Stroke::NONE)
                                                        .corner_radius(CornerRadius::same(6))
                                                        .min_size(egui::vec2((nav_w - 22.0).max(100.0), 32.0));
                                                        let resp = ui.add(btn);
                                                        if resp.hovered() {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                        }
                                                        if resp.clicked() {
                                                            app.help_tab = idx as u8;
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                });
                        },
                    );
                    let nr = nav_resp.response.rect;
                    ui.painter()
                        .vline(nr.right(), nr.y_range(), Stroke::new(1.0_f32, p.border_main));

                    // Right Content Area (.help-content-area)
                    ui.allocate_ui_with_layout(
                        egui::vec2(content_w, body_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            Frame::new()
                                .fill(p.bg_card)
                                .inner_margin(Margin::symmetric(24, 20))
                                .show(ui, |ui| {
                                    ui.set_width(content_inner_w);
                                    ui.set_height(content_scroll_h);
                                    ui.spacing_mut().item_spacing.y = 6.0;
                                    egui::ScrollArea::vertical()
                                        .id_salt(("help_content_scroll", app.help_tab))
                                        .max_width(content_inner_w)
                                        .max_height(content_scroll_h)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.set_width((content_inner_w - 14.0).max(160.0));
                                                render_help_tab_content(
                                                    ui,
                                                    ctx,
                                                    app,
                                                    p,
                                                    is_de,
                                                    &mut open_keymap,
                                                );
                                            });
                                        });
                                });
                        },
                    );
                },
            );

            // --- Modal Footer (.rb-modal-footer) ---
            let ftr_inner_w = (modal_w - 36.0).max(200.0);
            let close_btn_w = 138.0_f32;
            let tip_w = (ftr_inner_w - close_btn_w - 12.0).max(100.0);
            let ftr_resp = Frame::new()
                .fill(surface)
                .corner_radius(CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 10,
                    se: 10,
                })
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_width(ftr_inner_w);
                    let tip = if is_de {
                        "Tipp: Druecken Sie jederzeit **F1** fuer diese Hilfe oder **M** bei gewaehlten Gleisen fuer Hoehen & Rampen."
                    } else {
                        "Tip: Press **F1** anywhere to open this guide, or **M** when tracks are selected for elevation & ramps."
                    };
                    ui.allocate_ui_with_layout(
                        egui::vec2(ftr_inner_w, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 12.0;
                            ui.allocate_ui_with_layout(
                                egui::vec2(tip_w, 28.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(tip_w);
                                    rich_label(ui, p, tip, 11.5);
                                },
                            );
                            let close_lbl = if is_de { "Handbuch schliessen" } else { "Close Guide" };
                            let btn = egui::Button::new(RichText::new(close_lbl).strong().size(12.5).color(Color32::WHITE))
                                .fill(p.primary_hover)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(egui::vec2(close_btn_w, 28.0));
                            if ui.add(btn).clicked() {
                                open = false;
                            }
                        },
                    );
                });
            let fr = ftr_resp.response.rect;
            ui.painter()
                .hline(fr.x_range(), fr.top(), Stroke::new(1.0_f32, p.border_main));
        });
    if let Some(ir) = win_resp {
        ctx.move_to_top(ir.response.layer_id);
    }

    app.show_help_dialog = open;
    if open_keymap {
        app.show_help_dialog = false;
        app.settings_tab = 1;
        app.show_settings_dialog = true;
    }
}

fn render_help_tab_content(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app: &mut RailbenchApp,
    p: RailbenchPalette,
    is_de: bool,
    open_keymap: &mut bool,
) {
    let dark = app.dark_mode;
    match app.help_tab {
        // 0: Overview / Getting Started
        0 => {
            render_section_header(
                ui,
                p,
                if is_de { "Erste Schritte mit Railbench" } else { "Getting Started with Railbench" },
                if is_de {
                    "**Railbench** ist eine moderne, hochpraezise CAD-Gleisplanungssoftware fuer den Modellbahn-Bereich. Sie verbindet millimetergenaue CAD-Geometrien mit elektrischer Traktionssimulation, flexibler Mehrebenen-Verwaltung und druckfertigen Bauplaenen."
                } else {
                    "**Railbench** is a high-precision track planning CAD application engineered for model railway hobbyists. It combines millimetric CAD geometry with electrical simulation, multi-level layout design, and print-ready schematics."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "1. Gleise hinzufuegen",
                        body: "Waehlen Sie Gleise aus dem **linken Katalog** (`Gleise` / `Strassen`). Klicken Sie auf ein Gleis oder ziehen Sie es per **Drag & Drop** direkt auf die Arbeitsflaeche.",
                    },
                    HelpCard {
                        title: "2. Intelligentes Einrasten (Snapping)",
                        body: "Das automatische magnetische Einrasten (`S`) verbindet Gleisenden nahtlos, berechnet exakte Winkel und schliesst Verbindungen im Millimeterbereich.",
                    },
                    HelpCard {
                        title: "3. Drehen & Verschieben",
                        body: "Gleise mit der Maus ziehen zum Verschieben. Drehen um den gewaehlten Winkelschritt mit `R` (bzw. `Shift+R` gegen den Uhrzeigersinn). `Shift + Mausrad` dreht fein unter dem Cursor!",
                    },
                    HelpCard {
                        title: "4. Mehrfachauswahl (Aufziehrahmen)",
                        body: "Klicken Sie auf ein Gleis zum Auswaehlen, oder ziehen Sie mit der **linken Maustaste im freien Bereich** ein Rechteck (Marquee Box). `Strg` oder `Umschalt` erweitert die Auswahl.",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "1. Adding Tracks",
                        body: "Browse track pieces in the **left catalog sidebar**. Click any piece to append it, or **drag and drop** it directly onto the canvas.",
                    },
                    HelpCard {
                        title: "2. Smart Snapping",
                        body: "Automatic magnetic snapping (`S` key) seamlessly aligns track ends, calculates exact angles, and connects track geometry within millimetric tolerances.",
                    },
                    HelpCard {
                        title: "3. Rotating & Moving",
                        body: "Drag tracks with the mouse to move them. Rotate by your active step using `R` (`Shift+R` counter-clockwise). Hold `Shift + Mouse Wheel` or edit exact angles in the bottom selection bar.",
                    },
                    HelpCard {
                        title: "4. Multi-Selection (Marquee Box)",
                        body: "Click directly on any track to select it, or drag with the **Left Mouse Button in an empty area** to draw a selection rectangle. Hold `Ctrl` or `Shift` to add/toggle tracks.",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);
            render_callout(
                ui,
                p,
                dark,
                if is_de {
                    "**Navigationstipp:** Halten Sie die **rechte Maustaste** oder das **Mausrad (Mittelklick)** gedrueckt, um den Plan frei zu verschieben (Pan). Drehen Sie das **Mausrad** zum stufenlosen Zoomen und druecken Sie `Strg+0` zum Zentrieren."
                } else {
                    "**Navigation Pro-Tip:** Hold **Right Mouse Button** or **Middle Click** to pan the canvas. Use the **Mouse Wheel** (or `+` / `-`) to zoom smoothly, and press `Ctrl+0` anytime to re-center the layout."
                },
            );
        }

        // 1: Visual Guide
        1 => {
            render_section_header(
                ui,
                p,
                if is_de { "Bildanleitung: Menues & Oberflaeche" } else { "Visual Guide: Menus & Interface" },
                if is_de {
                    "Ein kurzer Rundgang durch das Railbench-Fenster. Jedes Bild erklaert die Bereiche und die wichtigsten Schaltflaechen der Oberflaeche."
                } else {
                    "A quick tour of the Railbench window. Each picture highlights where to find the tools and explains what the controls do."
                },
            );
            let steps: [(&str, GuideImage, &str, &str); 5] = if is_de {
                [
                    (
                        "1. Das Hauptfenster",
                        GuideImage::Layout,
                        "Bereiche des Hauptfensters",
                        "- **Obere Werkzeugleiste**: Enthaelt Hauptmenues, Katalog-Reiter, Schnellaktionen und Modus-Schalter.\n- **Linke Katalog-Seitenleiste**: Gleiskatalog mit Suche & Filtern, Strassen, 3D-Modelle, Gelaende, Platte und Stueckliste.\n- **2D-Zeichenflaeche / 3D-Ansicht (`3`)**: Arbeitsflaeche mit Millimeter-Linealen. Rechts/Mittel-Ziehen zum Verschieben, Mausrad zum Zoomen, `Strg+0` zum Zentrieren.\n- **Rechtes Ebenen-Panel (`Shift+B`)**: Listet alle Hoehenebenen mit Sichtbarkeits-, Sperr- und Loesch-Schaltern sowie **+ Ebene** und **Hoehen / Rampe...**.\n- **Untere Auswahlleiste**: Erscheint automatisch, sobald ein oder mehrere Gleise bzw. Modelle markiert sind.",
                    ),
                    (
                        "2. Die Werkzeugleiste & alle Schaltflaechen",
                        GuideImage::Toolbar,
                        "Menues und Schnellschalter der Werkzeugleiste",
                        "- **Obere Zeile (Menues)**: Anlage (Neu, Oeffnen, Demo, Speichern, Bauplan-Export & Drucken, XML/CSV, Sync), Bearbeiten, Ansicht, Einstellungen & Hilfe.\n- **Zweite Zeile (v.l.n.r.)**: Reiter (`Gleise`, `Strassen`, `3D`, `Gelaende`, `Platte`, `Liste`) · Neu/Oeffnen/Speichern · `Snap (S)` · `Ebenenschutz (Alt+L)` · `Messen (M)` · `CAD-Tools` · Drehen `R` / `Shift+R` + Winkelschritt · `Farben (C)` · `Nummern (N)` · `Spannung (P)` · `3D (3)` & `Zuege 3D` · `Sprache` · `Collab` · `Design` · `Ebenen (Shift+B)`.",
                    ),
                    (
                        "3. Die Auswahlleiste & Eigenschafts-Schalter",
                        GuideImage::Selection,
                        "Auswahlleiste mit Hoehe, Winkel und Eigenschaften",
                        "Erscheint unten, sobald Elemente markiert sind: Auswahl-Anzahl, Ebenen-Auswahl, **Hoehenfeld (mm)**, **Winkelfeld (deg)**, `Einspeisung (F)`, `Trennstelle (I)`, `Oberleitung (O)`, `OL-Einspeisung (Shift+F)`, `OL-Trennstelle (Shift+I)`, `Signal (Shift+S)`, `Weichenlaterne (W)` & `Decoder (Shift+W)`, `Bruecke (B)`, `Tunnel (T)`, Drehen und `Loeschen (Entf)`.",
                    ),
                    (
                        "4. Rechtsklick-Kontextmenue am Gleis",
                        GuideImage::ContextMenu,
                        "Rechtsklick-Kontextmenue eines Gleisstuecks",
                        "Rechtsklick auf ein Gleis oeffnet das Kontextmenue mit Artikelnummer, Ebene/Hoehe, `Hoehe & Steigung... (M)`, `Verbundene Gleise auswaehlen`, `Ganze Ebene auswaehlen`, `Eigenschaften / Info...`, `Gleis teilen / ersetzen`, Signal-Optionen (`Shift+S`), Bruecke (`B`), Tunnel (`T`), Oberleitung (`O`) und Weichen-Zubehoer (`W` / `Shift+W`).",
                    ),
                    (
                        "5. Panel Hoehe & Rampe",
                        GuideImage::HeightRamp,
                        "Panel Hoehe & Rampe, Reiter Steigung",
                        "- **Hoehe / Ebene**: Setzt gewaehlte Gleise auf die Hoehe einer Zielebene oder auf einen festen Millimeterwert.\n- **Steigung / Rampe**: Berechnet gleichmaessige lineare oder S-Kurven-Uebergaenge zwischen Start A und Ende B inkl. `Auto-Fit`, Gefaelle-Pruefung (`<= 3,0%` sicher, `3,0-4,5%` maessig, `> 4,5%` steil).\n- **Generator**: Erzeugt automatisch gerade, gebogene oder kreisfoermige Wendel-Rampen ab einem offenen Verbinder.",
                    ),
                ]
            } else {
                [
                    (
                        "1. The Main Window",
                        GuideImage::Layout,
                        "Main window areas",
                        "- **Top Toolbar**: Dropdown menus, catalog tab switchers, quick file/edit actions, and mode toggles.\n- **Left Catalog Sidebar**: Tracks (with search & filters), Roads, 3D Buildings, Terrain Editor, Table Benchwork, and Parts List (BOM).\n- **2D Canvas / 3D View (`3`)**: Workspace with edge millimeter rulers. Right/middle drag to pan, scroll wheel to zoom, `Ctrl+0` to fit.\n- **Right Layers Panel (`Shift+B`)**: Lists height decks with Eye (visibility), Lock (freeze), Delete, `+ Add Layer`, and `Heights / Ramp...`.\n- **Bottom Selection Bar**: Appears automatically whenever tracks or models are selected.",
                    ),
                    (
                        "2. The Toolbar & Quick Buttons",
                        GuideImage::Toolbar,
                        "Toolbar menus and quick action buttons",
                        "- **Top Row (Menus)**: Layout (New, Open, Demos, Save, Blueprint Export & Print, XML/CSV, Collab), Edit, View, Settings & Help.\n- **Second Row (Left to Right)**: Tabs (`Tracks`, `Roads`, `3D`, `Terrain`, `Board`, `BOM`) · New/Open/Save · `Snap (S)` · `Layer Lock (Alt+L)` · `Measure (M)` · `CAD Tools` · Rotate `R` / `Shift+R` + Step · `Colors (C)` · `Labels (N)` · `Voltage (P)` · `3D (3)` & `Trains 3D` · `Language` · `Collab` · `Theme` · `Layers (Shift+B)`.",
                    ),
                    (
                        "3. The Selection Bar & Property Toggles",
                        GuideImage::Selection,
                        "Selection bar with height, angle and property toggles",
                        "Appears at the bottom when items are selected: piece count, layer selector, **Height (mm)**, **Angle (deg)**, `Power Feeder (F)`, `Rail Isolator (I)`, `Catenary (O)`, `Catenary Feeder (Shift+F)`, `Catenary Isolator (Shift+I)`, `Signal (Shift+S)`, `Turnout Lantern (W)` & `Decoder (Shift+W)`, `Bridge (B)`, `Tunnel (T)`, Rotate buttons, and `Delete (Del)`.",
                    ),
                    (
                        "4. Track Right-Click Context Menu",
                        GuideImage::ContextMenu,
                        "Right-click context menu on a track piece",
                        "Right-click any track piece to inspect its catalog ID, layer & elevation, open `Height & Ramp... (M)`, select connected tracks or whole layer, inspect `Properties / Info...`, split/replace track, and toggle Signal (`Shift+S`), Bridge (`B`), Tunnel (`T`), Catenary (`O`), or Turnout Accessories (`W` / `Shift+W`).",
                    ),
                    (
                        "5. Height & Ramp Panel",
                        GuideImage::HeightRamp,
                        "Height & Ramp panel, Slope tab",
                        "- **Height / Level tab**: Moves tracks to a target deck height or fixed millimeter elevation.\n- **Slope / Ramp tab**: Computes smooth linear or S-curve grade transitions between Start A and End B with live gradient check (`<= 3.0%` safe, `3.0-4.5%` moderate, `> 4.5%` steep).\n- **Generator tab**: Builds straight, curved, or spiral helix ramp tracks directly from an open connector.",
                    ),
                ]
            };
            for (title, img, caption, body) in steps {
                render_guide_step_card(ui, ctx, p, dark, title, img, caption, body);
            }
        }

        // 2: Layers & Heights
        2 => {
            render_section_header(
                ui,
                p,
                if is_de { "Mehrebenen-Verwaltung & Hoehenplanung" } else { "Multi-Deck Layers & Height Management" },
                if is_de {
                    "Mit Railbench konstruieren Sie mehrstoeckige Modellbahnanlagen (z. B. Grundebene, Bergetappen, Rampen und verdeckte Schattenbahnhoefe) mit echten vertikalen Millimeterhoehen."
                } else {
                    "Design multi-level model train layouts (ground level, upper mountain decks, ramps, and hidden staging yards) with real vertical millimeter elevations."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "Schwebendes Ebenen-Menue",
                        body: "Druecken Sie `Shift+B` oder klicken Sie auf **Ebenen** in der Leiste. Ebenen sind nach ihrer Hoehe sortiert (hoechste Ebene oben).",
                    },
                    HelpCard {
                        title: "Sichtbarkeit & Sperrschutz",
                        body: "Blendet ueberliegende Ebenen per **Auge-Symbol** aus oder friert sie per **Schloss-Symbol** ein. `Alt+L` schaltet den globalen Ebenenschutz um.",
                    },
                    HelpCard {
                        title: "Ebenen bearbeiten & umbenennen",
                        body: "Ueber das Ebenen-Menue lassen sich Name, Farbe, Standardhoehe (mm) sowie pro Ebene optionale Trassenbretter ('Holzplatten') konfigurieren.",
                    },
                    HelpCard {
                        title: "Gleise Ebene zuweisen (M)",
                        body: "Gleise markieren und Taste `M` druecken (oder **Hoehen / Rampe...** oeffnen), um ausgewaehlte Gleise einer Ebene oder Wunschhoehe zuzuordnen.",
                    },
                    HelpCard {
                        title: "Rampen- & Steigungsrechner",
                        body: "Berechnet gleichmaessige Uebergaenge (z. B. `0 mm -> 80 mm`) und prueft die Steigung:\n- `<= 3.0%`: Empfohlen (Sanft)\n- `3.0% - 4.5%`: Maessig (schwere Zuege pruefen)\n- `> 4.5%`: Warnung (Schleudergefahr)",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "Floating Layers Menu",
                        body: "Press `Shift+B` or click the **Layers** button in the toolbar to toggle the floating layers manager sorted by deck height.",
                    },
                    HelpCard {
                        title: "Visibility & Lock Protection",
                        body: "Click the **Eye** icon to hide upper decks while working below, or lock a layer with the **Lock** icon. Press `Alt+L` to toggle layer selection lock.",
                    },
                    HelpCard {
                        title: "Edit Name, Color & Sub-Plates",
                        body: "Configure each deck's name, color swatch, base millimeter height, and optional contoured wood sub-plates.",
                    },
                    HelpCard {
                        title: "Move Tracks to Layer (M)",
                        body: "Select tracks and press `M` (or use the bottom Selection Bar dropdown) to assign tracks to another deck and synchronize elevation.",
                    },
                    HelpCard {
                        title: "Ramp & Incline Calculator",
                        body: "Computes smooth elevation transitions (`0 mm -> 80 mm`) with live gradient diagnostics:\n- `<= 3.0%`: Recommended\n- `3.0% - 4.5%`: Moderate\n- `> 4.5%`: Steep warning (slippage risk)",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);
            render_callout(
                ui,
                p,
                dark,
                if is_de {
                    "**Schwebende Auswahlleiste:** Sobald Gleise markiert sind, erscheint unten eine kompakte Leiste, um Ebene, Hoehe, Winkel und alle Elektrik-Schalter mit einem Klick zu steuern."
                } else {
                    "**Floating Selection Bar:** Whenever one or more tracks are selected, a quick bar appears at the bottom center of the viewport to edit layer, height, angle, and accessories in one click."
                },
            );
        }

        // 3: Print & Export
        3 => {
            render_section_header(
                ui,
                p,
                if is_de { "Ebenengetrennter PDF-Export & Bauplan-Druck (Strg+P)" } else { "Layer-Separated PDF Export & Blueprints (Ctrl+P)" },
                if is_de {
                    "Railbench exportiert echte ebenengetrennte Bauplaene sowie verlustfreie Projekt- und Stuecklisten-Dateien fuer den schrittweisen Anlagenbau."
                } else {
                    "Railbench exports true layer-separated vector blueprints alongside lossless project backups and parts lists designed for real layout construction."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "1. Eine Seite je Ebene (PDF)",
                        body: "Jede Ebene wird auf einem eigenen vektorisierten PDF-Blatt ausgegeben (`Uebersicht`, pro Ebene ein Blatt und abschliessende Stueckliste).",
                    },
                    HelpCard {
                        title: "2. Exakt identischer Massstab",
                        body: "Alle Ebenenblaetter teilen sich exakt denselben Ursprung, Massstab und Rahmen - so liegen Brueckenpfeiler und Wendeln deckungsgleich uebereinander.",
                    },
                    HelpCard {
                        title: "3. Abgeblendete Referenzkonturen (Ghosting)",
                        body: "Auf Wunsch werden Nachbarebenen zart angedeutet, um Ueberfahrten, Tunnelportale und Stuetzen sofort ablesen zu koennen.",
                    },
                    HelpCard {
                        title: "4. SVG, XML, CSV & Projektformat",
                        body: "Unterstuetzt `.railbench` (JSON), `.xml`, massstabstreues `1:1 mm SVG` (inkl. Raster & Elektrik-Symbolen) und Semikolon-getrennte **Stuecklisten-CSV (`Strg+E`)**.",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "1. One Sheet Per Layer (Vector PDF)",
                        body: "Exports a clean multi-page vector PDF containing a full overview page, dedicated sheet per layer, and final Bill of Materials.",
                    },
                    HelpCard {
                        title: "2. Identical Bounding Box & Scale",
                        body: "Every exported deck sheet shares the exact same world origin and scale so bridge piers, tunnel portals, and helix overlaps align perfectly.",
                    },
                    HelpCard {
                        title: "3. Ghosted Reference Outlines",
                        body: "Optionally overlay faint dimmed outlines of adjacent layers on each page for clear vertical spatial reference.",
                    },
                    HelpCard {
                        title: "4. 1:1 SVG, XML & BOM CSV",
                        body: "Export lossless `.railbench`, interoperable `.xml`, 1:1 mm vector `.svg` blueprints, and Excel-ready **BOM CSV (`Ctrl+E`)** with track totals.",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);
            render_callout(
                ui,
                p,
                dark,
                if is_de {
                    "**So exportieren Sie:** Druecken Sie `Strg+P`, waehlen Sie die gewuenschten Ebenen & Optionen im Dialog aus und exportieren Sie direkt als PDF oder SVG."
                } else {
                    "**How to Export / Print:** Press `Ctrl+P`, select layers, ghosting, and paper options in the Blueprint dialog, and save directly to PDF or SVG."
                },
            );
        }

        // 4: Voltage Simulation
        4 => {
            render_section_header(
                ui,
                p,
                if is_de { "Spannungsabfall, Kurzschluss & Traktionsstrom" } else { "Voltage Drop, Short-Circuit & Traction Simulation" },
                if is_de {
                    "Pruefen Sie Spannungseinbrueche, getrennte Blockabschnitte, 2-Leiter-Kehrschleifen und Oberleitungs-Speisekreise in Echtzeit."
                } else {
                    "Verify reliable locomotive voltage, isolated block sections, 2-wire reversing loop short circuits, and overhead catenary feeds in real time."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "Farb-Heatmap (Taste P)",
                        body: "Taste `P` blendet die Spannungs-Heatmap ein:\n- **Gruen (> 92%)**: Optimale Spannung\n- **Gelb/Orange (> 80%)**: Maessiger Abfall\n- **Rot (< 80%)**: Kritisch - Einspeisung ergaenzen!\n- **Grau/Rot markiert**: Stromlos oder **2-Leiter-Kurzschluss** (rote Bake in 3D!)",
                    },
                    HelpCard {
                        title: "Einspeisepunkte (F / Shift+F)",
                        body: "Gleis markieren und `F` druecken, um einen Schienen-Einspeisepunkt zu setzen (`Shift+F` fuer Oberleitung). Das Netz wird sofort neu berechnet.",
                    },
                    HelpCard {
                        title: "Trennstellen & Isolatoren (I / Shift+I)",
                        body: "`I` setzt einen Schienen-Isolierstoss (`Shift+I` in der Oberleitung) zur Trennung von Boosterkreisen, Kehrschleifen und Blockabschnitten.",
                    },
                    HelpCard {
                        title: "Spannung, Loks & Weichen-Zubehoer",
                        body: "Konfigurieren Sie `10V-24V` Presets, Lokstrom (`100-1000 mA`), Zuganzahl, **getrennte/parallele Oberleitung** sowie beleuchtete Weichenlaternen (`W`) & Decoder (`Shift+W`).",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "Color Heatmap (P Key)",
                        body: "Press `P` to toggle the voltage heatmap:\n- **Green (> 92%)**: Optimal voltage\n- **Amber (> 80%)**: Moderate drop\n- **Red (< 80%)**: Critical drop - add feeder!\n- **Disconnected / Short**: Highlighted in 2D and marked with a red beacon in 3D!",
                    },
                    HelpCard {
                        title: "Power Feeders (F / Shift+F)",
                        body: "Select any track and press `F` to attach a rail power feeder (`Shift+F` for overhead catenary feeder). Network voltage recomputes live.",
                    },
                    HelpCard {
                        title: "Track Isolators (I / Shift+I)",
                        body: "Press `I` to place a rail isolator (`Shift+I` for catenary insulator) for booster districts, reversing loop isolation, and signal blocks.",
                    },
                    HelpCard {
                        title: "Voltage, Locos & Turnout Accessories",
                        body: "Configure `10V-24V` presets, per-loco load (`100-1000 mA`), train count, separate/parallel catenary mode, and lighted turnout lanterns (`W`) & decoders (`Shift+W`).",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);
        }

        // 5: Baseboards & Terrain
        5 => {
            render_section_header(
                ui,
                p,
                if is_de { "Grundplatte, Ebenenplatten, CAD-Sketcher & Gelaende" } else { "Table, Layer Plates, 2D CAD Sketcher & Terrain" },
                if is_de {
                    "Planen Sie Ihre Anlage passgenau fuer Ihren Raum - mit parametrischen Tischformen, interaktivem 2D-CAD-Sketcher und 3D-Gelaende-Editor."
                } else {
                    "Design your benchwork to match your room using modular table shapes, the interactive 2D CAD Sketcher, and the 3D Terrain Editor."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "Mehr-Tisch Formen & CAD-Sketcher",
                        body: "Kombinieren Sie mehrere Tische (`Rechteck`, `L-Form`, `U-Form`, `T-Form`, `Dreieck`) oder zeichnen Sie freie Grund- und Ebenenplatten per **Polygon-Stift**, **Rechteck** und **Ecken-Editor**.",
                    },
                    HelpCard {
                        title: "Gelaende- & Berg-Editor (Auto-Tunnel)",
                        body: "Modellieren Sie 3D-Berge mit **Berg-** & **Grund-Pinseln** (`Heben`, `Glaetten`, `Plateau`, `Wasser/Sand/Erde`) oder Berg-Vorlagen. Gleise im Berg erhalten automatisch 3D-Tunnelroehren und Portale.",
                    },
                    HelpCard {
                        title: "Holzarten & Trassenbretter",
                        body: "Waehlen Sie Texturen wie **Birke hell**, **Nussbaum dunkel**, **Multiplex**, **Schiefer**, **Grasmatte** oder **CAD-Weiss**. Ebenen-Platten schneiden Steigungs-Trassen automatisch frei.",
                    },
                    HelpCard {
                        title: "Platte anpassen & Lineale (Shift+L)",
                        body: "Klicken Sie im `Platte`-Reiter auf **Platte an Gleise anpassen**, um die kompakteste Tischgroesse automatisch zu berechnen. `Shift+L` schaltet die Rand-Lineale um.",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "Modular Tables & 2D CAD Sketcher",
                        body: "Combine multiple tables (`Rectangle`, `L-Shape`, `U-Shape`, `T-Shape`, `Triangle`) or sketch freeform tables and layer sub-plates using `Polygon Pen`, `Rectangle`, and `Vertex Edit`.",
                    },
                    HelpCard {
                        title: "Terrain & Mountain Editor (Auto-Tunnels)",
                        body: "Sculpt 3D mountains and ground surfaces with brushes (`Mountain Raise/Smooth/Flatten`, `Water/Sand/Dirt/Hills`) or presets. Intersecting tracks automatically generate 3D tunnel portals.",
                    },
                    HelpCard {
                        title: "Wood Finishes & Contoured Sub-Plates",
                        body: "Choose **Natural Birch**, **Dark Walnut**, **Plywood**, **Slate**, **Grass Mat**, or **Drafting White**, with automatic table cutouts for sloping tracks.",
                    },
                    HelpCard {
                        title: "Fit Table to Tracks & Rulers (Shift+L)",
                        body: "Click **Fit Table to Tracks** in the Board tab to compute tight benchwork bounds with margin. Toggle millimeter edge rulers anytime with `Shift+L`.",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);
        }

        // 6: 3D View
        6 => {
            render_section_header(
                ui,
                p,
                if is_de { "3D-Perspektive, ViewCube, Zuege 3D & 3D Bearbeiten" } else { "3D View, CAD ViewCube, Trains 3D & 3D Editing" },
                if is_de {
                    "Der native `wgpu` 3D-Viewer rendert Gleise, Bruecken, Tunnel, Oberleitung, Zuege und Gebaeude in Echtzeit mit 4x MSAA und Tag/Nacht-Zyklus."
                } else {
                    "The native `wgpu` 3D viewer renders tracks, bridges, tunnels, catenary, locomotives, and scenery in real time with 4x MSAA and a 24h Day/Night cycle."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "Volle 360°/180° Kamera & CAD ViewCube",
                        body: "- **Linke Maustaste**: Freie Rundumsicht inkl. Blick **unter den Tisch** fuer Schattenbahnhoefe.\n- **Rechte/Mittlere Maustaste**: Verschieben (Pan) · **Mausrad / +/-**: Zoom.\n- **ViewCube**: Klicken Sie direkt auf `Vorne`, `Hinten`, `Links`, `Rechts`, `Oben`, `Unten` oder die Isometrie-Ecken.",
                    },
                    HelpCard {
                        title: "Direkt in 3D bearbeiten (`3D Bearbeiten`)",
                        body: "Aktivieren Sie **3D Bearbeiten** in der 3D-Leiste (im `CAD / Minimal`- oder `Halle`-Modus): Gleise und Modelle direkt anklicken, per `XYZ`-Gizmo verschieben, mit `Alt+Ziehen` heben und per `R` / `Entf` / `Auto-Luecke` / `Parallelgleis` bearbeiten.",
                    },
                    HelpCard {
                        title: "3D Zug-Simulation (`Zuege`)",
                        body: "Starten Sie den separaten Zug-Modus mit **BR 243 (E-Lok)**, **BR 96 Mallet (Dampflok)** und **V 60 (Diesellok)**. Steuern mit `W`/`S` oder `Auf`/`Ab`, Bremsen mit `Leertaste`, naechste Weiche schalten mit `T` (sichtbar an jeder Weichenlaterne!).",
                    },
                    HelpCard {
                        title: "Integrierte 3D-Modelle & .glb / .gltf Import",
                        body: "16 prozedurale 3D-Gebaeude & Betriebswerke (`Bekohlung`, `Dieseltankstelle`, `Ringlokschuppen`, `Drehscheibe`, `Schiebebuehne`, `Bahnhof`, `Stellwerk`, `Signale`, `Baeume`) plus Drag & Drop Import eigener `.glb` / `.gltf` Dateien.",
                    },
                    HelpCard {
                        title: "Umgebungsmodi & 24h Tag/Nacht-Zyklus",
                        body: "Wechseln Sie oben links zwischen **Halle & Landschaft** und **CAD / Minimal** und steuern Sie den dynamischen 24h-Sonnenlauf samt beleuchteten Fenstern, Signalen und Weichenlaternen.",
                    },
                    HelpCard {
                        title: "Zurueck zur 2D-Ansicht",
                        body: "Druecken Sie jederzeit `3`, `V` oder `Esc`, um nahtlos zwischen 3D-Ansicht und 2D-CAD-Planung umzuschalten.",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "Full 360/180 Camera & CAD ViewCube",
                        body: "- **Left Drag**: Free orbit anywhere, including **underneath the baseboard** for staging yards.\n- **Right / Middle Drag**: Pan · **Wheel / +/-**: Zoom.\n- **ViewCube**: Click `Front`, `Back`, `Left`, `Right`, `Top`, `Bottom`, or isometric corners.",
                    },
                    HelpCard {
                        title: "Direct 3D Editing (`Edit 3D`)",
                        body: "Toggle **Edit 3D** in the 3D top HUD bar (works in `CAD / Minimal` and `Hall`): pick tracks/models in 3D, drag on the table plane, `Alt+Drag` height, or run `R`, `Del`, Close Gap, and Parallel Tracks.",
                    },
                    HelpCard {
                        title: "3D Train Simulation (`Trains`)",
                        body: "Drive detailed **BR 243**, **BR 96 Mallet**, and **V 60** locomotives across your layout (`W`/`S` or `Up`/`Down` throttle, `Space` brake, `T` switches nearest turnout and updates every 3D turnout lantern aspect!).",
                    },
                    HelpCard {
                        title: "Built-in Scenery & Custom .glb / .gltf",
                        body: "Includes 16 procedural scale-aware models (Coaling Station, Diesel Fueling, Roundhouse with stall snap, Turntable, Transfer Table, Station, Signal Box, Trees) plus `.glb`/`.gltf` import.",
                    },
                    HelpCard {
                        title: "Environments & 24h Day/Night Cycle",
                        body: "Switch between **Hall & Landscape** and **CAD / Minimal**, and scrub the 24-hour sun/moon slider to inspect night-lit windows, signals, and equipped turnout lanterns.",
                    },
                    HelpCard {
                        title: "Return to 2D Plan",
                        body: "Press `3`, `V`, or `Esc` anytime to switch smoothly back to 2D CAD drafting.",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);
            render_callout(
                ui,
                p,
                dark,
                if is_de {
                    "**Weichenlaternen:** Jede Weiche zeigt automatisch eine mechanische DB-Weichenlaterne zur Simulation der Weichenstellung (unbeleuchtet, 0 mA). Erst mit montierter **Laterne (`W`)** leuchtet sie und zaehlt in der Stromsimulation mit."
                } else {
                    "**Turnout Lanterns:** Every turnout shows a mechanical DB position lantern so switch alignment is always visible (unlit, 0 mA). Equipping **Lantern (`W`)** lights it up and includes its current draw in the power simulation."
                },
            );
        }

        // 7: Multi-User & Server
        7 => {
            render_section_header(
                ui,
                p,
                if is_de { "Multi-User Zusammenarbeit & Server-Autostart" } else { "Multi-User Collaboration & Server Auto-Start" },
                if is_de {
                    "Arbeiten Sie mit mehreren Personen gleichzeitig ueber LAN oder Internet per WebSocket an einem gemeinsamen Gleisplan - kompatibel mit `node server.mjs` und dem eingebetteten Rust-Server."
                } else {
                    "Collaborate simultaneously over LAN or Internet via WebSocket on the same layout - fully compatible between the embedded Rust host server and `node server.mjs`."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "1. Echtzeit-Synchronisation",
                        body: "Gleisplatzierungen, Verschieben, Drehungen, Hoehenrampen und Grundplatten werden sofort an alle verbundenen Planer synchronisiert.",
                    },
                    HelpCard {
                        title: "2. Live-Auswahl, Benutzer & Chat",
                        body: "Jeder Teilnehmer besitzt Name & Farbe; aktuelle Teilnehmerliste, Markierungen und Raum-Chat stehen direkt im Sync-Dialog zur Verfuegung.",
                    },
                    HelpCard {
                        title: "3. Passwort-Schutz & Raeume",
                        body: "Raeume koennen optional mit einem Passwort gesichert werden; unbefugte Clients ohne Kennwort werden abgewiesen.",
                    },
                    HelpCard {
                        title: "4. Automatisches Speichern",
                        body: "Der integrierte Server sichert Raumstaende automatisch im Verzeichnis `~/.railbench/saved_plans/<raum>.railbench`.",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "1. Real-Time Synchronization",
                        body: "Track placements, drags, rotations, gradient ramps, and baseboard edits sync immediately across all connected peers.",
                    },
                    HelpCard {
                        title: "2. Live Presence, Selections & Chat",
                        body: "Each participant picks a display name and color; active peer selections and live room chat update without blocking your canvas.",
                    },
                    HelpCard {
                        title: "3. Room Passwords & Multi-Room Hosting",
                        body: "Secure rooms with an optional password so only authorized collaborators can join and edit the layout.",
                    },
                    HelpCard {
                        title: "4. Automatic Persistence",
                        body: "The embedded server persists room layouts automatically to `~/.railbench/saved_plans/<room>.railbench`.",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);

            ui.add_space(4.0);
            ui.label(
                RichText::new(if is_de {
                    "Startvariablen fuer Server-Autostart (Kommandozeile)"
                } else {
                    "CLI Flags for Collaboration Server & Auto-Connect"
                })
                .strong()
                .size(14.0)
                .color(p.text_main),
            );
            ui.add_space(6.0);
            let cli_rows = if is_de {
                vec![
                    ("--server / --collab-server".into(), "aus".into(), "Startet den eingebetteten WebSocket-Server beim Programmstart.".into()),
                    ("--port <Port>".into(), "3000".into(), "Legt den TCP-Port fest (z. B. `--port 8080`, Auto-Fallback +20).".into()),
                    ("--password <PW>".into(), "keines".into(), "Passwort fuer den Standardraum festlegen.".into()),
                    ("--room <Name>".into(), "default".into(), "Namen des Startraums festlegen.".into()),
                    ("--connect <URL>".into(), "aus".into(), "Verbindet den Client beim Start direkt mit einem WebSocket-Server.".into()),
                ]
            } else {
                vec![
                    ("--server / --collab-server".into(), "off".into(), "Automatically starts the embedded WebSocket server on launch.".into()),
                    ("--port <Port>".into(), "3000".into(), "Sets server TCP port (e.g. `--port 8080`, auto-fallback up to +20).".into()),
                    ("--password <PW>".into(), "none".into(), "Sets room password requirement.".into()),
                    ("--room <Name>".into(), "default".into(), "Sets active collaborative room name.".into()),
                    ("--connect <URL>".into(), "off".into(), "Auto-connects the client to the given `ws://...` server on startup.".into()),
                ]
            };
            render_three_col_table(
                ui,
                p,
                dark,
                if is_de {
                    ["Startflag", "Standard", "Bedeutung / Funktion"]
                } else {
                    ["CLI Flag", "Default", "Description"]
                },
                &cli_rows,
                [0.34, 0.18, 0.48],
                false,
            );

            ui.add_space(12.0);
            ui.label(
                RichText::new(if is_de { "Umgebungsvariablen" } else { "Environment Variables" })
                    .strong()
                    .size(13.5)
                    .color(p.text_main),
            );
            ui.add_space(6.0);
            let env_rows = if is_de {
                vec![
                    ("`RAILBENCH_SERVER`".into(), "`1` / `true`".into(), "Aktiviert Server-Autostart.".into()),
                    ("`RAILBENCH_PORT`".into(), "`3000`".into(), "Konfiguriert den Server-Port.".into()),
                    ("`RAILBENCH_PASSWORD`".into(), "`geheim123`".into(), "Passwort fuer den Raumschutz.".into()),
                    ("`RAILBENCH_ROOM`".into(), "`bergstrecke`".into(), "Standard-Raumnamen setzen.".into()),
                    ("`RAILBENCH_CONNECT`".into(), "`1`".into(), "Automatische Verbindung beim Start.".into()),
                ]
            } else {
                vec![
                    ("`RAILBENCH_SERVER`".into(), "`1` / `true`".into(), "Auto-starts embedded server on launch.".into()),
                    ("`RAILBENCH_PORT`".into(), "`3000`".into(), "Configures the WebSocket server port.".into()),
                    ("`RAILBENCH_PASSWORD`".into(), "`secret123`".into(), "Sets the room password.".into()),
                    ("`RAILBENCH_ROOM`".into(), "`staging-yard`".into(), "Configures default room name.".into()),
                    ("`RAILBENCH_CONNECT`".into(), "`1`".into(), "Auto-connects local client on start.".into()),
                ]
            };
            render_three_col_table(
                ui,
                p,
                dark,
                if is_de {
                    ["Variable", "Beispiel", "Wirkung"]
                } else {
                    ["Variable", "Example", "Effect"]
                },
                &env_rows,
                [0.34, 0.24, 0.42],
                false,
            );
            ui.add_space(10.0);
            render_callout(
                ui,
                p,
                dark,
                if is_de {
                    "**Start-Beispiele:**\n- Native App als Host + Auto-Connect: `railbench-app --server --port 3000 --password geheim --connect`\n- Node.js Server: `node server.mjs --port 8080 --password geheim --room anlage1`"
                } else {
                    "**Quickstart Examples:**\n- Native App Hosting + Connect: `railbench-app --server --port 3000 --password secret --connect`\n- Standalone Node.js Server: `node server.mjs --port 8080 --password secret --room yard1`"
                },
            );
        }

        // 8: Keyboard Shortcuts
        8 => {
            ui.horizontal_top(|ui| {
                let avail = ui.available_width();
                let btn_w = 172.0_f32;
                let left_w = (avail - btn_w - 10.0).max(160.0);
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(left_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(left_w);
                        ui.label(
                            RichText::new(if is_de { "Tastaturkuerzel-Referenz" } else { "Keyboard Shortcuts Reference" })
                                .strong()
                                .size(18.0)
                                .color(p.text_main),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(if is_de {
                                    "Nutzen Sie diese Kurzbefehle fuer schnelles und fluessiges Arbeiten:"
                                } else {
                                    "Master these shortcuts for rapid layout design and editing:"
                                })
                                .size(12.5)
                                .color(p.text_sub),
                            )
                            .wrap(),
                        );
                    },
                );
                let btn = egui::Button::new(
                    RichText::new(if is_de {
                        "Tastaturkuerzel anpassen..."
                    } else {
                        "Customize Shortcuts..."
                    })
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
                )
                .fill(p.primary_hover)
                .corner_radius(CornerRadius::same(6))
                .min_size(egui::vec2(btn_w, 28.0));
                if ui.add(btn).clicked() {
                    *open_keymap = true;
                }
            });
            ui.add_space(6.0);
            let (line_r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 2.0), egui::Sense::hover());
            ui.painter().rect_filled(line_r, 0.0, p.border_main);
            ui.add_space(10.0);

            let mut sc_rows: Vec<(String, String, String)> = KEY_ACTION_DEFS
                .iter()
                .map(|def| {
                    let key = app.app_settings.effective_key(def.action);
                    let lbl = if is_de { def.label_de } else { def.label_en };
                    let cat = match def.category {
                        KeyCategory::File => if is_de { "Datei" } else { "File" },
                        KeyCategory::Edit => if is_de { "Bearbeiten" } else { "Edit" },
                        KeyCategory::View => if is_de { "Ansicht" } else { "View" },
                        KeyCategory::Tools => if is_de { "Werkzeuge" } else { "Tools" },
                    };
                    (key, format!("**{lbl}**"), cat.to_string())
                })
                .collect();

            if is_de {
                sc_rows.push(("Shift+Mausrad".into(), "Gleis unter Cursor stufenlos drehen".into(), "Bearbeiten".into()));
                sc_rows.push(("Rechts / Mittel + Ziehen".into(), "Arbeitsflaeche verschieben (Pan)".into(), "Navigation".into()));
                sc_rows.push(("Strg / Shift + Klick".into(), "Auswahl erweitern / umschalten".into(), "Auswahl".into()));
            } else {
                sc_rows.push(("Shift+Wheel".into(), "Rotate track smoothly with scroll wheel".into(), "Editing".into()));
                sc_rows.push(("Right / Middle + Drag".into(), "Pan canvas or 3D viewport".into(), "Navigation".into()));
                sc_rows.push(("Ctrl / Shift + Click".into(), "Add or toggle item in selection".into(), "Selection".into()));
            }

            render_three_col_table(
                ui,
                p,
                dark,
                if is_de {
                    ["Kuerzel", "Aktion", "Kategorie"]
                } else {
                    ["Shortcut", "Action", "Category"]
                },
                &sc_rows,
                [0.27, 0.48, 0.25],
                true,
            );
        }

        // 9: Track Catalog JSON
        9 => {
            render_section_header(
                ui,
                p,
                if is_de { "Gleiskatalog-Format & JSON-Beispiel (Spur H0)" } else { "Track Catalog Format & JSON Example (H0 Scale)" },
                if is_de {
                    "Railbench unterstuetzt das Importieren eigener Gleisbibliotheken ueber **Anlage -> Gleiskatalog importieren (.json)...**. Eigene Kataloge bleiben dauerhaft getrennt vom eingebauten Katalog gespeichert."
                } else {
                    "Import custom track libraries via **Layout -> Import Track Catalog (.json)...**. Custom catalogs persist cleanly as user overrides without overwriting built-in data."
                },
            );
            let cards = if is_de {
                vec![
                    HelpCard {
                        title: "Katalog-Kopfdaten",
                        body: "- `scale`: Nenngroesse (`\"H0\"`, `\"TT\"`, `\"N\"`, `\"Z\"`, `\"1\"`, `\"G\"`, `\"0\"`)\n- `gauge`: Spurweite in mm (`16.5`, `12.0`, `9.0`, `6.5`, `45.0`, `32.0`)\n- `hasRoadbed`: `true` fuer Bettungsgleis\n- `conductors`: `2` (Zweileiter) oder `3` (Dreileiter)",
                    },
                    HelpCard {
                        title: "Unterstuetzte Gleistypen (17 Typen)",
                        body: "`straight`, `curve`, `turnout-left`, `turnout-right`, `curved-turnout-left`, `curved-turnout-right`, `three-way`, `double-slip`, `crossing`, `uncoupler`, `buffer-stop`, `feeder`, `isolating`, `flex`, `turntable`, `transfer-table`, `accessory`.",
                    },
                ]
            } else {
                vec![
                    HelpCard {
                        title: "Catalog Header Properties",
                        body: "- `scale`: Scale name (`\"H0\"`, `\"TT\"`, `\"N\"`, `\"Z\"`, `\"1\"`, `\"G\"`, `\"0\"`)\n- `gauge`: Track gauge in mm (`16.5`, `12.0`, `9.0`, `6.5`, `45.0`, `32.0`)\n- `hasRoadbed`: `true` for roadbed ballast\n- `conductors`: `2` (2-rail DC) or `3` (3-rail AC)",
                    },
                    HelpCard {
                        title: "Supported Track Types (17 Kinds)",
                        body: "`straight`, `curve`, `turnout-left`, `turnout-right`, `curved-turnout-left`, `curved-turnout-right`, `three-way`, `double-slip`, `crossing`, `uncoupler`, `buffer-stop`, `feeder`, `isolating`, `flex`, `turntable`, `transfer-table`, `accessory`.",
                    },
                ]
            };
            render_card_grid(ui, p, dark, &cards);

            ui.horizontal_wrapped(|ui| {
                let copy_btn = egui::Button::new(
                    RichText::new(if is_de { "JSON kopieren" } else { "Copy JSON" })
                        .strong()
                        .size(12.0)
                        .color(Color32::WHITE),
                )
                .fill(p.primary_hover)
                .corner_radius(CornerRadius::same(6));
                if ui.add(copy_btn).clicked() {
                    ui.ctx().copy_text(EXAMPLE_H0_CATALOG_JSON.to_string());
                    app.show_toast(if is_de {
                        "In Zwischenablage kopiert!"
                    } else {
                        "Copied to clipboard!"
                    });
                }

                let dl_btn = egui::Button::new(
                    RichText::new(if is_de {
                        "JSON herunterladen (railbench-catalog-h0-example.json)"
                    } else {
                        "Download JSON (railbench-catalog-h0-example.json)"
                    })
                    .strong()
                    .size(12.0)
                    .color(p.text_main),
                )
                .fill(surface_color(dark))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .corner_radius(CornerRadius::same(6));
                if ui.add(dl_btn).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("railbench-catalog-h0-example.json")
                        .add_filter("Catalog JSON", &["json"])
                        .save_file()
                    {
                        if std::fs::write(&path, EXAMPLE_H0_CATALOG_JSON).is_ok() {
                            app.show_toast(if is_de {
                                "Beispiel-Katalog gespeichert"
                            } else {
                                "Example catalog saved"
                            });
                        }
                    }
                }

                ui.label(
                    RichText::new(if is_de {
                        "Spur H0 · 16,5 mm · 17 Gleistypen"
                    } else {
                        "Scale H0 · 16.5 mm Gauge · 17 Track Kinds"
                    })
                    .size(11.5)
                    .color(p.text_muted),
                );
            });
            ui.add_space(8.0);

            Frame::new()
                .fill(Color32::from_rgb(15, 23, 42))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(12))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::ScrollArea::both()
                        .id_salt("help_catalog_json_scroll")
                        .max_height(270.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(EXAMPLE_H0_CATALOG_JSON)
                                        .monospace()
                                        .size(11.0)
                                        .color(Color32::from_rgb(226, 232, 240)),
                                )
                                .extend(),
                            );
                        });
                });
        }

        // 10: About & Track Systems
        _ => {
            render_section_header(
                ui,
                p,
                if is_de { "Ueber Railbench & Gleissysteme" } else { "About Railbench & Track Libraries" },
                &if is_de {
                    format!(
                        "**Railbench {}** ist eine Open-Source-Modellbahn-CAD-Anwendung fuer die Planung, 3D-Visualisierung, Zugsimulation und elektrische Netzanalyse (Native Rust + `egui` / `wgpu`).",
                        crate::version::APP_VERSION_TAG
                    )
                } else {
                    format!(
                        "**Railbench {}** is an open-source model railway layout design, 3D visualization, train simulation, and electrical analysis CAD application (Native Rust + `egui` / `wgpu`).",
                        crate::version::APP_VERSION_TAG
                    )
                },
            );

            ui.label(
                RichText::new(if is_de {
                    "Unterstuetzte Nenngroessen & Gleisbibliotheken:"
                } else {
                    "Supported Scale Systems & Catalogs:"
                })
                .strong()
                .size(13.5)
                .color(p.text_main),
            );
            ui.add_space(6.0);

            let systems: [&str; 6] = if is_de {
                [
                    "**H0 (1:87)**: Maerklin C-Gleis, Maerklin K-Gleis, Maerklin M-Gleis",
                    "**H0 (1:87)**: Trix C-Gleis, Piko A-Gleis, Roco Line, Peco Streamline Code 100, Tillig Elite",
                    "**TT (1:120)**: Tillig TT Modellgleis, Tillig TT Bettungsgleis, Kuehn TT Gleis",
                    "**N (1:160)**: Fleischmann Piccolo, Kato Unitrack, Minitrix, Peco Streamline N Code 55",
                    "**Z (1:220)**: Maerklin mini-club, Rokuhan",
                    "**0 / 1 / G**: Lenz Spur 0, Maerklin Spur 1, LGB Spur G + Strassensysteme",
                ]
            } else {
                [
                    "**H0 (1:87)**: Maerklin C-Track, Maerklin K-Track, Maerklin M-Track",
                    "**H0 (1:87)**: Trix C-Track, Piko A-Track, Roco Line, Peco Streamline Code 100, Tillig Elite",
                    "**TT (1:120)**: Tillig TT Model Track, Tillig TT Roadbed Track, Kuehn TT Track",
                    "**N (1:160)**: Fleischmann Piccolo, Kato Unitrack, Minitrix, Peco Streamline N Code 55",
                    "**Z (1:220)**: Maerklin mini-club, Rokuhan",
                    "**0 / 1 / G**: Lenz Scale 0, Maerklin Scale 1, LGB Scale G + Street Systems",
                ]
            };
            for sys in systems {
                ui.horizontal_top(|ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new("•").strong().color(p.primary));
                    rich_label(ui, p, sys, 12.2);
                });
            }

            ui.add_space(10.0);
            render_callout(
                ui,
                p,
                dark,
                if is_de {
                    "**Plattform-Unterstuetzung:** Native Builds fuer **Linux (x86_64)**, **Windows (x86_64 MSVC / GNU)** und **macOS (Apple Silicon ARM64 & Intel x64)** sowie Web/Electron."
                } else {
                    "**Platform Support:** Native builds for **Linux (x86_64)**, **Windows (x86_64 MSVC / GNU)**, and **macOS (Apple Silicon ARM64 & Intel x64)**, alongside Web/Electron."
                },
            );

            ui.label(
                RichText::new(if is_de {
                    "Lizenziert unter der GNU General Public License v3 (GPLv3). Alle eingebauten prozeduralen 3D-Modelle sind quelloffen und frei nutzbar. Alle Gleismarken (Maerklin, Trix, Piko, Roco, Fleischmann, Tillig, Kato, Lenz, LGB) sind Warenzeichen der jeweiligen Hersteller."
                } else {
                    "Licensed under the GNU General Public License v3 (GPLv3). All built-in procedural 3D models are open source and royalty-free. Track product names and brands are trademarks of their respective manufacturers."
                })
                .size(11.0)
                .color(p.text_muted),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_eleven_help_tabs_have_localized_labels() {
        assert_eq!(HELP_TAB_LABELS_DE.len(), 11);
        assert_eq!(HELP_TAB_LABELS_EN.len(), 11);
        assert!(HELP_TAB_LABELS_DE.iter().all(|s| !s.trim().is_empty()));
        assert!(HELP_TAB_LABELS_EN.iter().all(|s| !s.trim().is_empty()));
    }

    #[test]
    fn all_embedded_guide_screenshots_decode_as_valid_images() {
        for kind in GuideImage::ALL {
            let decoded = decode_guide_color_image(kind).expect("guide png decodes");
            assert!(
                decoded.size[0] >= 100 && decoded.size[1] >= 24,
                "unexpected guide dimensions: {:?}",
                decoded.size
            );
        }
    }

    #[test]
    fn example_h0_catalog_json_parses_with_all_17_track_kinds() {
        let v: serde_json::Value = serde_json::from_str(EXAMPLE_H0_CATALOG_JSON).expect("valid catalog json");
        assert_eq!(v["scale"].as_str(), Some("H0"));
        assert_eq!(v["gauge"].as_f64(), Some(16.5));
        let entries = v["entries"].as_array().expect("entries array");
        assert_eq!(entries.len(), 17);
        let kinds: std::collections::HashSet<&str> = entries
            .iter()
            .filter_map(|e| e["kind"].as_str())
            .collect();
        for expected in [
            "straight",
            "curve",
            "turnout-left",
            "turnout-right",
            "curved-turnout-left",
            "curved-turnout-right",
            "three-way",
            "double-slip",
            "crossing",
            "uncoupler",
            "buffer-stop",
            "feeder",
            "isolating",
            "flex",
            "turntable",
            "transfer-table",
            "accessory",
        ] {
            assert!(kinds.contains(expected), "missing catalog example kind: {expected}");
        }
    }

    #[test]
    fn rich_job_formats_inline_code_and_bold_without_raw_markers() {
        let job = build_rich_job(
            "Legt **TCP-Port** fest (z. B. `--port 8080`).",
            12.0,
            Color32::GRAY,
            Color32::WHITE,
            Color32::BLACK,
        );
        assert!(!job.text.contains("**"));
        assert!(!job.text.contains('`'));
        assert!(job.text.contains("TCP-Port"));
        assert!(job.text.contains("--port 8080"));
    }

    #[test]
    fn help_dialog_renders_all_tabs_within_bounds_and_keeps_footer_visible() {
        let mut app = crate::RailbenchApp::default();
        app.show_help_dialog = true;
        let ctx = egui::Context::default();

        for (lang, (vw, vh)) in [
            (crate::i18n::Lang::En, (1280.0_f32, 800.0_f32)),
            (crate::i18n::Lang::De, (800.0_f32, 600.0_f32)),
        ] {
            app.lang = lang;
            let expected_modal_w = 900.0_f32.min((vw * 0.96).max(460.0));
            let expected_modal_h = 640.0_f32.min((vh * 0.90).max(380.0));
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(vw, vh));
            for tab in 0u8..=10 {
                app.help_tab = tab;
                app.show_help_dialog = true;
                for _ in 0..2 {
                    let mut raw = egui::RawInput::default();
                    raw.screen_rect = Some(screen);
                    let _ = ctx.run(raw, |c| draw_help_dialog(c, &mut app));
                }
                assert!(app.show_help_dialog, "dialog stayed open for tab {tab}");
                ctx.memory(|m| {
                    let dlg_rect = m
                        .area_rect(egui::Id::new("railbench_help_dialog"))
                        .expect("help dialog area rect");
                    assert!(
                        dlg_rect.width() <= expected_modal_w + 4.0
                            && dlg_rect.height() <= expected_modal_h + 4.0,
                        "tab {tab} ({vw}x{vh}) exceeded modal bounds ({expected_modal_w}x{expected_modal_h}): {dlg_rect:?}"
                    );
                    assert!(
                        screen.contains_rect(dlg_rect.shrink(1.0)),
                        "tab {tab} escaped viewport {screen:?}: {dlg_rect:?}"
                    );
                    assert_eq!(
                        m.layer_id_at(dlg_rect.center()),
                        Some(egui::LayerId::new(
                            egui::Order::Foreground,
                            egui::Id::new("railbench_help_dialog")
                        )),
                        "dialog must sit above backdrop on tab {tab}"
                    );
                });
            }
        }

        // Escape key closes help dialog
        let mut esc_raw = egui::RawInput::default();
        esc_raw.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 800.0),
        ));
        esc_raw.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = ctx.run(esc_raw, |c| draw_help_dialog(c, &mut app));
        assert!(!app.show_help_dialog);
    }
}
