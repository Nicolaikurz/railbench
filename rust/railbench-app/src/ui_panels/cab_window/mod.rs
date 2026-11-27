mod desk;
mod viewport_and_controls;

use desk::draw_desk_card;
#[cfg(test)]
pub(crate) use viewport_and_controls::cab_camera_offsets;

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke};

use crate::i18n::Lang;
use crate::icons::{icon_button_sized, paint_icon, Icon};
use crate::vehicle_sim::{VehicleDomain, VehicleKind};
use crate::viewer3d::{
    draw_cab_viewport_3d, CabCameraMode, CabHudInfo, SceneMesh3D, Viewer3DEnv,
};
use crate::RailbenchApp;

pub(super) struct CabActions {
    spawn: Option<VehicleKind>,
    remove_id: Option<String>,
    clear_all: bool,
    focus_pt: Option<(f64, f64)>,
    add_wagon: Option<(String, VehicleKind)>,
    uncouple_at: Option<(String, usize)>,
    couple_nearby_id: Option<String>,
    trigger_uncoupler: Option<String>,
    toggle_uncoupler_armed: Option<String>,
    toggle_turnout: Option<(String, usize)>,
}

impl Default for CabActions {
    fn default() -> Self {
        Self {
            spawn: None,
            remove_id: None,
            clear_all: false,
            focus_pt: None,
            add_wagon: None,
            uncouple_at: None,
            couple_nearby_id: None,
            trigger_uncoupler: None,
            toggle_uncoupler_armed: None,
            toggle_turnout: None,
        }
    }
}

pub(super) fn resolve_active_vehicle_idx(sim: &crate::vehicle_sim::VehicleSimState) -> Option<usize> {
    sim.selected_vehicle_id
        .as_deref()
        .and_then(|sid| sim.vehicles.iter().position(|v| v.id == sid))
        .or_else(|| sim.vehicles.iter().position(|v| v.kind.is_locomotive()))
        .or_else(|| if !sim.vehicles.is_empty() { Some(0) } else { None })
}

/// Renders a dedicated, independent OS window (`ViewportId`) containing:
/// - Top: Frameless RAILbench header + unified mode/simulation/camera bar
/// - Left/Center: Live 3D First-Person Cab View and/or SpDrS60 Signal Box
/// - Right Panel: Driver's Control Desk (Telemetry, Throttle, Direction, Fleet, Spawn & Turnouts)
pub use viewport_and_controls::CabCameraSmoother;

