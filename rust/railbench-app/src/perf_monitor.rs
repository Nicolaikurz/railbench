use std::collections::VecDeque;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Pos2, Rect, RichText, Stroke};
use railbench_core::types::Layout;

use crate::i18n::Lang;
use crate::viewer3d::Scene3DBundle;

#[derive(Debug, Clone)]
pub struct GpuAdapterMeta {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub driver: String,
}

impl Default for GpuAdapterMeta {
    fn default() -> Self {
        Self {
            name: "wgpu Adapter (Headless)".into(),
            backend: "Unknown".into(),
            device_type: "Unknown".into(),
            driver: String::new(),
        }
    }
}

impl GpuAdapterMeta {
    pub fn from_creation_context(cc: &eframe::CreationContext<'_>) -> Self {
        let Some(rs) = &cc.wgpu_render_state else {
            return Self::default();
        };
        let info = rs.adapter.get_info();
        let driver = if info.driver_info.is_empty() {
            info.driver
        } else if info.driver.is_empty() {
            info.driver_info
        } else {
            format!("{} ({})", info.driver, info.driver_info)
        };
        Self {
            name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            driver,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GpuTelemetry {
    pub busy_pct: Option<f32>,
    pub vram_used_mb: Option<f64>,
    pub vram_total_mb: Option<f64>,
    pub temp_c: Option<f32>,
}

pub struct PerfMonitor {
    pub visible: bool,
    last_frame_instant: Option<Instant>,
    window_start: Option<Instant>,
    window_frames: u32,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub cpu_frame_ms: f32,
    fps_history: VecDeque<f32>,

    last_sys_poll: Option<Instant>,
    prev_sys_total: Option<u64>,
    prev_sys_idle: Option<u64>,
    prev_proc_ticks: Option<u64>,
    pub cpu_cores: usize,
    pub process_threads: usize,
    pub cpu_model: String,
    pub sys_cpu_pct: Option<f32>,
    pub app_cpu_pct: Option<f32>,
    pub app_ram_mb: Option<f64>,
    pub sys_ram_used_gb: Option<f64>,
    pub sys_ram_total_gb: Option<f64>,

    pub gpu_meta: GpuAdapterMeta,
    pub gpu_telemetry: GpuTelemetry,
    nvidia_slot: Arc<Mutex<Option<GpuTelemetry>>>,
    nvidia_poll_in_flight: Arc<AtomicBool>,
    nvidia_disabled: Arc<AtomicBool>,
    last_nvidia_poll: Option<Instant>,
}

impl Default for PerfMonitor {
    fn default() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let cpu_model = std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|s| parse_cpu_model_name(&s))
            .unwrap_or_else(|| format!("{cores}-Core CPU"));
        Self {
            visible: false,
            last_frame_instant: None,
            window_start: None,
            window_frames: 0,
            fps: 60.0,
            frame_time_ms: 16.6,
            cpu_frame_ms: 0.5,
            fps_history: VecDeque::with_capacity(64),
            last_sys_poll: None,
            prev_sys_total: None,
            prev_sys_idle: None,
            prev_proc_ticks: None,
            cpu_cores: cores,
            process_threads: cores,
            cpu_model,
            sys_cpu_pct: None,
            app_cpu_pct: None,
            app_ram_mb: None,
            sys_ram_used_gb: None,
            sys_ram_total_gb: None,
            gpu_meta: GpuAdapterMeta::default(),
            gpu_telemetry: GpuTelemetry::default(),
            nvidia_slot: Arc::new(Mutex::new(None)),
            nvidia_poll_in_flight: Arc::new(AtomicBool::new(false)),
            nvidia_disabled: Arc::new(AtomicBool::new(false)),
            last_nvidia_poll: None,
        }
    }
}

impl PerfMonitor {
    pub fn set_gpu_from_creation_context(&mut self, cc: &eframe::CreationContext<'_>) {
        self.gpu_meta = GpuAdapterMeta::from_creation_context(cc);
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.last_frame_instant = Some(Instant::now());
            self.window_start = Some(Instant::now());
            self.window_frames = 0;
            self.last_sys_poll = None;
            self.poll_system_metrics();
        }
    }

