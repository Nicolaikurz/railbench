use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use eframe::egui::{self, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke};
use eframe::egui_wgpu::{self, wgpu};
use railbench_core::geometry::{
    build_tunnel_sweep_positions, is_point_in_any_polygon, rotate, sample_arc_between_connectors,
    sample_flex_spline_points, smoothstep,
};
use railbench_core::terrain::{carve_ground_terrain_grid, mountain_noise};
use railbench_core::types::{BaseboardConfig, Catalog, CatalogEntry, Layout, Model3D, TrackPiece, Vec2};

use crate::board_tools;
use crate::catalog::{find_entry, is_road_id};
use crate::i18n::Lang;
use crate::terrain_tools::{
    build_connected_corridor_chains, collect_terrain_support_polygons,
    compute_tunnel_runs_for_mountain, create_tunnel_profile, evaluate_mountain_local,
    is_point_inside_any_mountain, mountain_local_to_world, tunnel_profile_points, CorridorChain,
    TunnelProfile, APPROACH_SLOPE, R_PORTAL,
};

// ============================================================================
//  WGSL Shaders: 3D Lit Scene (with Depth Buffer + Fog + Day/Night) & Blit
// ============================================================================

const SCENE_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_dir: vec4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    sun_color_int: vec4<f32>,
    hemi_sky_int: vec4<f32>,
    hemi_ground_pad: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) mat: vec3<f32>,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) mat: vec3<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip_pos = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.world_pos = in.pos;
    out.normal = in.normal;
    out.color = in.color;
    out.mat = in.mat;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let l = normalize(u.sun_dir.xyz);
    let v_vec = u.cam_pos.xyz - in.world_pos;
    let dist = length(v_vec);
    let v = v_vec / max(dist, 0.001);
    let h = normalize(l + v);
    let night = clamp(u.hemi_ground_pad.w, 0.0, 1.0);

    var base_col = in.color;
    var rough = clamp(in.mat.x, 0.04, 1.0);
    let metal = clamp(in.mat.y, 0.0, 1.0);
    var emissive_col = vec3<f32>(0.0);

    if (in.mat.z < -0.001) {
        let glow_t = smoothstep(0.08, 0.82, night);
        let glass_day = vec3<f32>(0.10, 0.19, 0.30);
        base_col = mix(glass_day, in.color, glow_t * 0.92);
        emissive_col = in.color * (-in.mat.z) * (0.03 + 1.15 * glow_t);
        rough = 0.12;
    } else if (in.mat.z > 0.001) {
        emissive_col = in.color * in.mat.z * (0.55 + 0.90 * night);
    } else if (rough > 0.52) {
        let detail_fade = 1.0 - smoothstep(450.0, 1800.0, dist);
        let p = in.world_pos * 0.24;
        let grain = sin(p.x * 3.1 + sin(p.z * 2.7)) * cos(p.y * 4.3 + p.z * 1.9);
        let brick = select(0.0, sin(in.world_pos.y * 1.25) * 0.022, abs(n.y) < 0.35 && rough > 0.72);
        base_col = clamp(base_col * (1.0 + (grain * 0.036 + brick) * detail_fade), vec3<f32>(0.0), vec3<f32>(1.0));
    }

    let ndotl = max(dot(n, l), 0.0);
    let diff = ndotl * u.sun_color_int.w;
    let fill_l = normalize(vec3<f32>(-u.sun_dir.x, 0.45, -u.sun_dir.z));
    let fill_diff = max(dot(n, fill_l), 0.0) * (0.22 * u.hemi_sky_int.w);

    let hemi = n.y * 0.5 + 0.5;
    let crevice_ao = mix(0.76, 1.0, clamp(n.y * 0.5 + 0.65, 0.0, 1.0));
    let ambient = mix(u.hemi_ground_pad.rgb, u.hemi_sky_int.rgb, hemi) * (0.64 * u.hemi_sky_int.w * crevice_ao);

    let f0 = mix(vec3<f32>(0.04), base_col, metal);
    let ndoth = max(dot(n, h), 0.0);
    let ndotv = max(dot(n, v), 0.0);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - max(dot(h, v), 0.0), 0.0, 1.0), 5.0);
    let shininess = mix(160.0, 10.0, rough * rough);
    let spec_norm = mix(0.45, 0.08, rough);
    let direct_spec = fresnel * pow(ndoth, shininess) * spec_norm * ndotl * u.sun_color_int.w;

    let r_dir = reflect(-v, n);
    let r_hemi = clamp(r_dir.y * 0.5 + 0.5, 0.0, 1.0);
    let env_col = mix(u.hemi_ground_pad.rgb, u.hemi_sky_int.rgb * 1.15, r_hemi);
    let view_fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - ndotv, 0.0, 1.0), 4.0);
    let env_spec = view_fresnel * env_col * (1.0 - rough * 0.72) * 0.55 * max(u.hemi_sky_int.w, 0.35);

    let diffuse_col = base_col * (1.0 - metal * 0.55);
    var lit = diffuse_col * (ambient + u.sun_color_int.rgb * diff + u.hemi_sky_int.rgb * fill_diff)
            + u.sun_color_int.rgb * direct_spec
            + env_spec
            + emissive_col;
    let excess = max(lit - vec3<f32>(0.88), vec3<f32>(0.0));
    lit = min(lit, vec3<f32>(0.88)) + excess / (vec3<f32>(1.0) + excess * 1.6);

    let fog_factor = clamp((dist - 5500.0) / 18000.0, 0.0, u.fog_color.w);
    let final_col = mix(lit, u.fog_color.rgb, fog_factor);
    return vec4<f32>(final_col, 1.0);
}
"#;

const BLIT_SHADER_WGSL: &str = r#"
@group(0) @binding(0)
var t_color: texture_2d<f32>;
@group(0) @binding(1)
var s_color: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) idx: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    let p = positions[idx];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}

@fragment
fn fs_blit(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(t_color, s_color, in.uv);
}
"#;

pub(crate) const MAT_DEFAULT: [f32; 3] = [0.78, 0.04, 0.0];
const MAT_TERRAIN: [f32; 3] = [0.92, 0.0, 0.0];
const MAT_WATER: [f32; 3] = [0.10, 0.14, 0.0];
const MAT_METAL_RAIL: [f32; 3] = [0.24, 0.84, 0.0];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct Vertex3D {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub mat: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer3DEnv {
    Hall = 0,
    Landscape = 1,
    Minimal = 2,
}

#[derive(Debug, Clone)]
pub struct Scene3DOptions {
    pub show_wood_plates: bool,
    pub env_mode: Viewer3DEnv,
    pub dark_mode: bool,
    pub short_circuit_points: Vec<Vec2>,
}

impl Default for Scene3DOptions {
    fn default() -> Self {
        Self {
            show_wood_plates: true,
            env_mode: Viewer3DEnv::Hall,
            dark_mode: true,
            short_circuit_points: Vec::new(),
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
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms3D {
    view_proj: [[f32; 4]; 4],
    sun_dir: [f32; 4],
    cam_pos: [f32; 4],
    fog_color: [f32; 4],
    sun_color_int: [f32; 4],
    hemi_sky_int: [f32; 4],
    hemi_ground_pad: [f32; 4],
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
    SkyKeyframe { hour: 0.0,  sky: 0x030712, fog: 0x081120, sun: 0x60a5fa, sun_int: 0.35, hemi_sky: 0x0c192c, hemi_ground: 0x040812, hemi_int: 0.35 },
    SkyKeyframe { hour: 4.8,  sky: 0x0c1b33, fog: 0x14203d, sun: 0x818cf8, sun_int: 0.42, hemi_sky: 0x1e293b, hemi_ground: 0x0f172a, hemi_int: 0.42 },
    SkyKeyframe { hour: 6.0,  sky: 0xf97316, fog: 0xfdba74, sun: 0xfb923c, sun_int: 0.90, hemi_sky: 0xfdba74, hemi_ground: 0x78350f, hemi_int: 0.65 },
    SkyKeyframe { hour: 8.0,  sky: 0x60a5fa, fog: 0xbae6fd, sun: 0xfef08a, sun_int: 1.15, hemi_sky: 0xbde0fe, hemi_ground: 0x334155, hemi_int: 0.80 },
    SkyKeyframe { hour: 12.0, sky: 0x38bdf8, fog: 0xb5daf0, sun: 0xfffaed, sun_int: 1.35, hemi_sky: 0xe0f2fe, hemi_ground: 0x3b495c, hemi_int: 0.92 },
    SkyKeyframe { hour: 17.5, sky: 0xf59e0b, fog: 0xfde047, sun: 0xfb923c, sun_int: 1.25, hemi_sky: 0xfef08a, hemi_ground: 0x451a03, hemi_int: 0.82 },
    SkyKeyframe { hour: 19.2, sky: 0xd946ef, fog: 0xf472b6, sun: 0xf43f5e, sun_int: 0.78, hemi_sky: 0xa855f7, hemi_ground: 0x3b0764, hemi_int: 0.58 },
    SkyKeyframe { hour: 21.0, sky: 0x0f172a, fog: 0x1e1b4b, sun: 0x93c5fd, sun_int: 0.42, hemi_sky: 0x1e293b, hemi_ground: 0x050d1a, hemi_int: 0.40 },
    SkyKeyframe { hour: 24.0, sky: 0x030712, fog: 0x081120, sun: 0x60a5fa, sun_int: 0.35, hemi_sky: 0x0c192c, hemi_ground: 0x040812, hemi_int: 0.35 },
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

struct SkyUniformState {
    clear_color: [f64; 3],
    fog_color: [f32; 4],
    sun_dir: [f32; 4],
    sun_color_int: [f32; 4],
    hemi_sky_int: [f32; 4],
    hemi_ground_pad: [f32; 4],
}

fn eval_sky_state(time_of_day: f32, env: Viewer3DEnv, dark_mode: bool) -> SkyUniformState {
    if env == Viewer3DEnv::Minimal {
        let (clear, fog, sun_int, hemi_int, hemi_gnd) = if dark_mode {
            ([0.014, 0.022, 0.052], [0.06, 0.09, 0.16, 0.22], 0.90, 0.72, [0.16, 0.20, 0.28])
        } else {
            ([0.76, 0.73, 0.67], [0.85, 0.83, 0.79, 0.16], 1.15, 0.86, [0.50, 0.48, 0.44])
        };
        return SkyUniformState {
            clear_color: clear,
            fog_color: fog,
            sun_dir: [0.55, 0.85, 0.45, 0.0],
            sun_color_int: [0.95, 0.93, 0.88, sun_int],
            hemi_sky_int: [0.85, 0.90, 0.98, hemi_int],
            hemi_ground_pad: [hemi_gnd[0], hemi_gnd[1], hemi_gnd[2], if dark_mode { 0.25 } else { 0.0 }],
        };
    }

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
    let fog_srgb = lerp3(hex_to_srgb_norm(k0.fog), hex_to_srgb_norm(k1.fog), t);
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

    SkyUniformState {
        clear_color: [sky_lin[0] as f64, sky_lin[1] as f64, sky_lin[2] as f64],
        fog_color: [fog_srgb[0], fog_srgb[1], fog_srgb[2], 0.55],
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

    fn view_proj(&self, aspect: f32) -> [[f32; 4]; 4] {
        let eye = self.eye();
        let view = mat4_look_at(eye, self.target, [0.0, 1.0, 0.0]);
        let proj = mat4_perspective(45.0_f32.to_radians(), aspect.max(0.1), 8.0, 100000.0);
        mat4_mul(proj, view)
    }
}

pub struct SceneMesh3D {
    vertices: Vec<Vertex3D>,
    indices: Vec<u32>,
}

impl SceneMesh3D {
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn gpu_buffer_bytes(&self) -> usize {
        self.vertices.len() * std::mem::size_of::<Vertex3D>()
            + self.indices.len() * std::mem::size_of::<u32>()
    }
}

struct OffscreenTarget {
    size: [u32; 2],
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    blit_bg: wgpu::BindGroup,
}

struct Gpu3DState {
    scene_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    blit_bgl: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    scene_bg: wgpu::BindGroup,
    vertex_buf: Option<wgpu::Buffer>,
    index_buf: Option<wgpu::Buffer>,
    index_count: u32,
    uploaded_version: u64,
    offscreen: Option<OffscreenTarget>,
}

impl Gpu3DState {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let scene_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("railbench_3d_scene_shader"),
            source: wgpu::ShaderSource::Wgsl(SCENE_SHADER_WGSL.into()),
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("railbench_3d_blit_shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER_WGSL.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("railbench_3d_uniforms"),
            size: std::mem::size_of::<Uniforms3D>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("railbench_3d_scene_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let scene_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("railbench_3d_scene_bg"),
            layout: &scene_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let scene_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("railbench_3d_scene_pipe_layout"),
            bind_group_layouts: &[&scene_bgl],
            push_constant_ranges: &[],
        });

        // Offscreen 3D pipeline with REAL Depth32Float hardware depth buffer!
        let scene_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("railbench_3d_scene_pipeline"),
            layout: Some(&scene_pipe_layout),
            vertex: wgpu::VertexState {
                module: &scene_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex3D>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                        wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                        wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
                        wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32x3 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &scene_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Fullscreen blit pipeline to copy offscreen 3D render into egui's render pass
        let blit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("railbench_3d_blit_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("railbench_3d_blit_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let blit_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("railbench_3d_blit_pipe_layout"),
            bind_group_layouts: &[&blit_bgl],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("railbench_3d_blit_pipeline"),
            layout: Some(&blit_pipe_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_blit"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_blit"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            scene_pipeline,
            blit_pipeline,
            blit_bgl,
            blit_sampler,
            uniform_buf,
            scene_bg,
            vertex_buf: None,
            index_buf: None,
            index_count: 0,
            uploaded_version: 0,
            offscreen: None,
        }
    }

    fn ensure_offscreen(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let w = width.clamp(16, 4096);
        let h = height.clamp(16, 4096);
        if let Some(ref cur) = self.offscreen {
            if cur.size == [w, h] {
                return;
            }
        }
        let color_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("railbench_3d_color_tex"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("railbench_3d_depth_tex"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let blit_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("railbench_3d_blit_bg"),
            layout: &self.blit_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
            ],
        });

        self.offscreen = Some(OffscreenTarget {
            size: [w, h],
            color_view,
            depth_view,
            blit_bg,
        });
    }
}

struct Viewport3DCallback {
    uniforms: Uniforms3D,
    mesh: Option<Arc<SceneMesh3D>>,
    mesh_version: u64,
    viewport_px: [u32; 2],
    clear_color: [f64; 3],
}

impl egui_wgpu::CallbackTrait for Viewport3DCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let state = callback_resources
            .entry::<Gpu3DState>()
            .or_insert_with(|| Gpu3DState::new(device, wgpu::TextureFormat::Bgra8Unorm));

        state.ensure_offscreen(device, self.viewport_px[0], self.viewport_px[1]);
        queue.write_buffer(&state.uniform_buf, 0, bytemuck::bytes_of(&self.uniforms));

        if state.uploaded_version != self.mesh_version {
            if let Some(mesh) = &self.mesh {
                if !mesh.vertices.is_empty() && !mesh.indices.is_empty() {
                    let vb = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("railbench_3d_vb"),
                        size: (mesh.vertices.len() * std::mem::size_of::<Vertex3D>()) as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&vb, 0, bytemuck::cast_slice(&mesh.vertices));

                    let ib = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("railbench_3d_ib"),
                        size: (mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&ib, 0, bytemuck::cast_slice(&mesh.indices));

                    state.vertex_buf = Some(vb);
                    state.index_buf = Some(ib);
                    state.index_count = mesh.indices.len() as u32;
                } else {
                    state.index_count = 0;
                }
                state.uploaded_version = self.mesh_version;
            }
        }

        // Render the 3D scene into our offscreen Color + Depth32Float buffer
        if let Some(ref target) = state.offscreen {
            let mut rpass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("railbench_3d_offscreen_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color[0],
                            g: self.clear_color[1],
                            b: self.clear_color[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if state.index_count > 0 {
                if let (Some(vb), Some(ib)) = (&state.vertex_buf, &state.index_buf) {
                    rpass.set_pipeline(&state.scene_pipeline);
                    rpass.set_bind_group(0, &state.scene_bg, &[]);
                    rpass.set_vertex_buffer(0, vb.slice(..));
                    rpass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    rpass.draw_indexed(0..state.index_count, 0, 0..1);
                }
            }
        }

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(state) = callback_resources.get::<Gpu3DState>() {
            if let Some(ref target) = state.offscreen {
                render_pass.set_pipeline(&state.blit_pipeline);
                render_pass.set_bind_group(0, &target.blit_bg, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }
    }
}

pub fn init_wgpu_3d_resources(cc: &eframe::CreationContext<'_>) {
    if let Some(render_state) = &cc.wgpu_render_state {
        let mut res = render_state.renderer.write();
        let state = Gpu3DState::new(&render_state.device, render_state.target_format);
        res.callback_resources.insert(state);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_viewport_3d(
    ui: &mut egui::Ui,
    camera: &mut Camera3D,
    cached_mesh: &Option<Arc<SceneMesh3D>>,
    mesh_version: u64,
    layout: &Layout,
    env_mode: &mut Viewer3DEnv,
    day_night_active: &mut bool,
    time_of_day: &mut f32,
    day_night_speed: &mut f32,
    show_wood_plates: &mut bool,
    first_short_circuit: Option<Vec2>,
    dark_mode: bool,
    lang: Lang,
    dt_secs: f32,
) {
    if *day_night_active && *env_mode != Viewer3DEnv::Minimal {
        let delta_hours = dt_secs * (24.0 / 1200.0) * (*day_night_speed);
        *time_of_day = (*time_of_day + delta_hours).rem_euclid(24.0);
    }

    let avail = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

    // Full 360° / 180° Orbit (left drag, including underneath table)
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        camera.yaw -= d.x * 0.0065;
        camera.pitch = (camera.pitch + d.y * 0.0065).clamp(-1.48, 1.48);
    }
    // Pan (right or middle drag)
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let d = response.drag_delta();
        let speed = camera.distance * 0.0012;
        let rx = camera.yaw.cos();
        let rz = -camera.yaw.sin();
        let fx = -camera.yaw.sin();
        let fz = -camera.yaw.cos();
        camera.target[0] -= (rx * d.x - fx * d.y) * speed;
        camera.target[2] -= (rz * d.x - fz * d.y) * speed;
    }
    // Zoom (scroll)
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let factor = (1.0 - scroll * 0.0018).clamp(0.75, 1.35);
            camera.distance = (camera.distance * factor).clamp(60.0, 25000.0);
        }
    }

    let ppp = ui.ctx().pixels_per_point();
    let px_w = ((rect.width() * ppp).round() as u32).max(16);
    let px_h = ((rect.height() * ppp).round() as u32).max(16);
    let aspect = rect.width() / rect.height().max(1.0);
    let eye = camera.eye();
    let sky = eval_sky_state(*time_of_day, *env_mode, dark_mode);

    let uniforms = Uniforms3D {
        view_proj: camera.view_proj(aspect),
        sun_dir: sky.sun_dir,
        cam_pos: [eye[0], eye[1], eye[2], 1.0],
        fog_color: sky.fog_color,
        sun_color_int: sky.sun_color_int,
        hemi_sky_int: sky.hemi_sky_int,
        hemi_ground_pad: sky.hemi_ground_pad,
    };

    let cb = egui_wgpu::Callback::new_paint_callback(
        rect,
        Viewport3DCallback {
            uniforms,
            mesh: cached_mesh.clone(),
            mesh_version,
            viewport_px: [px_w, px_h],
            clear_color: sky.clear_color,
        },
    );
    ui.painter().add(cb);

    let is_de = lang == Lang::De;

    // Top-left 3D Controls Bar (Environment, Wood Plates toggle, Day/Night Cycle, Short Focus)
    egui::Area::new(egui::Id::new("hud_3d_top_bar"))
        .fixed_pos(rect.min + egui::vec2(14.0, 12.0))
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 215))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(10, 6))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 70)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("3D").strong().size(11.5).color(Color32::from_rgb(56, 189, 248)));
                        let env_lbl = match *env_mode {
                            Viewer3DEnv::Hall => if is_de { "Halle" } else { "Hall" },
                            Viewer3DEnv::Landscape => if is_de { "Landschaft" } else { "Landscape" },
                            Viewer3DEnv::Minimal => "CAD / Minimal",
                        };
                        if ui
                            .small_button(env_lbl)
                            .on_hover_text("Cycle 3D Environment (Hall / Landscape / Minimal)")
                            .clicked()
                        {
                            *env_mode = match *env_mode {
                                Viewer3DEnv::Hall => Viewer3DEnv::Landscape,
                                Viewer3DEnv::Landscape => Viewer3DEnv::Minimal,
                                Viewer3DEnv::Minimal => Viewer3DEnv::Hall,
                            };
                        }

                        let wood_lbl = if is_de { "Holzplatten" } else { "Wood Plates" };
                        if ui.selectable_label(*show_wood_plates, wood_lbl).clicked() {
                            *show_wood_plates = !*show_wood_plates;
                        }

                        if *env_mode != Viewer3DEnv::Minimal {
                            ui.separator();
                            let h = (*time_of_day).floor() as u32 % 24;
                            let m = (((*time_of_day) - (*time_of_day).floor()) * 60.0).floor() as u32 % 60;
                            let play_sym = if *day_night_active { "||" } else { ">" };
                            if ui.selectable_label(*day_night_active, format!("{play_sym} {h:02}:{m:02}")).clicked() {
                                *day_night_active = !*day_night_active;
                            }
                            ui.add_sized(
                                [88.0, 16.0],
                                egui::Slider::new(time_of_day, 0.0..=24.0).show_value(false),
                            );
                            if ui.small_button(format!("{}x", *day_night_speed as u32)).clicked() {
                                *day_night_speed = if (*day_night_speed - 1.0).abs() < 0.1 {
                                    10.0
                                } else if (*day_night_speed - 10.0).abs() < 0.1 {
                                    60.0
                                } else if (*day_night_speed - 60.0).abs() < 0.1 {
                                    240.0
                                } else {
                                    1.0
                                };
                            }
                        }

                        if let Some(sc) = first_short_circuit {
                            ui.separator();
                            if ui
                                .button(RichText::new("SHORT!").strong().color(Color32::from_rgb(248, 113, 113)))
                                .on_hover_text("Focus 3D camera on short circuit point")
                                .clicked()
                            {
                                camera.focus_point(sc.x, sc.y);
                            }
                        }

                        ui.label(
                            RichText::new("LMB: Orbit  •  RMB: Pan  •  Wheel: Zoom")
                                .size(10.5)
                                .color(Color32::from_rgb(148, 163, 184)),
                        );
                    });
                });
        });

    // Interactive CAD ViewCube Widget below top-left HUD bar
    draw_viewcube_widget(ui, rect, camera, layout, is_de);
}

