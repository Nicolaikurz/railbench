use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    FilePlus,
    FolderOpen,
    Save,
    Undo,
    Redo,
    Magnet,
    Ruler,
    RotateCcw,
    RotateCw,
    Palette,
    Tag,
    Zap,
    #[allow(dead_code)]
    Isolator,
    Box3d,
    Sun,
    Moon,
    Layers,
    Users,
    Eye,
    EyeOff,
    Lock,
    Unlock,
    Trash,
    WinMinimize,
    WinMaximize,
    WinRestore,
    WinClose,
}

/// Renders a crisp monochrome vector icon button with tooltip (Lucide-style strokes).
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    active: bool,
    fill_normal: Color32,
    fill_active: Color32,
    stroke_col: Color32,
    icon_col: Color32,
    tooltip: &str,
) -> Response {
    icon_button_sized(ui, icon, Vec2::new(28.0, 24.0), active, fill_normal, fill_active, stroke_col, icon_col, tooltip)
}

#[allow(clippy::too_many_arguments)]
pub fn icon_button_sized(
    ui: &mut egui::Ui,
    icon: Icon,
    size: Vec2,
    active: bool,
    fill_normal: Color32,
    fill_active: Color32,
    stroke_col: Color32,
    icon_col: Color32,
    tooltip: &str,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let bg = if active {
            fill_active
        } else if resp.hovered() {
            Color32::from_rgba_unmultiplied(
                fill_normal.r().saturating_add(22),
                fill_normal.g().saturating_add(22),
                fill_normal.b().saturating_add(22),
                fill_normal.a().max(220),
            )
        } else {
            fill_normal
        };
        let border = if active || resp.hovered() {
            Stroke::new(1.1_f32, stroke_col)
        } else {
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 38))
        };
        ui.painter().rect(rect, CornerRadius::same(6), bg, border, egui::StrokeKind::Inside);
        let fg = if active { Color32::WHITE } else { icon_col };
        paint_icon(ui.painter(), rect.shrink2(Vec2::new((size.x - 14.0) * 0.5, (size.y - 14.0) * 0.5)), icon, fg);
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.on_hover_text(tooltip)
}