    pub fn begin_frame(&mut self) -> Instant {
        let now = Instant::now();
        if let Some(prev) = self.last_frame_instant {
            let dt_ms = (now - prev).as_secs_f32() * 1000.0;
            if dt_ms.is_finite() && dt_ms > 0.01 && dt_ms < 2000.0 {
                self.frame_time_ms = self.frame_time_ms * 0.75 + dt_ms * 0.25;
            }
        }
        self.last_frame_instant = Some(now);

        let wstart = *self.window_start.get_or_insert(now);
        self.window_frames = self.window_frames.saturating_add(1);
        let elapsed = (now - wstart).as_secs_f32();
        if elapsed >= 0.45 && self.window_frames > 0 {
            let inst_fps = self.window_frames as f32 / elapsed;
            self.fps = inst_fps;
            if self.fps_history.len() >= 60 {
                self.fps_history.pop_front();
            }
            self.fps_history.push_back(inst_fps);
            self.window_start = Some(now);
            self.window_frames = 0;
        }

        if self.visible {
            let need_poll = self
                .last_sys_poll
                .map_or(true, |last| now.duration_since(last) >= Duration::from_millis(500));
            if need_poll {
                self.poll_system_metrics();
                self.last_sys_poll = Some(now);
            }
        }
        now
    }

    pub fn end_frame(&mut self, frame_start: Instant) {
        let cpu_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        if cpu_ms.is_finite() {
            self.cpu_frame_ms = self.cpu_frame_ms * 0.8 + cpu_ms * 0.2;
        }
    }