pub fn draw_cab_view_window(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let title = if is_de {
        "Railbench — Fahrbetrieb, Fuehrerstand & Stellwerk (2. Monitor)"
    } else {
        "Railbench — Drive, Cab View & Signal Box (2nd Monitor)"
    };

    let vp_id = egui::ViewportId::from_hash_of("railbench_cab_view_window");
    let builder = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size([1240.0, 780.0])
        .with_min_inner_size([720.0, 460.0])
        .with_decorations(false)
        .with_resizable(true);

    // Build/sync 3D meshes only when a 3D cab pane is actually visible (skip in Stellwerk-only mode)
    let vehicles_mesh = if app.cab_view_layout != 1 {
        app.ensure_mesh_3d();
        app.gpu_3d_resident = true;
        app.vehicle_sim.get_or_build_vehicles_mesh_3d(&app.layout.scale)
    } else {
        None
    };

    ctx.show_viewport_immediate(vp_id, builder, |vp_ctx, _class| {
        if vp_ctx.input(|i| i.viewport().close_requested()) {
            app.show_cab_window = false;
            return;
        }
        let theme_id = egui::Id::new("cab_applied_theme");
        let prev_theme = vp_ctx.data(|d| d.get_temp::<bool>(theme_id));
        if prev_theme != Some(app.dark_mode) {
            crate::theme::apply_railbench_theme(vp_ctx, app.dark_mode);
            vp_ctx.data_mut(|d| d.insert_temp(theme_id, app.dark_mode));
        }
        let p = crate::theme::palette(app.dark_mode);

        let active_idx = resolve_active_vehicle_idx(&app.vehicle_sim);

        let mut actions = CabActions::default();

        // Direct keyboard driving shortcuts when cab window has focus
        if let Some(idx) = active_idx {
            if !vp_ctx.wants_keyboard_input() {
                let mut uncouple_all_key = false;
                let mut uncouple_last_key = false;
                let mut couple_key = false;
                vp_ctx.input(|inp| {
                    let mut start_sim = false;
                    let thr_up = inp.key_pressed(egui::Key::W) || inp.key_pressed(egui::Key::ArrowUp);
                    let thr_dn = inp.key_pressed(egui::Key::S) || inp.key_pressed(egui::Key::ArrowDown);
                    if thr_up {
                        if let Some(v) = app.vehicle_sim.vehicles.get_mut(idx) {
                            v.throttle = (v.throttle + 0.10).clamp(0.0, 1.0);
                            if v.throttle > 0.0 {
                                start_sim = true;
                            }
                        }
                    }
                    if thr_dn {
                        if let Some(v) = app.vehicle_sim.vehicles.get_mut(idx) {
                            v.throttle = (v.throttle - 0.10).clamp(0.0, 1.0);
                        }
                    }
                    if inp.key_pressed(egui::Key::Space) {
                        if inp.modifiers.shift {
                            app.vehicle_sim.emergency_stop_all();
                        } else if let Some(v) = app.vehicle_sim.vehicles.get_mut(idx) {
                            v.throttle = 0.0;
                            v.velocity_mm_s = 0.0;
                            v.current_speed_mm_s = 0.0;
                        }
                    }
                    if inp.key_pressed(egui::Key::R) {
                        if let Some(v) = app.vehicle_sim.vehicles.get_mut(idx) {
                            v.reverse = !v.reverse;
                        }
                    }
                    if inp.key_pressed(egui::Key::L) {
                        if let Some(v) = app.vehicle_sim.vehicles.get_mut(idx) {
                            v.headlights = !v.headlights;
                        }
                    }
                    if inp.key_pressed(egui::Key::U) {
                        if inp.modifiers.shift {
                            uncouple_all_key = true;
                        } else {
                            uncouple_last_key = true;
                        }
                    }
                    if inp.key_pressed(egui::Key::C) {
                        couple_key = true;
                    }
                    if start_sim {
                        app.vehicle_sim.running = true;
                    }
                });
                if let Some(vid) = app.vehicle_sim.vehicles.get(idx).map(|v| v.id.clone()) {
                    if uncouple_all_key {
                        actions.uncouple_at = Some((vid.clone(), 0));
                    } else if uncouple_last_key {
                        let last = app
                            .vehicle_sim
                            .vehicles
                            .get(idx)
                            .and_then(|v| v.coupled_cars.len().checked_sub(1));
                        if let Some(si) = last {
                            actions.uncouple_at = Some((vid.clone(), si));
                        }
                    }
                    if couple_key {
                        actions.couple_nearby_id = Some(vid);
                    }
                }
            }
        }

        draw_cab_titlebar(vp_ctx, app, p, is_de);
        egui::TopBottomPanel::top("cab_toolbar_divider")
            .exact_height(2.0)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(p.toolbar_btn_hover)
                    .inner_margin(Margin::ZERO)
                    .stroke(Stroke::NONE),
            )
            .show(vp_ctx, |_| {});
        draw_cab_mode_toolbar(vp_ctx, app, p, is_de, &mut actions);
        let active_idx = resolve_active_vehicle_idx(&app.vehicle_sim);
        desk::draw_cab_control_desk(vp_ctx, app, p, is_de, active_idx, &mut actions);

        let pane_bg = if app.dark_mode {
            Color32::from_rgb(13, 19, 34)
        } else {
            p.canvas_bg
        };

        // Optional bottom Stellwerk pane when in Split mode (cab_view_layout == 2)
        if app.cab_view_layout == 2 {
            let max_split_h = (vp_ctx.screen_rect().height() - 250.0).max(180.0);
            egui::TopBottomPanel::bottom("cab_split_stellwerk")
                .resizable(true)
                .default_height(280.0)
                .min_height(160.0)
                .max_height(max_split_h)
                .frame(
                    Frame::new()
                        .fill(if app.dark_mode { Color32::from_rgb(14, 21, 29) } else { p.canvas_bg })
                        .stroke(Stroke::new(1.0_f32, p.border_main)),
                )
                .show(vp_ctx, |ui| {
                    super::stellwerk_panel::draw_stellwerk_canvas(ui, app);
                });
        }

        // Center Workspace: 3D Cab View (modes 0 & 2) OR Full-Window Stellwerk (mode 1)
        egui::CentralPanel::default()
            .frame(Frame::new().fill(pane_bg))
            .show(vp_ctx, |ui| {
                if app.cab_view_layout == 1 {
                    super::stellwerk_panel::draw_stellwerk_canvas(ui, app);
                } else if let Some(idx) = active_idx.filter(|&i| i < app.vehicle_sim.vehicles.len()) {
                    viewport_and_controls::draw_active_cab_viewport(ui, app, vehicles_mesh.clone(), idx, is_de);
                } else {
                    viewport_and_controls::draw_empty_cab_state(ui, p, app.dark_mode, is_de, &mut actions.spawn);
                }
            });

        let needs_ui_refresh =
            actions.clear_all || actions.remove_id.is_some() || actions.spawn.is_some();
        apply_cab_actions(app, actions);

        if needs_ui_refresh {
            vp_ctx.request_repaint();
        } else if (app.vehicle_sim.running && !app.vehicle_sim.vehicles.is_empty())
            || app.vehicle_sim.has_moving_bridges()
            || app.vehicle_sim.has_active_uncoupler_timers()
        {
            if app.low_gpu_mode {
                vp_ctx.request_repaint_after(std::time::Duration::from_millis(33));
            } else {
                vp_ctx.request_repaint();
            }
        }
        crate::handle_window_resize_borders(vp_ctx);
    });
}