fn draw_viewcube_widget(
    ui: &mut egui::Ui,
    viewport_rect: Rect,
    camera: &mut Camera3D,
    layout: &Layout,
    is_de: bool,
) {
    egui::Area::new(egui::Id::new("hud_3d_viewcube"))
        .fixed_pos(viewport_rect.min + egui::vec2(14.0, 50.0))
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 212))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(8))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 65)))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let stage_size = egui::vec2(96.0, 90.0);
                        let (stage_rect, stage_resp) =
                            ui.allocate_exact_size(stage_size, Sense::click_and_drag());
                        if stage_resp.dragged_by(egui::PointerButton::Primary) {
                            let d = stage_resp.drag_delta();
                            camera.yaw -= d.x * 0.015;
                            camera.pitch = (camera.pitch + d.y * 0.015).clamp(-1.48, 1.48);
                        }

                        let painter = ui.painter_at(stage_rect);
                        let center = stage_rect.center();

                        // 4 Isometric corner buttons around the cube
                        let mut clicked_corner: Option<CameraViewPreset> = None;
                        let corners = [
                            (stage_rect.min + egui::vec2(10.0, 10.0), "NW", CameraViewPreset::IsoNw),
                            (Pos2::new(stage_rect.max.x - 10.0, stage_rect.min.y + 10.0), "NE", CameraViewPreset::IsoNe),
                            (Pos2::new(stage_rect.min.x + 10.0, stage_rect.max.y - 10.0), "SW", CameraViewPreset::IsoSw),
                            (stage_rect.max - egui::vec2(10.0, 10.0), "SE", CameraViewPreset::IsoSe),
                        ];
                        for (cpos, clbl, preset) in corners {
                            let crect = Rect::from_center_size(cpos, egui::vec2(19.0, 15.0));
                            let hov = ui.input(|i| i.pointer.hover_pos().map_or(false, |hp| crect.contains(hp)));
                            painter.rect_filled(
                                crect,
                                3.0,
                                if hov { Color32::from_rgb(2, 132, 199) } else { Color32::from_rgb(30, 41, 59) },
                            );
                            painter.text(crect.center(), egui::Align2::CENTER_CENTER, clbl, FontId::proportional(8.5), Color32::WHITE);
                            if hov && stage_resp.clicked() {
                                clicked_corner = Some(preset);
                            }
                        }

                        // Project unit cube rotated to match camera yaw & pitch
                        let cy = (-camera.yaw).cos();
                        let sy = (-camera.yaw).sin();
                        let cp = camera.pitch.cos();
                        let sp = camera.pitch.sin();
                        let project = |p: [f32; 3]| -> (Pos2, f32) {
                            let x1 = p[0] * cy - p[2] * sy;
                            let z1 = p[0] * sy + p[2] * cy;
                            let y2 = p[1] * cp - z1 * sp;
                            let z2 = p[1] * sp + z1 * cp;
                            let scale = 25.0 * (4.2 / (4.2 - z2 * 0.45));
                            (Pos2::new(center.x + x1 * scale, center.y - y2 * scale), z2)
                        };
                        let xform_norm = |n: [f32; 3]| -> [f32; 3] {
                            let x1 = n[0] * cy - n[2] * sy;
                            let z1 = n[0] * sy + n[2] * cy;
                            let y2 = n[1] * cp - z1 * sp;
                            let z2 = n[1] * sp + z1 * cp;
                            [x1, y2, z2]
                        };

                        let cube_verts: [[f32; 3]; 8] = [
                            [-1.0, -1.0, -1.0],
                            [ 1.0, -1.0, -1.0],
                            [ 1.0,  1.0, -1.0],
                            [-1.0,  1.0, -1.0],
                            [-1.0, -1.0,  1.0],
                            [ 1.0, -1.0,  1.0],
                            [ 1.0,  1.0,  1.0],
                            [-1.0,  1.0,  1.0],
                        ];
                        let faces: [([usize; 4], [f32; 3], &str, CameraViewPreset, Color32); 6] = [
                            ([4, 5, 6, 7], [ 0.0,  0.0,  1.0], if is_de { "VOR" } else { "FRONT" }, CameraViewPreset::Front, Color32::from_rgb(59, 130, 246)),
                            ([1, 0, 3, 2], [ 0.0,  0.0, -1.0], if is_de { "HIN" } else { "BACK" },  CameraViewPreset::Back,  Color32::from_rgb(59, 130, 246)),
                            ([5, 1, 2, 6], [ 1.0,  0.0,  0.0], if is_de { "REC" } else { "RIGHT" }, CameraViewPreset::Right, Color32::from_rgb(239, 68, 68)),
                            ([0, 4, 7, 3], [-1.0,  0.0,  0.0], if is_de { "LIN" } else { "LEFT" },  CameraViewPreset::Left,  Color32::from_rgb(239, 68, 68)),
                            ([3, 2, 6, 7], [ 0.0,  1.0,  0.0], if is_de { "OBN" } else { "TOP" },   CameraViewPreset::Top,   Color32::from_rgb(34, 197, 94)),
                            ([0, 1, 5, 4], [ 0.0, -1.0,  0.0], if is_de { "UNT" } else { "BOT" },   CameraViewPreset::Bottom,Color32::from_rgb(34, 197, 94)),
                        ];

                        let mut vis: Vec<(f32, Vec<Pos2>, &str, CameraViewPreset, Color32)> = faces
                            .iter()
                            .filter_map(|(idx, n, lbl, preset, col)| {
                                let vn = xform_norm(*n);
                                if vn[2] <= 0.05 {
                                    return None;
                                }
                                let mut spts = Vec::with_capacity(4);
                                let mut z_sum = 0.0_f32;
                                for &vi in idx {
                                    let (sp_v, zv) = project(cube_verts[vi]);
                                    spts.push(sp_v);
                                    z_sum += zv;
                                }
                                Some((z_sum * 0.25, spts, *lbl, *preset, *col))
                            })
                            .collect();
                        vis.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                        let hover_pos = ui.input(|i| i.pointer.hover_pos());
                        let mut clicked_face: Option<CameraViewPreset> = None;
                        for (_, spts, lbl, preset, accent) in &vis {
                            let frect = Rect::from_points(spts);
                            let hov = clicked_corner.is_none()
                                && hover_pos.map_or(false, |hp| frect.contains(hp));
                            let fill = if hov {
                                Color32::from_rgb(2, 132, 199)
                            } else {
                                Color32::from_rgba_unmultiplied(30, 41, 59, 240)
                            };
                            painter.add(egui::Shape::convex_polygon(
                                spts.clone(),
                                fill,
                                Stroke::new(1.3_f32, *accent),
                            ));
                            let c_face = Pos2::new(
                                spts.iter().map(|p| p.x).sum::<f32>() * 0.25,
                                spts.iter().map(|p| p.y).sum::<f32>() * 0.25,
                            );
                            painter.text(
                                c_face,
                                egui::Align2::CENTER_CENTER,
                                *lbl,
                                FontId::proportional(9.5),
                                Color32::WHITE,
                            );
                            if hov && stage_resp.clicked() {
                                clicked_face = Some(*preset);
                            }
                        }
                        if let Some(p) = clicked_corner.or(clicked_face) {
                            camera.apply_preset(p, layout);
                        }

                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            if ui.small_button("-45").clicked() { camera.yaw -= std::f32::consts::FRAC_PI_4; }
                            if ui.small_button("ISO").clicked() { camera.apply_preset(CameraViewPreset::Home, layout); }
                            if ui.small_button("+45").clicked() { camera.yaw += std::f32::consts::FRAC_PI_4; }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            if ui.small_button(if is_de { "Oben" } else { "Top" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Top, layout);
                            }
                            if ui.small_button(if is_de { "Vor" } else { "Front" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Front, layout);
                            }
                            if ui.small_button(if is_de { "Unten" } else { "Bot" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Bottom, layout);
                            }
                        });
                    });
                });
        });
}

// ============================================================================
//  Batched 3D Scene Mesh Builder (Baseboard + Detailed Tracks + Models + Terrain)
// ============================================================================

// (Corridor chains built cleanly via terrain_tools::build_connected_corridor_chains)

