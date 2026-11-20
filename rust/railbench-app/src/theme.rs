use eframe::egui::{self, Color32, CornerRadius, Margin, Stroke, Vec2};

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
            bg_sidebar: Color32::from_rgb(248, 250, 252),   // #f8fafc
            bg_card: Color32::from_rgb(255, 255, 255),      // #ffffff
            bg_card_hover: Color32::from_rgb(241, 245, 249), // #f1f5f9
            bg_input: Color32::from_rgb(255, 255, 255),     // #ffffff
            text_main: Color32::from_rgb(15, 23, 42),       // #0f172a
            text_sub: Color32::from_rgb(51, 65, 85),        // #334155
            text_muted: Color32::from_rgb(100, 116, 139),   // #64748b
            border_main: Color32::from_rgb(226, 232, 240),  // #e2e8f0
            primary: Color32::from_rgb(2, 132, 199),        // #0284c7
            primary_hover: Color32::from_rgb(3, 105, 161),  // #0369a1
            canvas_bg: Color32::from_rgb(228, 223, 213),    // #e4dfd5
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
    v.selection.stroke = Stroke::new(1.2_f32, p.primary);

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
    v.override_text_color = Some(p.toolbar_text);
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
    v.widgets.active.fg_stroke = Stroke::new(1.4_f32, Color32::WHITE);
}
