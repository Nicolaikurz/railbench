use std::collections::BTreeMap;

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Stroke};
use railbench_core::types::{Elevation, Layer};

use crate::board_tools::{self, SketchMode};
use crate::canvas2d::{parse_hex_color, ColorMode};
use crate::export::get_piece_list_summary;
use crate::i18n::{tr, Lang, DE, EN};
use crate::icons::{icon_button_sized, paint_chevron, paint_icon, ChevronDir, Icon};
use crate::model_tools;
use crate::settings::{
    capture_pressed_shortcut, KeyCapture, KeyCategory, StartupAction, KEY_ACTION_DEFS,
};
use crate::terrain_tools::{self, TerrainBrushMode};
use crate::theme::{palette, style_toolbar_ui};
use crate::{RailbenchApp, SidebarTab};

/// Uniform height (px) shared by every control in the quick toolbar row.
const TB_H: f32 = 26.0;
pub(super) const LEFT_SIDEBAR_MIN_W_KEY: &str = "left_sidebar_min_w";
pub(super) const LEFT_SIDEBAR_ACTUAL_W_KEY: &str = "left_sidebar_actual_w";

pub(crate) fn stable_text_width(
    ui: &egui::Ui,
    de: &str,
    en: &str,
    size: Option<f32>,
    strong: bool,
) -> f32 {
    let measure = |s: &str| {
        let mut rt = RichText::new(s);
        if let Some(sz) = size {
            rt = rt.size(sz);
        }
        if strong {
            rt = rt.strong();
        }
        let wt: egui::WidgetText = rt.into();
        wt.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        )
        .size()
        .x
    };
    measure(de).max(measure(en)).ceil()
}

pub(crate) fn format_undo_redo_tooltip(lang: Lang, is_redo: bool, count: usize) -> String {
    let t = tr(lang);
    let (label, shortcut) = match (lang, is_redo) {
        (Lang::De, false) => (t.undo, "Strg+Z"),
        (Lang::De, true) => (t.redo, "Strg+Y"),
        (Lang::En, false) => (t.undo, "Ctrl+Z"),
        (Lang::En, true) => (t.redo, "Ctrl+Y"),
    };
    if count == 0 {
        match (lang, is_redo) {
            (Lang::De, false) => format!("{label} ({shortcut}) — nichts rückgängig zu machen"),
            (Lang::De, true) => format!("{label} ({shortcut}) — nichts zu wiederholen"),
            (Lang::En, false) => format!("{label} ({shortcut}) — nothing to undo"),
            (Lang::En, true) => format!("{label} ({shortcut}) — nothing to redo"),
        }
    } else if lang == Lang::De {
        let unit = if count == 1 { "Schritt" } else { "Schritte" };
        format!("{label} ({shortcut}) · {count} {unit}")
    } else {
        let unit = if count == 1 { "step" } else { "steps" };
        format!("{label} ({shortcut}) · {count} {unit}")
    }
}


pub fn draw_top_bars(ctx: &egui::Context, app: &mut RailbenchApp) {
    top_bar_menu::draw_menu_bar(ctx, app);
    top_bar_quick::draw_quick_toolbar(ctx, app);
}

mod top_bar_menu;
mod top_bar_quick;

