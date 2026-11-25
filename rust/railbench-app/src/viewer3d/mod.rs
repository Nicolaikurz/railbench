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
    @location(1) normal: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) mat: vec4<f32>,
    @location(4) inst_pos_rot: vec4<f32>,
    @location(5) inst_tint_w: vec4<f32>,
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
    let c = cos(in.inst_pos_rot.w);
    let s = sin(in.inst_pos_rot.w);
    let rx = in.pos.x * c - in.pos.z * s;
    let rz = in.pos.x * s + in.pos.z * c;
    let wp = vec3<f32>(
        in.inst_pos_rot.x + rx,
        in.inst_pos_rot.y + in.pos.y,
        in.inst_pos_rot.z + rz,
    );
    let nx = in.normal.x * c - in.normal.z * s;
    let nz = in.normal.x * s + in.normal.z * c;
    out.clip_pos = u.view_proj * vec4<f32>(wp, 1.0);
    out.world_pos = wp;
    out.normal = vec3<f32>(nx, in.normal.y, nz);
    // Unpack emissive (in.mat.z stored scaled by 0.5 in Snorm8x4 to cover [-2.0, +2.0])
    let mat_xyz = vec3<f32>(in.mat.x, in.mat.y, in.mat.z * 2.0);
    // Only tint non-metallic ballast vertices (MAT_BALLAST = roughness ~0.96) when inst_tint_w.w > 0
    let apply_tint = select(0.0, in.inst_tint_w.w, mat_xyz.y < 0.25 && mat_xyz.x > 0.90 && in.pos.y < 6.0);
    out.color = mix(in.color.rgb, in.inst_tint_w.rgb, apply_tint);
    out.mat = mat_xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var n = normalize(in.normal);
    let l = normalize(u.sun_dir.xyz);
    let v_vec = u.cam_pos.xyz - in.world_pos;
    let dist = length(v_vec);
    let v = v_vec / max(dist, 0.001);
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
    } else if (rough > 0.935 && metal < 0.1) {
        // Procedural multi-octave crushed granite/basalt gravel ballast texture & micro-normal relief
        let gravel_fade = 1.0 - smoothstep(380.0, 1450.0, dist);
        let gp1 = in.world_pos * 1.45;
        let gp2 = in.world_pos * 3.10;
        let gp3 = in.world_pos * 5.80;
        let n1 = sin(gp1.x * 1.7 + cos(gp1.z * 1.9)) * cos(gp1.z * 1.6 - sin(gp1.x * 1.4));
        let n2 = sin(gp2.x * 2.3 - gp2.z * 1.8 + sin(gp2.y * 2.1)) * cos(gp2.z * 2.5 + gp2.x * 1.3);
        let n3 = sin(gp3.x * 2.7 + gp3.z * 2.9) * cos(gp3.x * 3.1 - gp3.z * 2.4);
        let pebble = (n1 * 0.45 + n2 * 0.35 + n3 * 0.20) * gravel_fade;
        let dark_pore = smoothstep(0.28, 0.75, -n2 * n3) * gravel_fade;
        // Varied mineral aggregate hues (warm granite brown, dark basalt, light quartz fleck)
        let mineral_warm = vec3<f32>(0.025, 0.014, -0.012) * n1 * gravel_fade;
        base_col = clamp(
            base_col * (1.0 + pebble * 0.19 - dark_pore * 0.16) + mineral_warm,
            vec3<f32>(0.04),
            vec3<f32>(0.96),
        );
        n = normalize(n + vec3<f32>(n2 * 0.18, 0.0, n3 * 0.18) * gravel_fade);
    } else if (rough > 0.52) {
        let detail_fade = 1.0 - smoothstep(260.0, 950.0, dist);
        let p = in.world_pos * 0.14;
        let grain = sin(p.x * 2.1 + sin(p.z * 1.9)) * cos(p.y * 2.7 + p.z * 1.5);
        base_col = clamp(base_col * (1.0 + grain * 0.016 * detail_fade), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    let h = normalize(l + v);

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
                255,
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
    pub short_circuit_points: Vec<Vec2>,
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

    if env == Viewer3DEnv::Minimal {
        let dusk_warmth =
            (1.0 - ((h - 6.0).abs().min((h - 18.5).abs()) / 2.2).clamp(0.0, 1.0)) * 0.35;
        let day_clear = if dark_mode {
            [0.014_f32, 0.022, 0.052]
        } else {
            [0.76_f32, 0.73, 0.67]
        };
        let night_clear = [0.005_f32, 0.009, 0.022];
        let clear_lin = lerp3(lerp3(day_clear, night_clear, night_factor), sky_lin, dusk_warmth);
        let fog_day = if dark_mode {
            [0.06_f32, 0.09, 0.16]
        } else {
            [0.85_f32, 0.83, 0.79]
        };
        let fog_night = [0.03_f32, 0.05, 0.10];
        let fog_rgb = lerp3(lerp3(fog_day, fog_night, night_factor), fog_srgb, dusk_warmth);
        let fog_a = if dark_mode { 0.22 } else { 0.16 + 0.08 * night_factor };
        return SkyUniformState {
            clear_color: [clear_lin[0] as f64, clear_lin[1] as f64, clear_lin[2] as f64],
            fog_color: [fog_rgb[0], fog_rgb[1], fog_rgb[2], fog_a],
            sun_dir,
            sun_color_int: [sun_c[0], sun_c[1], sun_c[2], sun_int.max(0.40)],
            hemi_sky_int: [hemi_sky[0], hemi_sky[1], hemi_sky[2], hemi_int.max(0.38)],
            hemi_ground_pad: [hemi_gnd[0], hemi_gnd[1], hemi_gnd[2], night_factor],
        };
    }

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

#[derive(Clone, Default)]
pub struct SceneMesh3D {
    pub(crate) vertices: Vec<Vertex3D>,
    indices: Vec<u32>,
    pub instanced_tracks: Vec<InstancedTrackPrototype3D>,
}

impl SceneMesh3D {
    pub(crate) fn from_parts(vertices: Vec<Vertex3D>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            indices,
            instanced_tracks: Vec::new(),
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
            + self
                .instanced_tracks
                .iter()
                .map(|p| p.vertices.len())
                .sum::<usize>()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
            + self
                .instanced_tracks
                .iter()
                .map(|p| (p.indices.len() / 3) * p.instances.len())
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
            + self.indices.len() * std::mem::size_of::<u32>();
        let proto_bytes: usize = self
            .instanced_tracks
            .iter()
            .map(|p| {
                p.vertices.len() * std::mem::size_of::<Vertex3D>()
                    + p.indices.len() * std::mem::size_of::<u32>()
                    + p.instances.len() * std::mem::size_of::<Instance3D>()
            })
            .sum();
        base_bytes + proto_bytes
    }

    pub fn append_from(&mut self, other: &SceneMesh3D) {
        if !other.vertices.is_empty() {
            let base = self.vertices.len() as u32;
            self.vertices.extend_from_slice(&other.vertices);
            self.indices.reserve(other.indices.len());
            for &idx in &other.indices {
                self.indices.push(base + idx);
            }
        }
        if !other.instanced_tracks.is_empty() {
            self.instanced_tracks.extend_from_slice(&other.instanced_tracks);
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

#[derive(Clone, Default)]
pub struct Scene3DBundle {
    pub env: Option<(u64, Arc<SceneMesh3D>)>,
    pub boards: Option<(u64, Arc<SceneMesh3D>)>,
    pub terrain: Option<(u64, Arc<SceneMesh3D>)>,
    pub tracks: Option<(u64, Arc<SceneMesh3D>)>,
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

const MSAA_SAMPLE_COUNT: u32 = 4;
pub const WINDOW_MSAA_SAMPLES: u32 = 4;

mod gpu;
#[allow(unused_imports)]
use gpu::*;
#[allow(unused_imports)]
pub use gpu::{init_wgpu_3d_resources, release_wgpu_3d_resources};
mod viewport;
#[allow(unused_imports)]
use viewport::*;
#[allow(unused_imports)]
pub use viewport::{draw_viewport_3d};
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
mod terrain_and_models;
#[allow(unused_imports)]
use terrain_and_models::*;
#[allow(unused_imports)]
pub(crate) use terrain_and_models::{rot_xz};
mod primitives;
#[allow(unused_imports)]
use primitives::*;
#[allow(unused_imports)]
pub(crate) use primitives::{polyline_miter_normal, push_box_polyline_mat_3d, push_quad_mat, push_tri, dot3, cross3, normalize3};
#[cfg(test)]
mod tests;
