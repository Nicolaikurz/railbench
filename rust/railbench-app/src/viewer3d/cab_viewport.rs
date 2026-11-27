use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CabCameraMode {
    Cab,
    Roof,
    Chase,
}

#[derive(Debug, Clone, Copy)]
pub struct CabHudInfo<'a> {
    pub name: &'a str,
    pub kind: crate::vehicle_sim::VehicleKind,
    pub kmh: f64,
    pub max_kmh: f64,
    pub throttle: f32,
    pub grade_pct: f64,
    pub reverse: bool,
    pub headlights: bool,
    pub pantograph_up: bool,
    pub steam_on: bool,
    pub blocked_ahead: bool,
    pub sim_running: bool,
    pub coupled_count: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CabHudInteraction {
    pub set_throttle: Option<f32>,
    pub toggle_reverse: bool,
    pub toggle_headlights: bool,
    pub emergency_stop: bool,
    pub uncouple_last: bool,
}

/// Computes the dial needle angle (radians, 0 rad = 3 o'clock, sweeping clockwise
/// from `-220°` at `0 km/h` to `+40°` at `max_kmh`) for cockpit gauges.
pub fn speedometer_needle_angle(kmh: f64, max_kmh: f64) -> f32 {
    let denom = max_kmh.max(20.0);
    let frac = (kmh.abs() / denom).clamp(0.0, 1.0) as f32;
    (-220.0 + frac * 260.0).to_radians()
}