fn board_style_rgb(style: &str) -> ([f32; 3], [f32; 3]) {
    match style {
        "grass" => ([0.22, 0.42, 0.14], [0.16, 0.30, 0.10]),
        "dark-wood" => ([0.34, 0.22, 0.14], [0.24, 0.15, 0.09]),
        "plywood" => ([0.82, 0.69, 0.48], [0.64, 0.50, 0.33]),
        "slate" => ([0.22, 0.28, 0.36], [0.16, 0.20, 0.27]),
        "white" => ([0.88, 0.90, 0.92], [0.74, 0.77, 0.80]),
        _ => ([0.68, 0.54, 0.36], [0.46, 0.34, 0.21]),
    }
}

#[allow(clippy::too_many_arguments)]
fn push_extruded_polygon_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    outer: &[Vec2],
    holes: &[Vec<Vec2>],
    y_top: f32,
    thickness: f32,
    top_col: [f32; 3],
    side_col: [f32; 3],
) {
    let tris = board_tools::triangulate_polygon_with_holes(outer, holes);
    let y_bot = y_top - thickness;
    for tri in &tris {
        let base_t = verts.len() as u32;
        for pt in tri {
            verts.push(Vertex3D {
                pos: [pt.x as f32, y_top, pt.y as f32],
                normal: [0.0, 1.0, 0.0],
                color: top_col,
                mat: MAT_DEFAULT,
            });
        }
        idxs.extend_from_slice(&[base_t, base_t + 1, base_t + 2]);

        let base_b = verts.len() as u32;
        for pt in tri.iter().rev() {
            verts.push(Vertex3D {
                pos: [pt.x as f32, y_bot, pt.y as f32],
                normal: [0.0, -1.0, 0.0],
                color: side_col,
                mat: MAT_DEFAULT,
            });
        }
        idxs.extend_from_slice(&[base_b, base_b + 1, base_b + 2]);
    }

    let push_wall_ring = |verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, ring: &[Vec2], flip_norm: bool| {
        if ring.len() < 3 {
            return;
        }
        let area_sign = if board_tools::signed_polygon_area(ring) >= 0.0 { 1.0_f32 } else { -1.0 };
        let sign = if flip_norm { -area_sign } else { area_sign };
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let dx = (b.x - a.x) as f32;
            let dz = (b.y - a.y) as f32;
            let len = dx.hypot(dz);
            if len < 1e-4 {
                continue;
            }
            let n = normalize3([sign * (dz / len), 0.0, sign * (-dx / len)]);
            push_quad(
                verts,
                idxs,
                [a.x as f32, y_bot, a.y as f32],
                [b.x as f32, y_bot, b.y as f32],
                [b.x as f32, y_top, b.y as f32],
                [a.x as f32, y_top, a.y as f32],
                n,
                side_col,
            );
        }
    };

    push_wall_ring(verts, idxs, outer, false);
    for hole in holes {
        push_wall_ring(verts, idxs, hole, true);
    }
}

fn collect_turntable_holes(
    layout: &Layout,
    catalogs: &[Catalog],
    outer: &[Vec2],
    layer_elev: f64,
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
) -> Vec<Vec<Vec2>> {
    let mut holes = Vec::new();
    for p in &layout.pieces {
        if p.connectors.len() < 6 {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
        if !vis {
            continue;
        }
        let piece_elev = lh + p.elevation.map(|e| e.start).unwrap_or(0.0);
        if (piece_elev - layer_elev).abs() > 18.0 {
            continue;
        }
        let c = Vec2 { x: p.transform.x, y: p.transform.y };
        if !board_tools::is_point_in_polygon(c, outer) {
            continue;
        }
        let r = find_entry(catalogs, &p.catalog_id)
            .and_then(|e| e.radius.or_else(|| e.length.map(|l| l * 0.5)))
            .unwrap_or(185.0)
            + 2.0;
        let segs = 18usize;
        let mut ring = Vec::with_capacity(segs);
        for k in 0..segs {
            let a = (k as f64 / segs as f64) * std::f64::consts::TAU;
            ring.push(Vec2 { x: c.x + r * a.cos(), y: c.y + r * a.sin() });
        }
        holes.push(ring);
    }
    holes
}

fn push_env_pine(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, x: f32, floor_y: f32, z: f32, h: f32) {
    crate::models3d_mesh::push_env_pine_3d(verts, idxs, x, floor_y, z, h);
}

fn push_train_hall_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, boards: &[BaseboardConfig], dark_mode: bool) {
    let (mut min_x, mut max_x, mut min_z, mut max_z) = (-1200.0_f64, 1200.0_f64, -800.0_f64, 800.0_f64);
    let mut has = false;
    for b in boards {
        if !b.enabled {
            continue;
        }
        for pt in board_tools::get_baseboard_polygon(b) {
            if !has {
                min_x = pt.x; max_x = pt.x; min_z = pt.y; max_z = pt.y; has = true;
            } else {
                min_x = min_x.min(pt.x); max_x = max_x.max(pt.x);
                min_z = min_z.min(pt.y); max_z = max_z.max(pt.y);
            }
        }
    }
    let cx = ((min_x + max_x) * 0.5) as f32;
    let cz = ((min_z + max_z) * 0.5) as f32;
    let w_table = (max_x - min_x).max(1200.0) as f32;
    let l_table = (max_z - min_z).max(1000.0) as f32;
    let w_hall = (w_table + 4600.0).max(8600.0);
    let l_hall = (l_table + 5200.0).max(9400.0);
    let floor_y = -755.0_f32;
    let wall_h = 3200.0_f32;

    let concrete = if dark_mode { [0.18, 0.21, 0.27] } else { [0.36, 0.40, 0.46] };
    let steel = if dark_mode { [0.12, 0.16, 0.22] } else { [0.20, 0.25, 0.33] };
    let brick = if dark_mode { [0.28, 0.16, 0.13] } else { [0.43, 0.22, 0.17] };
    let yellow = [0.88, 0.62, 0.08];

    push_box(verts, idxs, [cx, floor_y - 12.0, cz], [w_hall, 24.0, l_hall], concrete);
    // Safety stripe border around table bay
    let sw = w_table + 720.0;
    let sl = l_table + 720.0;
    push_box(verts, idxs, [cx, floor_y + 1.5, cz - sl * 0.5], [sw, 3.0, 26.0], yellow);
    push_box(verts, idxs, [cx, floor_y + 1.5, cz + sl * 0.5], [sw, 3.0, 26.0], yellow);
    push_box(verts, idxs, [cx - sw * 0.5, floor_y + 1.5, cz], [26.0, 3.0, sl], yellow);
    push_box(verts, idxs, [cx + sw * 0.5, floor_y + 1.5, cz], [26.0, 3.0, sl], yellow);

    // Lower brick wainscoting walls + corner steel pillars
    push_box(verts, idxs, [cx, floor_y + 520.0, cz - l_hall * 0.5], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [cx, floor_y + 520.0, cz + l_hall * 0.5], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [cx - w_hall * 0.5, floor_y + 520.0, cz], [120.0, 1040.0, l_hall], brick);
    push_box(verts, idxs, [cx + w_hall * 0.5, floor_y + 520.0, cz], [120.0, 1040.0, l_hall], brick);

    let bays = 4;
    for i in 0..=bays {
        let pz = (cz - l_hall * 0.5) + (i as f32 / bays as f32) * l_hall;
        for sx in [-1.0_f32, 1.0] {
            push_box(verts, idxs, [cx + sx * w_hall * 0.5, floor_y + wall_h * 0.5, pz], [160.0, wall_h, 160.0], steel);
        }
        // Overhead steel truss beam (leaves large open skylight bays so camera above is unobstructed)
        push_box(verts, idxs, [cx, floor_y + wall_h, pz], [w_hall + 160.0, 120.0, 110.0], steel);
    }

    // Outer pine silhouettes outside hall bay
    for k in 0..10 {
        let ang = (k as f32 / 10.0) * std::f32::consts::TAU;
        let rx = cx + ang.cos() * (w_hall * 0.62);
        let rz = cz + ang.sin() * (l_hall * 0.62);
        push_env_pine(verts, idxs, rx, floor_y, rz, 1400.0 + ((k % 3) as f32) * 350.0);
    }
}

fn push_landscape_env_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, boards: &[BaseboardConfig], dark_mode: bool) {
    let (mut min_x, mut max_x, mut min_z, mut max_z) = (-1200.0_f64, 1200.0_f64, -800.0_f64, 800.0_f64);
    let mut has = false;
    for b in boards {
        if !b.enabled { continue; }
        for pt in board_tools::get_baseboard_polygon(b) {
            if !has {
                min_x = pt.x; max_x = pt.x; min_z = pt.y; max_z = pt.y; has = true;
            } else {
                min_x = min_x.min(pt.x); max_x = max_x.max(pt.x);
                min_z = min_z.min(pt.y); max_z = max_z.max(pt.y);
            }
        }
    }
    let cx = ((min_x + max_x) * 0.5) as f32;
    let cz = ((min_z + max_z) * 0.5) as f32;
    let floor_y = -755.0_f32;

    let meadow = if dark_mode { [0.10, 0.21, 0.11] } else { [0.28, 0.45, 0.14] };
    let hill = if dark_mode { [0.09, 0.17, 0.10] } else { [0.22, 0.36, 0.12] };
    let rock = if dark_mode { [0.20, 0.25, 0.32] } else { [0.38, 0.44, 0.50] };
    let snow = [0.92, 0.94, 0.98];

    // Broad valley ground disc
    push_box(verts, idxs, [cx, floor_y - 14.0, cz], [26000.0, 28.0, 26000.0], meadow);
    // Distant alpine ridge peaks on perimeter
    for k in 0..12 {
        let ang = (k as f32 / 12.0) * std::f32::consts::TAU;
        let dist = 8500.0 + ((k % 3) as f32) * 1400.0;
        let px = cx + ang.cos() * dist;
        let pz = cz + ang.sin() * dist;
        let mh = 2200.0 + ((k * 7) % 5) as f32 * 450.0;
        push_box(verts, idxs, [px, floor_y + mh * 0.25, pz], [3200.0, mh * 0.5, 2800.0], hill);
        push_box(verts, idxs, [px, floor_y + mh * 0.62, pz], [1800.0, mh * 0.32, 1600.0], rock);
        push_box(verts, idxs, [px, floor_y + mh * 0.84, pz], [820.0, mh * 0.18, 740.0], snow);
    }
    for k in 0..16 {
        let ang = (k as f32 / 16.0) * std::f32::consts::TAU + 0.18;
        let dist = 4200.0 + ((k % 4) as f32) * 650.0;
        push_env_pine(verts, idxs, cx + ang.cos() * dist, floor_y, cz + ang.sin() * dist, 1150.0 + ((k % 3) as f32) * 280.0);
    }
}

