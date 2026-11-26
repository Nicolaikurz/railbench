use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Stroke, Vec2};
use crate::icons::{icon_button_sized, paint_chevron, paint_icon, ChevronDir, Icon};

#[derive(Debug, Clone, Copy)]
pub struct RailbenchPalette {
    pub bg_toolbar: Color32,
    pub toolbar_text: Color32,
    pub toolbar_btn: Color32,
    pub toolbar_btn_hover: Color32,
    pub bg_sidebar: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub bg_input: Color32,
    pub text_main: Color32,
    pub text_sub: Color32,
    pub text_muted: Color32,
    pub border_main: Color32,
    pub primary: Color32,
    pub primary_hover: Color32,
    pub canvas_bg: Color32,
}

pub fn palette(dark: bool) -> RailbenchPalette {
    if dark {
        RailbenchPalette {
            bg_toolbar: Color32::from_rgb(4, 7, 17),       // #040711
            toolbar_text: Color32::from_rgb(248, 250, 252), // #f8fafc
            toolbar_btn: Color32::from_rgb(21, 31, 50),     // #151f32
            toolbar_btn_hover: Color32::from_rgb(38, 53, 74), // #26354a
            bg_sidebar: Color32::from_rgb(15, 23, 42),      // #0f172a
            bg_card: Color32::from_rgb(30, 41, 59),         // #1e293b
            bg_card_hover: Color32::from_rgb(38, 53, 74),   // #26354a
            bg_input: Color32::from_rgb(21, 31, 50),        // #151f32
            text_main: Color32::from_rgb(248, 250, 252),    // #f8fafc
            text_sub: Color32::from_rgb(203, 213, 225),     // #cbd5e1
            text_muted: Color32::from_rgb(148, 163, 184),   // #94a3b8
            border_main: Color32::from_rgb(36, 52, 77),     // #24344d
            primary: Color32::from_rgb(56, 189, 248),       // #38bdf8
            primary_hover: Color32::from_rgb(2, 132, 199),  // #0284c7
            canvas_bg: Color32::from_rgb(17, 23, 38),       // #111726
        }
    } else {
        RailbenchPalette {
            bg_toolbar: Color32::from_rgb(30, 41, 59),      // #1e293b (dark header like TS UI!)
            toolbar_text: Color32::from_rgb(248, 250, 252), // #f8fafc
            toolbar_btn: Color32::from_rgb(51, 65, 85),     // #334155
            toolbar_btn_hover: Color32::from_rgb(71, 85, 105), // #475569
            bg_sidebar: Color32::from_rgb(245, 240, 230),   // #f5f0e6 (warm crema)
            bg_card: Color32::from_rgb(250, 246, 238),      // #faf6ee (soft crema card)
            bg_card_hover: Color32::from_rgb(238, 231, 217), // #eee7d9
            bg_input: Color32::from_rgb(252, 249, 242),     // #fcf9f2
            text_main: Color32::from_rgb(32, 28, 24),       // #201c18
            text_sub: Color32::from_rgb(76, 68, 58),        // #4c443a
            text_muted: Color32::from_rgb(122, 112, 98),    // #7a7062
            border_main: Color32::from_rgb(218, 209, 194),  // #dad1c2
            primary: Color32::from_rgb(2, 132, 199),        // #0284c7
            primary_hover: Color32::from_rgb(3, 105, 161),  // #0369a1
            canvas_bg: Color32::from_rgb(236, 229, 214),    // #ece5d6 (warm crema canvas)
        }
    }
}