/// Renders a first-person locomotive Cab View (or Roof / Chase camera) into the secondary OS window.
/// Uses its own offscreen GPU target (`is_cab_view: true`) so both main 3D view and 2nd-monitor Cab View
/// can render simultaneously at independent resolutions while sharing all scene geometry on the GPU.
#[allow(clippy::too_many_arguments)]
pub fn draw_cab_viewport_3d(
    ui: &mut egui::Ui,
    cached_bundle: &Option<Scene3DBundle>,
    vehicles_mesh: Option<(u64, Arc<SceneMesh3D>)>,
    eye: [f32; 3],
    target: [f32; 3],
    fov_deg: f32,
    env_mode: Viewer3DEnv,
    time_of_day: f32,
    day_night_active: bool,
    dark_mode: bool,
    low_gpu_mode: bool,
    show_windshield_frame: bool,
    look_yaw_offset: &mut f32,
    look_pitch_offset: &mut f32,
    hud_info: Option<CabHudInfo<'_>>,
    is_de: bool,
) -> CabHudInteraction {
    let mut interaction = CabHudInteraction::default();
    let avail = ui.available_size().max(egui::vec2(64.0, 64.0));
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

    let desk_h = if show_windshield_frame {
        (rect.height() * 0.255).clamp(82.0, 126.0)
    } else {
        0.0
    };
    let press_in_desk = ui
        .input(|i| i.pointer.press_origin())
        .is_some_and(|po| show_windshield_frame && po.y >= rect.max.y - desk_h);

    // Drag inside cab viewport (above the driver's desk) to look around; double-click to recenter
    if response.dragged_by(egui::PointerButton::Primary) && !press_in_desk {
        let d = response.drag_delta();
        *look_yaw_offset = (*look_yaw_offset + d.x * 0.005).clamp(-1.8, 1.8);
        *look_pitch_offset = (*look_pitch_offset - d.y * 0.004).clamp(-0.75, 0.75);
    }
    if response.double_clicked() && !press_in_desk {
        *look_yaw_offset = 0.0;
        *look_pitch_offset = 0.0;
    }

    let ppp = ui.ctx().pixels_per_point();
    let res_scale = if low_gpu_mode { 0.75_f32 } else { 1.0_f32 };
    let px_w = ((rect.width() * ppp * res_scale).round() as u32).max(16);
    let px_h = ((rect.height() * ppp * res_scale).round() as u32).max(16);
    let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
    let sky = eval_sky_state(time_of_day, env_mode, dark_mode);
    let (cam_right, cam_up, cam_fwd) = camera_basis(eye, target);
    let tan_half_y = (fov_deg.to_radians() * 0.5).tan();

    let view = mat4_look_at(eye, target, [0.0, 1.0, 0.0]);
    let proj = mat4_perspective(fov_deg.to_radians(), aspect, 2.0, 400000.0);
    let uniforms = build_uniforms_3d(
        mat4_mul(proj, view),
        cached_bundle,
        &sky,
        eye,
        cam_right,
        cam_up,
        cam_fwd,
        aspect,
        tan_half_y,
        env_mode,
        low_gpu_mode,
    );

    let cb = egui_wgpu::Callback::new_paint_callback(
        rect,
        Viewport3DCallback {
            uniforms,
            bundle: cached_bundle.clone(),
            vehicles_mesh,
            time_of_day,
            env_mode,
            viewport_px: [px_w, px_h],
            clear_color: sky.clear_color,
            is_cab_view: true,
            shadows_enabled: !low_gpu_mode,
            day_night_active,
        },
    );
    ui.painter().add(cb);

    let painter = ui.painter_at(rect);

    if show_windshield_frame {
        if let Some(info) = hud_info {
            interaction = draw_cockpit_interior(ui, &painter, rect, desk_h, info, is_de);
        }
    } else if let Some(info) = hud_info {
        let hud_w = 400.0_f32.min(rect.width() - 24.0);
        let hud_h = 36.0_f32;
        let hud_rect = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.max.y - hud_h * 0.5 - 10.0),
            egui::vec2(hud_w, hud_h),
        );
        let (pill_bg, pill_border, text_col) = if dark_mode {
            (
                Color32::from_rgba_unmultiplied(15, 23, 42, 232),
                Color32::from_rgb(56, 189, 248),
                Color32::from_rgb(248, 250, 252),
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(250, 246, 238, 242),
                Color32::from_rgb(2, 132, 199),
                Color32::from_rgb(32, 28, 24),
            )
        };
        painter.rect_filled(hud_rect, 9.0, pill_bg);
        painter.rect_stroke(hud_rect, 9.0, Stroke::new(1.2_f32, pill_border), egui::StrokeKind::Inside);
        let dir_txt = if info.reverse {
            if is_de { "< RUECK" } else { "< REV" }
        } else if is_de {
            "> VOR"
        } else {
            "> FWD"
        };
        let status_txt = if info.blocked_ahead {
            format!(
                "{}  |  {}  |  {}  |  {:+.1}%",
                info.name,
                if is_de { "HALT" } else { "BLOCKED" },
                dir_txt,
                info.grade_pct
            )
        } else {
            format!("{}  |  {:.0} km/h  |  {}  |  {:+.1}%", info.name, info.kmh, dir_txt, info.grade_pct)
        };
        let text_center = Pos2::new(hud_rect.center().x - 10.0, hud_rect.center().y);
        painter.text(
            text_center,
            egui::Align2::CENTER_CENTER,
            status_txt,
            FontId::monospace(11.5),
            text_col,
        );
        // Painted headlight lamp dot on the right side of the pill (never monospace emoji)
        let lamp_pos = Pos2::new(hud_rect.max.x - 18.0, hud_rect.center().y);
        if info.headlights {
            painter.circle_filled(lamp_pos, 6.5, Color32::from_rgba_unmultiplied(250, 204, 21, 70));
            painter.circle_filled(lamp_pos, 3.8, Color32::from_rgb(250, 204, 21));
        } else {
            painter.circle_stroke(lamp_pos, 3.8, Stroke::new(1.4, Color32::from_rgb(148, 163, 184)));
        }
    }

    interaction
}

