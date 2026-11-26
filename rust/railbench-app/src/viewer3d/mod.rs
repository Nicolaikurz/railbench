use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use eframe::egui::{self, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke};
use eframe::egui_wgpu::{self, wgpu};
use railbench_core::geometry::{
    is_point_in_any_polygon, rotate, sample_arc_between_connectors, sample_flex_spline_points,
    smoothstep,
};
use railbench_core::terrain::{carve_ground_terrain_grid, mountain_noise};
use railbench_core::types::{BaseboardConfig, Catalog, CatalogEntry, Layout, Model3D, TrackPiece, Vec2};

use crate::board_tools;
use crate::catalog::{find_entry, is_road_id};
use crate::i18n::Lang;
use crate::terrain_tools::{
    build_connected_corridor_chains, collect_terrain_support_polygons,
    compute_tunnel_runs_for_mountain, create_tunnel_profile, evaluate_mountain_local,
    mountain_local_to_world, CorridorChain, TunnelProfile, APPROACH_SLOPE, R_PORTAL,
};
use crate::ColorMode;

// ============================================================================
//  WGSL Shaders: 3D Lit Scene (with Depth Buffer + Fog + Day/Night) & Blit
// ============================================================================


mod shaders;
#[allow(unused_imports)]
use shaders::*;

pub(crate) const MAT_DEFAULT: [f32; 3] = [0.78, 0.04, 0.0];
const MAT_BALLAST: [f32; 3] = [0.96, 0.02, 0.0];
const MAT_TERRAIN: [f32; 3] = [0.92, 0.0, 0.0];
const MAT_WATER: [f32; 3] = [0.10, 0.14, 0.0];
pub(crate) const MAT_METAL_RAIL: [f32; 3] = [0.24, 0.84, 0.0];
pub(crate) const MAT_RAIL_WEB: [f32; 3] = [0.58, 0.62, 0.0];

#[inline]
fn pack_snorm8(v: f32) -> i8 {
    (v.clamp(-1.0, 1.0) * 127.0).round() as i8
}

#[inline]
fn pack_unorm8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex3D {
    pub pos: [f32; 3],
    pub normal: [i8; 4],
    pub color: [u8; 4],
    pub mat: [i8; 4],
}

impl Vertex3D {
    #[inline]
    pub fn new(pos: [f32; 3], normal: [f32; 3], color: [f32; 3], mat: [f32; 3]) -> Self {
        Self::new_alpha(pos, normal, color, 1.0, mat)
    }

    #[inline]
    pub fn new_alpha(pos: [f32; 3], normal: [f32; 3], color: [f32; 3], alpha: f32, mat: [f32; 3]) -> Self {
        Self {
            pos,
            normal: [
                pack_snorm8(normal[0]),
                pack_snorm8(normal[1]),
                pack_snorm8(normal[2]),
                0,
            ],
            color: [
                pack_unorm8(color[0]),
                pack_unorm8(color[1]),
                pack_unorm8(color[2]),
                pack_unorm8(alpha),
            ],
            mat: [
                pack_snorm8(mat[0]),
                pack_snorm8(mat[1]),
                pack_snorm8(mat[2] * 0.5),
                0,
            ],
        }
    }

    #[inline]
    pub fn normal_f32(&self) -> [f32; 3] {
        [
            (self.normal[0] as f32 / 127.0).clamp(-1.0, 1.0),
            (self.normal[1] as f32 / 127.0).clamp(-1.0, 1.0),
            (self.normal[2] as f32 / 127.0).clamp(-1.0, 1.0),
        ]
    }

    #[inline]
    pub fn color_f32(&self) -> [f32; 3] {
        [
            self.color[0] as f32 / 255.0,
            self.color[1] as f32 / 255.0,
            self.color[2] as f32 / 255.0,
        ]
    }