pub(super) fn draw_cab_titlebar(
    vp_ctx: &egui::Context,
    app: &mut RailbenchApp,
    p: crate::theme::RailbenchPalette,
    is_de: bool,
) {
    let is_maximized = vp_ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    egui::TopBottomPanel::top("cab_window_titlebar")
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(p.bg_toolbar)
                .inner_margin(Margin::symmetric(12, 5))
                .stroke(Stroke::NONE),
        )
        .show(vp_ctx, |ui| {
            crate::theme::style_toolbar_ui(ui, p);
            ui.horizontal(|ui| {
                let (tr_r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                paint_icon(ui.painter(), tr_r, Icon::Train, p.toolbar_accent);
                ui.add_space(2.0);
                ui.label(RichText::new("RAIL").strong().size(14.5).color(p.toolbar_accent));
                ui.add_space(-6.0);
                ui.label(RichText::new("bench").strong().size(14.5).color(Color32::WHITE));
                ui.add_space(-2.0);
                ui.label(
                    RichText::new(crate::version::APP_VERSION_TAG)
                        .size(10.5)
                        .monospace()
                        .color(Color32::from_rgb(148, 163, 184)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(if is_de {
                        "Fahrbetrieb, Fuehrerstand & Stellwerk"
                    } else {
                        "Drive, Cab View & Signal Box"
                    })
                    .size(12.0)
                    .color(Color32::from_rgb(203, 213, 225)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::theme::draw_os_window_controls(ui, vp_ctx, p) {
                        app.show_cab_window = false;
                    }
                    ui.add_space(8.0);
                    Frame::new()
                        .fill(p.toolbar_btn)
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{}.railbench", app.layout.name))
                                    .size(11.5)
                                    .color(Color32::from_rgb(203, 213, 225)),
                            );
                        });
                    ui.add_space(6.0);

                    let veh_cnt = app.vehicle_sim.vehicles.len();
                    let (status_txt, status_bg) = if app.vehicle_sim.running && veh_cnt > 0 {
                        (
                            if is_de {
                                format!("FAHRT · {veh_cnt}")
                            } else {
                                format!("RUNNING · {veh_cnt}")
                            },
                            Color32::from_rgb(22, 163, 74),
                        )
                    } else {
                        (
                            if is_de {
                                format!("PAUSE · {veh_cnt}")
                            } else {
                                format!("PAUSED · {veh_cnt}")
                            },
                            p.toolbar_btn,
                        )
                    };
                    Frame::new()
                        .fill(status_bg)
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(status_txt)
                                    .size(10.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                        });

                    let drag_rect = ui.available_rect_before_wrap();
                    if drag_rect.width() > 8.0 {
                        let drag_resp = ui.interact(
                            drag_rect,
                            egui::Id::new("cab_titlebar_drag"),
                            Sense::click_and_drag(),
                        );
                        if drag_resp.double_clicked() {
                            vp_ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        } else if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                            vp_ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    }
                });
            });
        });
}