    fn poll_system_metrics(&mut self) {
        if let Ok(stat_txt) = std::fs::read_to_string("/proc/stat") {
            if let Some((total, idle)) = parse_proc_stat_cpu(&stat_txt) {
                let self_ticks = std::fs::read_to_string("/proc/self/stat")
                    .ok()
                    .and_then(|s| parse_proc_self_stat_ticks(&s));

                if let (Some(pt), Some(pi)) = (self.prev_sys_total, self.prev_sys_idle) {
                    let dt = total.saturating_sub(pt);
                    let di = idle.saturating_sub(pi);
                    if dt > 0 {
                        let busy = dt.saturating_sub(di) as f32 / dt as f32 * 100.0;
                        self.sys_cpu_pct = Some(busy.clamp(0.0, 100.0));
                        if let (Some(cur_p), Some(prev_p)) = (self_ticks, self.prev_proc_ticks) {
                            let dp = cur_p.saturating_sub(prev_p);
                            let app_pct = (dp as f32 / dt as f32) * 100.0;
                            self.app_cpu_pct = Some(app_pct.clamp(0.0, 100.0));
                        }
                    }
                }
                self.prev_sys_total = Some(total);
                self.prev_sys_idle = Some(idle);
                self.prev_proc_ticks = self_ticks;
            }
        }

        if let Ok(status_txt) = std::fs::read_to_string("/proc/self/status") {
            if let Some(rss_kb) = parse_vm_rss_kb(&status_txt) {
                self.app_ram_mb = Some(rss_kb as f64 / 1024.0);
            }
            if let Some(th) = parse_proc_threads(&status_txt) {
                self.process_threads = th;
            }
        }

        if let Ok(mem_txt) = std::fs::read_to_string("/proc/meminfo") {
            if let Some((total_kb, avail_kb)) = parse_meminfo_kb(&mem_txt) {
                let used_kb = total_kb.saturating_sub(avail_kb);
                self.sys_ram_total_gb = Some(total_kb as f64 / (1024.0 * 1024.0));
                self.sys_ram_used_gb = Some(used_kb as f64 / (1024.0 * 1024.0));
            }
        }

        let drm = read_linux_drm_gpu();
        if drm.busy_pct.is_some() || drm.vram_total_mb.is_some() {
            self.gpu_telemetry = drm;
        } else {
            if let Ok(mut guard) = self.nvidia_slot.try_lock() {
                if let Some(latest) = guard.take() {
                    self.gpu_telemetry = latest;
                }
            }
            if !self.nvidia_disabled.load(Ordering::Relaxed)
                && !self.nvidia_poll_in_flight.load(Ordering::Relaxed)
                && self
                    .last_nvidia_poll
                    .map_or(true, |last| last.elapsed() >= Duration::from_secs(1))
            {
                self.last_nvidia_poll = Some(Instant::now());
                self.nvidia_poll_in_flight.store(true, Ordering::Relaxed);
                let slot = Arc::clone(&self.nvidia_slot);
                let in_flight = Arc::clone(&self.nvidia_poll_in_flight);
                let disabled = Arc::clone(&self.nvidia_disabled);
                std::thread::spawn(move || {
                    let mut cmd = Command::new("nvidia-smi");
                    cmd.args([
                        "--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu",
                        "--format=csv,noheader,nounits",
                    ]);
                    #[cfg(target_os = "windows")]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
                    }
                    match cmd.output() {
                        Ok(out) if out.status.success() => {
                            let stdout = String::from_utf8_lossy(&out.stdout);
                            if let Some(first_line) = stdout.lines().next() {
                                if let Some(parsed) = parse_nvidia_smi_csv(first_line) {
                                    if let Ok(mut g) = slot.lock() {
                                        *g = Some(parsed);
                                    }
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            disabled.store(true, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                    in_flight.store(false, Ordering::Relaxed);
                });
            }
        }
    }

    pub fn draw_overlay(
        &mut self,
        ctx: &egui::Context,
        layout: &Layout,
        mesh_3d: Option<&Scene3DBundle>,
        is_3d_mode: bool,
        lang: Lang,
    ) -> Option<usize> {
        if !self.visible {
            return None;
        }
        let is_de = lang == Lang::De;
        let mut open = self.visible;
        let mut stress_request: Option<usize> = None;

        let default_pos = ctx.screen_rect().max - egui::vec2(298.0, 396.0);
        let title = if is_de {
            "HARDWARE MONITOR (F3)"
        } else {
            "HARDWARE MONITOR (F3)"
        };

        egui::Window::new(title)
            .id(egui::Id::new("railbench_perf_monitor_window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(280.0)
            .default_pos(default_pos)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 236))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(10))
                    .stroke(Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 120))),
            )
            .show(ctx, |ui| {
                // 1. FPS + Sparkline
                let fps_col = if self.fps >= 55.0 {
                    Color32::from_rgb(16, 185, 129)
                } else if self.fps >= 30.0 {
                    Color32::from_rgb(245, 158, 11)
                } else {
                    Color32::from_rgb(239, 68, 68)
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new("FPS:").size(11.5).color(Color32::from_rgb(148, 163, 184)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:.0} FPS ({:.1} ms)", self.fps, self.frame_time_ms))
                                .strong()
                                .size(12.0)
                                .color(fps_col),
                        );
                    });
                });
                draw_fps_sparkline(ui, &self.fps_history, fps_col);
                ui.add_space(4.0);

                // 2. CPU Metrics
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CPU:").size(11.5).color(Color32::from_rgb(148, 163, 184)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let sys_s = self.sys_cpu_pct.map_or("--".into(), |v| format!("{v:.1}%"));
                        let app_s = self.app_cpu_pct.map_or("--".into(), |v| format!("{v:.1}%"));
                        ui.label(
                            RichText::new(format!("Sys {sys_s} | App {app_s} ({:.2} ms)", self.cpu_frame_ms))
                                .size(11.0)
                                .color(Color32::WHITE),
                        );
                    });
                });
                if let Some(sys_pct) = self.sys_cpu_pct {
                    draw_usage_bar(ui, sys_pct / 100.0, Color32::from_rgb(56, 189, 248));
                }
                ui.label(
                    RichText::new(format!(
                        "{} ({} Cores | {} Active Threads)",
                        self.cpu_model, self.cpu_cores, self.process_threads
                    ))
                    .size(10.0)
                    .color(Color32::from_rgb(100, 116, 139)),
                );
                ui.add_space(3.0);

                // 3. RAM Metrics
                ui.horizontal(|ui| {
                    ui.label(RichText::new("RAM:").size(11.5).color(Color32::from_rgb(148, 163, 184)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let app_mb = self.app_ram_mb.map_or("--".into(), |v| format!("{v:.1} MB"));
                        let sys_s = match (self.sys_ram_used_gb, self.sys_ram_total_gb) {
                            (Some(u), Some(t)) if t > 0.0 => format!("{u:.1}/{t:.1} GB"),
                            _ => "--".into(),
                        };
                        ui.label(
                            RichText::new(format!("App {app_mb} | Sys {sys_s}"))
                                .size(11.0)
                                .color(Color32::WHITE),
                        );
                    });
                });
                if let (Some(u), Some(t)) = (self.sys_ram_used_gb, self.sys_ram_total_gb) {
                    if t > 0.0 {
                        draw_usage_bar(ui, (u / t) as f32, Color32::from_rgb(168, 85, 247));
                    }
                }
                ui.add_space(3.0);

                // 4. GPU Adapter + Live Telemetry + 3D Buffer Stats
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "GPU: {} [{} | {}]",
                        self.gpu_meta.name, self.gpu_meta.backend, self.gpu_meta.device_type
                    ))
                    .strong()
                    .size(10.5)
                    .color(Color32::from_rgb(56, 189, 248)),
                );
                if !self.gpu_meta.driver.is_empty() {
                    ui.label(
                        RichText::new(format!("Driver: {}", self.gpu_meta.driver))
                            .size(9.8)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                }
                if let Some(gpu_pct) = self.gpu_telemetry.busy_pct {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("GPU Load:").size(10.8).color(Color32::from_rgb(148, 163, 184)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let temp = self
                                .gpu_telemetry
                                .temp_c
                                .map_or(String::new(), |t| format!(" | {t:.0} C"));
                            ui.label(
                                RichText::new(format!("{gpu_pct:.0}%{temp}"))
                                    .size(10.8)
                                    .color(Color32::WHITE),
                            );
                        });
                    });
                    draw_usage_bar(ui, gpu_pct / 100.0, Color32::from_rgb(34, 197, 94));
                }
                if let (Some(vu), Some(vt)) = (self.gpu_telemetry.vram_used_mb, self.gpu_telemetry.vram_total_mb) {
                    if vt > 0.0 {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("VRAM:").size(10.8).color(Color32::from_rgb(148, 163, 184)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{vu:.0} / {vt:.0} MB ({:.0}%)", (vu / vt) * 100.0))
                                        .size(10.8)
                                        .color(Color32::WHITE),
                                );
                            });
                        });
                    }
                }
                if let Some(mesh) = mesh_3d {
                    let mb = mesh.gpu_buffer_bytes() as f64 / (1024.0 * 1024.0);
                    ui.label(
                        RichText::new(format!(
                            "3D Mesh: {} tris | {} verts ({mb:.2} MB)",
                            mesh.triangle_count(),
                            mesh.vertex_count()
                        ))
                        .size(10.2)
                        .color(Color32::from_rgb(148, 163, 184)),
                    );
                    let proto_cnt = mesh.track_prototype_count();
                    let inst_cnt = mesh.track_instance_count();
                    if proto_cnt > 0 {
                        ui.label(
                            RichText::new(format!(
                                "GPU Instancing: {proto_cnt} track types -> {inst_cnt} instances"
                            ))
                            .size(9.8)
                            .color(Color32::from_rgb(56, 189, 248)),
                        );
                    }
                }

                // 5. Scene Objects & Mode
                ui.separator();
                let piece_cnt = layout.pieces.len();
                let model_cnt = layout.models.as_ref().map_or(0, |m| m.len());
                let board_cnt = crate::board_tools::get_baseboards_slice(layout).len();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if is_de { "Szene:" } else { "Scene:" })
                            .size(10.8)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mode_tag = if is_3d_mode { "3D" } else { "2D" };
                        ui.label(
                            RichText::new(format!(
                                "{piece_cnt} tracks | {model_cnt} models | {board_cnt} boards [{mode_tag}]"
                            ))
                            .size(10.5)
                            .color(Color32::WHITE),
                        );
                    });
                });

                // 6. Stress Test Buttons (matches TS PerfMonitor)
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if is_de { "Stresstest:" } else { "Stress Test:" })
                            .size(10.5)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                    if ui.small_button("+500").clicked() {
                        stress_request = Some(500);
                    }
                    if ui.small_button("+1000").clicked() {
                        stress_request = Some(1000);
                    }
                });
            });

        self.visible = open;
        stress_request
    }
}