    #[inline]
    pub fn mat_f32(&self) -> [f32; 3] {
        [
            (self.mat[0] as f32 / 127.0).clamp(-1.0, 1.0),
            (self.mat[1] as f32 / 127.0).clamp(-1.0, 1.0),
            (self.mat[2] as f32 / 127.0).clamp(-1.0, 1.0) * 2.0,
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Instance3D {
    pub pos_rot: [f32; 4], // [tx, ty, tz, rot_rad]
    pub tint_w: [f32; 4],  // [r, g, b, tint_weight]
}

impl Instance3D {
    pub const IDENTITY: Self = Self {
        pos_rot: [0.0, 0.0, 0.0, 0.0],
        tint_w: [0.0, 0.0, 0.0, 0.0],
    };
}

#[derive(Clone, Default)]
pub struct InstancedTrackPrototype3D {
    pub catalog_id: String,
    pub vertices: Arc<Vec<Vertex3D>>,
    pub indices: Arc<Vec<u32>>,
    pub instances: Vec<Instance3D>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer3DEnv {
    HallLandscape = 0,
    Minimal = 1,
}

#[derive(Debug, Clone)]
pub struct Scene3DOptions {
    pub show_wood_plates: bool,
    pub env_mode: Viewer3DEnv,
    pub dark_mode: bool,
    pub color_mode: ColorMode,
    pub time_of_day: f32,
    pub short_circuit_points: Vec<[f64; 3]>,
    pub turnout_states: std::collections::HashMap<String, usize>,
    pub turntable_angles_deg: std::collections::HashMap<String, f64>,
    pub transfer_offsets_mm: std::collections::HashMap<String, f64>,
    pub active_uncouplers: std::collections::HashSet<String>,
}

impl Default for Scene3DOptions {
    fn default() -> Self {
        Self {
            show_wood_plates: true,
            env_mode: Viewer3DEnv::HallLandscape,
            dark_mode: true,
            color_mode: ColorMode::Off,
            time_of_day: 11.5,
            short_circuit_points: Vec::new(),
            turnout_states: std::collections::HashMap::new(),
            turntable_angles_deg: std::collections::HashMap::new(),
            transfer_offsets_mm: std::collections::HashMap::new(),
            active_uncouplers: std::collections::HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraViewPreset {
    Top,
    Bottom,
    Front,
    Back,
    Left,
    Right,
    IsoNw,
    IsoNe,
    IsoSw,
    IsoSe,
    Home,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TexturedVertex3D {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub mat: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct TexturedSubmesh3D {
    pub texture_id: u64,
    pub tex_width: u32,
    pub tex_height: u32,
    pub tex_rgba: Arc<Vec<u8>>,
    pub vertices: Vec<TexturedVertex3D>,
    pub indices: Vec<u32>,
}

pub(super) fn merge_textured_submesh(dest: &mut Vec<TexturedSubmesh3D>, src: TexturedSubmesh3D) {
    if src.vertices.is_empty() || src.indices.is_empty() || src.tex_width == 0 || src.tex_height == 0 {
        return;
    }
    if let Some(existing) = dest.iter_mut().find(|d| d.texture_id == src.texture_id) {
        let base = existing.vertices.len() as u32;
        existing.vertices.extend_from_slice(&src.vertices);
        existing.indices.extend(src.indices.into_iter().map(|i| base + i));
    } else {
        dest.push(src);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct Uniforms3D {
    view_proj: [[f32; 4]; 4],
    sun_view_proj: [[f32; 4]; 4],
    sun_dir: [f32; 4],
    cam_pos: [f32; 4],
    fog_color: [f32; 4],
    sun_color_int: [f32; 4],
    hemi_sky_int: [f32; 4],
    hemi_ground_pad: [f32; 4],
    cam_right: [f32; 4],
    cam_up: [f32; 4],
    cam_fwd: [f32; 4],
    sky_zenith: [f32; 4],
    shadow_params: [f32; 4],
}

#[derive(Clone, Copy)]
struct SkyKeyframe {
    hour: f32,
    sky: u32,
    fog: u32,
    sun: u32,
    sun_int: f32,
    hemi_sky: u32,
    hemi_ground: u32,
    hemi_int: f32,
}

const SKY_KEYFRAMES: &[SkyKeyframe] = &[
    SkyKeyframe { hour: 0.0,  sky: 0x030712, fog: 0x0b1528, sun: 0x60a5fa, sun_int: 0.35, hemi_sky: 0x0c192c, hemi_ground: 0x040812, hemi_int: 0.35 },
    SkyKeyframe { hour: 4.8,  sky: 0x0c1b33, fog: 0x1a2848, sun: 0x818cf8, sun_int: 0.42, hemi_sky: 0x1e293b, hemi_ground: 0x0f172a, hemi_int: 0.42 },
    SkyKeyframe { hour: 6.0,  sky: 0xea580c, fog: 0xfdba74, sun: 0xfb923c, sun_int: 0.90, hemi_sky: 0xfdba74, hemi_ground: 0x78350f, hemi_int: 0.65 },
    SkyKeyframe { hour: 8.0,  sky: 0x3b82f6, fog: 0xc2e6fc, sun: 0xfef08a, sun_int: 1.15, hemi_sky: 0xbde0fe, hemi_ground: 0x334155, hemi_int: 0.80 },
    SkyKeyframe { hour: 12.0, sky: 0x0284c7, fog: 0xc8e7f8, sun: 0xfffaed, sun_int: 1.35, hemi_sky: 0xe0f2fe, hemi_ground: 0x3b495c, hemi_int: 0.92 },
    SkyKeyframe { hour: 17.5, sky: 0xd97706, fog: 0xfde047, sun: 0xfb923c, sun_int: 1.25, hemi_sky: 0xfef08a, hemi_ground: 0x451a03, hemi_int: 0.82 },
    SkyKeyframe { hour: 19.2, sky: 0x9333ea, fog: 0xf472b6, sun: 0xf43f5e, sun_int: 0.78, hemi_sky: 0xa855f7, hemi_ground: 0x3b0764, hemi_int: 0.58 },
    SkyKeyframe { hour: 21.0, sky: 0x0a1122, fog: 0x151e38, sun: 0x93c5fd, sun_int: 0.42, hemi_sky: 0x1e293b, hemi_ground: 0x050d1a, hemi_int: 0.40 },
    SkyKeyframe { hour: 24.0, sky: 0x030712, fog: 0x0b1528, sun: 0x60a5fa, sun_int: 0.35, hemi_sky: 0x0c192c, hemi_ground: 0x040812, hemi_int: 0.35 },
];

fn hex_to_linear_rgb(hex: u32) -> [f32; 3] {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    [r.powf(2.2), g.powf(2.2), b.powf(2.2)]
}

fn hex_to_srgb_norm(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    ]
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

pub(super) fn camera_basis(eye: [f32; 3], target: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let fwd = normalize3([
        target[0] - eye[0],
        target[1] - eye[1],
        target[2] - eye[2],
    ]);
    let right_raw = cross3(fwd, [0.0, 1.0, 0.0]);
    let right = if dot3(right_raw, right_raw) < 1e-6 {
        [1.0, 0.0, 0.0]
    } else {
        normalize3(right_raw)
    };
    let up = normalize3(cross3(right, fwd));
    (right, up, fwd)
}

pub(super) struct SkyUniformState {
    pub(super) clear_color: [f64; 3],
    pub(super) sky_zenith: [f32; 4],
    pub(super) fog_color: [f32; 4],
    pub(super) sun_dir: [f32; 4],
    pub(super) sun_color_int: [f32; 4],
    pub(super) hemi_sky_int: [f32; 4],
    pub(super) hemi_ground_pad: [f32; 4],
}

fn eval_sky_state(time_of_day: f32, env: Viewer3DEnv, dark_mode: bool) -> SkyUniformState {
    let h = time_of_day.rem_euclid(24.0);
    let mut k0 = &SKY_KEYFRAMES[0];
    let mut k1 = &SKY_KEYFRAMES[SKY_KEYFRAMES.len() - 1];
    for w in SKY_KEYFRAMES.windows(2) {
        if h >= w[0].hour && h <= w[1].hour {
            k0 = &w[0];
            k1 = &w[1];
            break;
        }
    }
    let span = (k1.hour - k0.hour).max(0.001);
    let t = ((h - k0.hour) / span).clamp(0.0, 1.0);

    let sky_lin = lerp3(hex_to_linear_rgb(k0.sky), hex_to_linear_rgb(k1.sky), t);
    let fog_lin = lerp3(hex_to_linear_rgb(k0.fog), hex_to_linear_rgb(k1.fog), t);
    let sun_c = lerp3(hex_to_srgb_norm(k0.sun), hex_to_srgb_norm(k1.sun), t);
    let sun_int = k0.sun_int + (k1.sun_int - k0.sun_int) * t;
    let hemi_sky = lerp3(hex_to_srgb_norm(k0.hemi_sky), hex_to_srgb_norm(k1.hemi_sky), t);
    let hemi_gnd = lerp3(hex_to_srgb_norm(k0.hemi_ground), hex_to_srgb_norm(k1.hemi_ground), t);
    let hemi_int = k0.hemi_int + (k1.hemi_int - k0.hemi_int) * t;

    let sun_angle = ((h - 6.0) / 24.0) * std::f32::consts::TAU;
    let sun_elev = sun_angle.sin();
    let night_t = ((0.24 - sun_elev) / 0.36).clamp(0.0, 1.0);
    let night_factor = night_t * night_t * (3.0 - 2.0 * night_t);
    let sun_dir = if sun_elev > -0.05 {
        [
            -sun_angle.cos(),
            sun_elev.max(0.18),
            -0.35 * (sun_angle * 0.5).cos(),
            0.0,
        ]
    } else {
        let moon = sun_angle + std::f32::consts::PI;
        [
            -moon.cos(),
            moon.sin().max(0.22),
            -0.35 * (moon * 0.5).cos(),
            0.0,
        ]
    };

    if env == Viewer3DEnv::Minimal {
        let dusk_warmth =
            (1.0 - ((h - 6.0).abs().min((h - 18.5).abs()) / 2.2).clamp(0.0, 1.0)) * 0.18;
        let day_zenith = if dark_mode {
            [0.038_f32, 0.056, 0.092]
        } else {
            [0.25_f32, 0.37, 0.56]
        };
        let day_horizon = if dark_mode {
            [0.095_f32, 0.118, 0.156]
        } else {
            [0.66_f32, 0.74, 0.84]
        };
        let night_zenith = [0.014_f32, 0.022, 0.044];
        let night_horizon = [0.030_f32, 0.044, 0.072];
        let zenith_lin = lerp3(lerp3(day_zenith, night_zenith, night_factor), sky_lin, dusk_warmth);
        let horizon_lin = lerp3(
            lerp3(day_horizon, night_horizon, night_factor),
            fog_lin,
            dusk_warmth,
        );
        let fog_a = if dark_mode { 0.22 } else { 0.18 + 0.06 * night_factor };
        return SkyUniformState {
            clear_color: [
                horizon_lin[0] as f64,
                horizon_lin[1] as f64,
                horizon_lin[2] as f64,
            ],
            sky_zenith: [zenith_lin[0], zenith_lin[1], zenith_lin[2], night_factor],
            fog_color: [horizon_lin[0], horizon_lin[1], horizon_lin[2], fog_a],
            sun_dir,
            sun_color_int: [sun_c[0], sun_c[1], sun_c[2], sun_int.max(0.45)],
            hemi_sky_int: [hemi_sky[0], hemi_sky[1], hemi_sky[2], hemi_int.max(0.44)],
            hemi_ground_pad: [hemi_gnd[0], hemi_gnd[1], hemi_gnd[2], night_factor],
        };
    }

    SkyUniformState {
        clear_color: [fog_lin[0] as f64, fog_lin[1] as f64, fog_lin[2] as f64],
        sky_zenith: [sky_lin[0], sky_lin[1], sky_lin[2], night_factor],
        fog_color: [fog_lin[0], fog_lin[1], fog_lin[2], 1.0],
        sun_dir,
        sun_color_int: [sun_c[0], sun_c[1], sun_c[2], sun_int],
        hemi_sky_int: [hemi_sky[0], hemi_sky[1], hemi_sky[2], hemi_int],
        hemi_ground_pad: [hemi_gnd[0], hemi_gnd[1], hemi_gnd[2], night_factor],
    }
}

#[derive(Debug, Clone)]
pub struct Camera3D {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            target: [0.0, 20.0, 0.0],
            yaw: 0.62,
            pitch: 0.52,
            distance: 2200.0,
        }
    }
}

impl Camera3D {
    pub fn eye(&self) -> [f32; 3] {
        let cp = self.pitch.cos();
        [
            self.target[0] + self.distance * cp * self.yaw.sin(),
            self.target[1] + self.distance * self.pitch.sin(),
            self.target[2] + self.distance * cp * self.yaw.cos(),
        ]
    }

    pub fn fit_layout(&mut self, layout: &Layout) {
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for p in &layout.pieces {
            for c in &p.connectors {
                min_x = min_x.min(c.x as f32);
                max_x = max_x.max(c.x as f32);
                min_z = min_z.min(c.y as f32);
                max_z = max_z.max(c.y as f32);
            }
        }
        for b in board_tools::get_baseboards_slice(layout) {
            if !b.enabled {
                continue;
            }
            for pt in board_tools::get_baseboard_polygon(&b) {
                min_x = min_x.min(pt.x as f32);
                max_x = max_x.max(pt.x as f32);
                min_z = min_z.min(pt.y as f32);
                max_z = max_z.max(pt.y as f32);
            }
        }

        if !min_x.is_finite() {
            *self = Self::default();
            return;
        }
        self.target = [(min_x + max_x) * 0.5, 25.0, (min_z + max_z) * 0.5];
        let span = (max_x - min_x).max(max_z - min_z).max(600.0);
        self.distance = (span * 1.15).clamp(400.0, 18000.0);
    }

    pub fn focus_point(&mut self, wx: f64, wy: f64) {
        self.target = [wx as f32, 20.0, wy as f32];
        self.distance = self.distance.clamp(260.0, 850.0);
    }

    pub fn apply_preset(&mut self, preset: CameraViewPreset, layout: &Layout) {
        match preset {
            CameraViewPreset::Home => {
                self.fit_layout(layout);
                self.yaw = 0.62;
                self.pitch = 0.52;
            }
            CameraViewPreset::Top => self.pitch = 1.48,
            CameraViewPreset::Bottom => self.pitch = -1.48,
            CameraViewPreset::Front => {
                self.yaw = 0.0;
                self.pitch = 0.08;
            }
            CameraViewPreset::Back => {
                self.yaw = std::f32::consts::PI;
                self.pitch = 0.08;
            }
            CameraViewPreset::Right => {
                self.yaw = std::f32::consts::FRAC_PI_2;
                self.pitch = 0.08;
            }
            CameraViewPreset::Left => {
                self.yaw = -std::f32::consts::FRAC_PI_2;
                self.pitch = 0.08;
            }
            CameraViewPreset::IsoSe => {
                self.yaw = 0.72;
                self.pitch = 0.52;
            }
            CameraViewPreset::IsoSw => {
                self.yaw = -0.72;
                self.pitch = 0.52;
            }
            CameraViewPreset::IsoNe => {
                self.yaw = std::f32::consts::PI - 0.72;
                self.pitch = 0.52;
            }
            CameraViewPreset::IsoNw => {
                self.yaw = -std::f32::consts::PI + 0.72;
                self.pitch = 0.52;
            }
        }
    }

    pub(crate) fn view_proj(&self, aspect: f32) -> [[f32; 4]; 4] {
        let eye = self.eye();
        let view = mat4_look_at(eye, self.target, [0.0, 1.0, 0.0]);
        let proj = mat4_perspective(45.0_f32.to_radians(), aspect.max(0.1), 8.0, 400000.0);
        mat4_mul(proj, view)
    }

    /// Casts a normalized world-space ray `(origin, dir)` from screen pixel `pos` inside `rect`.
    pub fn screen_to_ray(&self, rect: Rect, pos: Pos2) -> ([f32; 3], [f32; 3]) {
        let eye = self.eye();
        let f = normalize3([
            self.target[0] - eye[0],
            self.target[1] - eye[1],
            self.target[2] - eye[2],
        ]);
        let s = normalize3(cross3(f, [0.0, 1.0, 0.0]));
        let u = cross3(s, f);
        let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
        let tan_half_y = (45.0_f32.to_radians() * 0.5).tan();
        let ndc_x = ((pos.x - rect.min.x) / rect.width().max(1.0)) * 2.0 - 1.0;
        let ndc_y = 1.0 - ((pos.y - rect.min.y) / rect.height().max(1.0)) * 2.0;
        let dx = f[0] + s[0] * (ndc_x * tan_half_y * aspect) + u[0] * (ndc_y * tan_half_y);
        let dy = f[1] + s[1] * (ndc_x * tan_half_y * aspect) + u[1] * (ndc_y * tan_half_y);
        let dz = f[2] + s[2] * (ndc_x * tan_half_y * aspect) + u[2] * (ndc_y * tan_half_y);
        (eye, normalize3([dx, dy, dz]))
    }

    /// Projects a 3D world point `[x, y, z]` into egui screen coordinates inside `rect`.
    pub fn world_to_screen(&self, rect: Rect, world: [f32; 3]) -> Option<Pos2> {
        let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
        let vp = self.view_proj(aspect);
        let cx = vp[0][0] * world[0] + vp[1][0] * world[1] + vp[2][0] * world[2] + vp[3][0];
        let cy = vp[0][1] * world[0] + vp[1][1] * world[1] + vp[2][1] * world[2] + vp[3][1];
        let cw = vp[0][3] * world[0] + vp[1][3] * world[1] + vp[2][3] * world[2] + vp[3][3];
        if cw <= 0.1 {
            return None;
        }
        let ndc_x = cx / cw;
        let ndc_y = cy / cw;
        Some(Pos2::new(
            rect.min.x + (ndc_x * 0.5 + 0.5) * rect.width(),
            rect.min.y + (0.5 - ndc_y * 0.5) * rect.height(),
        ))
    }
}

#[derive(Clone, Default)]
pub struct SceneMesh3D {
    pub(crate) vertices: Vec<Vertex3D>,
    indices: Vec<u32>,
    pub(crate) glass_indices: Vec<u32>,
    pub instanced_tracks: Vec<InstancedTrackPrototype3D>,
    pub textured_meshes: Vec<TexturedSubmesh3D>,
}

impl SceneMesh3D {
    pub fn from_raw(vertices: Vec<Vertex3D>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            indices,
            glass_indices: Vec::new(),
            instanced_tracks: Vec::new(),
            textured_meshes: Vec::new(),
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
            + self
                .instanced_tracks
                .iter()
                .map(|p| p.vertices.len())
                .sum::<usize>()
            + self
                .textured_meshes
                .iter()
                .map(|m| m.vertices.len())
                .sum::<usize>()
    }

    pub fn triangle_count(&self) -> usize {
        (self.indices.len() + self.glass_indices.len()) / 3
            + self
                .instanced_tracks
                .iter()
                .map(|p| (p.indices.len() / 3) * p.instances.len())
                .sum::<usize>()
            + self
                .textured_meshes
                .iter()
                .map(|m| m.indices.len() / 3)
                .sum::<usize>()
    }

    pub fn track_prototype_count(&self) -> usize {
        self.instanced_tracks.len()
    }

    pub fn track_instance_count(&self) -> usize {
        self.instanced_tracks.iter().map(|p| p.instances.len()).sum()
    }

    pub fn gpu_buffer_bytes(&self) -> usize {
        let base_bytes = self.vertices.len() * std::mem::size_of::<Vertex3D>()
            + (self.indices.len() + self.glass_indices.len()) * std::mem::size_of::<u32>();
        let proto_bytes: usize = self
            .instanced_tracks
            .iter()
            .map(|p| {
                p.vertices.len() * std::mem::size_of::<Vertex3D>()
                    + p.indices.len() * std::mem::size_of::<u32>()
                    + p.instances.len() * std::mem::size_of::<Instance3D>()
            })
            .sum();
        let tex_bytes: usize = self
            .textured_meshes
            .iter()
            .map(|m| {
                m.vertices.len() * std::mem::size_of::<TexturedVertex3D>()
                    + m.indices.len() * std::mem::size_of::<u32>()
            })
            .sum();
        base_bytes + proto_bytes + tex_bytes
    }

    pub fn append_from(&mut self, other: &SceneMesh3D) {
        if !other.vertices.is_empty() {
            let base = self.vertices.len() as u32;
            self.vertices.extend_from_slice(&other.vertices);
            self.indices.reserve(other.indices.len());
            for &idx in &other.indices {
                self.indices.push(base + idx);
            }
            if !other.glass_indices.is_empty() {
                self.glass_indices.reserve(other.glass_indices.len());
                for &idx in &other.glass_indices {
                    self.glass_indices.push(base + idx);
                }
            }
        }
        if !other.instanced_tracks.is_empty() {
            self.instanced_tracks.extend_from_slice(&other.instanced_tracks);
        }
        for tm in &other.textured_meshes {
            merge_textured_submesh(&mut self.textured_meshes, tm.clone());
        }
    }
}

#[derive(Default)]
pub struct Scene3DCache {
    pub env: Option<(u64, Arc<SceneMesh3D>)>,
    pub boards: Option<(u64, Arc<SceneMesh3D>)>,
    pub terrain: Option<(u64, Arc<SceneMesh3D>)>,
    pub tracks_and_models: Option<(u64, Arc<SceneMesh3D>)>,
    pub track_prototypes: std::collections::HashMap<String, (Arc<Vec<Vertex3D>>, Arc<Vec<u32>>)>,
    pub env_rebuilds: u64,
    pub boards_rebuilds: u64,
    pub terrain_rebuilds: u64,
    pub tracks_rebuilds: u64,
    pub last_build_threads: usize,
}

impl Scene3DCache {
    pub fn clear(&mut self) {
        self.env = None;
        self.boards = None;
        self.terrain = None;
        self.tracks_and_models = None;
        self.track_prototypes.clear();
    }
}

#[derive(Clone)]
pub struct Scene3DBundle {
    pub env: Option<(u64, Arc<SceneMesh3D>)>,
    pub boards: Option<(u64, Arc<SceneMesh3D>)>,
    pub terrain: Option<(u64, Arc<SceneMesh3D>)>,
    pub tracks: Option<(u64, Arc<SceneMesh3D>)>,
    pub shadow_center: [f32; 3],
    pub shadow_half_extent: f32,
}

impl Default for Scene3DBundle {
    fn default() -> Self {
        Self {
            env: None,
            boards: None,
            terrain: None,
            tracks: None,
            shadow_center: [0.0, 40.0, 0.0],
            shadow_half_extent: 1600.0,
        }
    }
}

impl Scene3DBundle {
    fn iter_meshes(&self) -> impl Iterator<Item = &SceneMesh3D> {
        [&self.env, &self.boards, &self.terrain, &self.tracks]
            .into_iter()
            .filter_map(|opt| opt.as_ref().map(|(_, m)| m.as_ref()))
    }

    pub fn vertex_count(&self) -> usize {
        self.iter_meshes().map(SceneMesh3D::vertex_count).sum()
    }

    pub fn triangle_count(&self) -> usize {
        self.iter_meshes().map(SceneMesh3D::triangle_count).sum()
    }

    pub fn track_prototype_count(&self) -> usize {
        self.iter_meshes().map(SceneMesh3D::track_prototype_count).sum()
    }

    pub fn track_instance_count(&self) -> usize {
        self.iter_meshes().map(SceneMesh3D::track_instance_count).sum()
    }

    pub fn gpu_buffer_bytes(&self) -> usize {
        self.iter_meshes().map(SceneMesh3D::gpu_buffer_bytes).sum()
    }
}

pub(super) const MSAA_SAMPLE_COUNT: u32 = 4;
pub const WINDOW_MSAA_SAMPLES: u32 = 4;
pub const SHADOW_MAP_SIZE: u32 = 2048;

mod gpu;
#[allow(unused_imports)]
use gpu::*;
#[allow(unused_imports)]
pub use gpu::{init_wgpu_3d_resources, release_wgpu_3d_resources};
mod viewport;
#[allow(unused_imports)]
use viewport::*;
#[allow(unused_imports)]
pub use viewport::draw_viewport_3d;
mod cab_viewport;
#[allow(unused_imports)]
use cab_viewport::*;
#[allow(unused_imports)]
pub use cab_viewport::{
    draw_cab_viewport_3d, speedometer_needle_angle, CabCameraMode, CabHudInfo,
    CabHudInteraction,
};
mod viewcube;
pub mod edit3d;
#[allow(unused_imports)]
pub use edit3d::*;
mod environment;
#[allow(unused_imports)]
use environment::*;
mod scene_cache;
#[allow(unused_imports)]
use scene_cache::*;
#[allow(unused_imports)]
pub use scene_cache::{build_scene_mesh_3d, build_scene_mesh_3d_cached};
mod scene_builder;
#[allow(unused_imports)]
use scene_builder::*;
#[allow(unused_imports)]
pub use scene_builder::{update_scene_3d_cache};
mod tracks;
#[allow(unused_imports)]
use tracks::*;
#[allow(unused_imports)]
pub(crate) use tracks::{sample_track_branches_3d};
mod mountain_mesh;
#[allow(unused_imports)]
use mountain_mesh::*;
mod ground_and_models;
#[allow(unused_imports)]
use ground_and_models::*;
pub(crate) use ground_and_models::rot_xz;
mod heightfield_mesh;
#[allow(unused_imports)]
use heightfield_mesh::*;
mod primitives;
#[allow(unused_imports)]
use primitives::*;
#[allow(unused_imports)]
pub(crate) use primitives::{polyline_miter_normal, push_box_polyline_mat_3d, push_quad_mat, push_tri, dot3, cross3, normalize3};
#[cfg(test)]
mod tests;