pub(super) fn draw_cab_mode_toolbar(
    vp_ctx: &egui::Context,
    app: &mut RailbenchApp,
    p: crate::theme::RailbenchPalette,
    is_de: bool,
    actions: &mut CabActions,
) {
    let mode_bar_resp = egui::TopBottomPanel::top("cab_window_mode_bar")
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(p.bg_toolbar)
                .inner_margin(Margin::symmetric(10, 5))
                .stroke(Stroke::NONE),
        )
        .show(vp_ctx, |ui| {
            crate::theme::style_toolbar_ui(ui, p);
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
            ui.horizontal(|ui| {
                // Mode register tabs
                ui.spacing_mut().item_spacing.x = 3.0;
                for (mode_id, lbl_de, lbl_en) in [
                    (2u8, "Geteilt (Cab + Stellwerk)", "Split (Cab + Signal Box)"),
                    (0u8, "Fuehrerstand", "Cab View"),
                    (1u8, "Stellwerk", "Signal Box"),
                ] {
                    let active = app.cab_view_layout == mode_id;
                    let fill = if active { p.primary_hover } else { Color32::TRANSPARENT };
                    let txt_col = if active {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(203, 213, 225)
                    };
                    let tab_w = super::stable_text_width(ui, lbl_de, lbl_en, Some(11.5), true)
                        + ui.spacing().button_padding.x * 2.0;
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if is_de { lbl_de } else { lbl_en })
                                    .strong()
                                    .size(11.5)
                                    .color(txt_col),
                            )
                            .fill(fill)
                            .stroke(if active {
                                Stroke::new(1.0_f32, p.primary)
                            } else {
                                Stroke::NONE
                            })
                            .min_size(egui::vec2(tab_w, 26.0))
                            .corner_radius(CornerRadius::same(5)),
                        )
                        .clicked()
                    {
                        app.cab_view_layout = mode_id;
                    }
                }

                ui.spacing_mut().item_spacing.x = 5.0;
                ui.separator();

                // Master Simulation Play/Pause + Emergency Stop
                let play_de = if app.vehicle_sim.running { "Pause" } else { "Fahren" };
                let play_en = if app.vehicle_sim.running { "Pause" } else { "Start" };
                let play_w = super::stable_text_width(ui, play_de, play_en, Some(11.5), true) + 18.0;
                let play_fill = if app.vehicle_sim.running {
                    Color32::from_rgb(217, 119, 6)
                } else {
                    Color32::from_rgb(22, 163, 74)
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(if is_de { play_de } else { play_en })
                                .strong()
                                .size(11.5)
                                .color(Color32::WHITE),
                        )
                        .fill(play_fill)
                        .stroke(Stroke::NONE)
                        .min_size(egui::vec2(play_w, 26.0))
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    app.vehicle_sim.running = !app.vehicle_sim.running;
                }

                let estop_w = super::stable_text_width(ui, "Not-Halt", "E-Stop", Some(11.5), true) + 18.0;
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(if is_de { "Not-Halt" } else { "E-Stop" })
                                .strong()
                                .size(11.5)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(185, 28, 28))
                        .stroke(Stroke::NONE)
                        .min_size(egui::vec2(estop_w, 26.0))
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    app.vehicle_sim.emergency_stop_all();
                }

                // Cab 3D Camera + Environment Controls
                if app.cab_view_layout != 1 {
                    ui.separator();
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (mode, lbl_de, lbl_en) in [
                        (CabCameraMode::Cab, "Fuehrerstand", "Cab"),
                        (CabCameraMode::Roof, "Dach", "Roof"),
                        (CabCameraMode::Chase, "Verfolger", "Chase"),
                    ] {
                        let active = app.cab_camera_mode == mode;
                        let cw = super::stable_text_width(ui, lbl_de, lbl_en, Some(11.0), true) + 14.0;
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if is_de { lbl_de } else { lbl_en })
                                        .strong()
                                        .size(11.0)
                                        .color(if active { Color32::WHITE } else { Color32::from_rgb(203, 213, 225) }),
                                )
                                .fill(if active { p.primary_hover } else { p.toolbar_btn })
                                .stroke(if active {
                                    Stroke::new(1.0_f32, p.primary)
                                } else {
                                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35))
                                })
                                .min_size(egui::vec2(cw, 26.0))
                                .corner_radius(CornerRadius::same(5)),
                            )
                            .clicked()
                        {
                            app.cab_camera_mode = mode;
                            app.cab_look_yaw = 0.0;
                            app.cab_look_pitch = 0.0;
                        }
                    }

                    ui.spacing_mut().item_spacing.x = 5.0;
                    let ctr_w = super::stable_text_width(ui, "Zentrieren", "Center", Some(11.0), false) + 14.0;
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if is_de { "Zentrieren" } else { "Center" })
                                    .size(11.0)
                                    .color(Color32::WHITE),
                            )
                            .fill(p.toolbar_btn)
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35)))
                            .min_size(egui::vec2(ctr_w, 26.0))
                            .corner_radius(CornerRadius::same(5)),
                        )
                        .on_hover_text(if is_de { "Blick zentrieren" } else { "Center cab look" })
                        .clicked()
                    {
                        app.cab_look_yaw = 0.0;
                        app.cab_look_pitch = 0.0;
                    }

                    let env_lbl = match app.viewer3d_env {
                        Viewer3DEnv::HallLandscape => if is_de { "Halle" } else { "Hall" },
                        Viewer3DEnv::Minimal => "CAD",
                    };
                    let env_w = super::stable_text_width(ui, "Halle", "Hall", Some(11.0), false) + 16.0;
                    if ui
                        .add(
                            egui::Button::new(RichText::new(env_lbl).size(11.0).color(Color32::WHITE))
                                .fill(p.toolbar_btn)
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35)))
                                .min_size(egui::vec2(env_w, 26.0))
                                .corner_radius(CornerRadius::same(5)),
                        )
                        .clicked()
                    {
                        app.viewer3d_env = match app.viewer3d_env {
                            Viewer3DEnv::HallLandscape => Viewer3DEnv::Minimal,
                            Viewer3DEnv::Minimal => Viewer3DEnv::HallLandscape,
                        };
                    }

                    let is_night = app.time_of_day >= 19.5 || app.time_of_day < 6.0;
                    if icon_button_sized(
                        ui,
                        if is_night { Icon::Moon } else { Icon::Sun },
                        egui::vec2(28.0, 26.0),
                        is_night,
                        p.toolbar_btn,
                        p.primary_hover,
                        p.primary,
                        Color32::WHITE,
                        if is_de { "Tag / Nacht wechseln" } else { "Toggle Day / Night" },
                    )
                    .clicked()
                    {
                        app.time_of_day = if is_night { 13.5 } else { 22.0 };
                    }
                }

                // Right-aligned utilities (Fit + Clear All)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let fit_w = super::stable_text_width(ui, "Einpassen", "Fit", Some(11.5), false)
                        + ui.spacing().button_padding.x * 2.0;
                    if app.cab_view_layout >= 1
                        && ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if is_de { "Einpassen" } else { "Fit" })
                                        .size(11.5)
                                        .color(Color32::WHITE),
                                )
                                .fill(p.toolbar_btn)
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(148, 163, 184, 35),
                                ))
                                .min_size(egui::vec2(fit_w, 26.0))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                    {
                        app.stellwerk_zoom = 1.0;
                        app.stellwerk_pan = [0.0, 0.0];
                        app.stellwerk_layer_views.clear();
                    }

                    if !app.vehicle_sim.vehicles.is_empty() {
                        let clr_w = super::stable_text_width(ui, "Alle entfernen", "Clear All", Some(11.0), false) + 16.0;
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if is_de { "Alle entfernen" } else { "Clear All" })
                                        .size(11.0)
                                        .color(Color32::from_rgb(252, 165, 165)),
                                )
                                .fill(p.toolbar_btn)
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(239, 68, 68, 90)))
                                .min_size(egui::vec2(clr_w, 26.0))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            actions.clear_all = true;
                        }
                    }
                });
            });
        });

    let mode_bottom_col = if app.dark_mode {
        p.border_main
    } else {
        Color32::from_rgba_unmultiplied(15, 23, 42, 95)
    };
    let mr = mode_bar_resp.response.rect;
    vp_ctx.layer_painter(mode_bar_resp.response.layer_id).hline(
        mr.x_range(),
        mr.bottom(),
        Stroke::new(1.0_f32, mode_bottom_col),
    );
}