pub fn apply_railbench_theme(ctx: &egui::Context, dark: bool) {
    let p = palette(dark);
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = Vec2::new(7.0, 6.0);
    style.spacing.button_padding = Vec2::new(9.0, 5.0);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.menu_margin = Margin::same(6);

    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    v.panel_fill = p.bg_sidebar;
    v.window_fill = p.bg_card;
    v.faint_bg_color = p.bg_input;
    v.extreme_bg_color = p.bg_input;
    v.override_text_color = Some(p.text_main);
    v.window_corner_radius = CornerRadius::same(10);
    v.window_stroke = Stroke::new(1.0_f32, p.border_main);
    v.menu_corner_radius = CornerRadius::same(8);

    v.selection.bg_fill = Color32::from_rgba_unmultiplied(p.primary.r(), p.primary.g(), p.primary.b(), 55);
    v.selection.stroke = Stroke::new(1.2_f32, if dark { p.primary } else { p.text_main });

    v.widgets.noninteractive.bg_fill = p.bg_sidebar;
    v.widgets.noninteractive.weak_bg_fill = p.bg_input;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border_main);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_sub);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    v.widgets.inactive.bg_fill = p.bg_card;
    v.widgets.inactive.weak_bg_fill = p.bg_card;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_main);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_main);
    v.widgets.inactive.corner_radius = CornerRadius::same(6);

    v.widgets.hovered.bg_fill = p.bg_card_hover;
    v.widgets.hovered.weak_bg_fill = p.bg_card_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.primary);
    v.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, p.text_main);
    v.widgets.hovered.corner_radius = CornerRadius::same(6);

    v.widgets.active.bg_fill = p.primary_hover;
    v.widgets.active.weak_bg_fill = p.primary_hover;
    v.widgets.active.bg_stroke = Stroke::new(1.2_f32, p.primary);
    v.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(6);

    style.visuals = v;
    ctx.set_style(style);
}

/// Applies the dark RAILbench toolbar visuals inside a `TopBottomPanel` (even in light mode!).
pub fn style_toolbar_ui(ui: &mut egui::Ui, p: RailbenchPalette) {
    let v = ui.visuals_mut();
    let bar_sep = Color32::from_rgba_unmultiplied(148, 163, 184, 42);
    let popup_border = Color32::from_rgba_unmultiplied(148, 163, 184, 56);
    v.panel_fill = p.bg_toolbar;
    v.window_fill = p.bg_toolbar;
    v.window_stroke = Stroke::new(1.0_f32, popup_border);
    v.faint_bg_color = p.toolbar_btn;
    v.extreme_bg_color = p.bg_toolbar;
    v.override_text_color = Some(p.toolbar_text);

    v.widgets.noninteractive.bg_fill = p.bg_toolbar;
    v.widgets.noninteractive.weak_bg_fill = p.bg_toolbar;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, bar_sep);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.toolbar_text);

    v.widgets.inactive.bg_fill = p.toolbar_btn;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.toolbar_text);

    v.widgets.hovered.bg_fill = p.toolbar_btn_hover;
    v.widgets.hovered.weak_bg_fill = p.toolbar_btn_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.primary);
    v.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, Color32::WHITE);

    v.widgets.active.bg_fill = p.primary_hover;
    v.widgets.active.weak_bg_fill = p.primary_hover;
    v.widgets.active.bg_stroke = Stroke::new(1.2_f32, p.primary);
    v.widgets.active.fg_stroke = Stroke::new(1.4_f32, Color32::WHITE);

    v.widgets.open.bg_fill = p.toolbar_btn_hover;
    v.widgets.open.weak_bg_fill = p.toolbar_btn_hover;
    v.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.primary);
    v.widgets.open.fg_stroke = Stroke::new(1.2_f32, Color32::WHITE);
}

pub fn railbench_window_frame(dark: bool) -> Frame {
    let p = palette(dark);
    Frame::new()
        .fill(p.bg_sidebar)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(0))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 135 } else { 52 }),
        })
}