fn draw_usage_bar(ui: &mut egui::Ui, frac: f32, col: Color32) {
    let f = frac.clamp(0.0, 1.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 5.0), egui::Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 2.0, Color32::from_rgb(30, 41, 59));
    if f > 0.0 {
        let fill_r = Rect::from_min_max(r.min, Pos2::new(r.min.x + r.width() * f, r.max.y));
        p.rect_filled(fill_r, 2.0, col);
    }
}

fn draw_fps_sparkline(ui: &mut egui::Ui, samples: &VecDeque<f32>, stroke_col: Color32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
    let painter = ui.painter_at(r);
    painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(2, 6, 23, 180));
    painter.rect_stroke(
        r,
        4.0,
        Stroke::new(0.8_f32, Color32::from_rgba_unmultiplied(51, 65, 85, 180)),
        egui::StrokeKind::Inside,
    );
    if samples.len() < 2 {
        return;
    }
    let max_fps = samples.iter().copied().fold(60.0_f32, f32::max).max(60.0);
    let n = (samples.len() - 1) as f32;
    let pts: Vec<Pos2> = samples
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let x = r.min.x + 2.0 + (i as f32 / n) * (r.width() - 4.0);
            let y = r.max.y - 2.0 - (val / max_fps).clamp(0.0, 1.0) * (r.height() - 4.0);
            Pos2::new(x, y)
        })
        .collect();
    painter.add(egui::Shape::line(pts, Stroke::new(1.5_f32, stroke_col)));
}