pub fn build_scene_mesh_3d(layout: &Layout, catalogs: &[Catalog], opts: &Scene3DOptions) -> SceneMesh3D {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    // Layer height & visibility lookup
    let mut layer_info: std::collections::HashMap<&str, (f64, bool)> = std::collections::HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_info.insert(l.id.as_str(), (l.height_mm.unwrap_or(0.0), l.visible));
        }
    }

    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();

    // 0. 3D Environment (Lokhalle / Landscape)
    match opts.env_mode {
        Viewer3DEnv::Hall => push_train_hall_3d(&mut verts, &mut idxs, &boards, opts.dark_mode),
        Viewer3DEnv::Landscape => push_landscape_env_3d(&mut verts, &mut idxs, &boards, opts.dark_mode),
        Viewer3DEnv::Minimal => {}
    }

    // 1. Multi-Table Baseboards (concave polygons, track cutouts, turntable pits, frames & legs)
    let table_layer = layout.layers.as_ref().and_then(|ls| {
        ls.iter().find(|l| l.height_mm.unwrap_or(0.0).abs() < 1e-6)
    });
    let table_visible = table_layer.map_or(true, |l| l.visible);
    if table_visible {
        for b in &boards {
            let poly = board_tools::get_baseboard_polygon(b);
            if poly.len() < 3 {
                continue;
            }
            let matching_plate = table_layer.and_then(|tl| {
                tl.plates.as_ref().and_then(|ps| {
                    ps.iter().find(|pl| {
                        pl.baseboard_id.as_deref().or_else(|| boards.first().and_then(|fb| fb.id.as_deref()))
                            == b.id.as_deref()
                    })
                })
            });
            let eff_style = matching_plate
                .and_then(|p| p.style.as_deref())
                .or_else(|| table_layer.and_then(|tl| tl.wood_plate_style.as_deref()))
                .unwrap_or(b.style.as_str());
            let (top_col, side_col) = board_style_rgb(eff_style);
            let thick = table_layer.and_then(|tl| tl.wood_plate_thickness_mm).unwrap_or(20.0) as f32;
            let cutouts = board_tools::compute_table_track_cutouts(layout, catalogs, b, 0.0, thick as f64);
            let mut holes: Vec<Vec<Vec2>> = cutouts.iter().map(|c| c.points.clone()).collect();
            holes.extend(collect_turntable_holes(layout, catalogs, &poly, 0.0, &layer_info));

            push_extruded_polygon_3d(&mut verts, &mut idxs, &poly, &holes, 0.0, thick, top_col, side_col);

            // Recessed perimeter structural frame beams
            let frame_col = [0.42, 0.29, 0.18];
            for i in 0..poly.len() {
                let a = poly[i];
                let c = poly[(i + 1) % poly.len()];
                push_box_segment(
                    &mut verts,
                    &mut idxs,
                    [a.x as f32, -(thick + 28.0), a.y as f32],
                    [c.x as f32, -(thick + 28.0), c.y as f32],
                    20.0,
                    28.0,
                    frame_col,
                );
            }

            // Cutout-avoiding legs down to floor_y = -755
            let leg_col = [0.35, 0.24, 0.15];
            let leg_h = 755.0 - thick;
            for lp in board_tools::compute_baseboard_leg_positions(b, 17.0, 55.0) {
                if !board_tools::is_point_inside_cutouts(lp, &cutouts, 24.0) {
                    push_box(
                        &mut verts,
                        &mut idxs,
                        [lp.x as f32, -(thick + leg_h * 0.5), lp.y as f32],
                        [38.0, leg_h, 38.0],
                        leg_col,
                    );
                }
            }
        }
    }

    // 1.5. Per-layer wood sub-plates & risers (custom plates, table decks, trackbed & building pads)
    if opts.show_wood_plates {
        if let Some(layers) = &layout.layers {
            for layer in layers {
                if !layer.visible {
                    continue;
                }
                let lh = layer.height_mm.unwrap_or(0.0);
                let lthick = layer.wood_plate_thickness_mm.unwrap_or(12.0);
                if let Some(plates) = &layer.plates {
                    if !plates.is_empty() {
                        for plate in plates {
                            let target_board = plate
                                .baseboard_id
                                .as_deref()
                                .and_then(|bid| boards.iter().find(|b| b.id.as_deref() == Some(bid)))
                                .or_else(|| boards.first());
                            let poly = board_tools::get_layer_plate_polygon(plate, target_board);
                            if poly.len() < 3 {
                                continue;
                            }
                            let pbb = board_tools::layer_plate_as_baseboard(plate, target_board);
                            let pthick = plate.thickness_mm.unwrap_or(lthick);
                            let cutouts = board_tools::compute_table_track_cutouts(layout, catalogs, &pbb, lh, pthick);
                            let mut holes: Vec<Vec<Vec2>> = cutouts.iter().map(|c| c.points.clone()).collect();
                            holes.extend(collect_turntable_holes(layout, catalogs, &poly, lh, &layer_info));
                            let (top_col, side_col) = board_style_rgb(
                                plate.style.as_deref().or(layer.wood_plate_style.as_deref()).unwrap_or("plywood"),
                            );
                            push_extruded_polygon_3d(&mut verts, &mut idxs, &poly, &holes, lh as f32, pthick as f32, top_col, side_col);

                            if lh > 6.0 {
                                let riser_h = (lh - pthick).max(2.0) as f32;
                                let riser_col = [0.46, 0.34, 0.22];
                                for pt in poly.iter().step_by((poly.len() / 4).max(1)) {
                                    if !board_tools::is_point_inside_cutouts(*pt, &cutouts, 22.0) {
                                        push_box(&mut verts, &mut idxs, [pt.x as f32, riser_h * 0.5, pt.y as f32], [20.0, riser_h, 20.0], riser_col);
                                    }
                                }
                            }
                        }
                        continue;
                    }
                }

                let mode = layer.wood_plate_mode.as_deref().unwrap_or("none");
                if mode == "table" && lh.abs() >= 1e-6 {
                    let (top_col, side_col) = board_style_rgb(layer.wood_plate_style.as_deref().unwrap_or("plywood"));
                    for b in &boards {
                        let poly = board_tools::get_baseboard_polygon(b);
                        let cutouts = board_tools::compute_table_track_cutouts(layout, catalogs, b, lh, lthick);
                        let mut holes: Vec<Vec<Vec2>> = cutouts.iter().map(|c| c.points.clone()).collect();
                        holes.extend(collect_turntable_holes(layout, catalogs, &poly, lh, &layer_info));
                        push_extruded_polygon_3d(&mut verts, &mut idxs, &poly, &holes, lh as f32, lthick as f32, top_col, side_col);
                    }
                } else if mode == "tight" {
                    let (top_col, _) = board_style_rgb(layer.wood_plate_style.as_deref().unwrap_or("plywood"));
                    for piece in &layout.pieces {
                        if piece.layer_id.as_deref().unwrap_or("layer-1") != layer.id || piece.has_bridge == Some(true) {
                            continue;
                        }
                        let e0 = lh + piece.elevation.map(|e| e.start).unwrap_or(0.0);
                        let e1 = lh + piece.elevation.map(|e| e.end).unwrap_or(0.0);
                        for br in sample_track_branches_3d(piece, e0, e1) {
                            for w in br.windows(2) {
                                push_box_segment(
                                    &mut verts,
                                    &mut idxs,
                                    [w[0][0], w[0][1] - lthick as f32, w[0][2]],
                                    [w[1][0], w[1][1] - lthick as f32, w[1][2]],
                                    56.0,
                                    lthick as f32,
                                    top_col,
                                );
                            }
                        }
                    }
                    if let Some(models) = &layout.models {
                        for m in models {
                            if m.layer_id.as_deref().unwrap_or("layer-1") != layer.id
                                || matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain"))
                            {
                                continue;
                            }
                            let sc = m.scale.unwrap_or(1.0) as f32;
                            let y_top = (lh + m.elevation.unwrap_or(0.0)) as f32;
                            push_rotated_box(
                                &mut verts,
                                &mut idxs,
                                [m.transform.x as f32, y_top, m.transform.y as f32],
                                (m.transform.rotation as f32).to_radians(),
                                [0.0, -(lthick as f32) * 0.5, 0.0],
                                [(m.width as f32) * sc + 14.0, lthick as f32, (m.depth as f32) * sc + 14.0],
                                top_col,
                            );
                        }
                    }
                }
            }
        }
    }

    let chains = build_connected_corridor_chains(layout, catalogs, &layer_info);
    let all_mountains: Vec<&Model3D> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| m.builtin_id.as_deref() == Some("mountain"))
                .collect()
        })
        .unwrap_or_default();

    // 2. Mountains (with Tunnel Carving, Arched Portals & Swept Tube), Carved Ground Terrain & 3D Models
    if let Some(models) = &layout.models {
        for m in models {
            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
            let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
            if !vis {
                continue;
            }
            let y0 = (lh + m.elevation.unwrap_or(0.0)) as f32;
            match m.builtin_id.as_deref() {
                Some("mountain") => push_mountain_with_tunnels_3d(
                    &mut verts,
                    &mut idxs,
                    m,
                    y0,
                    layout,
                    &boards,
                    &chains,
                    &all_mountains,
                    &layer_info,
                ),
                Some("ground-terrain") => {
                    push_ground_terrain_3d(&mut verts, &mut idxs, m, y0, layout, &layer_info, &boards, &chains);
                }
                _ => push_model_3d(&mut verts, &mut idxs, m, y0),
            }
        }
    }

    // 3. Detailed 3D Tracks & Roads (+ Standalone Piece Tunnels outside mountains)
    for piece in &layout.pieces {
        let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
        let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
        if !vis {
            continue;
        }
        let e0 = lh + piece.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = lh + piece.elevation.map(|e| e.end).unwrap_or(0.0);
        let entry = find_entry(catalogs, &piece.catalog_id);
        let is_road = is_road_id(catalogs, &piece.catalog_id);
        push_track_piece_3d(&mut verts, &mut idxs, piece, entry, is_road, e0, e1);
        if piece.has_tunnel == Some(true) && piece.connectors.len() < 6 {
            push_piece_tunnel_3d(
                &mut verts,
                &mut idxs,
                piece,
                entry,
                is_road,
                e0,
                e1,
                layout,
                &layer_info,
            );
        }
    }

    // 4. Short-Circuit Warning Beacons
    for sc in &opts.short_circuit_points {
        push_box(&mut verts, &mut idxs, [sc.x as f32, 34.0, sc.y as f32], [6.0, 68.0, 6.0], [0.95, 0.15, 0.15]);
        push_box(&mut verts, &mut idxs, [sc.x as f32, 74.0, sc.y as f32], [20.0, 20.0, 20.0], [1.0, 0.30, 0.12]);
    }

    SceneMesh3D { vertices: verts, indices: idxs }
}

fn push_track_piece_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    is_road: bool,
    e0: f64,
    e1: f64,
) {
    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
    let branches = sample_track_branches_3d(piece, e0, e1);

    if is_road {
        let rw = entry.and_then(|e| e.road_width).unwrap_or(60.0) as f32;
        let asphalt = [0.22, 0.25, 0.29];
        let stripe = [0.92, 0.94, 0.96];
        for pts in &branches {
            for w in pts.windows(2) {
                push_trapezoid_segment(verts, idxs, w[0], w[1], rw, rw + 4.0, 2.2, asphalt);
                push_box_segment(verts, idxs, [w[0][0], w[0][1] + 2.2, w[0][2]], [w[1][0], w[1][1] + 2.2, w[1][2]], 2.2, 0.4, stripe);
            }
        }
        return;
    }

    let ballast_top_w = (gauge / 16.5) * 28.0;
    let ballast_bot_w = (gauge / 16.5) * 40.0;
    let ballast_h = 5.5_f32;
    let tie_w = (gauge / 16.5) * 22.0;
    let tie_thick = 2.6_f32;
    let tie_h = 1.6_f32;
    let rail_h = 2.5_f32;
    let rail_w = 1.4_f32;

    let ballast_col = if piece.has_tunnel == Some(true) {
        [0.24, 0.26, 0.30]
    } else {
        [0.42, 0.44, 0.48]
    };
    let tie_col = [0.24, 0.17, 0.12];
    let rail_col = [0.84, 0.87, 0.90];

    // Pass 1: Trapezoidal 3D Ballast + Bridge girders & pillars
    for pts in &branches {
        if pts.len() < 2 {
            continue;
        }
        for w in pts.windows(2) {
            push_trapezoid_segment(verts, idxs, w[0], w[1], ballast_top_w, ballast_bot_w, ballast_h, ballast_col);
            if piece.has_bridge == Some(true) {
                let dx = w[1][0] - w[0][0];
                let dz = w[1][2] - w[0][2];
                let len = dx.hypot(dz).max(1e-4);
                let nx = -dz / len;
                let nz = dx / len;
                let side_off = ballast_bot_w * 0.56;
                for s in [-1.0_f32, 1.0] {
                    let ga = [w[0][0] + nx * side_off * s, w[0][1] - 2.0, w[0][2] + nz * side_off * s];
                    let gb = [w[1][0] + nx * side_off * s, w[1][1] - 2.0, w[1][2] + nz * side_off * s];
                    push_box_segment(verts, idxs, ga, gb, 4.0, 22.0, [0.46, 0.50, 0.56]);
                }
            }
        }
        // Bridge support pillar at midpoint if elevated
        if piece.has_bridge == Some(true) {
            let mid = pts[pts.len() / 2];
            if mid[1] > 12.0 {
                push_box(verts, idxs, [mid[0], mid[1] * 0.5, mid[2]], [ballast_top_w * 0.9, mid[1], 16.0], [0.52, 0.53, 0.55]);
            }
        }
    }

    // Pass 2: 3D Wooden Sleepers/Ties along every branch
    let tie_step = (gauge / 16.5) * 9.5;
    for pts in &branches {
        if pts.len() < 2 {
            continue;
        }
        let mut accum = 0.0_f32;
        for w in pts.windows(2) {
            let a = w[0];
            let b = w[1];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let dz = b[2] - a[2];
            let seg_len = dx.hypot(dz);
            if seg_len < 0.1 {
                continue;
            }
            let nx = -dz / seg_len;
            let nz = dx / seg_len;
            let mut d = tie_step - accum;
            while d <= seg_len {
                let u = d / seg_len;
                let tx = a[0] + dx * u;
                let ty = a[1] + dy * u + ballast_h;
                let tz = a[2] + dz * u;
                let p0 = [tx + nx * tie_w * 0.5, ty, tz + nz * tie_w * 0.5];
                let p1 = [tx - nx * tie_w * 0.5, ty, tz - nz * tie_w * 0.5];
                push_box_segment(verts, idxs, p0, p1, tie_thick, tie_h, tie_col);
                d += tie_step;
            }
            accum = seg_len - (d - tie_step);
        }
    }

    // Pass 3: Twin 3D Metallic Rails + optional Catenary Mast & Wire
    for pts in &branches {
        if pts.len() < 2 {
            continue;
        }
        for w in pts.windows(2) {
            let a = w[0];
            let b = w[1];
            let dx = b[0] - a[0];
            let dz = b[2] - a[2];
            let len = dx.hypot(dz).max(1e-4);
            let nx = -dz / len;
            let nz = dx / len;
            let hg = gauge * 0.5;
            for side in [-1.0_f32, 1.0] {
                let ra = [a[0] + nx * hg * side, a[1] + ballast_h + tie_h, a[2] + nz * hg * side];
                let rb = [b[0] + nx * hg * side, b[1] + ballast_h + tie_h, b[2] + nz * hg * side];
                push_box_segment_mat(verts, idxs, ra, rb, rail_w, rail_h, rail_col, MAT_METAL_RAIL);
            }
            if piece.has_catenary == Some(true) {
                let wa = [a[0], a[1] + 68.0, a[2]];
                let wb = [b[0], b[1] + 68.0, b[2]];
                push_box_segment(verts, idxs, wa, wb, 0.8, 0.8, [0.18, 0.20, 0.22]);
            }
        }
        if piece.has_catenary == Some(true) && pts.len() >= 2 {
            let m = pts[pts.len() / 2];
            let prev = pts[(pts.len() / 2).saturating_sub(1)];
            let dx = m[0] - prev[0];
            let dz = m[2] - prev[2];
            let len = dx.hypot(dz).max(1e-4);
            let nx = -dz / len;
            let nz = dx / len;
            let mast_off = ballast_bot_w * 0.72;
            let mx = m[0] + nx * mast_off;
            let mz = m[2] + nz * mast_off;
            // Mast pole + cantilever arm
            push_box(verts, idxs, [mx, m[1] + 40.0, mz], [3.0, 80.0, 3.0], [0.22, 0.34, 0.26]);
            push_box_segment(verts, idxs, [mx, m[1] + 70.0, mz], [m[0], m[1] + 69.0, m[2]], 1.6, 1.6, [0.22, 0.34, 0.26]);
        }
    }

    // Turnout lantern in 3D (glowing housing next to switch blades)
    if piece.has_lantern == Some(true) {
        let is_left = entry.map_or(false, |e| {
            e.kind == "turnout-left" || e.kind == "curved-turnout-left"
        });
        let side_y = if is_left { -22.0 } else { 22.0 };
        let rp = rotate(Vec2 { x: 26.0, y: side_y }, piece.transform.rotation);
        let lx = (piece.transform.x + rp.x) as f32;
        let lz = (piece.transform.y + rp.y) as f32;
        let ly = e0 as f32;
        let yaw = (piece.transform.rotation as f32).to_radians();
        push_rotated_box(verts, idxs, [lx, ly, lz], yaw, [0.0, 6.0, 0.0], [3.2, 12.0, 3.2], [0.16, 0.19, 0.23]);
        push_rotated_box_mat(verts, idxs, [lx, ly, lz], yaw, [0.0, 15.0, 0.0], [6.5, 6.5, 6.5], [0.98, 0.94, 0.55], [0.22, 0.05, 1.05]);
        push_rotated_box(verts, idxs, [lx, ly, lz], yaw, [0.0, 19.0, 0.0], [7.5, 2.0, 7.5], [0.14, 0.16, 0.20]);
    }

    // Trackside light signal(s) in 3D
    if piece.has_signal == Some(true) {
        if let Some(main_pts) = branches.first().filter(|b| b.len() >= 2) {
            let lat_dist = gauge * 0.95 + 8.0;
            let side_cfg = piece.signal_side.as_deref().unwrap_or("right");
            let dir_cfg = piece.signal_direction.as_deref().unwrap_or("forward");
            let sides: &[f32] = match side_cfg {
                "left" => &[1.0],
                "both" => &[-1.0, 1.0],
                _ => &[-1.0],
            };
            let dirs: &[bool] = match dir_cfg {
                "backward" => &[false],
                "both" => &[true, false],
                _ => &[true],
            };
            for &side_f in sides {
                for &is_fwd in dirs {
                    let frac = if is_fwd { 0.35 } else { 0.65 };
                    let idx = ((main_pts.len() - 1) as f32 * frac).round() as usize;
                    let idx = idx.clamp(0, main_pts.len() - 2);
                    let a = main_pts[idx];
                    let b = main_pts[idx + 1];
                    let dx = b[0] - a[0];
                    let dz = b[2] - a[2];
                    let len = dx.hypot(dz).max(1e-4);
                    let nx = -dz / len;
                    let nz = dx / len;
                    let sx = a[0] + nx * lat_dist * side_f;
                    let sy = a[1];
                    let sz = a[2] + nz * lat_dist * side_f;
                    let travel_x = if is_fwd { dx } else { -dx };
                    let travel_z = if is_fwd { dz } else { -dz };
                    let yaw = (-travel_x).atan2(-travel_z);
                    push_rotated_box(verts, idxs, [sx, sy, sz], yaw, [0.0, 3.0, 0.0], [10.0, 6.0, 10.0], [0.55, 0.57, 0.60]);
                    push_rotated_box(verts, idxs, [sx, sy, sz], yaw, [0.0, 29.0, 0.0], [3.5, 48.0, 3.5], [0.28, 0.32, 0.36]);
                    push_rotated_box(verts, idxs, [sx, sy, sz], yaw, [0.0, 56.0, 0.0], [10.0, 18.0, 6.0], [0.10, 0.12, 0.15]);
                    push_rotated_box_mat(verts, idxs, [sx, sy, sz], yaw, [0.0, 60.0, 3.2], [4.5, 4.5, 1.5], [0.95, 0.18, 0.18], [0.20, 0.05, 0.95]);
                    push_rotated_box_mat(verts, idxs, [sx, sy, sz], yaw, [0.0, 52.0, 3.2], [4.5, 4.5, 1.5], [0.16, 0.92, 0.34], [0.20, 0.05, 1.10]);
                }
            }
        }
    }
}