/// Renders the frameless OS window controls (Close, Maximize/Restore, Minimize) right-to-left.
/// Returns `true` if the Close button was clicked.
pub fn draw_os_window_controls(ui: &mut egui::Ui, ctx: &egui::Context, p: RailbenchPalette) -> bool {
    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let win_sz = Vec2::new(28.0, 22.0);
    let mut closed = false;

    let close_resp = icon_button_sized(
        ui,
        Icon::WinClose,
        win_sz,
        false,
        Color32::TRANSPARENT,
        Color32::from_rgb(220, 38, 38),
        Color32::from_rgb(239, 68, 68),
        Color32::from_rgb(226, 232, 240),
        "Close",
    );
    if close_resp.hovered() {
        ui.painter()
            .rect_filled(close_resp.rect, CornerRadius::same(6), Color32::from_rgb(220, 38, 38));
        paint_icon(
            ui.painter(),
            close_resp.rect.shrink2(Vec2::new(7.0, 4.0)),
            Icon::WinClose,
            Color32::WHITE,
        );
    }
    if close_resp.clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        closed = true;
    }

    let max_icon = if is_maximized {
        Icon::WinRestore
    } else {
        Icon::WinMaximize
    };
    let max_tip = if is_maximized { "Restore" } else { "Maximize" };
    if icon_button_sized(
        ui,
        max_icon,
        win_sz,
        false,
        Color32::TRANSPARENT,
        p.toolbar_btn_hover,
        p.primary,
        Color32::from_rgb(226, 232, 240),
        max_tip,
    )
    .clicked()
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
    }

    if icon_button_sized(
        ui,
        Icon::WinMinimize,
        win_sz,
        false,
        Color32::TRANSPARENT,
        p.toolbar_btn_hover,
        p.primary,
        Color32::from_rgb(226, 232, 240),
        "Minimize",
    )
    .clicked()
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    closed
}

pub fn draw_window_header_bar(
    ui: &mut egui::Ui,
    p: RailbenchPalette,
    title: &str,
    open: Option<&mut bool>,
    collapsed: Option<&mut bool>,
) {
    let header_resp = Frame::new()
        .fill(p.bg_toolbar)
        .corner_radius(CornerRadius {
            nw: 10,
            ne: 10,
            sw: 0,
            se: 0,
        })
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            style_toolbar_ui(ui, p);
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (bar_rect, _) =
                    ui.allocate_exact_size(Vec2::new(3.0, 14.0), egui::Sense::hover());
                ui.painter().rect_filled(bar_rect, 1.5, p.primary);
                ui.label(RichText::new(title).strong().size(12.5).color(Color32::WHITE));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(open_flag) = open {
                        let close_sz = Vec2::new(26.0, 22.0);
                        let close_resp = icon_button_sized(
                            ui,
                            Icon::WinClose,
                            close_sz,
                            false,
                            Color32::TRANSPARENT,
                            Color32::from_rgb(220, 38, 38),
                            Color32::from_rgb(239, 68, 68),
                            Color32::from_rgb(226, 232, 240),
                            "Close",
                        );
                        if close_resp.hovered() {
                            ui.painter().rect_filled(
                                close_resp.rect,
                                CornerRadius::same(6),
                                Color32::from_rgb(220, 38, 38),
                            );
                            paint_icon(
                                ui.painter(),
                                close_resp.rect.shrink2(Vec2::new(6.0, 4.0)),
                                Icon::WinClose,
                                Color32::WHITE,
                            );
                        }
                        if close_resp.clicked() {
                            *open_flag = false;
                        }
                    }
                    if let Some(is_col) = collapsed {
                        let (btn_rect, btn_resp) =
                            ui.allocate_exact_size(Vec2::new(24.0, 22.0), egui::Sense::click());
                        if btn_resp.hovered() {
                            ui.painter().rect_filled(
                                btn_rect,
                                CornerRadius::same(6),
                                p.toolbar_btn_hover,
                            );
                        }
                        paint_chevron(
                            ui.painter(),
                            btn_rect.center(),
                            if *is_col {
                                ChevronDir::Right
                            } else {
                                ChevronDir::Down
                            },
                            3.5,
                            Color32::from_rgb(226, 232, 240),
                        );
                        if btn_resp.clicked() {
                            *is_col = !*is_col;
                        }
                    }
                });
            });
        });
    let rect = header_resp.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.max.y,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 38)),
    );
}

pub fn show_railbench_dialog_body<R>(
    ui: &mut egui::Ui,
    dark: bool,
    title: &str,
    open: Option<&mut bool>,
    body_margin: i8,
    add_body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let p = palette(dark);
    let prev_y = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    draw_window_header_bar(ui, p, title, open, None);
    let inner = Frame::new()
        .inner_margin(Margin::same(body_margin))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = prev_y;
            add_body(ui)
        });
    ui.spacing_mut().item_spacing.y = prev_y;
    inner.inner
}
