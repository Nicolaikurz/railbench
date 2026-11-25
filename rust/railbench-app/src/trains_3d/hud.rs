use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Stroke};
use railbench_core::types::{Catalog, Layout};

use crate::i18n::Lang;
use crate::viewer3d::Camera3D;

use super::track_graph::build_rail_network;
use super::{Train3DState, TrainCameraMode, TrainLocoKind};

pub(super) fn draw_train_hud(
    state: &mut Train3DState,
    ctx: &egui::Context,
    viewport_rect: egui::Rect,
    layout: &Layout,
    catalogs: &[Catalog],
    camera: &mut Camera3D,
    dark_mode: bool,
    lang: Lang,
) {
    if !state.active {
        return;
    }
    let is_de = lang == Lang::De;
    let bg = if dark_mode {
        Color32::from_rgba_unmultiplied(15, 23, 42, 224)
    } else {
        Color32::from_rgba_unmultiplied(248, 250, 252, 236)
    };
    let border = if dark_mode {
        Color32::from_rgba_unmultiplied(56, 189, 248, 120)
    } else {
        Color32::from_rgba_unmultiplied(2, 132, 199, 140)
    };
    let text_main = if dark_mode { Color32::WHITE } else { Color32::from_rgb(15, 23, 42) };
    let text_muted = if dark_mode { Color32::from_rgb(148, 163, 184) } else { Color32::from_rgb(71, 85, 105) };
    let accent = if dark_mode { Color32::from_rgb(56, 189, 248) } else { Color32::from_rgb(2, 132, 199) };

    // Keyboard driving shortcuts when no text box is focused
    if !ctx.wants_keyboard_input() {
        ctx.input(|i| {
            if let Some(train) = state.trains.get_mut(state.selected_idx) {
                if i.key_pressed(egui::Key::W) || i.key_pressed(egui::Key::ArrowUp) {
                    train.throttle = (train.throttle + 0.15).clamp(-1.0, 1.0);
                }
                if i.key_pressed(egui::Key::S) || i.key_pressed(egui::Key::ArrowDown) {
                    train.throttle = (train.throttle - 0.15).clamp(-1.0, 1.0);
                }
                if i.key_pressed(egui::Key::Space) {
                    train.throttle = 0.0;
                }
            }
        });
    }

    let toggle_next_turnout = !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::T));
    let mut exit_requested = false;
    let mut focus_requested = false;
    let mut add_requested = false;
    let mut remove_requested = false;
    let mut switch_turnout_clicked = toggle_next_turnout;

    egui::Area::new(egui::Id::new("hud_3d_train_driver"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
        .constrain_to(viewport_rect.shrink(6.0))
        .show(ctx, |ui| {
            Frame::new()
                .fill(bg)
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::symmetric(12, 8))
                .stroke(Stroke::new(1.2_f32, border))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 5.0;

                        // Row 1: Train selection, locomotive type, camera mode, focus & exit
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if is_de { "3D ZUG-MODUS" } else { "3D TRAIN MODE" }).strong().size(11.0).color(accent));

                            if state.trains.len() > 1 {
                                ui.separator();
                                for idx in 0..state.trains.len() {
                                    let sel = idx == state.selected_idx;
                                    if ui.selectable_label(sel, format!("#{}", idx + 1)).clicked() {
                                        state.selected_idx = idx;
                                    }
                                }
                            }

                            ui.separator();
                            if let Some(train) = state.trains.get_mut(state.selected_idx) {
                                for &kind in TrainLocoKind::ALL {
                                    if ui.selectable_label(train.kind == kind, kind.short_label()).clicked() {
                                        train.kind = kind;
                                    }
                                }
                            }

                            ui.separator();
                            if ui
                                .small_button(if is_de { "+ Zug" } else { "+ Train" })
                                .on_hover_text(if is_de { "Weitere Lok auf die Anlage stellen" } else { "Add another locomotive on the layout" })
                                .clicked()
                            {
                                add_requested = true;
                            }
                            if state.trains.len() > 1 && ui.small_button("−").on_hover_text(if is_de { "Zug entfernen" } else { "Remove selected train" }).clicked() {
                                remove_requested = true;
                            }

                            ui.separator();
                            let cam_lbl = match state.camera_mode {
                                TrainCameraMode::Free => if is_de { "Kamera: Frei" } else { "Cam: Free" },
                                TrainCameraMode::Follow => if is_de { "Kamera: Folgen" } else { "Cam: Follow" },
                                TrainCameraMode::Cab => if is_de { "Kamera: Fuehrerstand" } else { "Cam: Cab" },
                            };
                            if ui.small_button(cam_lbl).clicked() {
                                state.camera_mode = state.camera_mode.next();
                                focus_requested = true;
                            }
                            if ui.small_button(if is_de { "Fokus" } else { "Focus" }).clicked() {
                                focus_requested = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(if is_de { "Beenden" } else { "Exit" }).strong().size(11.0).color(Color32::WHITE))
                                        .fill(Color32::from_rgb(185, 28, 28))
                                        .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                exit_requested = true;
                            }
                        });

                        // Row 2: Throttle, Reverse/Stop/Fwd, Scale Speed km/h, Headlights, Panto/Steam, Switch Turnout
                        if let Some(train) = state.trains.get_mut(state.selected_idx) {
                            ui.horizontal(|ui| {
                                if ui.small_button("<<").on_hover_text(if is_de { "Rueckwaerts" } else { "Reverse" }).clicked() {
                                    train.throttle = (-train.throttle.abs().max(0.45)).clamp(-1.0, -0.2);
                                }
                                if ui.small_button(if is_de { "Halt" } else { "Stop" }).clicked() {
                                    train.throttle = 0.0;
                                }
                                if ui.small_button(">>").on_hover_text(if is_de { "Vorwaerts" } else { "Forward" }).clicked() {
                                    train.throttle = train.throttle.abs().max(0.45).clamp(0.2, 1.0);
                                }

                                ui.add_sized(
                                    [140.0, 16.0],
                                    egui::Slider::new(&mut train.throttle, -1.0..=1.0).show_value(false),
                                );
                                let kmh = (train.throttle.abs() * train.kind.max_scale_kmh()).round() as i32;
                                let dir_s = if train.throttle > 0.02 {
                                    "FWD"
                                } else if train.throttle < -0.02 {
                                    "REV"
                                } else {
                                    "IDLE"
                                };
                                ui.label(
                                    RichText::new(format!("{dir_s} {kmh:3} km/h"))
                                        .monospace()
                                        .size(11.0)
                                        .color(text_main),
                                );

                                ui.separator();
                                ui.checkbox(&mut train.headlights_on, if is_de { "Licht (Zg1)" } else { "Lights" });
                                match train.kind {
                                    TrainLocoKind::Br243 => {
                                        ui.checkbox(&mut train.pantograph_up, if is_de { "Stromabnehmer" } else { "Pantograph" });
                                    }
                                    TrainLocoKind::Br96Mallet => {
                                        ui.checkbox(&mut train.steam_on, if is_de { "Dampf" } else { "Steam" });
                                    }
                                    TrainLocoKind::V60Diesel => {}
                                }
                                if ui
                                    .small_button(if is_de { "Weiche (T)" } else { "Switch (T)" })
                                    .on_hover_text(if is_de { "Naechste Weiche umschalten" } else { "Toggle nearest turnout branch" })
                                    .clicked()
                                {
                                    switch_turnout_clicked = true;
                                }
                                ui.label(
                                    RichText::new("W/S: Speed  Space: Stop")
                                        .size(10.0)
                                        .color(text_muted),
                                );
                            });
                        }
                    });
                });
        });

    if add_requested {
        state.add_train(layout, catalogs);
    }
    if remove_requested && state.trains.len() > 1 {
        state.trains.remove(state.selected_idx);
        state.selected_idx = state.selected_idx.min(state.trains.len() - 1);
    }
    if switch_turnout_clicked {
        state.toggle_nearest_turnout(layout, catalogs);
    }
    if focus_requested {
        let net = build_rail_network(layout, catalogs, &state.turnout_branches);
        if let Some(train) = state.trains.get(state.selected_idx) {
            if !net.segments.is_empty() {
                let s_idx = train.seg_idx.min(net.segments.len() - 1);
                let (pose, _) = net.compute_train_pose(s_idx, train.dist_s, train.forward_on_seg, super::scale_factor_for_layout(&layout.scale));
                camera.target = [pose.pos[0], pose.pos[1] + 22.0 * pose.scale, pose.pos[2]];
                camera.distance = camera.distance.clamp(120.0, 380.0);
                net.apply_camera_tracking(camera, pose, state.camera_mode);
            }
        }
    }
    if exit_requested {
        state.active = false;
    }
}