fn sample_track_branches_3d(piece: &TrackPiece, e0: f64, e1: f64) -> Vec<Vec<[f32; 3]>> {
    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let steps = 18;
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], steps);
            let mut out = Vec::with_capacity(steps + 1);
            for k in 0..=steps {
                let u = k as f64 / steps as f64;
                let rp = rotate(Vec2 { x: flat[k * 5], y: flat[k * 5 + 1] }, piece.transform.rotation);
                out.push([
                    (piece.transform.x + rp.x) as f32,
                    (e0 + (e1 - e0) * u) as f32,
                    (piece.transform.y + rp.y) as f32,
                ]);
            }
            return vec![out];
        }
    }

    let lift = |pts2d: Vec<Vec2>| -> Vec<[f32; 3]> {
        let n = pts2d.len().saturating_sub(1).max(1);
        pts2d
            .into_iter()
            .enumerate()
            .map(|(k, p)| {
                let u = k as f64 / n as f64;
                [p.x as f32, (e0 + (e1 - e0) * u) as f32, p.y as f32]
            })
            .collect()
    };

    let c = &piece.connectors;
    match c.len() {
        0 => vec![],
        1 => {
            let rad = (c[0].angle + 180.0).to_radians();
            vec![lift(vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[0].x + rad.cos() * 70.0, y: c[0].y + rad.sin() * 70.0 },
            ])]
        }
        2 => vec![lift(sample_arc_between_connectors(
            c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 16,
        ))],
        3 => vec![
            lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 14)),
            lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 14)),
        ],
        4 => {
            let a0 = c[0].angle.to_radians();
            let v0x = a0.cos();
            let v0y = a0.sin();
            let facing_opposite = c[1..4]
                .iter()
                .filter(|ci| {
                    let ai = ci.angle.to_radians();
                    v0x * ai.cos() + v0y * ai.sin() < -0.15
                })
                .count();
            if facing_opposite == 3 {
                vec![
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 14)),
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 14)),
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[3].x, c[3].y, c[3].angle, 14)),
                ]
            } else {
                vec![
                    lift(sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 14)),
                    lift(sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[3].x, c[3].y, c[3].angle, 14)),
                ]
            }
        }
        _ => {
            let half = c.len() / 2;
            vec![lift(vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[half].x, y: c[half].y },
            ])]
        }
    }
}

// ============================================================================
//  Detailed 3D Buildings, Trees, Signals & Smooth-Normal Alpine Mountains
// ============================================================================

fn mountain_biome_color(h_ratio: f32, style: Option<&str>) -> [f32; 3] {
    match style.unwrap_or("alpine") {
        "rocky" => {
            if h_ratio > 0.88 {
                [0.78, 0.78, 0.80]
            } else if h_ratio > 0.22 {
                [0.34, 0.33, 0.31]
            } else {
                [0.28, 0.34, 0.20]
            }
        }
        "forest" => {
            if h_ratio > 0.78 {
                [0.38, 0.36, 0.33]
            } else if h_ratio > 0.28 {
                [0.14, 0.28, 0.11]
            } else {
                [0.20, 0.35, 0.12]
            }
        }
        "meadow" => {
            if h_ratio > 0.65 {
                [0.32, 0.44, 0.16]
            } else {
                [0.24, 0.40, 0.13]
            }
        }
        _ => {
            if h_ratio > 0.82 {
                [0.96, 0.98, 1.00]
            } else if h_ratio > 0.35 {
                [0.36, 0.34, 0.32]
            } else {
                [0.24, 0.38, 0.12]
            }
        }
    }
}