// ============================================================================
//  Pure-Rust Linux ProcFS & DRM SysFS Parsers
// ============================================================================

pub fn parse_cpu_model_name(cpuinfo: &str) -> Option<String> {
    for line in cpuinfo.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            if key == "model name" || key == "Hardware" || key == "Processor" {
                let name = v.trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

pub fn parse_proc_stat_cpu(text: &str) -> Option<(u64, u64)> {
    let line = text.lines().find(|l| l.starts_with("cpu "))?;
    let mut vals = line.split_whitespace().skip(1).filter_map(|s| s.parse::<u64>().ok());
    let user = vals.next()?;
    let nice = vals.next()?;
    let sys = vals.next()?;
    let idle = vals.next()?;
    let iowait = vals.next().unwrap_or(0);
    let irq = vals.next().unwrap_or(0);
    let softirq = vals.next().unwrap_or(0);
    let steal = vals.next().unwrap_or(0);
    let total = user + nice + sys + idle + iowait + irq + softirq + steal;
    Some((total, idle + iowait))
}

pub fn parse_proc_self_stat_ticks(text: &str) -> Option<u64> {
    let rparen = text.rfind(')')?;
    let rest = text.get(rparen + 1..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // Fields after `(comm)`: index 0 is field 3 (`state`); field 14 (`utime`) is 11; field 15 (`stime`) is 12.
    let utime = fields.get(11)?.parse::<u64>().ok()?;
    let stime = fields.get(12)?.parse::<u64>().ok()?;
    Some(utime + stime)
}

pub fn parse_vm_rss_kb(status_text: &str) -> Option<u64> {
    for line in status_text.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.split_whitespace().next()?.parse::<u64>().ok();
        }
    }
    None
}

pub fn parse_proc_threads(status_text: &str) -> Option<usize> {
    for line in status_text.lines() {
        if let Some(rest) = line.strip_prefix("Threads:") {
            return rest.split_whitespace().next()?.parse::<usize>().ok();
        }
    }
    None
}

pub fn parse_meminfo_kb(meminfo_text: &str) -> Option<(u64, u64)> {
    let mut total = None;
    let mut avail = None;
    let mut free = None;
    for line in meminfo_text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            avail = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("MemFree:") {
            free = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok());
        }
    }
    Some((total?, avail.or(free)?))
}