fn apply_cab_actions(app: &mut RailbenchApp, actions: CabActions) {
    if actions.clear_all {
        app.vehicle_sim.clear_all();
        app.cab_look_yaw = 0.0;
        app.cab_look_pitch = 0.0;
    }
    if let Some(kind) = actions.spawn {
        let pref = if app.selected.len() == 1 {
            app.selected.iter().next().cloned()
        } else {
            None
        };
        match app
            .vehicle_sim
            .spawn_vehicle(&app.layout, &app.catalogs, kind, pref.as_deref())
        {
            Ok(_) => app.vehicle_sim.running = true,
            Err(e) => app.status_message = Some(e),
        }
    }
    if let Some(rid) = actions.remove_id {
        app.vehicle_sim.remove_vehicle(&rid);
    }
    if let Some((x, y)) = actions.focus_pt {
        app.focus_world_point(x, y);
    }
    if let Some((vid, wk)) = actions.add_wagon {
        app.vehicle_sim.add_wagon(&vid, wk, &app.layout, &app.catalogs);
    }
    if let Some((vid, split_idx)) = actions.uncouple_at {
        app.vehicle_sim
            .uncouple_at(&vid, split_idx, &app.layout, &app.catalogs);
    }
    if let Some(vid) = actions.couple_nearby_id {
        app.vehicle_sim
            .couple_nearby_wagons(&vid, &app.layout, &app.catalogs);
    }
    if let Some(pid) = actions.trigger_uncoupler {
        app.vehicle_sim
            .trigger_uncoupler(&pid, &app.layout, &app.catalogs);
    }
    if let Some(pid) = actions.toggle_uncoupler_armed {
        app.vehicle_sim.toggle_uncoupler_armed(&pid);
    }
    if let Some((tid, cnt)) = actions.toggle_turnout {
        app.vehicle_sim.toggle_turnout(&tid, cnt);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cab_camera_offsets_clear_loco_cab_walls() {
        let (br243_fwd, br243_h, br243_side, _) = cab_camera_offsets(VehicleKind::Br243, false);
        assert!(br243_fwd > 0.49 && br243_h > 0.70 && br243_side.abs() < 1e-4);

        let (v60_fwd, v60_h, _, _) = cab_camera_offsets(VehicleKind::V60Diesel, false);
        assert!(v60_fwd > 0.05 && v60_fwd < 0.15 && v60_h > 0.80);

        let (v60_back, _, _, _) = cab_camera_offsets(VehicleKind::V60Diesel, true);
        assert!(v60_back > 0.17 && v60_back < 0.25);

        let (br96_fwd, br96_h, br96_side, _) = cab_camera_offsets(VehicleKind::Br96Mallet, false);
        assert!(br96_fwd > -0.23 && br96_fwd < -0.19);
        assert!(br96_h >= 0.85 && br96_side > 0.15);
    }

    #[test]
    fn clear_all_action_empties_vehicles_without_closing_cab_window() {
        let mut app = RailbenchApp::default();
        app.show_cab_window = true;
        app.cab_look_yaw = 0.4;
        app.cab_look_pitch = -0.2;

        let catalogs = crate::catalog::builtin_catalogs();
        let e24188 = crate::catalog::find_entry(&catalogs, "24188").unwrap();
        app.layout.pieces = vec![crate::catalog::create_piece(
            "t1".into(),
            e24188,
            railbench_core::types::Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        )];
        app.vehicle_sim
            .spawn_vehicle(&app.layout, &catalogs, VehicleKind::Br243, None)
            .expect("spawn succeeds");
        app.vehicle_sim.running = true;
        assert!(resolve_active_vehicle_idx(&app.vehicle_sim).is_some());

        apply_cab_actions(
            &mut app,
            CabActions {
                clear_all: true,
                ..Default::default()
            },
        );

        assert!(app.vehicle_sim.vehicles.is_empty());
        assert!(app.vehicle_sim.selected_vehicle_id.is_none());
        assert!(!app.vehicle_sim.running);
        assert!(resolve_active_vehicle_idx(&app.vehicle_sim).is_none());
        assert!(app.show_cab_window);
        assert_eq!(app.cab_look_yaw, 0.0);
        assert_eq!(app.cab_look_pitch, 0.0);
    }

    #[test]
    fn cab_camera_smoother_damps_jitter_wraps_yaw_and_snaps_on_switch() {
        let mut sm = CabCameraSmoother::default();
        // First sample snaps immediately
        let (e0, y0, p0) = sm.update(
            "veh-1",
            CabCameraMode::Cab,
            false,
            [10.0, 20.0, 30.0],
            0.1,
            -0.05,
            Some(0.016),
        );
        assert_eq!(e0, [10.0, 20.0, 30.0]);
        assert!((y0 - 0.1).abs() < 1e-5);
        assert!((p0 - (-0.05)).abs() < 1e-5);

        // Small step + wrap across 0 / 2pi moves smoothly via shortest arc (no 2pi flip)
        let (e1, y1, _) = sm.update(
            "veh-1",
            CabCameraMode::Cab,
            false,
            [20.0, 20.0, 30.0],
            std::f32::consts::TAU - 0.1,
            -0.05,
            Some(0.016),
        );
        assert!(e1[0] > 10.0 && e1[0] < 20.0, "eye X should damp between 10 and 20, got {}", e1[0]);
        assert!(y1 > std::f32::consts::PI || y1 < 0.1, "yaw must cross 0 via shortest arc, got {y1}");

        // Switching vehicle snaps immediately without lag
        let (e2, y2, _) = sm.update(
            "veh-2",
            CabCameraMode::Cab,
            false,
            [500.0, 20.0, -40.0],
            1.2,
            0.0,
            Some(0.016),
        );
        assert_eq!(e2, [500.0, 20.0, -40.0]);
        assert!((y2 - 1.2).abs() < 1e-5);

        // Switching camera mode snaps immediately too
        let (e3, _, _) = sm.update(
            "veh-2",
            CabCameraMode::Chase,
            false,
            [420.0, 80.0, -40.0],
            1.2,
            -0.16,
            Some(0.016),
        );
        assert_eq!(e3, [420.0, 80.0, -40.0]);
    }
}