fn push_swept_tunnel_lining(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    path_xyz: &[f32],
    prof: &TunnelProfile,
    inset: f32,
    arc_steps: usize,
    color: [f32; 3],
    inward_normals: bool,
) {
    let n_pts = path_xyz.len() / 3;
    if n_pts < 2 {
        return;
    }
    let section = tunnel_profile_points(prof, inset, arc_steps);
    let mut sec_flat = Vec::with_capacity(section.len() * 2);
    for [sx, sy] in &section {
        sec_flat.push(*sx);
        sec_flat.push(*sy);
    }
    let swept = build_tunnel_sweep_positions(path_xyz, &sec_flat);
    let n_sec = section.len();
    let n_ring = swept.len() / (n_sec * 3);
    for r_i in 0..n_ring.saturating_sub(1) {
        for s_j in 0..(n_sec - 1) {
            let b0 = (r_i * n_sec + s_j) * 3;
            let b1 = (r_i * n_sec + s_j + 1) * 3;
            let b2 = ((r_i + 1) * n_sec + s_j + 1) * 3;
            let b3 = ((r_i + 1) * n_sec + s_j) * 3;
            let p0 = [swept[b0], swept[b0 + 1], swept[b0 + 2]];
            let p1 = [swept[b1], swept[b1 + 1], swept[b1 + 2]];
            let p2 = [swept[b2], swept[b2 + 1], swept[b2 + 2]];
            let p3 = [swept[b3], swept[b3 + 1], swept[b3 + 2]];
            let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let v = [p3[0] - p0[0], p3[1] - p0[1], p3[2] - p0[2]];
            let n = normalize3(cross3(u, v));
            let n_use = if inward_normals { [-n[0], -n[1], -n[2]] } else { n };
            push_quad(verts, idxs, p0, p1, p2, p3, n_use, color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_arched_tunnel_portal_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    world_pos: [f32; 3],
    inward_dir_xz: [f32; 2],
    elev_for_found: f32,
    prof: &TunnelProfile,
) {
    let dlen = inward_dir_xz[0].hypot(inward_dir_xz[1]).max(1e-6);
    let fwd = [inward_dir_xz[0] / dlen, inward_dir_xz[1] / dlen]; // local +Z into mountain
    let right = [fwd[1], -fwd[0]]; // local +X right
    let [wx, wy, wz] = world_pos;

    let to_world = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
        [
            wx + right[0] * lx + fwd[0] * lz,
            wy + ly,
            wz + right[1] * lx + fwd[1] * lz,
        ]
    };
    let to_norm = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
        normalize3([right[0] * nx + fwd[0] * nz, ny, right[1] * nx + fwd[1] * nz])
    };

    let c = prof.inner_width * 0.5;
    let outer_hw = c + prof.pier;
    let is_multi = prof.num_tracks > 1;
    let lintel = if prof.is_road { 12.0 } else if is_multi { 18.0 } else { 16.0 };
    let headwall_h = if prof.is_road { 22.0 } else if is_multi { 32.0 } else { 28.0 };
    let face_depth = if is_multi { 16.0 } else { 14.0 };
    let collar_depth = if is_multi { 70.0 } else { 65.0 };

    let stone = [0.38, 0.39, 0.42];
    let capstone = [0.50, 0.51, 0.54];
    let lining = [0.22, 0.23, 0.25];
    let arch_pts = tunnel_profile_points(prof, 0.0, 14);
    let wall_h = prof.wall_h;

    let push_arch_wall_block = |verts: &mut Vec<Vertex3D>,
                                idxs: &mut Vec<u32>,
                                z0: f32,
                                z1: f32,
                                hw_out: f32,
                                h_out: f32,
                                col: [f32; 3]| {
        let h_out = h_out.max(prof.total_h + 6.0);
        let n = arch_pts.len();
        if n < 5 {
            return;
        }
        let fn_front = to_norm(0.0, 0.0, -1.0);
        let fn_back = to_norm(0.0, 0.0, 1.0);

        // Front & Back face slabs with hollow arch opening
        for &(z, norm) in &[(z0, fn_front), (z1, fn_back)] {
            // Left pier rectangle
            push_quad(
                verts,
                idxs,
                to_world(-hw_out, 0.0, z),
                to_world(-c, 0.0, z),
                to_world(-c, wall_h, z),
                to_world(-hw_out, wall_h, z),
                norm,
                col,
            );
            // Right pier rectangle
            push_quad(
                verts,
                idxs,
                to_world(c, 0.0, z),
                to_world(hw_out, 0.0, z),
                to_world(hw_out, wall_h, z),
                to_world(c, wall_h, z),
                norm,
                col,
            );
            // Upper-left & upper-right corner triangles
            push_tri(
                verts,
                idxs,
                to_world(-hw_out, wall_h, z),
                to_world(-c, wall_h, z),
                to_world(-hw_out, h_out, z),
                norm,
                col,
            );
            push_tri(
                verts,
                idxs,
                to_world(c, wall_h, z),
                to_world(hw_out, wall_h, z),
                to_world(hw_out, h_out, z),
                norm,
                col,
            );
            // Spandrel quads over arch curve up to headwall top
            for k in 1..(n - 2) {
                let p_a = arch_pts[k];
                let p_b = arch_pts[k + 1];
                let tx_a = (p_a[0] / c) * hw_out;
                let tx_b = (p_b[0] / c) * hw_out;
                push_quad(
                    verts,
                    idxs,
                    to_world(p_a[0], p_a[1], z),
                    to_world(p_b[0], p_b[1], z),
                    to_world(tx_b, h_out, z),
                    to_world(tx_a, h_out, z),
                    norm,
                    col,
                );
            }
        }

        // Outer box perimeter walls
        push_quad(
            verts,
            idxs,
            to_world(-hw_out, 0.0, z1),
            to_world(-hw_out, 0.0, z0),
            to_world(-hw_out, h_out, z0),
            to_world(-hw_out, h_out, z1),
            to_norm(-1.0, 0.0, 0.0),
            col,
        );
        push_quad(
            verts,
            idxs,
            to_world(hw_out, 0.0, z0),
            to_world(hw_out, 0.0, z1),
            to_world(hw_out, h_out, z1),
            to_world(hw_out, h_out, z0),
            to_norm(1.0, 0.0, 0.0),
            col,
        );
        push_quad(
            verts,
            idxs,
            to_world(-hw_out, h_out, z0),
            to_world(hw_out, h_out, z0),
            to_world(hw_out, h_out, z1),
            to_world(-hw_out, h_out, z1),
            to_norm(0.0, 1.0, 0.0),
            col,
        );

        // Inner arch reveal walls along tunnel opening
        for k in 0..(n - 1) {
            let p_a = arch_pts[k];
            let p_b = arch_pts[k + 1];
            let tx = p_b[0] - p_a[0];
            let ty = p_b[1] - p_a[1];
            let inward_n = to_norm(ty, -tx, 0.0);
            push_quad(
                verts,
                idxs,
                to_world(p_a[0], p_a[1], z0),
                to_world(p_b[0], p_b[1], z0),
                to_world(p_b[0], p_b[1], z1),
                to_world(p_a[0], p_a[1], z1),
                inward_n,
                lining,
            );
        }
    };

    // 1. Stone headwall with open arch cutout
    let face_h = prof.total_h + lintel + headwall_h;
    push_arch_wall_block(verts, idxs, 0.0, face_depth, outer_hw, face_h, stone);

    // 2. Crown cornice parapet
    let portal_yaw = fwd[0].atan2(fwd[1]);
    push_rotated_box(
        verts,
        idxs,
        world_pos,
        portal_yaw,
        [0.0, face_h + 3.0, face_depth * 0.5],
        [outer_hw * 2.0 + 8.0, 6.0, face_depth + 6.0],
        capstone,
    );

    // 3. Deep arch collar extending behind headwall into mountain slope
    push_arch_wall_block(
        verts,
        idxs,
        face_depth,
        face_depth + collar_depth,
        outer_hw - 4.0,
        prof.total_h + 14.0,
        stone,
    );

    // 4. Flared wing retaining walls out front in approach cutting
    let wing_w = if is_multi { 56.0 } else { 48.0 };
    let wing_h = prof.total_h + 4.0;
    let wing_d = 10.0_f32;
    let wing_ang = std::f32::consts::PI * 0.16;
    let dx_wing = outer_hw + (wing_w * 0.5) * wing_ang.cos();
    let dz_wing = -(wing_w * 0.5) * wing_ang.sin();

    push_rotated_box(
        verts,
        idxs,
        to_world(-dx_wing, 0.0, dz_wing),
        portal_yaw - wing_ang,
        [0.0, wing_h * 0.5, 0.0],
        [wing_w, wing_h, wing_d],
        stone,
    );
    push_rotated_box(
        verts,
        idxs,
        to_world(dx_wing, 0.0, dz_wing),
        portal_yaw + wing_ang,
        [0.0, wing_h * 0.5, 0.0],
        [wing_w, wing_h, wing_d],
        stone,
    );

    // 5. Left & right stone foundations beneath piers & wing walls (keeps track opening clear!)
    let side_w = outer_hw - c + wing_w * wing_ang.cos() + 6.0;
    let found_h = (elev_for_found + 16.0).max(24.0);
    let min_z = -wing_w * wing_ang.sin() - 2.0;
    let max_z = face_depth + 6.0;
    let found_d = max_z - min_z;
    let found_z = (min_z + max_z) * 0.5;
    push_rotated_box(
        verts,
        idxs,
        world_pos,
        portal_yaw,
        [-(c + side_w * 0.5), -found_h * 0.5, found_z],
        [side_w, found_h, found_d],
        stone,
    );
    push_rotated_box(
        verts,
        idxs,
        world_pos,
        portal_yaw,
        [c + side_w * 0.5, -found_h * 0.5, found_z],
        [side_w, found_h, found_d],
        stone,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_mountain_with_tunnels_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    m: &Model3D,
    y0: f32,
    layout: &Layout,
    boards: &[BaseboardConfig],
    chains: &[CorridorChain],
    all_mountains: &[&Model3D],
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
) {
    let scale = m.scale.unwrap_or(1.0).max(0.05) as f32;
    let w = (m.width as f32 * scale).max(60.0);
    let d = (m.depth as f32 * scale).max(60.0);
    let h = (m.height as f32 * scale).max(20.0);
    let cx = m.transform.x as f32;
    let cz = m.transform.y as f32;
    let rot_rad = (m.transform.rotation as f32).to_radians();
    let cos_f = (m.transform.rotation.to_radians()).cos();
    let sin_f = (m.transform.rotation.to_radians()).sin();
    let cos_inv = (-m.transform.rotation.to_radians()).cos();
    let sin_inv = (-m.transform.rotation.to_radians()).sin();

    let is_path = m.mountain_path.as_ref().map_or(false, |p| !p.is_empty());
    let nx = if is_path { 52usize } else { 40usize };
    let nz = if is_path { 52usize } else { 40usize };
    let cols = nx + 1;
    let rows = nz + 1;

    // Support polygon clipping from tables & layer wood sub-plates
    let support_polys = collect_terrain_support_polygons(layout, m.layer_id.as_deref(), boards);
    let clip_to_support = !support_polys.is_empty();
    let mut poly_offsets: Vec<u32> = Vec::new();
    let mut poly_coords: Vec<f32> = Vec::new();
    if clip_to_support {
        poly_offsets.push(0);
        for poly in &support_polys {
            for pt in poly {
                poly_coords.push(pt.x as f32);
                poly_coords.push(pt.y as f32);
            }
            poly_offsets.push((poly_coords.len() / 2) as u32);
        }
    }

    let mut heights = vec![0.0_f32; cols * rows];
    let mut inside_support = vec![true; cols * rows];
    let mut norm_r_grid = vec![2.0_f32; cols * rows];

    for iz in 0..rows {
        let lz = (iz as f64 / nz as f64 - 0.5) * d as f64;
        for ix in 0..cols {
            let lx = (ix as f64 / nx as f64 - 0.5) * w as f64;
            let v_idx = iz * cols + ix;
            let wx = cx as f64 + (lx * cos_f - lz * sin_f);
            let wz = cz as f64 + (lx * sin_f + lz * cos_f);
            if clip_to_support && !is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                inside_support[v_idx] = false;
                heights[v_idx] = 0.0;
                continue;
            }
            let (nr, base_h) = evaluate_mountain_local(lx, lz, m);
            norm_r_grid[v_idx] = nr as f32;
            heights[v_idx] = base_h as f32;
        }
    }

    // Tunnel runs & portal placement using connected chains & neighboring mountains
    let tunnel_runs =
        compute_tunnel_runs_for_mountain(m, y0 as f64, chains, all_mountains, layer_info);

    // Collect local collision points across intersecting chains to carve approach cuttings & ensure headroom
    struct LocalCorridor {
        prof: TunnelProfile,
        inner: f32,
        outer_inside: f32,
        outer_approach: f32,
        min_lx: f32,
        max_lx: f32,
        min_lz: f32,
        max_lz: f32,
        pts: Vec<[f32; 3]>, // [lx, lz, rel_elev]
    }
    let bound_r = w.max(d) * 0.65;
    let mut prepared: Vec<LocalCorridor> = Vec::new();
    for ch in chains {
        let near = ch
            .pts_xze
            .iter()
            .any(|p| (p[0] - cx).hypot(p[1] - cz) <= bound_r + 120.0);
        if !near {
            continue;
        }
        let prof = create_tunnel_profile(ch.is_road, ch.has_catenary, ch.gauge, 1, 0.0);
        let inner = prof.inner_width * 0.5 + prof.pier;
        let outer_inside = inner + 45.0;
        let outer_approach = inner + APPROACH_SLOPE as f32;
        let max_reach = outer_inside.max(outer_approach);
        let (mut min_lx, mut max_lx, mut min_lz, mut max_lz) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        let mut pts = Vec::new();
        for p in &ch.pts_xze {
            let rel_elev = p[2] - y0;
            if rel_elev < -35.0 || rel_elev > h - 10.0 {
                continue;
            }
            let dx = p[0] as f64 - cx as f64;
            let dz = p[1] as f64 - cz as f64;
            let lx = (dx * cos_inv - dz * sin_inv) as f32;
            let lz = (dx * sin_inv + dz * cos_inv) as f32;
            min_lx = min_lx.min(lx - max_reach);
            max_lx = max_lx.max(lx + max_reach);
            min_lz = min_lz.min(lz - max_reach);
            max_lz = max_lz.max(lz + max_reach);
            pts.push([lx, lz, rel_elev]);
        }
        if !pts.is_empty() {
            prepared.push(LocalCorridor {
                prof,
                inner,
                outer_inside,
                outer_approach,
                min_lx,
                max_lx,
                min_lz,
                max_lz,
                pts,
            });
        }
    }

    if !prepared.is_empty() {
        for iz in 0..rows {
            let lz = (iz as f32 / nz as f32 - 0.5) * d;
            for ix in 0..cols {
                let lx = (ix as f32 / nx as f32 - 0.5) * w;
                let v_idx = iz * cols + ix;
                if !inside_support[v_idx] {
                    continue;
                }
                let r_dist = norm_r_grid[v_idx];
                if r_dist >= 0.999 {
                    continue;
                }
                let inside_portal = r_dist <= R_PORTAL as f32;
                let mut base_h = heights[v_idx];

                for col in &prepared {
                    if lx < col.min_lx || lx > col.max_lx || lz < col.min_lz || lz > col.max_lz {
                        continue;
                    }
                    let outer = if inside_portal { col.outer_inside } else { col.outer_approach };
                    for pt in &col.pts {
                        let dx = lx - pt[0];
                        if dx.abs() > outer {
                            continue;
                        }
                        let dz = lz - pt[1];
                        if dz.abs() > outer {
                            continue;
                        }
                        let dist = dx.hypot(dz);
                        if dist > outer {
                            continue;
                        }
                        let blend = 1.0 - smoothstep(col.inner as f64, outer as f64, dist as f64) as f32;
                        if inside_portal {
                            let target = pt[2] + col.prof.total_h + 16.0;
                            if base_h < target {
                                base_h += (target - base_h) * blend;
                            }
                        } else {
                            let target = pt[2].max(0.0);
                            if base_h > target {
                                base_h += (target - base_h) * blend;
                            }
                        }
                    }
                }
                heights[v_idx] = base_h;
            }
        }
    }

    // Gather all portals (for entrance curtain quad culling + mesh generation)
    let mut all_portals = Vec::new();
    for tr in &tunnel_runs {
        if let Some(ref sp) = tr.start_portal {
            all_portals.push(sp.clone());
        }
        if let Some(ref ep) = tr.end_portal {
            all_portals.push(ep.clone());
        }
    }

    let is_entrance_curtain_quad = |i_a: usize, i_b: usize, i_c: usize, i_d: usize| -> bool {
        if all_portals.is_empty() {
            return false;
        }
        let coords = [
            ((i_a % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_a / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_a],
            ((i_b % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_b / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_b],
            ((i_c % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_c / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_c],
            ((i_d % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_d / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_d],
        ];
        let min_y = coords[2].min(coords[5]).min(coords[8]).min(coords[11]);
        let max_y = coords[2].max(coords[5]).max(coords[8]).max(coords[11]);
        let cx_q = (coords[0] + coords[3] + coords[6] + coords[9]) * 0.25;
        let cz_q = (coords[1] + coords[4] + coords[7] + coords[10]) * 0.25;
        let vpts = [
            [coords[0], coords[1]],
            [coords[3], coords[4]],
            [coords[6], coords[7]],
            [coords[9], coords[10]],
            [cx_q, cz_q],
        ];

        for po in &all_portals {
            let pe = po.at.rel_elev as f32;
            if min_y >= pe + po.prof.total_h || max_y <= pe - 2.0 {
                continue;
            }
            let (mut min_fwd, mut max_fwd, mut min_lat, mut max_lat) =
                (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
            let px = po.at.lx as f32;
            let pz = po.at.lz as f32;
            let dx_dir = po.dir_xz[0] as f32;
            let dz_dir = po.dir_xz[1] as f32;
            for [vx, vz] in vpts {
                let dx = vx - px;
                let dz = vz - pz;
                let fwd = dx * dx_dir + dz * dz_dir;
                let lat = dx * (-dz_dir) + dz * dx_dir;
                min_fwd = min_fwd.min(fwd);
                max_fwd = max_fwd.max(fwd);
                min_lat = min_lat.min(lat);
                max_lat = max_lat.max(lat);
            }
            let half_clear_w = po.prof.inner_width * 0.5 + 4.0;
            if max_lat >= -half_clear_w && min_lat <= half_clear_w && max_fwd >= -42.0 && min_fwd <= 52.0 {
                return true;
            }
        }
        false
    };

    let step_x = w / nx as f32;
    let step_z = d / nz as f32;
    let base_v = verts.len() as u32;

    for iz in 0..rows {
        let lz = (iz as f32 / nz as f32 - 0.5) * d;
        for ix in 0..cols {
            let lx = (ix as f32 / nx as f32 - 0.5) * w;
            let v_idx = iz * cols + ix;
            let mh = heights[v_idx];
            let hl = heights[iz * cols + ix.saturating_sub(1)];
            let hr = heights[iz * cols + (ix + 1).min(nx)];
            let hd = heights[iz.saturating_sub(1) * cols + ix];
            let hu = heights[(iz + 1).min(nz) * cols + ix];
            let normal = normalize3([(hl - hr) / (2.0 * step_x), 1.0, (hd - hu) / (2.0 * step_z)]);
            let ratio = (mh / h.max(1.0)).clamp(0.0, 1.0);
            let col = mountain_biome_color(ratio, m.mountain_style.as_deref());
            let (rx, rz) = rot_xz(lx, lz, rot_rad);
            let (rnx, rnz) = rot_xz(normal[0], normal[2], rot_rad);
            verts.push(Vertex3D {
                pos: [cx + rx, y0 + mh + 0.5, cz + rz],
                normal: [rnx, normal[1], rnz],
                color: col,
                mat: MAT_TERRAIN,
            });
        }
    }

    for iz in 0..nz {
        for ix in 0..nx {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            let (ha, hb, hc, hd) = (heights[i_a], heights[i_b], heights[i_c], heights[i_d]);
            if ha <= 0.1 && hb <= 0.1 && hc <= 0.1 && hd <= 0.1 {
                continue;
            }
            if is_entrance_curtain_quad(i_a, i_b, i_c, i_d) {
                continue;
            }
            if (ha > 0.1 || hc > 0.1 || hb > 0.1)
                && inside_support[i_a]
                && inside_support[i_c]
                && inside_support[i_b]
            {
                idxs.extend_from_slice(&[base_v + i_a as u32, base_v + i_c as u32, base_v + i_b as u32]);
            }
            if (hb > 0.1 || hc > 0.1 || hd > 0.1)
                && inside_support[i_b]
                && inside_support[i_c]
                && inside_support[i_d]
            {
                idxs.extend_from_slice(&[base_v + i_b as u32, base_v + i_c as u32, base_v + i_d as u32]);
            }
        }
    }

    // 2. Arched Stone Portals (skip overlap-suppressed interior joins)
    for po in &all_portals {
        if po.suppressed_by_overlap {
            continue;
        }
        let (pwx, pwz) = mountain_local_to_world(po.at.lx, po.at.lz, Some(m));
        let dwx = po.dir_xz[0] * cos_f - po.dir_xz[1] * sin_f;
        let dwz = po.dir_xz[0] * sin_f + po.dir_xz[1] * cos_f;
        push_arched_tunnel_portal_3d(
            verts,
            idxs,
            [pwx as f32, y0 + po.at.rel_elev as f32, pwz as f32],
            [dwx as f32, dwz as f32],
            (y0 + po.at.rel_elev as f32).max(0.0),
            &po.prof,
        );
    }

    // 3. Vaulted Interior Tunnel Linings along all runs
    for tr in &tunnel_runs {
        if tr.pts.len() < 2 {
            continue;
        }
        let mut pts = tr.pts.clone();
        if tr.start_portal.as_ref().map_or(false, |sp| sp.suppressed_by_overlap) {
            let p0 = pts[0];
            let d = tr.start_portal.as_ref().unwrap().dir_xz;
            pts.insert(
                0,
                crate::terrain_tools::LocalTunnelPt {
                    lx: p0.lx - d[0] * 35.0,
                    lz: p0.lz - d[1] * 35.0,
                    rel_elev: p0.rel_elev,
                    norm_r: p0.norm_r,
                },
            );
        }
        if tr.end_portal.as_ref().map_or(false, |ep| ep.suppressed_by_overlap) {
            let pn = *pts.last().unwrap();
            let d = tr.end_portal.as_ref().unwrap().dir_xz;
            pts.push(crate::terrain_tools::LocalTunnelPt {
                lx: pn.lx - d[0] * 35.0,
                lz: pn.lz - d[1] * 35.0,
                rel_elev: pn.rel_elev,
                norm_r: pn.norm_r,
            });
        }
        let mut path_xyz = Vec::with_capacity(pts.len() * 3);
        for p in &pts {
            let (wx, wz) = mountain_local_to_world(p.lx, p.lz, Some(m));
            path_xyz.push(wx as f32);
            path_xyz.push(y0 + p.rel_elev as f32);
            path_xyz.push(wz as f32);
        }
        push_swept_tunnel_lining(verts, idxs, &path_xyz, &tr.prof, 1.0, 10, [0.14, 0.15, 0.17], true);
    }
}

#[allow(clippy::too_many_arguments)]
fn push_piece_tunnel_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    is_road: bool,
    e0: f64,
    e1: f64,
    layout: &Layout,
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
) {
    let raw_branches = sample_track_branches_3d(piece, e0, e1);
    let Some(main_br) = raw_branches.first().filter(|b| b.len() >= 2) else {
        return;
    };
    let mid = main_br[main_br.len() / 2];
    if is_point_inside_any_mountain(mid[0] as f64, mid[2] as f64, mid[1] as f64, layout, layer_info, 0.84) {
        return;
    }

    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5);
    let prof = create_tunnel_profile(is_road, piece.has_catenary == Some(true), gauge, 1, 0.0);

    for br in &raw_branches {
        if br.len() < 2 {
            continue;
        }
        let mut path_xyz = Vec::with_capacity(br.len() * 3);
        for p in br {
            path_xyz.push(p[0]);
            path_xyz.push(p[1]);
            path_xyz.push(p[2]);
        }
        push_swept_tunnel_lining(verts, idxs, &path_xyz, &prof, 1.0, 10, [0.14, 0.15, 0.17], true);
        push_swept_tunnel_lining(verts, idxs, &path_xyz, &prof, -8.5, 10, [0.25, 0.38, 0.10], false);
    }

    let neighbor_has_tunnel_or_mtn = |c_idx: usize, elev_at_c: f64| -> bool {
        let Some(conn) = piece.connectors.get(c_idx) else {
            return false;
        };
        if let Some(ref link) = conn.connected_to {
            if let Some(other) = layout.pieces.iter().find(|p| p.id == link.piece_id) {
                if other.has_tunnel == Some(true) {
                    return true;
                }
            }
        }
        is_point_inside_any_mountain(conn.x, conn.y, elev_at_c, layout, layer_info, 0.84)
    };

    if !neighbor_has_tunnel_or_mtn(0, e0) {
        let p0 = main_br[0];
        let p1 = main_br[1];
        let dx = p1[0] - p0[0];
        let dz = p1[2] - p0[2];
        if dx.hypot(dz) > 1e-4 {
            push_arched_tunnel_portal_3d(verts, idxs, p0, [dx, dz], e0.max(0.0) as f32, &prof);
        }
    }
    if piece.connectors.len() >= 2 && !neighbor_has_tunnel_or_mtn(1, e1) {
        let p_end = main_br[main_br.len() - 1];
        let p_prev = main_br[main_br.len() - 2];
        let dx = p_prev[0] - p_end[0];
        let dz = p_prev[2] - p_end[2];
        if dx.hypot(dz) > 1e-4 {
            push_arched_tunnel_portal_3d(verts, idxs, p_end, [dx, dz], e1.max(0.0) as f32, &prof);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_ground_terrain_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    m: &Model3D,
    y0: f32,
    layout: &Layout,
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
    boards: &[BaseboardConfig],
    corridors: &[CorridorChain],
) {
    let scale = m.scale.unwrap_or(1.0).max(0.05);
    let width = (m.width * scale).max(60.0);
    let depth = (m.depth * scale).max(60.0);
    let ttype = m.ground_terrain_type.as_deref().unwrap_or("hill");
    let is_water = ttype == "water";
    let default_h: f64 = match ttype {
        "water" => 6.0,
        "sand" => 12.0,
        "dirt" => 14.0,
        _ => 26.0,
    };
    let max_height = (if m.height > 0.0 { m.height } else { default_h }).max(2.0) * scale;
    let mx = m.transform.x;
    let mz = m.transform.y;
    let rot_rad = m.transform.rotation.to_radians();
    let cos_m = rot_rad.cos();
    let sin_m = rot_rad.sin();

    let segs_x = 44usize;
    let segs_z = 44usize;
    let cols = segs_x + 1;
    let rows = segs_z + 1;
    let has_path = m.mountain_path.as_ref().map_or(false, |p| !p.is_empty());

    let mut r_dist_grid = Vec::with_capacity(cols * rows);
    for iz in 0..rows {
        let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
        let nz = lz / (depth * 0.5);
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
            let nx = lx / (width * 0.5);
            let nr = if has_path {
                evaluate_mountain_local(lx, lz, m).0
            } else {
                nx.hypot(nz)
            };
            r_dist_grid.push(nr as f32);
        }
    }

    // Support polygon from tables + active layer wood plates
    let support_polys = collect_terrain_support_polygons(layout, m.layer_id.as_deref(), boards);
    let mut poly_offsets: Vec<u32> = Vec::new();
    let mut poly_coords: Vec<f32> = Vec::new();
    if !support_polys.is_empty() {
        poly_offsets.push(0);
        for poly in &support_polys {
            for pt in poly {
                poly_coords.push(pt.x as f32);
                poly_coords.push(pt.y as f32);
            }
            poly_offsets.push((poly_coords.len() / 2) as u32);
        }
    }

    // Buildings to flatten ground underneath
    let mut buildings: Vec<f32> = Vec::new();
    if let Some(models) = &layout.models {
        for other in models {
            if other.id == m.id {
                continue;
            }
            if matches!(other.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                continue;
            }
            let lid = other.layer_id.as_deref().unwrap_or("layer-1");
            if !layer_info.get(lid).map_or(true, |x| x.1) {
                continue;
            }
            let bscale = other.scale.unwrap_or(1.0);
            let brot = (-other.transform.rotation).to_radians();
            buildings.push(other.transform.x as f32);
            buildings.push(other.transform.y as f32);
            buildings.push(brot.cos() as f32);
            buildings.push(brot.sin() as f32);
            buildings.push((other.width * bscale * 0.5 + 10.0) as f32);
            buildings.push((other.depth * bscale * 0.5 + 10.0) as f32);
        }
    }

    // Corridors metadata + flattened points
    let mut corridor_meta: Vec<f32> = Vec::with_capacity(corridors.len() * 8);
    let mut corridor_pts: Vec<f32> = Vec::new();
    for br in corridors {
        if br.pts_xze.is_empty() {
            continue;
        }
        let (mut min_x, mut max_x, mut min_z, mut max_z) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        let pad = br.half_w + 48.0;
        let pt_start = (corridor_pts.len() / 3) as f32;
        for p in &br.pts_xze {
            min_x = min_x.min(p[0] - pad);
            max_x = max_x.max(p[0] + pad);
            min_z = min_z.min(p[1] - pad);
            max_z = max_z.max(p[1] + pad);
            corridor_pts.push(p[0]);
            corridor_pts.push(p[1]);
            corridor_pts.push(p[2]);
        }
        corridor_meta.extend_from_slice(&[
            min_x,
            max_x,
            min_z,
            max_z,
            br.half_w,
            if br.is_turntable { 1.0 } else { 0.0 },
            pt_start,
            br.pts_xze.len() as f32,
        ]);
    }

    let grid = carve_ground_terrain_grid(
        segs_x,
        segs_z,
        width,
        depth,
        max_height,
        y0 as f64,
        mx,
        mz,
        cos_m,
        sin_m,
        46.0,
        is_water,
        &r_dist_grid,
        &poly_offsets,
        &poly_coords,
        &buildings,
        &corridor_meta,
        &corridor_pts,
    );

    let step_x = (width / segs_x as f64) as f32;
    let step_z = (depth / segs_z as f64) as f32;
    let base_v = verts.len() as u32;

    for iz in 0..rows {
        let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
        let nz = lz / (depth * 0.5);
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
            let nx = lx / (width * 0.5);
            let n_col = (mountain_noise(nx * 1.4, nz * 1.4) * 0.04) as f32;
            let v_idx = iz * cols + ix;
            let gh = grid[v_idx * 4];
            let is_w = grid[v_idx * 4 + 2] > 0.5;
            let flat = grid[v_idx * 4 + 3];

            let hl = grid[(iz * cols + ix.saturating_sub(1)) * 4];
            let hr = grid[(iz * cols + (ix + 1).min(segs_x)) * 4];
            let hd = grid[(iz.saturating_sub(1) * cols + ix) * 4];
            let hu = grid[((iz + 1).min(segs_z) * cols + ix) * 4];
            let norm = normalize3([(hl - hr) / (2.0 * step_x), 1.0, (hd - hu) / (2.0 * step_z)]);
            let (rnx, rnz) = rot_xz(norm[0], norm[2], rot_rad as f32);

            let h_ratio = (gh / max_height.max(1.0) as f32).clamp(0.0, 1.0);
            let col = match ttype {
                "sand" => [
                    (0.84 + h_ratio * 0.06 + n_col - flat * 0.04).clamp(0.0, 1.0),
                    (0.74 + h_ratio * 0.05 + n_col - flat * 0.04).clamp(0.0, 1.0),
                    (0.51 + h_ratio * 0.03 + n_col).clamp(0.0, 1.0),
                ],
                "dirt" => [
                    (0.44 + h_ratio * 0.08 + n_col).clamp(0.0, 1.0),
                    (0.32 + h_ratio * 0.06 + n_col).clamp(0.0, 1.0),
                    (0.21 + h_ratio * 0.04 + n_col).clamp(0.0, 1.0),
                ],
                "water" => {
                    if !is_w || flat > 0.35 || h_ratio > 0.25 {
                        [0.74 + n_col, 0.66 + n_col, 0.48 + n_col]
                    } else {
                        [0.14 + n_col * 0.5, 0.32 + n_col * 0.5, 0.42 + n_col * 0.5]
                    }
                }
                _ => [
                    (0.24 + h_ratio * 0.10 + n_col).clamp(0.0, 1.0),
                    (0.42 + h_ratio * 0.12 + n_col).clamp(0.0, 1.0),
                    (0.14 + h_ratio * 0.04 + n_col).clamp(0.0, 1.0),
                ],
            };

            let wx = mx + (lx * cos_m - lz * sin_m);
            let wz = mz + (lx * sin_m + lz * cos_m);
            verts.push(Vertex3D {
                pos: [wx as f32, y0 + gh + 0.3, wz as f32],
                normal: [rnx, norm[1], rnz],
                color: col,
                mat: MAT_TERRAIN,
            });
        }
    }

    for iz in 0..segs_z {
        for ix in 0..segs_x {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            let in_poly = |vi: usize| grid[vi * 4 + 1] > 0.5;
            let vis_h = |vi: usize| grid[vi * 4] > 0.08;
            if in_poly(i_a)
                && in_poly(i_b)
                && in_poly(i_c)
                && in_poly(i_d)
                && (vis_h(i_a) || vis_h(i_b) || vis_h(i_c) || vis_h(i_d))
            {
                let a = base_v + i_a as u32;
                let b = base_v + i_b as u32;
                let c = base_v + i_c as u32;
                let d_i = base_v + i_d as u32;
                idxs.extend_from_slice(&[a, c, b, b, c, d_i]);
            }
        }
    }

    // Reflective cyan water surface over water basin cells (spares tracks, roads & buildings)
    if is_water {
        let water_base_v = verts.len() as u32;
        let water_y = y0 + 1.85;
        let water_col = [0.06, 0.56, 0.84];
        for iz in 0..rows {
            let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
            for ix in 0..cols {
                let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
                let wx = mx + (lx * cos_m - lz * sin_m);
                let wz = mz + (lx * sin_m + lz * cos_m);
                verts.push(Vertex3D {
                    pos: [wx as f32, water_y, wz as f32],
                    normal: [0.0, 1.0, 0.0],
                    color: water_col,
                    mat: MAT_WATER,
                });
            }
        }
        for iz in 0..segs_z {
            for ix in 0..segs_x {
                let i_a = iz * cols + ix;
                let i_b = i_a + 1;
                let i_c = (iz + 1) * cols + ix;
                let i_d = i_c + 1;
                let is_wat = |vi: usize| grid[vi * 4 + 1] > 0.5 && grid[vi * 4 + 2] > 0.5;
                if is_wat(i_a) && is_wat(i_b) && is_wat(i_c) {
                    idxs.extend_from_slice(&[
                        water_base_v + i_a as u32,
                        water_base_v + i_c as u32,
                        water_base_v + i_b as u32,
                    ]);
                }
                if is_wat(i_b) && is_wat(i_c) && is_wat(i_d) {
                    idxs.extend_from_slice(&[
                        water_base_v + i_b as u32,
                        water_base_v + i_c as u32,
                        water_base_v + i_d as u32,
                    ]);
                }
            }
        }
    }
}

fn push_model_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, m: &Model3D, y0: f32) {
    let scale = m.scale.unwrap_or(1.0) as f32;
    let cx = m.transform.x as f32;
    let cz = m.transform.y as f32;
    let rot_rad = (m.transform.rotation as f32).to_radians();

    // Imported custom .glb / .gltf mesh
    if m.file_data.is_some() && matches!(m.model_type.as_str(), "glb" | "gltf") {
        if let Some(parsed) = crate::model_tools::decode_custom_model_mesh(m) {
            let cx_mesh = (parsed.min[0] + parsed.max[0]) * 0.5;
            let cz_mesh = (parsed.min[2] + parsed.max[2]) * 0.5;
            let base_y = parsed.min[1];
            for tri in &parsed.triangles {
                let base_v = verts.len() as u32;
                for k in 0..3 {
                    let lx = (tri.positions[k][0] - cx_mesh) * scale;
                    let ly = (tri.positions[k][1] - base_y) * scale;
                    let lz = (tri.positions[k][2] - cz_mesh) * scale;
                    let (rx, rz) = rot_xz(lx, lz, rot_rad);
                    let (nx, nz) = rot_xz(tri.normals[k][0], tri.normals[k][2], rot_rad);
                    verts.push(Vertex3D {
                        pos: [cx + rx, y0 + ly, cz + rz],
                        normal: normalize3([nx, tri.normals[k][1], nz]),
                        color: tri.color,
                        mat: [0.68, 0.08, 0.0],
                    });
                }
                idxs.extend_from_slice(&[base_v, base_v + 1, base_v + 2]);
            }
            return;
        }
    }

    crate::models3d_mesh::push_builtin_model_3d(verts, idxs, m, y0);
}

// ============================================================================
//  3D Geometry Primitives (Trapezoidal Ballast, Box Segments, Rotated Boxes)
// ============================================================================

pub(crate) fn rot_xz(x: f32, z: f32, rad: f32) -> (f32, f32) {
    let c = rad.cos();
    let s = rad.sin();
    (x * c - z * s, x * s + z * c)
}

#[allow(clippy::too_many_arguments)]
fn push_trapezoid_segment(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    top_w: f32,
    bot_w: f32,
    height: f32,
    color: [f32; 3],
) {
    let dx = b[0] - a[0];
    let dz = b[2] - a[2];
    let len = dx.hypot(dz).max(1e-4);
    let nx = -dz / len;
    let nz = dx / len;
    let ht = top_w * 0.5;
    let hb = bot_w * 0.5;

    let atl = [a[0] + nx * ht, a[1] + height, a[2] + nz * ht];
    let atr = [a[0] - nx * ht, a[1] + height, a[2] - nz * ht];
    let btl = [b[0] + nx * ht, b[1] + height, b[2] + nz * ht];
    let btr = [b[0] - nx * ht, b[1] + height, b[2] - nz * ht];

    let abl = [a[0] + nx * hb, a[1], a[2] + nz * hb];
    let abr = [a[0] - nx * hb, a[1], a[2] - nz * hb];
    let bbl = [b[0] + nx * hb, b[1], b[2] + nz * hb];
    let bbr = [b[0] - nx * hb, b[1], b[2] - nz * hb];

    let slope_col = [color[0] * 0.86, color[1] * 0.86, color[2] * 0.86];
    // Top roadbed
    push_quad(verts, idxs, atl, atr, btr, btl, [0.0, 1.0, 0.0], color);
    // Left embankment slope
    push_quad(verts, idxs, abl, atl, btl, bbl, normalize3([nx, 0.65, nz]), slope_col);
    // Right embankment slope
    push_quad(verts, idxs, atr, abr, bbr, btr, normalize3([-nx, 0.65, -nz]), slope_col);
}

fn push_box_segment(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    width: f32,
    height: f32,
    color: [f32; 3],
) {
    push_box_segment_mat(verts, idxs, a, b, width, height, color, MAT_DEFAULT);
}

#[allow(clippy::too_many_arguments)]
fn push_box_segment_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    width: f32,
    height: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    let dx = b[0] - a[0];
    let dz = b[2] - a[2];
    let len = dx.hypot(dz).max(1e-4);
    let nx = -dz / len;
    let nz = dx / len;
    let hw = width * 0.5;

    let atl = [a[0] + nx * hw, a[1] + height, a[2] + nz * hw];
    let atr = [a[0] - nx * hw, a[1] + height, a[2] - nz * hw];
    let btl = [b[0] + nx * hw, b[1] + height, b[2] + nz * hw];
    let btr = [b[0] - nx * hw, b[1] + height, b[2] - nz * hw];

    let abl = [a[0] + nx * hw, a[1], a[2] + nz * hw];
    let abr = [a[0] - nx * hw, a[1], a[2] - nz * hw];
    let bbl = [b[0] + nx * hw, b[1], b[2] + nz * hw];
    let bbr = [b[0] - nx * hw, b[1], b[2] - nz * hw];

    push_quad_mat(verts, idxs, atl, atr, btr, btl, [0.0, 1.0, 0.0], color, mat);
    push_quad_mat(verts, idxs, abl, atl, btl, bbl, [nx, 0.0, nz], color, mat);
    push_quad_mat(verts, idxs, atr, abr, bbr, btr, [-nx, 0.0, -nz], color, mat);
}

fn push_rotated_box(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    local_center: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
) {
    push_rotated_box_mat(verts, idxs, origin, rot_rad, local_center, size, color, MAT_DEFAULT);
}

#[allow(clippy::too_many_arguments)]
fn push_rotated_box_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    local_center: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
    mat: [f32; 3],
) {
    let hx = size[0] * 0.5;
    let hy = size[1] * 0.5;
    let hz = size[2] * 0.5;
    let pt = |x: f32, y: f32, z: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(local_center[0] + x, local_center[2] + z, rot_rad);
        [origin[0] + rx, origin[1] + local_center[1] + y, origin[2] + rz]
    };
    let rn = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(nx, nz, rot_rad);
        [rx, ny, rz]
    };

    push_quad_mat(verts, idxs, pt(-hx, hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 1.0, 0.0), color, mat);
    push_quad_mat(verts, idxs, pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 0.0, 1.0), color, mat);
    push_quad_mat(verts, idxs, pt(hx, -hy, -hz), pt(-hx, -hy, -hz), pt(-hx, hy, -hz), pt(hx, hy, -hz), rn(0.0, 0.0, -1.0), color, mat);
    push_quad_mat(verts, idxs, pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), rn(1.0, 0.0, 0.0), color, mat);
    push_quad_mat(verts, idxs, pt(-hx, -hy, -hz), pt(-hx, -hy, hz), pt(-hx, hy, hz), pt(-hx, hy, -hz), rn(-1.0, 0.0, 0.0), color, mat);
}