fn draw_cockpit_interior(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    desk_h: f32,
    info: CabHudInfo<'_>,
    is_de: bool,
) -> CabHudInteraction {
    use crate::vehicle_sim::VehicleKind;

    let mut interaction = CabHudInteraction::default();
    let (accent, trim_col, desk_fill, desk_hi, is_steam) = match info.kind {
        VehicleKind::Br243 => (
            Color32::from_rgb(153, 27, 38),  // DR bordeaux accent
            Color32::from_rgb(226, 215, 195), // cream belly stripe hint
            Color32::from_rgb(20, 26, 36),
            Color32::from_rgb(56, 72, 92),
            false,
        ),
        VehicleKind::Br96Mallet => (
            Color32::from_rgb(212, 160, 36), // polished brass trim
            Color32::from_rgb(185, 28, 28),
            Color32::from_rgb(22, 21, 24),
            Color32::from_rgb(140, 108, 42),
            true,
        ),
        VehicleKind::V60Diesel => (
            Color32::from_rgb(180, 35, 45),  // Altrot industrial red
            Color32::from_rgb(245, 235, 214),
            Color32::from_rgb(22, 27, 34),
            Color32::from_rgb(71, 85, 105),
            false,
        ),
        _ => (
            Color32::from_rgb(56, 189, 248),
            Color32::from_rgb(148, 163, 184),
            Color32::from_rgb(18, 24, 34),
            Color32::from_rgb(64, 79, 100),
            false,
        ),
    };

    let top_h = (rect.height() * 0.085).clamp(22.0, 38.0);
    let side_top = (rect.width() * 0.025).clamp(11.0, 22.0);
    let side_bot = (rect.width() * 0.056).clamp(24.0, 48.0);
    let desk_top_y = rect.max.y - desk_h;
    let frame_col = Color32::from_rgba_unmultiplied(12, 16, 22, 232);
    let frame_rim = Color32::from_rgba_unmultiplied(desk_hi.r(), desk_hi.g(), desk_hi.b(), 195);

    // Top sculpted roof sun-visor & header beam
    let top_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + top_h));
    painter.rect_filled(top_rect, 0.0, frame_col);
    painter.line_segment(
        [Pos2::new(rect.min.x, top_rect.max.y), Pos2::new(rect.max.x, top_rect.max.y)],
        Stroke::new(2.0, frame_rim),
    );
    painter.line_segment(
        [Pos2::new(rect.min.x + side_top, top_rect.max.y - 4.0), Pos2::new(rect.max.x - side_top, top_rect.max.y - 4.0)],
        Stroke::new(1.2, Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 160)),
    );

    // Cab builder nameplate centered in top sun-visor
    let badge_w = (info.name.len() as f32 * 6.8 + 26.0).clamp(130.0, 280.0);
    let badge_rect = Rect::from_center_size(top_rect.center(), egui::vec2(badge_w, (top_h - 6.0).clamp(16.0, 24.0)));
    painter.rect_filled(badge_rect, 4.0, Color32::from_rgb(15, 20, 29));
    painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0, frame_rim), egui::StrokeKind::Inside);
    painter.text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        info.name,
        FontId::proportional(11.0),
        trim_col,
    );

    // Angled left & right cab A-pillars
    let left_pillar = vec![
        Pos2::new(rect.min.x, rect.min.y + top_h),
        Pos2::new(rect.min.x + side_top, rect.min.y + top_h),
        Pos2::new(rect.min.x + side_bot, desk_top_y),
        Pos2::new(rect.min.x, desk_top_y),
    ];
    painter.add(egui::Shape::convex_polygon(left_pillar, frame_col, Stroke::NONE));
    painter.line_segment(
        [Pos2::new(rect.min.x + side_top, rect.min.y + top_h), Pos2::new(rect.min.x + side_bot, desk_top_y)],
        Stroke::new(2.0, frame_rim),
    );

    let right_pillar = vec![
        Pos2::new(rect.max.x - side_top, rect.min.y + top_h),
        Pos2::new(rect.max.x, rect.min.y + top_h),
        Pos2::new(rect.max.x, desk_top_y),
        Pos2::new(rect.max.x - side_bot, desk_top_y),
    ];
    painter.add(egui::Shape::convex_polygon(right_pillar, frame_col, Stroke::NONE));
    painter.line_segment(
        [Pos2::new(rect.max.x - side_top, rect.min.y + top_h), Pos2::new(rect.max.x - side_bot, desk_top_y)],
        Stroke::new(2.0, frame_rim),
    );

    if is_steam {
        // Curved brass cab spectacle rings in upper corners for steam locomotive
        let spec_r = (rect.height() * 0.055).clamp(16.0, 26.0);
        for sx in [rect.min.x + side_bot + spec_r + 16.0, rect.max.x - side_bot - spec_r - 16.0] {
            let cy = rect.min.y + top_h + spec_r + 10.0;
            painter.circle_stroke(Pos2::new(sx, cy), spec_r, Stroke::new(3.0, Color32::from_rgba_unmultiplied(180, 134, 36, 155)));
            painter.circle_stroke(Pos2::new(sx, cy), spec_r - 2.5, Stroke::new(1.0, Color32::from_rgba_unmultiplied(24, 24, 28, 175)));
        }
    } else {
        // Subtle twin-windshield center mullion & parked wipers
        let cx = rect.center().x;
        let mullion = Rect::from_min_max(
            Pos2::new(cx - 2.5, rect.min.y + top_h),
            Pos2::new(cx + 2.5, desk_top_y),
        );
        painter.rect_filled(mullion, 0.0, Color32::from_rgba_unmultiplied(14, 19, 28, 155));
        painter.line_segment(
            [Pos2::new(cx - 2.5, rect.min.y + top_h), Pos2::new(cx - 2.5, desk_top_y)],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 116, 139, 90)),
        );
        // Parked wiper blade silhouettes near lower glass corners
        let wiper_col = Color32::from_rgba_unmultiplied(15, 23, 42, 185);
        painter.line_segment(
            [Pos2::new(rect.min.x + side_bot + 16.0, desk_top_y - 2.0), Pos2::new(rect.min.x + side_bot + 72.0, desk_top_y - 18.0)],
            Stroke::new(2.2, wiper_col),
        );
        painter.line_segment(
            [Pos2::new(cx + 22.0, desk_top_y - 2.0), Pos2::new(cx + 80.0, desk_top_y - 18.0)],
            Stroke::new(2.2, wiper_col),
        );
    }

    // Sloped Driver's Control Desk (Fuehrerpult) across the bottom
    let glaresheet_h = 8.0_f32;
    let hood_poly = vec![
        Pos2::new(rect.min.x + side_bot * 0.45, desk_top_y - glaresheet_h),
        Pos2::new(rect.max.x - side_bot * 0.45, desk_top_y - glaresheet_h),
        Pos2::new(rect.max.x, desk_top_y),
        Pos2::new(rect.min.x, desk_top_y),
    ];
    painter.add(egui::Shape::convex_polygon(
        hood_poly,
        Color32::from_rgb(30, 38, 52),
        Stroke::new(1.2, frame_rim),
    ));
    painter.line_segment(
        [
            Pos2::new(rect.min.x + side_bot * 0.45, desk_top_y - glaresheet_h + 1.5),
            Pos2::new(rect.max.x - side_bot * 0.45, desk_top_y - glaresheet_h + 1.5),
        ],
        Stroke::new(2.0, accent),
    );

    let desk_rect = Rect::from_min_max(Pos2::new(rect.min.x, desk_top_y), rect.max);
    painter.rect_filled(desk_rect, 0.0, desk_fill);
    painter.line_segment(
        [desk_rect.left_top(), desk_rect.right_top()],
        Stroke::new(1.5, desk_hi),
    );

    // Inner instrument console bay
    let console_w = (desk_rect.width() - side_bot * 0.9).clamp(240.0, 740.0);
    let console_rect = Rect::from_center_size(
        desk_rect.center(),
        egui::vec2(console_w, desk_rect.height() - 10.0),
    );
    painter.rect_filled(console_rect, 8.0, Color32::from_rgb(13, 18, 28));
    painter.rect_stroke(console_rect, 8.0, Stroke::new(1.0, desk_hi), egui::StrokeKind::Inside);

    // 1) Left Bay: Interactive Throttle Quadrant
    let thr_w = (console_rect.width() * 0.19).clamp(58.0, 92.0);
    let thr_rect = Rect::from_min_size(
        Pos2::new(console_rect.min.x + 8.0, console_rect.min.y + 4.0),
        egui::vec2(thr_w, console_rect.height() - 8.0),
    );
    painter.rect_filled(thr_rect, 6.0, Color32::from_rgb(20, 27, 40));
    painter.rect_stroke(thr_rect, 6.0, Stroke::new(1.0, Color32::from_rgb(45, 58, 78)), egui::StrokeKind::Inside);
    painter.text(
        Pos2::new(thr_rect.center().x, thr_rect.min.y + 9.0),
        egui::Align2::CENTER_CENTER,
        if is_de { "REGLER" } else { "POWER" },
        FontId::monospace(8.5),
        Color32::from_rgb(148, 163, 184),
    );
    let slot_rect = Rect::from_min_max(
        Pos2::new(thr_rect.center().x - 10.0, thr_rect.min.y + 18.0),
        Pos2::new(thr_rect.center().x + 10.0, thr_rect.max.y - 16.0),
    );
    painter.rect_filled(
        Rect::from_center_size(slot_rect.center(), egui::vec2(6.0, slot_rect.height())),
        3.0,
        Color32::from_rgb(8, 12, 18),
    );
    let thr = info.throttle.clamp(0.0, 1.0);
    let fill_h = slot_rect.height() * thr;
    if fill_h > 1.0 {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(slot_rect.center().x - 2.5, slot_rect.max.y - fill_h),
                Pos2::new(slot_rect.center().x + 2.5, slot_rect.max.y),
            ),
            2.0,
            Color32::from_rgb(56, 189, 248),
        );
    }
    for n in 0..=4 {
        let ny = slot_rect.max.y - slot_rect.height() * (n as f32 * 0.25);
        painter.line_segment(
            [Pos2::new(slot_rect.center().x - 8.0, ny), Pos2::new(slot_rect.center().x - 4.0, ny)],
            Stroke::new(1.0, Color32::from_rgb(100, 116, 139)),
        );
        painter.line_segment(
            [Pos2::new(slot_rect.center().x + 4.0, ny), Pos2::new(slot_rect.center().x + 8.0, ny)],
            Stroke::new(1.0, Color32::from_rgb(100, 116, 139)),
        );
    }
    let knob_y = slot_rect.max.y - fill_h;
    let knob_rect = Rect::from_center_size(Pos2::new(slot_rect.center().x, knob_y), egui::vec2(28.0, 9.0));
    painter.rect_filled(knob_rect, 3.0, Color32::from_rgb(203, 213, 225));
    painter.rect_stroke(knob_rect, 3.0, Stroke::new(1.0, Color32::from_rgb(15, 23, 42)), egui::StrokeKind::Inside);
    painter.text(
        Pos2::new(thr_rect.center().x, thr_rect.max.y - 8.0),
        egui::Align2::CENTER_CENTER,
        format!("{:.0}%", thr * 100.0),
        FontId::monospace(9.5),
        Color32::from_rgb(56, 189, 248),
    );
    let thr_hit = ui.interact(
        thr_rect,
        ui.id().with("cab_cockpit_throttle"),
        Sense::click_and_drag(),
    );
    if (thr_hit.clicked() || thr_hit.dragged()) && slot_rect.height() > 2.0 {
        if let Some(pos) = thr_hit.interact_pointer_pos() {
            let new_t = ((slot_rect.max.y - pos.y) / slot_rect.height()).clamp(0.0, 1.0);
            interaction.set_throttle = Some(new_t);
        }
    }

    // 2) Center-Left Bay: Live Analog Speedometer
    let dial_r = (console_rect.height() * 0.43).clamp(24.0, 41.0);
    let aux_r = (dial_r * 0.78).clamp(18.0, 31.0);
    let speedo_cx = (console_rect.min.x + thr_w + 18.0 + dial_r).min(console_rect.center().x - aux_r * 0.5);
    let speedo_center = Pos2::new(speedo_cx, console_rect.center().y);
    draw_cockpit_gauge(
        painter,
        speedo_center,
        dial_r,
        info.kmh,
        info.max_kmh,
        &format!("{:.0}", info.kmh),
        "km/h",
        if is_steam { Color32::from_rgb(212, 160, 36) } else { Color32::from_rgb(148, 163, 184) },
        Color32::from_rgb(249, 115, 22),
        true,
    );

    // 3) Center-Right Bay: Auxiliary Loco Gauge (kV / bar / RPM) + Grade
    let aux_cx = speedo_cx + dial_r + aux_r + 16.0;
    if aux_cx + aux_r + 4.0 < console_rect.max.x - 110.0 {
        let (val, max_val, val_txt, unit, bez_col, needle_col) = match info.kind {
            VehicleKind::Br243 => {
                let kv = if info.pantograph_up { 15.0 } else { 0.0 };
                (kv, 20.0, format!("{kv:.0}"), "kV", Color32::from_rgb(100, 116, 139), Color32::from_rgb(56, 189, 248))
            }
            VehicleKind::Br96Mallet => {
                let bar = if info.steam_on {
                    13.5 + (1.0 - info.throttle as f64) * 2.0
                } else {
                    5.0
                };
                (bar, 20.0, format!("{bar:.1}"), "bar", Color32::from_rgb(212, 160, 36), Color32::from_rgb(250, 204, 21))
            }
            _ => {
                let rpm = 0.65 + (info.throttle as f64 * 1.45 + (info.kmh / info.max_kmh.max(20.0)) * 0.45).min(1.85);
                (rpm, 3.0, format!("{rpm:.1}"), "x1k", Color32::from_rgb(100, 116, 139), Color32::from_rgb(74, 222, 128))
            }
        };
        draw_cockpit_gauge(
            painter,
            Pos2::new(aux_cx, console_rect.center().y - 6.0),
            aux_r,
            val,
            max_val,
            &val_txt,
            unit,
            bez_col,
            needle_col,
            false,
        );
        let grd_col = if info.grade_pct.abs() > 3.5 {
            Color32::from_rgb(239, 68, 68)
        } else if info.grade_pct.abs() > 1.5 {
            Color32::from_rgb(245, 158, 11)
        } else {
            Color32::from_rgb(203, 213, 225)
        };
        painter.text(
            Pos2::new(aux_cx, console_rect.max.y - 8.0),
            egui::Align2::CENTER_CENTER,
            format!("{:+.1}%", info.grade_pct),
            FontId::monospace(9.5),
            grd_col,
        );
    }

    // 4) Right Bay: Interactive Reverser, Uncouple, Indicator Lamps & E-Stop
    let right_w = (console_rect.width() * 0.31).clamp(124.0, 206.0);
    let right_rect = Rect::from_min_max(
        Pos2::new(console_rect.max.x - right_w - 8.0, console_rect.min.y + 5.0),
        Pos2::new(console_rect.max.x - 8.0, console_rect.max.y - 5.0),
    );
    painter.rect_filled(right_rect, 6.0, Color32::from_rgb(20, 27, 40));
    painter.rect_stroke(right_rect, 6.0, Stroke::new(1.0, Color32::from_rgb(45, 58, 78)), egui::StrokeKind::Inside);

    // Top row: Reverser + optional Uncouple Last + E-Stop
    let has_wagons = info.coupled_count > 0;
    let uncouple_w = if has_wagons { 44.0_f32 } else { 0.0_f32 };
    let stop_w = 40.0_f32;
    let rev_w = (right_rect.width() - 14.0 - stop_w - if has_wagons { uncouple_w + 4.0 } else { 0.0 }).max(48.0);
    let rev_rect = Rect::from_min_size(Pos2::new(right_rect.min.x + 5.0, right_rect.min.y + 5.0), egui::vec2(rev_w, 22.0));
    let mut next_x = rev_rect.max.x + 4.0;
    if has_wagons {
        let uncpl_rect = Rect::from_min_size(Pos2::new(next_x, right_rect.min.y + 5.0), egui::vec2(uncouple_w, 22.0));
        painter.rect_filled(uncpl_rect, 4.0, Color32::from_rgb(124, 58, 237));
        painter.text(
            uncpl_rect.center(),
            egui::Align2::CENTER_CENTER,
            if is_de { "ABK" } else { "UNCPL" },
            FontId::monospace(9.0),
            Color32::WHITE,
        );
        if ui.interact(uncpl_rect, ui.id().with("cab_cockpit_uncpl"), Sense::click()).clicked() {
            interaction.uncouple_last = true;
        }
        next_x = uncpl_rect.max.x + 4.0;
    }
    let estop_rect = Rect::from_min_size(Pos2::new(next_x, right_rect.min.y + 5.0), egui::vec2(stop_w, 22.0));

    let rev_lbl = if info.reverse {
        if is_de { "< RUECK" } else { "< REV" }
    } else if is_de {
        "VOR >"
    } else {
        "FWD >"
    };
    painter.rect_filled(
        rev_rect,
        4.0,
        if info.reverse { Color32::from_rgb(180, 83, 9) } else { Color32::from_rgb(2, 132, 199) },
    );
    painter.text(rev_rect.center(), egui::Align2::CENTER_CENTER, rev_lbl, FontId::monospace(9.5), Color32::WHITE);
    if ui.interact(rev_rect, ui.id().with("cab_cockpit_rev"), Sense::click()).clicked() {
        interaction.toggle_reverse = true;
    }

    painter.rect_filled(estop_rect, 4.0, Color32::from_rgb(185, 28, 28));
    painter.text(estop_rect.center(), egui::Align2::CENTER_CENTER, "STOP", FontId::monospace(9.5), Color32::WHITE);
    if ui.interact(estop_rect, ui.id().with("cab_cockpit_estop"), Sense::click()).clicked() {
        interaction.emergency_stop = true;
    }

    // Bottom row: Illuminated Cockpit Indicator Lamps (vector circles + crisp ASCII tags)
    let lamp_y = right_rect.max.y - 16.0;
    let aux_active = match info.kind {
        VehicleKind::Br243 => info.pantograph_up,
        VehicleKind::Br96Mallet => info.steam_on,
        _ => info.sim_running,
    };
    let aux_tag = match info.kind {
        VehicleKind::Br243 => "PAN",
        VehicleKind::Br96Mallet => "STM",
        _ => "ENG",
    };
    let lamps = [
        ("LGT", info.headlights, Color32::from_rgb(250, 204, 21), true),
        (aux_tag, aux_active, Color32::from_rgb(56, 189, 248), false),
        ("HLT", info.blocked_ahead, Color32::from_rgb(245, 158, 11), false),
        ("RUN", info.sim_running && info.kmh > 0.5, Color32::from_rgb(34, 197, 94), false),
    ];
    let step_x = right_rect.width() / lamps.len() as f32;
    for (li, &(tag, on, col, click_light)) in lamps.iter().enumerate() {
        let lx = right_rect.min.x + step_x * (li as f32 + 0.5);
        let lamp_center = Pos2::new(lx, lamp_y - 4.0);
        if on {
            painter.circle_filled(lamp_center, 7.5, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 65));
            painter.circle_filled(lamp_center, 4.3, col);
        } else {
            painter.circle_filled(lamp_center, 4.3, Color32::from_rgb(30, 41, 59));
            painter.circle_stroke(lamp_center, 4.3, Stroke::new(1.0, Color32::from_rgb(71, 85, 105)));
        }
        painter.text(
            Pos2::new(lx, lamp_y + 7.5),
            egui::Align2::CENTER_CENTER,
            tag,
            FontId::monospace(8.0),
            if on { col } else { Color32::from_rgb(100, 116, 139) },
        );
        if click_light {
            let l_rect = Rect::from_center_size(Pos2::new(lx, lamp_y + 1.0), egui::vec2(step_x, 26.0));
            if ui.interact(l_rect, ui.id().with("cab_cockpit_lights"), Sense::click()).clicked() {
                interaction.toggle_headlights = true;
            }
        }
    }

    interaction
}