pub fn read_linux_drm_gpu() -> GpuTelemetry {
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return GpuTelemetry::default();
    };
    let mut best = GpuTelemetry::default();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_s = name.to_string_lossy();
        if !name_s.starts_with("card") || name_s.contains('-') {
            continue;
        }
        let dev = entry.path().join("device");
        if !dev.exists() {
            continue;
        }
        let busy_pct = std::fs::read_to_string(dev.join("gpu_busy_percent"))
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok());
        let vram_used_mb = std::fs::read_to_string(dev.join("mem_info_vram_used"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|b| b as f64 / (1024.0 * 1024.0));
        let vram_total_mb = std::fs::read_to_string(dev.join("mem_info_vram_total"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|b| b as f64 / (1024.0 * 1024.0));
        let temp_c = read_drm_temp_c(&dev.join("hwmon"));

        if busy_pct.is_some() || vram_total_mb.is_some() {
            let cand = GpuTelemetry {
                busy_pct,
                vram_used_mb,
                vram_total_mb,
                temp_c,
            };
            if cand.busy_pct.is_some() {
                return cand;
            }
            if best.vram_total_mb.is_none() {
                best = cand;
            }
        }
    }
    best
}

fn read_drm_temp_c(hwmon_dir: &Path) -> Option<f32> {
    let entries = std::fs::read_dir(hwmon_dir).ok()?;
    for entry in entries.flatten() {
        let tpath = entry.path().join("temp1_input");
        if let Ok(raw) = std::fs::read_to_string(tpath) {
            if let Ok(millideg) = raw.trim().parse::<i64>() {
                return Some(millideg as f32 / 1000.0);
            }
        }
    }
    None
}

pub fn parse_nvidia_smi_csv(line: &str) -> Option<GpuTelemetry> {
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 4 {
        return None;
    }
    let busy_pct = parts[0].parse::<f32>().ok();
    let vram_used_mb = parts[1].parse::<f64>().ok();
    let vram_total_mb = parts[2].parse::<f64>().ok();
    let temp_c = parts[3].parse::<f32>().ok();
    Some(GpuTelemetry {
        busy_pct,
        vram_used_mb,
        vram_total_mb,
        temp_c,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proc_stat_and_self_ticks_work() {
        let proc_stat = "cpu  100 10 40 800 50 0 0 0 0 0\ncpu0 50 5 20 400 25 0 0 0 0 0\n";
        let (total, idle) = parse_proc_stat_cpu(proc_stat).unwrap();
        assert_eq!(total, 1000);
        assert_eq!(idle, 850);

        let self_stat = "12345 (railbench (dev)) S 1 1 1 0 -1 4194304 100 0 2 0 320 80 0 0 20 0 4 0 100 0 0";
        let ticks = parse_proc_self_stat_ticks(self_stat).unwrap();
        assert_eq!(ticks, 400);
    }

    #[test]
    fn parse_ram_status_and_meminfo_work() {
        let status = "Name:\trailbench-app\nVmSize:\t512000 kB\nVmRSS:\t131072 kB\nThreads:\t8\n";
        assert_eq!(parse_vm_rss_kb(status), Some(131072));
        assert_eq!(parse_proc_threads(status), Some(8));

        let meminfo = "MemTotal:       16384000 kB\nMemFree:         2048000 kB\nMemAvailable:    8192000 kB\n";
        assert_eq!(parse_meminfo_kb(meminfo), Some((16_384_000, 8_192_000)));
    }

    #[test]
    fn parse_nvidia_smi_csv_parses_metrics() {
        let parsed = parse_nvidia_smi_csv("42, 1024, 8192, 61").unwrap();
        assert_eq!(parsed.busy_pct, Some(42.0));
        assert_eq!(parsed.vram_used_mb, Some(1024.0));
        assert_eq!(parsed.vram_total_mb, Some(8192.0));
        assert_eq!(parsed.temp_c, Some(61.0));
    }
}