fn push_box(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    center: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
) {
    let hx = size[0] * 0.5;
    let hy = size[1] * 0.5;
    let hz = size[2] * 0.5;
    let [cx, cy, cz] = center;

    push_quad(verts, idxs, [cx - hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz + hz], [cx - hx, cy + hy, cz + hz], [0.0, 1.0, 0.0], color);
    push_quad(verts, idxs, [cx - hx, cy - hy, cz + hz], [cx + hx, cy - hy, cz + hz], [cx + hx, cy + hy, cz + hz], [cx - hx, cy + hy, cz + hz], [0.0, 0.0, 1.0], color);
    push_quad(verts, idxs, [cx + hx, cy - hy, cz - hz], [cx - hx, cy - hy, cz - hz], [cx - hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz - hz], [0.0, 0.0, -1.0], color);
    push_quad(verts, idxs, [cx + hx, cy - hy, cz + hz], [cx + hx, cy - hy, cz - hz], [cx + hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz + hz], [1.0, 0.0, 0.0], color);
    push_quad(verts, idxs, [cx - hx, cy - hy, cz - hz], [cx - hx, cy - hy, cz + hz], [cx - hx, cy + hy, cz + hz], [cx - hx, cy + hy, cz - hz], [-1.0, 0.0, 0.0], color);
}