#[allow(clippy::too_many_arguments)]
fn draw_cockpit_gauge(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    value: f64,
    max_val: f64,
    value_txt: &str,
    unit_txt: &str,
    bezel_col: Color32,
    needle_col: Color32,
    show_redzone: bool,
) {
    painter.circle_filled(center, radius, Color32::from_rgb(9, 13, 21));
    painter.circle_stroke(center, radius, Stroke::new(2.0, bezel_col));
    painter.circle_stroke(center, radius - 2.5, Stroke::new(1.0, Color32::from_rgb(30, 41, 59)));

    let tick_count = 10;
    for i in 0..=tick_count {
        let f = i as f32 / tick_count as f32;
        let ang = (-220.0 + f * 260.0).to_radians();
        let (c, s) = (ang.cos(), ang.sin());
        let r_out = radius - 3.5;
        let r_in = if i % 2 == 0 { radius - 8.5 } else { radius - 6.5 };
        let col = if show_redzone && i >= 9 {
            Color32::from_rgb(239, 68, 68)
        } else {
            Color32::from_rgb(148, 163, 184)
        };
        painter.line_segment(
            [
                Pos2::new(center.x + c * r_in, center.y + s * r_in),
                Pos2::new(center.x + c * r_out, center.y + s * r_out),
            ],
            Stroke::new(if i % 2 == 0 { 1.4 } else { 0.9 }, col),
        );
    }

    let ang = speedometer_needle_angle(value, max_val);
    let tip = Pos2::new(center.x + ang.cos() * (radius - 6.0), center.y + ang.sin() * (radius - 6.0));
    painter.line_segment([center, tip], Stroke::new(2.0, needle_col));
    painter.circle_filled(center, 3.0, Color32::from_rgb(226, 232, 240));

    painter.text(
        Pos2::new(center.x, center.y + radius * 0.42),
        egui::Align2::CENTER_CENTER,
        value_txt,
        FontId::monospace((radius * 0.34).clamp(9.0, 12.5)),
        Color32::from_rgb(248, 250, 252),
    );
    painter.text(
        Pos2::new(center.x, center.y + radius * 0.72),
        egui::Align2::CENTER_CENTER,
        unit_txt,
        FontId::monospace((radius * 0.22).clamp(7.0, 9.0)),
        Color32::from_rgb(148, 163, 184),
    );
}

// ============================================================================
//  Batched 3D Scene Mesh Builder (Baseboard + Detailed Tracks + Models + Terrain)
// ============================================================================

// (Corridor chains built cleanly via terrain_tools::build_connected_corridor_chains)

