use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};

use crate::vehicle_sim::{VehicleDomain, VehicleKind, VehicleRenderPose, VehicleSimState};

use super::Camera2D;

fn poly4(to_scr: impl Fn(f64, f64) -> Pos2, x0: f64, x1: f64, z0: f64, z1: f64) -> Vec<Pos2> {
    vec![to_scr(x1, z0), to_scr(x1, z1), to_scr(x0, z1), to_scr(x0, z0)]
}

fn draw_vehicle_top_detail(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    p: &VehicleRenderPose,
) {
    let hl = p.length_mm * 0.5;
    let hw = p.width_mm * 0.5;
    let rad = p.heading_deg.to_radians();
    let (cs, sn) = (rad.cos(), rad.sin());
    let to_scr = |lx: f64, ly: f64| -> Pos2 {
        camera.world_to_screen(rect, p.x + lx * cs - ly * sn, p.y + lx * sn + ly * cs)
    };
    let dark_outline = Stroke::new(1.25, Color32::from_rgb(15, 23, 42));
    let r = (p.livery_rgb[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (p.livery_rgb[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (p.livery_rgb[2].clamp(0.0, 1.0) * 255.0) as u8;
    let fill = Color32::from_rgb(r, g, b);
    let roof = Color32::from_rgb(
        (r as u16 * 72 / 100) as u8,
        (g as u16 * 72 / 100) as u8,
        (b as u16 * 72 / 100) as u8,
    );

    match p.kind {
        VehicleKind::SteamLoco => {
            // Tender body + coal heap (-1.0..=-0.42*hl), cab (-0.40..=-0.12), boiler (-0.12..=0.98)
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl, -hl * 0.42, -hw * 0.88, hw * 0.88),
                Color32::from_rgb(24, 28, 36),
                dark_outline,
            ));
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.90, -hl * 0.48, -hw * 0.58, hw * 0.58),
                Color32::from_rgb(10, 12, 16),
                Stroke::NONE,
            ));
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.38, hl * 0.96, -hw * 0.92, hw * 0.92),
                Color32::from_rgb(26, 30, 38),
                dark_outline,
            ));
            // Cylindrical black boiler + smoke deflectors + brass dome accents + chimney
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.12, hl * 0.96, -hw * 0.64, hw * 0.64),
                Color32::from_rgb(38, 44, 54),
                Stroke::new(1.0, Color32::from_rgb(212, 160, 23)),
            ));
            let rim = (camera.zoom * 3.0).clamp(1.6, 5.5);
            painter.circle_filled(to_scr(hl * 0.78, 0.0), rim, Color32::from_rgb(15, 23, 42));
            painter.circle_filled(to_scr(hl * 0.22, 0.0), rim * 1.15, Color32::from_rgb(55, 65, 81));
        }
        VehicleKind::ElectricLoco => {
            // Purpurrot base outline + tapered TEE noses + silver roof + twin crimson pantograph diamonds
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.92, hl * 0.92, -hw, hw),
                Color32::from_rgb(254, 243, 199),
                Stroke::new(1.6, fill),
            ));
            for sgn in [-1.0_f64, 1.0] {
                let tip = vec![
                    to_scr(sgn * hl * 0.92, -hw),
                    to_scr(sgn * hl, -hw * 0.62),
                    to_scr(sgn * hl, hw * 0.62),
                    to_scr(sgn * hl * 0.92, hw),
                ];
                painter.add(egui::Shape::convex_polygon(tip, fill, dark_outline));
            }
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.78, hl * 0.78, -hw * 0.72, hw * 0.72),
                Color32::from_rgb(148, 163, 184),
                Stroke::NONE,
            ));
            let p_stroke = Stroke::new((1.4 * camera.zoom).clamp(1.0, 2.4), Color32::from_rgb(220, 38, 38));
            for px in [-hl * 0.44, hl * 0.44] {
                let d_pts = vec![
                    to_scr(px - hl * 0.08, 0.0),
                    to_scr(px, -hw * 0.56),
                    to_scr(px + hl * 0.08, 0.0),
                    to_scr(px, hw * 0.56),
                ];
                painter.add(egui::Shape::closed_line(d_pts, p_stroke));
            }
        }
        VehicleKind::DieselLoco => {
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl, hl, -hw, hw),
                fill,
                dark_outline,
            ));
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.84, hl * 0.84, -hw * 0.74, hw * 0.74),
                Color32::from_rgb(71, 85, 105),
                Stroke::NONE,
            ));
            // 3 radiator cooling fan grilles + twin angled exhaust hoods
            let fan_r = (camera.zoom * 5.2).clamp(2.0, 7.0);
            for fx in [-hl * 0.40, 0.0, hl * 0.40] {
                painter.circle_filled(to_scr(fx, 0.0), fan_r, Color32::from_rgb(30, 41, 59));
            }
            for ex in [-hl * 0.17, hl * 0.17] {
                painter.add(egui::Shape::convex_polygon(
                    poly4(&to_scr, ex - hl * 0.04, ex + hl * 0.04, -hw * 0.25, hw * 0.25),
                    Color32::from_rgb(15, 23, 42),
                    Stroke::NONE,
                ));
            }
        }
        VehicleKind::PassengerCoach => {
            // Rubber gangway bellows + tapered ends + green body + silver roof with Kuckuck vents
            for sgn in [-1.0_f64, 1.0] {
                painter.add(egui::Shape::convex_polygon(
                    poly4(&to_scr, sgn * hl * 0.94, sgn * hl * 1.01, -hw * 0.46, hw * 0.46),
                    Color32::from_rgb(17, 24, 39),
                    Stroke::NONE,
                ));
            }
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.96, hl * 0.96, -hw, hw),
                fill,
                dark_outline,
            ));
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.94, hl * 0.94, -hw * 0.76, hw * 0.76),
                Color32::from_rgb(115, 128, 145),
                Stroke::NONE,
            ));
            let vr = (camera.zoom * 1.6).clamp(1.0, 3.0);
            for k in -3..=3 {
                painter.circle_filled(to_scr(k as f64 * hl * 0.22, 0.0), vr, Color32::from_rgb(71, 85, 105));
            }
        }
        VehicleKind::FreightCar => {
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl, hl, -hw, hw),
                fill,
                dark_outline,
            ));
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.96, hl * 0.96, -hw * 0.84, hw * 0.84),
                Color32::from_rgb(82, 96, 112),
                Stroke::NONE,
            ));
            // Sliding door strips along sides
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.22, hl * 0.22, hw * 0.88, hw * 1.04),
                Color32::from_rgb(63, 32, 20),
                Stroke::NONE,
            ));
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.22, hl * 0.22, -hw * 1.04, -hw * 0.88),
                Color32::from_rgb(63, 32, 20),
                Stroke::NONE,
            ));
        }
        VehicleKind::Sedan | VehicleKind::Truck | VehicleKind::Bus => {
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl, hl, -hw, hw),
                fill,
                dark_outline,
            ));
            painter.add(egui::Shape::convex_polygon(
                poly4(&to_scr, -hl * 0.72, hl * 0.72, -hw * 0.62, hw * 0.62),
                roof,
                Stroke::NONE,
            ));
        }
    }

    if (p.headlights && p.kind.is_locomotive())
        || (p.headlights && p.kind.domain() == VehicleDomain::Road)
    {
        let lr = (camera.zoom * 2.4).clamp(2.0, 5.5);
        let (fwd_col, rear_col) = if p.driving_backward {
            (Color32::from_rgb(239, 68, 68), Color32::from_rgb(254, 240, 138))
        } else {
            (Color32::from_rgb(254, 240, 138), Color32::from_rgb(239, 68, 68))
        };
        for &side in &[-0.55, 0.55] {
            painter.circle_filled(to_scr(hl * 0.96, hw * side), lr, fwd_col);
            painter.circle_filled(to_scr(-hl * 0.96, hw * side), lr * 0.78, rear_col);
        }
    }
    if !p.label.is_empty() && camera.zoom >= 0.22 {
        let cp = camera.world_to_screen(rect, p.x, p.y);
        let lpos = Pos2::new(cp.x, cp.y - (hw as f32 * camera.zoom).max(8.0) - 9.0);
        painter.text(
            lpos,
            egui::Align2::CENTER_CENTER,
            &p.label,
            FontId::proportional(10.0),
            Color32::from_rgb(248, 250, 252),
        );
    }
}

pub(super) fn draw_vehicles_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    vehicle_sim: &VehicleSimState,
    scale: &str,
) {
    if vehicle_sim.vehicles.is_empty() {
        return;
    }
    let poses = vehicle_sim.all_render_poses(scale);
    for pair in poses.windows(2) {
        if pair[0].vehicle_id == pair[1].vehicle_id {
            let r0 = pair[0].heading_deg.to_radians();
            let r1 = pair[1].heading_deg.to_radians();
            let rear0 = camera.world_to_screen(
                rect,
                pair[0].x - r0.cos() * (pair[0].length_mm * 0.5),
                pair[0].y - r0.sin() * (pair[0].length_mm * 0.5),
            );
            let front1 = camera.world_to_screen(
                rect,
                pair[1].x + r1.cos() * (pair[1].length_mm * 0.5),
                pair[1].y + r1.sin() * (pair[1].length_mm * 0.5),
            );
            painter.line_segment(
                [rear0, front1],
                Stroke::new((2.2 * camera.zoom).clamp(1.4, 3.6), Color32::from_rgb(51, 65, 85)),
            );
        }
    }
    for p in &poses {
        draw_vehicle_top_detail(painter, rect, camera, p);
    }
}