#[allow(clippy::too_many_arguments)]
fn push_quad(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
) {
    push_quad_mat(verts, idxs, p0, p1, p2, p3, normal, color, MAT_DEFAULT);
}

#[allow(clippy::too_many_arguments)]
fn push_quad_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    mat: [f32; 3],
) {
    let base = verts.len() as u32;
    verts.push(Vertex3D { pos: p0, normal, color, mat });
    verts.push(Vertex3D { pos: p1, normal, color, mat });
    verts.push(Vertex3D { pos: p2, normal, color, mat });
    verts.push(Vertex3D { pos: p3, normal, color, mat });
    idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[allow(clippy::too_many_arguments)]
fn push_tri(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
) {
    let base = verts.len() as u32;
    verts.push(Vertex3D { pos: p0, normal, color, mat: MAT_DEFAULT });
    verts.push(Vertex3D { pos: p1, normal, color, mat: MAT_DEFAULT });
    verts.push(Vertex3D { pos: p2, normal, color, mat: MAT_DEFAULT });
    idxs.extend_from_slice(&[base, base + 1, base + 2]);
}

// ============================================================================
//  Minimal 4x4 Column-Major Matrix Math
// ============================================================================

fn mat4_perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y * 0.5).tan();
    let nf = 1.0 / (near - far);
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far * nf, -1.0],
        [0.0, 0.0, far * near * nf, 0.0],
    ]
}

fn mat4_look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize3([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    let s = normalize3(cross3(f, up));
    let u = cross3(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot3(s, eye), -dot3(u, eye), dot3(f, eye), 1.0],
    ]
}

fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0_f32; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] = a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = dot3(v, v).sqrt().max(1e-6);
    [v[0] / len, v[1] / len, v[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};
    use railbench_core::types::Transform;

    #[test]
    fn mountain_with_intersecting_track_generates_tunnel_geometry() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24360").unwrap(); // 360 mm straight
        let p0 = create_piece("t0".into(), entry, Transform { x: -360.0, y: 0.0, rotation: 0.0 }, None);
        let p1 = create_piece("t1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        let mtn = Model3D {
            id: "m1".into(),
            name: "Alpine".into(),
            category: "terrain".into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 600.0,
            depth: 500.0,
            height: 260.0,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };

        let layout_only_mtn = Layout {
            name: "T".into(),
            scale: "H0".into(),
            pieces: vec![],
            models: Some(vec![mtn.clone()]),
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
        };
        let mut layout_with_tunnel = layout_only_mtn.clone();
        layout_with_tunnel.pieces = vec![p0, p1];

        let mesh_base = build_scene_mesh_3d(&layout_only_mtn, &catalogs, &Scene3DOptions::default());
        let mesh_tunnel = build_scene_mesh_3d(&layout_with_tunnel, &catalogs, &Scene3DOptions::default());
        assert!(mesh_tunnel.vertices.len() > mesh_base.vertices.len() + 100);
        assert!(mesh_tunnel.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
    }

    #[test]
    fn accessories_and_engine_shed_generate_3d_geometry() {
        let catalogs = builtin_catalogs();
        let turnout_entry = find_entry(&catalogs, "24611").unwrap();
        let mut turnout = create_piece("sw1".into(), turnout_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        turnout.has_lantern = Some(true);
        turnout.has_decoder = Some(true);
        turnout.has_signal = Some(true);
        turnout.signal_side = Some("both".into());
        turnout.signal_direction = Some("both".into());

        let base_turnout = create_piece("sw0".into(), turnout_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        let make_layout = |pieces, models| Layout {
            name: "Acc3D".into(),
            scale: "H0".into(),
            pieces,
            models,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
        };

        let opts = Scene3DOptions::default();
        let m_plain = build_scene_mesh_3d(&make_layout(vec![base_turnout], None), &catalogs, &opts);
        let shed = Model3D {
            id: "shed-1".into(),
            name: "Lokschuppen".into(),
            category: "railway".into(),
            transform: Transform { x: 300.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 240.0,
            depth: 110.0,
            height: 95.0,
            model_type: "builtin".into(),
            builtin_id: Some("engine-shed".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let hall = Model3D {
            id: "hall-1".into(),
            name: "Ringlokschuppen".into(),
            category: "railway".into(),
            transform: Transform { x: 600.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 320.0,
            depth: 310.0,
            height: 115.0,
            model_type: "builtin".into(),
            builtin_id: Some("turntable-trainhall".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let m_acc = build_scene_mesh_3d(&make_layout(vec![turnout], Some(vec![shed, hall])), &catalogs, &opts);
        assert!(m_acc.vertices.len() > m_plain.vertices.len() + 200);
        assert!(m_acc.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
    }

    #[test]
    fn l_shape_table_with_cutout_and_upper_layer_plate_generates_3d_mesh() {
        let catalogs = builtin_catalogs();
        let straight = find_entry(&catalogs, "24360").unwrap();
        let mut ramp_piece = create_piece("r1".into(), straight, Transform { x: -400.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
        ramp_piece.elevation = Some(railbench_core::types::Elevation { start: -45.0, end: 25.0 });

        let l_board = BaseboardConfig {
            id: Some("board-l".into()),
            name: Some("L-Tisch".into()),
            enabled: true,
            shape: Some("l-shape".into()),
            custom_polygon: None,
            triangle_type: None,
            width: 2400.0,
            depth: 1600.0,
            rotation: Some(0.0),
            l_wing_width: Some(800.0),
            l_wing_depth: Some(800.0),
            l_side: Some("left".into()),
            u_left_wing_width: None,
            u_right_wing_width: None,
            u_back_depth: None,
            t_bar_depth: None,
            t_stem_width: None,
            position_mode: "center".into(),
            custom_x: None,
            custom_y: None,
            style: "plywood".into(),
            corner_radius: Some(10.0),
            show_dimensions: Some(true),
        };
        let layer1 = railbench_core::types::Layer {
            id: "layer-1".into(),
            name: "Ebene 0".into(),
            color: Some("#38bdf8".into()),
            visible: true,
            locked: false,
            height_mm: Some(0.0),
            wood_plate_mode: Some("table".into()),
            wood_plate_thickness_mm: Some(20.0),
            wood_plate_style: None,
            plates: None,
        };
        let layer2 = railbench_core::types::Layer {
            id: "layer-2".into(),
            name: "Ebene 1".into(),
            color: Some("#f59e0b".into()),
            visible: true,
            locked: false,
            height_mm: Some(90.0),
            wood_plate_mode: Some("table".into()),
            wood_plate_thickness_mm: Some(12.0),
            wood_plate_style: Some("wood".into()),
            plates: Some(vec![railbench_core::types::LayerPlate {
                id: "plate-1".into(),
                name: Some("Oberplatte".into()),
                baseboard_id: Some("board-l".into()),
                shape: Some("rect".into()),
                custom_polygon: None,
                triangle_type: None,
                l_wing_width: None,
                l_wing_depth: None,
                l_side: None,
                u_left_wing_width: None,
                u_right_wing_width: None,
                u_back_depth: None,
                t_bar_depth: None,
                t_stem_width: None,
                rotation: Some(0.0),
                width: 900.0,
                depth: 500.0,
                x: Some(200.0),
                y: Some(200.0),
                thickness_mm: Some(12.0),
                position_mode: Some("custom".into()),
                style: Some("wood".into()),
            }]),
        };

        let layout = Layout {
            name: "LPlate3D".into(),
            scale: "H0".into(),
            pieces: vec![ramp_piece],
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: Some(vec![layer1, layer2]),
            active_layer_id: Some("layer-2".into()),
            baseboard: Some(l_board.clone()),
            baseboards: Some(vec![l_board]),
        };

        let mesh_no_plates = build_scene_mesh_3d(
            &layout,
            &catalogs,
            &Scene3DOptions { show_wood_plates: false, env_mode: Viewer3DEnv::Minimal, ..Default::default() },
        );
        let mesh_with_plates = build_scene_mesh_3d(
            &layout,
            &catalogs,
            &Scene3DOptions { show_wood_plates: true, env_mode: Viewer3DEnv::Minimal, ..Default::default() },
        );
        assert!(mesh_with_plates.vertices.len() > mesh_no_plates.vertices.len());
        assert!(mesh_with_plates.vertices.iter().any(|v| (v.pos[1] - 90.0).abs() < 0.5));
        assert!(mesh_with_plates.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
    }

    #[test]
    fn new_builtin_models_custom_glb_and_hall_env_generate_3d_mesh() {
        let catalogs = builtin_catalogs();
        // Build minimal GLB custom mesh
        let mut bin = Vec::new();
        for c in [0.0_f32, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 10.0] {
            bin.extend_from_slice(&c.to_le_bytes());
        }
        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": bin.len() }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": bin.len() }],
            "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
            "nodes": [{ "mesh": 0 }],
            "scenes": [{ "nodes": [0] }],
            "scene": 0
        }).to_string();
        let mut jb = json.into_bytes();
        while jb.len() % 4 != 0 { jb.push(b' '); }
        let total = (12 + 8 + jb.len() + 8 + bin.len()) as u32;
        let mut glb = Vec::new();
        glb.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
        glb.extend_from_slice(&2_u32.to_le_bytes());
        glb.extend_from_slice(&total.to_le_bytes());
        glb.extend_from_slice(&(jb.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
        glb.extend_from_slice(&jb);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
        glb.extend_from_slice(&bin);

        let mk = |id: &str, bid: &str, x: f64| Model3D {
            id: id.into(),
            name: bid.into(),
            category: "railway".into(),
            transform: Transform { x, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 120.0,
            depth: 80.0,
            height: 110.0,
            model_type: "builtin".into(),
            builtin_id: Some(bid.into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let custom = Model3D {
            id: "c1".into(),
            name: "CustomGLB".into(),
            category: "custom".into(),
            transform: Transform { x: 500.0, y: 100.0, rotation: 45.0 },
            elevation: Some(0.0),
            scale: Some(2.0),
            layer_id: None,
            width: 20.0,
            depth: 20.0,
            height: 20.0,
            model_type: "glb".into(),
            builtin_id: None,
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: Some(format!("data:model/gltf-binary;base64,{}", crate::model_tools::base64_encode(&glb))),
            file_name: Some("custom.glb".into()),
        };

        let models = vec![
            mk("m1", "coaling-station", -400.0),
            mk("m2", "diesel-fuel-station", -200.0),
            mk("m3", "buffer-stop", -50.0),
            mk("m4", "platform", 100.0),
            mk("m5", "skyscraper", 260.0),
            mk("m6", "industry-building", 420.0),
            custom,
        ];
        let layout = Layout {
            name: "Group4Test".into(),
            scale: "H0".into(),
            pieces: vec![],
            models: Some(models),
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
        };

        let minimal = build_scene_mesh_3d(&layout, &catalogs, &Scene3DOptions { env_mode: Viewer3DEnv::Minimal, ..Default::default() });
        let hall = build_scene_mesh_3d(&layout, &catalogs, &Scene3DOptions { env_mode: Viewer3DEnv::Hall, short_circuit_points: vec![Vec2 { x: 10.0, y: 10.0 }], ..Default::default() });
        let land = build_scene_mesh_3d(&layout, &catalogs, &Scene3DOptions { env_mode: Viewer3DEnv::Landscape, ..Default::default() });
        assert!(minimal.vertices.len() > 200);
        assert!(hall.vertices.len() > minimal.vertices.len());
        assert!(land.vertices.len() > minimal.vertices.len());
    }

    #[test]
    fn standalone_tunneled_piece_generates_3d_tube_cover_and_portals() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24360").unwrap();
        let mut t0 = create_piece("t0".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        t0.has_tunnel = Some(true);
        let t_plain = create_piece("t1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);

        let mk_layout = |pieces| Layout {
            name: "StandaloneTunnel".into(),
            scale: "H0".into(),
            pieces,
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
        };

        let plain_mesh = build_scene_mesh_3d(&mk_layout(vec![t_plain]), &catalogs, &Scene3DOptions::default());
        let tunnel_mesh = build_scene_mesh_3d(&mk_layout(vec![t0]), &catalogs, &Scene3DOptions::default());
        assert!(tunnel_mesh.vertices.len() > plain_mesh.vertices.len() + 200);
        assert!(tunnel_mesh.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
    }

    #[test]
    fn ground_terrain_brushed_water_and_sand_generate_surface_and_clip() {
        let catalogs = builtin_catalogs();
        let water = Model3D {
            id: "gt-water".into(),
            name: "River".into(),
            category: "terrain".into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 500.0,
            depth: 260.0,
            height: 6.0,
            model_type: "builtin".into(),
            builtin_id: Some("ground-terrain".into()),
            mountain_style: None,
            mountain_path: Some(vec![
                railbench_core::types::MountainBrushNode { x: -120.0, y: 0.0, radius: 110.0, height: 6.0 },
                railbench_core::types::MountainBrushNode { x: 120.0, y: 0.0, radius: 110.0, height: 6.0 },
            ]),
            ground_terrain_type: Some("water".into()),
            file_data: None,
            file_name: None,
        };
        let sand = Model3D {
            id: "gt-sand".into(),
            name: "Sand".into(),
            category: "terrain".into(),
            transform: Transform { x: 400.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 300.0,
            depth: 240.0,
            height: 12.0,
            model_type: "builtin".into(),
            builtin_id: Some("ground-terrain".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: Some("sand".into()),
            file_data: None,
            file_name: None,
        };
        let bb = BaseboardConfig {
            id: Some("b1".into()),
            name: Some("B1".into()),
            enabled: true,
            shape: Some("rect".into()),
            custom_polygon: None,
            triangle_type: None,
            width: 2400.0,
            depth: 1200.0,
            rotation: Some(0.0),
            l_wing_width: None,
            l_wing_depth: None,
            l_side: None,
            u_left_wing_width: None,
            u_right_wing_width: None,
            u_back_depth: None,
            t_bar_depth: None,
            t_stem_width: None,
            position_mode: "center".into(),
            custom_x: None,
            custom_y: None,
            style: "wood".into(),
            corner_radius: Some(12.0),
            show_dimensions: Some(true),
        };
        let layout = Layout {
            name: "GroundTest".into(),
            scale: "H0".into(),
            pieces: vec![],
            models: Some(vec![water, sand]),
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: Some(bb.clone()),
            baseboards: Some(vec![bb]),
        };

        let mesh = build_scene_mesh_3d(&layout, &catalogs, &Scene3DOptions::default());
        assert!(mesh.vertices.len() > 2500);
        // Must contain reflective blue water surface vertices at ~1.85 mm height
        assert!(mesh.vertices.iter().any(|v| (v.pos[1] - 1.85).abs() < 0.1 && v.color[2] > 0.7));
        // Must contain warm sand vertices
        assert!(mesh.vertices.iter().any(|v| v.color[0] > 0.78 && v.color[1] > 0.65 && v.color[2] < 0.60));
    }
}