fn draw_track_kind_icon(painter: &egui::Painter, rect: egui::Rect, kind: &str, accent: Color32, bg: Color32) {
    painter.rect_filled(rect, 6.0, bg);
    let stroke = Stroke::new(2.2_f32, accent);
    let c = rect.center();
    let l = rect.min.x + 5.0;
    let r = rect.max.x - 5.0;
    let t = rect.min.y + 6.0;
    let b = rect.max.y - 6.0;

    match kind {
        "curve" => {
            let mut pts = Vec::with_capacity(8);
            for i in 0..=7 {
                let u = i as f32 / 7.0;
                let ang = std::f32::consts::FRAC_PI_2 * u;
                pts.push(egui::pos2(l + (r - l) * ang.sin(), b - (b - t) * (1.0 - ang.cos())));
            }
            painter.add(egui::Shape::line(pts, stroke));
        }
        "turnout-left" | "curved-turnout-left" => {
            painter.line_segment([egui::pos2(l, c.y + 3.0), egui::pos2(r, c.y + 3.0)], stroke);
            painter.line_segment([egui::pos2(l + 4.0, c.y + 3.0), egui::pos2(r - 2.0, t)], stroke);
        }
        "turnout-right" | "curved-turnout-right" => {
            painter.line_segment([egui::pos2(l, c.y - 3.0), egui::pos2(r, c.y - 3.0)], stroke);
            painter.line_segment([egui::pos2(l + 4.0, c.y - 3.0), egui::pos2(r - 2.0, b)], stroke);
        }
        "three-way" => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r, c.y)], stroke);
            painter.line_segment([egui::pos2(l + 5.0, c.y), egui::pos2(r - 2.0, t)], stroke);
            painter.line_segment([egui::pos2(l + 5.0, c.y), egui::pos2(r - 2.0, b)], stroke);
        }
        "crossing" | "double-slip" => {
            painter.line_segment([egui::pos2(l, t + 1.0), egui::pos2(r, b - 1.0)], stroke);
            painter.line_segment([egui::pos2(l, b - 1.0), egui::pos2(r, t + 1.0)], stroke);
        }
        "flex" => {
            let mut pts = Vec::with_capacity(9);
            for i in 0..=8 {
                let u = i as f32 / 8.0;
                let x = l + (r - l) * u;
                let y = c.y + (u * std::f32::consts::TAU).sin() * 4.5;
                pts.push(egui::pos2(x, y));
            }
            painter.add(egui::Shape::line(pts, stroke));
        }
        "buffer" => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r - 3.0, c.y)], stroke);
            painter.line_segment([egui::pos2(r - 3.0, t + 2.0), egui::pos2(r - 3.0, b - 2.0)], Stroke::new(2.8_f32, accent));
        }
        "turntable" => {
            painter.circle_filled(c, 6.8, Color32::from_rgb(30, 41, 59));
            painter.circle_stroke(c, 9.5, Stroke::new(1.5_f32, Color32::from_rgb(148, 163, 184)));
            painter.circle_stroke(c, 5.4, Stroke::new(1.0_f32, Color32::from_rgb(100, 116, 139)));
            for i in 0..6 {
                let a = (i as f32 / 6.0) * std::f32::consts::TAU;
                let (s, cs) = a.sin_cos();
                painter.line_segment(
                    [egui::pos2(c.x + cs * 6.8, c.y + s * 6.8), egui::pos2(c.x + cs * 9.5, c.y + s * 9.5)],
                    Stroke::new(1.1_f32, accent),
                );
            }
            painter.line_segment([egui::pos2(c.x - 6.5, c.y), egui::pos2(c.x + 6.5, c.y)], stroke);
        }
        "transfer-table" => {
            let outer = egui::Rect::from_min_max(egui::pos2(l + 1.0, t + 1.0), egui::pos2(r - 1.0, b - 1.0));
            let pit = egui::Rect::from_min_max(egui::pos2(l + 4.0, t + 2.5), egui::pos2(r - 4.0, b - 2.5));
            painter.rect_stroke(
                outer,
                1.5,
                Stroke::new(1.3_f32, Color32::from_rgb(148, 163, 184)),
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(pit, 1.0, Color32::from_rgb(30, 41, 59));
            for fx in [0.22_f32, 0.50, 0.78] {
                let tx = pit.min.x + pit.width() * fx;
                painter.line_segment(
                    [egui::pos2(tx, pit.min.y + 1.0), egui::pos2(tx, pit.max.y - 1.0)],
                    Stroke::new(1.0_f32, Color32::from_rgb(100, 116, 139)),
                );
            }
            for fy in [0.22_f32, 0.50, 0.78] {
                let ty = outer.min.y + outer.height() * fy;
                painter.line_segment([egui::pos2(outer.min.x, ty), egui::pos2(pit.min.x, ty)], Stroke::new(1.1_f32, accent));
                painter.line_segment([egui::pos2(pit.max.x, ty), egui::pos2(outer.max.x, ty)], Stroke::new(1.1_f32, accent));
            }
            painter.line_segment([egui::pos2(pit.min.x + 0.5, c.y), egui::pos2(pit.max.x - 0.5, c.y)], stroke);
        }
        "feeder" => {
            painter.line_segment([egui::pos2(l, c.y + 3.0), egui::pos2(r, c.y + 3.0)], stroke);
            painter.line_segment([egui::pos2(c.x + 2.0, t + 1.0), egui::pos2(c.x - 2.0, c.y)], stroke);
            painter.line_segment([egui::pos2(c.x - 2.0, c.y), egui::pos2(c.x + 2.0, c.y)], stroke);
            painter.line_segment([egui::pos2(c.x + 2.0, c.y), egui::pos2(c.x - 1.0, c.y + 4.0)], stroke);
        }
        "uncoupler" => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r, c.y)], stroke);
            painter.rect_filled(
                egui::Rect::from_center_size(c, egui::vec2(8.0, 5.0)),
                1.5,
                accent,
            );
        }
        _ => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r, c.y)], stroke);
            let tie_s = Stroke::new(1.3_f32, accent);
            for tx in [l + 4.0, c.x, r - 4.0] {
                painter.line_segment([egui::pos2(tx, c.y - 3.5), egui::pos2(tx, c.y + 3.5)], tie_s);
            }
        }
    }
}