pub fn paint_icon(painter: &egui::Painter, r: Rect, icon: Icon, col: Color32) {
    let s = Stroke::new(1.5_f32, col);
    let thin = Stroke::new(1.25_f32, col);
    let p = |u: f32, v: f32| Pos2::new(r.min.x + r.width() * u, r.min.y + r.height() * v);

    match icon {
        Icon::FilePlus => {
            painter.add(egui::Shape::closed_line(
                vec![p(0.2, 0.08), p(0.62, 0.08), p(0.84, 0.30), p(0.84, 0.92), p(0.2, 0.92)],
                s,
            ));
            painter.line_segment([p(0.52, 0.48), p(0.52, 0.78)], s);
            painter.line_segment([p(0.37, 0.63), p(0.67, 0.63)], s);
        }
        Icon::FolderOpen => {
            painter.add(egui::Shape::closed_line(
                vec![
                    p(0.10, 0.22),
                    p(0.42, 0.22),
                    p(0.50, 0.34),
                    p(0.90, 0.34),
                    p(0.84, 0.84),
                    p(0.10, 0.84),
                ],
                s,
            ));
        }
        Icon::Save => {
            painter.add(egui::Shape::closed_line(
                vec![p(0.14, 0.12), p(0.74, 0.12), p(0.88, 0.26), p(0.88, 0.88), p(0.14, 0.88)],
                s,
            ));
            painter.rect_stroke(Rect::from_min_max(p(0.28, 0.12), p(0.64, 0.38)), 1.0, thin, egui::StrokeKind::Inside);
            painter.rect_stroke(Rect::from_min_max(p(0.28, 0.56), p(0.74, 0.88)), 1.0, thin, egui::StrokeKind::Inside);
        }
        Icon::Undo => {
            painter.line_segment([p(0.22, 0.40), p(0.66, 0.40)], s);
            painter.line_segment([p(0.22, 0.40), p(0.40, 0.22)], s);
            painter.line_segment([p(0.22, 0.40), p(0.40, 0.58)], s);
            painter.add(egui::Shape::line(
                vec![p(0.66, 0.40), p(0.82, 0.48), p(0.84, 0.64), p(0.72, 0.78), p(0.42, 0.78)],
                s,
            ));
        }
        Icon::Redo => {
            painter.line_segment([p(0.78, 0.40), p(0.34, 0.40)], s);
            painter.line_segment([p(0.78, 0.40), p(0.60, 0.22)], s);
            painter.line_segment([p(0.78, 0.40), p(0.60, 0.58)], s);
            painter.add(egui::Shape::line(
                vec![p(0.34, 0.40), p(0.18, 0.48), p(0.16, 0.64), p(0.28, 0.78), p(0.58, 0.78)],
                s,
            ));
        }
        Icon::Magnet => {
            // U-shaped horseshoe magnet
            painter.add(egui::Shape::line(
                vec![
                    p(0.24, 0.16),
                    p(0.24, 0.58),
                    p(0.32, 0.78),
                    p(0.50, 0.86),
                    p(0.68, 0.78),
                    p(0.76, 0.58),
                    p(0.76, 0.16),
                ],
                s,
            ));
            painter.line_segment([p(0.18, 0.30), p(0.30, 0.30)], thin);
            painter.line_segment([p(0.70, 0.30), p(0.82, 0.30)], thin);
        }
        Icon::Ruler => {
            painter.add(egui::Shape::closed_line(
                vec![p(0.12, 0.66), p(0.66, 0.12), p(0.88, 0.34), p(0.34, 0.88)],
                s,
            ));
            painter.line_segment([p(0.36, 0.42), p(0.46, 0.52)], thin);
            painter.line_segment([p(0.52, 0.26), p(0.62, 0.36)], thin);
        }
        Icon::RotateCcw => {
            let mut pts = Vec::with_capacity(10);
            for i in 0..=9 {
                let a = -2.4 + (i as f32 / 9.0) * 4.5;
                pts.push(p(0.5 + a.cos() * 0.34, 0.52 + a.sin() * 0.34));
            }
            painter.add(egui::Shape::line(pts, s));
            painter.line_segment([p(0.20, 0.22), p(0.20, 0.44)], s);
            painter.line_segment([p(0.20, 0.44), p(0.42, 0.44)], s);
        }
        Icon::RotateCw => {
            let mut pts = Vec::with_capacity(10);
            for i in 0..=9 {
                let a = -0.7 - (i as f32 / 9.0) * 4.5;
                pts.push(p(0.5 + a.cos() * 0.34, 0.52 + a.sin() * 0.34));
            }
            painter.add(egui::Shape::line(pts, s));
            painter.line_segment([p(0.80, 0.22), p(0.80, 0.44)], s);
            painter.line_segment([p(0.80, 0.44), p(0.58, 0.44)], s);
        }
        Icon::Palette => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.38, s);
            painter.circle_filled(p(0.36, 0.38), 1.3, col);
            painter.circle_filled(p(0.58, 0.34), 1.3, col);
            painter.circle_filled(p(0.64, 0.56), 1.3, col);
            painter.circle_filled(p(0.38, 0.62), 1.3, col);
        }
        Icon::Tag => {
            painter.add(egui::Shape::closed_line(
                vec![p(0.14, 0.14), p(0.52, 0.14), p(0.88, 0.50), p(0.50, 0.88), p(0.14, 0.52)],
                s,
            ));
            painter.circle_filled(p(0.32, 0.32), 1.4, col);
        }
        Icon::Zap => {
            painter.add(egui::Shape::line(
                vec![p(0.58, 0.10), p(0.24, 0.54), p(0.54, 0.54), p(0.42, 0.90), p(0.76, 0.46), p(0.46, 0.46), p(0.58, 0.10)],
                s,
            ));
        }
        Icon::Isolator => {
            painter.line_segment([p(0.10, 0.50), p(0.38, 0.50)], s);
            painter.line_segment([p(0.62, 0.50), p(0.90, 0.50)], s);
            painter.line_segment([p(0.42, 0.20), p(0.42, 0.80)], s);
            painter.line_segment([p(0.58, 0.20), p(0.58, 0.80)], s);
        }
        Icon::Box3d => {
            painter.add(egui::Shape::closed_line(
                vec![p(0.50, 0.10), p(0.86, 0.30), p(0.86, 0.72), p(0.50, 0.92), p(0.14, 0.72), p(0.14, 0.30)],
                s,
            ));
            painter.line_segment([p(0.14, 0.30), p(0.50, 0.50)], thin);
            painter.line_segment([p(0.86, 0.30), p(0.50, 0.50)], thin);
            painter.line_segment([p(0.50, 0.50), p(0.50, 0.92)], thin);
        }
        Icon::Sun => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.20, s);
            for i in 0..8 {
                let a = (i as f32) * std::f32::consts::FRAC_PI_4;
                painter.line_segment(
                    [
                        p(0.5 + a.cos() * 0.30, 0.5 + a.sin() * 0.30),
                        p(0.5 + a.cos() * 0.44, 0.5 + a.sin() * 0.44),
                    ],
                    thin,
                );
            }
        }
        Icon::Moon => {
            let mut pts = Vec::with_capacity(12);
            for i in 0..=11 {
                let a = -1.1 + (i as f32 / 11.0) * 4.4;
                pts.push(p(0.52 + a.cos() * 0.36, 0.52 + a.sin() * 0.36));
            }
            painter.add(egui::Shape::line(pts, s));
        }
        Icon::Layers => {
            painter.add(egui::Shape::closed_line(
                vec![p(0.50, 0.14), p(0.88, 0.34), p(0.50, 0.54), p(0.12, 0.34)],
                s,
            ));
            painter.add(egui::Shape::line(vec![p(0.12, 0.54), p(0.50, 0.72), p(0.88, 0.54)], thin));
            painter.add(egui::Shape::line(vec![p(0.12, 0.70), p(0.50, 0.88), p(0.88, 0.70)], thin));
        }
        Icon::Users => {
            painter.circle_stroke(p(0.42, 0.32), r.width() * 0.16, s);
            painter.add(egui::Shape::line(
                vec![p(0.16, 0.82), p(0.24, 0.64), p(0.60, 0.64), p(0.68, 0.82)],
                s,
            ));
            painter.circle_stroke(p(0.72, 0.38), r.width() * 0.12, thin);
            painter.add(egui::Shape::line(vec![p(0.70, 0.62), p(0.86, 0.78)], thin));
        }
        Icon::Eye => {
            painter.add(egui::Shape::line(
                vec![p(0.10, 0.50), p(0.30, 0.26), p(0.70, 0.26), p(0.90, 0.50), p(0.70, 0.74), p(0.30, 0.74), p(0.10, 0.50)],
                s,
            ));
            painter.circle_filled(p(0.50, 0.50), 2.0, col);
        }
        Icon::EyeOff => {
            painter.add(egui::Shape::line(
                vec![p(0.10, 0.50), p(0.30, 0.28), p(0.70, 0.28), p(0.90, 0.50)],
                thin,
            ));
            painter.line_segment([p(0.14, 0.14), p(0.86, 0.86)], s);
        }
        Icon::Lock => {
            painter.rect_stroke(Rect::from_min_max(p(0.20, 0.46), p(0.80, 0.88)), 2.0, s, egui::StrokeKind::Inside);
            painter.add(egui::Shape::line(
                vec![p(0.32, 0.46), p(0.32, 0.26), p(0.50, 0.16), p(0.68, 0.26), p(0.68, 0.46)],
                s,
            ));
        }
        Icon::Unlock => {
            painter.rect_stroke(Rect::from_min_max(p(0.20, 0.46), p(0.80, 0.88)), 2.0, s, egui::StrokeKind::Inside);
            painter.add(egui::Shape::line(
                vec![p(0.32, 0.46), p(0.32, 0.24), p(0.50, 0.14), p(0.68, 0.24)],
                s,
            ));
        }
        Icon::Trash => {
            painter.line_segment([p(0.16, 0.26), p(0.84, 0.26)], s);
            painter.line_segment([p(0.38, 0.14), p(0.62, 0.14)], s);
            painter.add(egui::Shape::line(
                vec![p(0.24, 0.26), p(0.30, 0.88), p(0.70, 0.88), p(0.76, 0.26)],
                s,
            ));
            painter.line_segment([p(0.42, 0.42), p(0.44, 0.74)], thin);
            painter.line_segment([p(0.58, 0.42), p(0.56, 0.74)], thin);
        }
        Icon::WinMinimize => {
            painter.line_segment([p(0.18, 0.56), p(0.82, 0.56)], s);
        }
        Icon::WinMaximize => {
            painter.rect_stroke(Rect::from_min_max(p(0.18, 0.18), p(0.82, 0.82)), 1.5, s, egui::StrokeKind::Inside);
        }
        Icon::WinRestore => {
            painter.rect_stroke(Rect::from_min_max(p(0.14, 0.30), p(0.68, 0.84)), 1.0, thin, egui::StrokeKind::Inside);
            painter.add(egui::Shape::line(
                vec![p(0.32, 0.30), p(0.32, 0.16), p(0.86, 0.16), p(0.86, 0.70), p(0.68, 0.70)],
                thin,
            ));
        }
        Icon::WinClose => {
            painter.line_segment([p(0.20, 0.20), p(0.80, 0.80)], s);
            painter.line_segment([p(0.80, 0.20), p(0.20, 0.80)], s);
        }
    }
}