pub fn default_layer() -> Layer {
    Layer {
        id: "layer-1".into(),
        name: "Ebene 0".into(),
        color: Some("#38bdf8".into()),
        visible: true,
        locked: false,
        height_mm: Some(0.0),
        wood_plate_mode: Some("table".into()),
        wood_plate_thickness_mm: Some(12.0),
        wood_plate_style: None,
        plates: None,
        terrain_field: None,
    }
}

mod catalog_tab;
#[allow(unused_imports)]
use catalog_tab::*;
mod left_tabs;
#[allow(unused_imports)]
use left_tabs::*;
#[allow(unused_imports)]
pub use left_tabs::{draw_left_sidebar};
#[cfg(test)]
pub(crate) use left_tabs::{
    collect_track_manufacturers_in_order, favorite_accent, format_catalog_trigger_subtitle,
    format_catalog_trigger_title,
};
mod terrain_tab;
#[allow(unused_imports)]
use terrain_tab::*;
mod board_tab;
#[allow(unused_imports)]
use board_tab::*;
mod bom_tab;
#[allow(unused_imports)]
use bom_tab::*;
mod right_sidebar;
mod selection_bar;
#[allow(unused_imports)]
use right_sidebar::*;
#[allow(unused_imports)]
use selection_bar::*;
#[allow(unused_imports)]
pub use right_sidebar::draw_right_sidebar;
pub use selection_bar::draw_floating_selection_bar;
mod dialogs;
#[allow(unused_imports)]
use dialogs::*;
#[allow(unused_imports)]
pub use dialogs::{draw_dialogs};
mod height_panel;
mod welcome_demo_dialogs;
#[allow(unused_imports)]
use welcome_demo_dialogs::*;
mod settings_help_dialogs;
#[allow(unused_imports)]
use settings_help_dialogs::*;
mod extra_dialogs;
#[allow(unused_imports)]
use extra_dialogs::*;
mod export_dialog;
#[allow(unused_imports)]
use export_dialog::*;
mod cab_window;
pub use cab_window::draw_cab_view_window;
mod stellwerk_theme;
mod stellwerk_panel;
