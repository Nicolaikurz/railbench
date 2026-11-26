pub(super) const SCENE_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_view_proj: mat4x4<f32>,
    sun_dir: vec4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    sun_color_int: vec4<f32>,
    hemi_sky_int: vec4<f32>,
    hemi_ground_pad: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    cam_fwd: vec4<f32>,
    sky_zenith: vec4<f32>,
    shadow_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var t_shadow: texture_depth_2d;
@group(0) @binding(2)
var s_shadow: sampler_comparison;

fn sample_shadow_pcf(world_pos: vec3<f32>, normal: vec3<f32>, l: vec3<f32>) -> f32 {
    if (u.shadow_params.x <= 0.001) {
        return 1.0;
    }
    let ndotl = max(dot(normal, l), 0.0);
    let bias_mm = u.shadow_params.z * (1.0 - 0.65 * ndotl);
    let biased_pos = world_pos + normal * bias_mm;
    let light_clip = u.sun_view_proj * vec4<f32>(biased_pos, 1.0);
    let proj = light_clip.xyz / max(light_clip.w, 0.0001);
    let uv = vec2<f32>(proj.x * 0.5 + 0.5, 0.5 - proj.y * 0.5);
    let depth_ref = clamp(proj.z - u.shadow_params.w, 0.0, 1.0);
    let texel = u.shadow_params.y;

    var sum = 0.0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let suv = clamp(uv + vec2<f32>(f32(dx), f32(dy)) * texel, vec2<f32>(0.001), vec2<f32>(0.999));
            sum = sum + textureSampleCompare(t_shadow, s_shadow, suv, depth_ref);
        }
    }
    let pcf = sum / 9.0;
    let edge = max(abs(proj.x), abs(proj.y));
    let in_bounds = select(0.0, 1.0, proj.z >= 0.0 && proj.z <= 1.0);
    let fade = 1.0 - smoothstep(0.88, 1.0, edge);
    return mix(1.0, mix(1.0 - u.shadow_params.x, 1.0, pcf), fade * in_bounds);
}

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
    @location(2) color: vec4<f32>,
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
    out.color = vec4<f32>(mix(in.color.rgb, in.inst_tint_w.rgb, apply_tint), in.color.a);
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

    var base_col = in.color.rgb;
    let base_alpha = in.color.a;
    let is_glass = base_alpha < 0.99;
    var rough = clamp(in.mat.x, 0.04, 1.0);
    let metal = clamp(in.mat.y, 0.0, 1.0);
    var emissive_col = vec3<f32>(0.0);

    if (is_glass) {
        if (dot(n, v) < 0.0) {
            n = -n;
        }
        rough = 0.08;
    } else if (in.mat.z < -0.001) {
        let glow_t = smoothstep(0.08, 0.82, night);
        let glass_day = vec3<f32>(0.10, 0.19, 0.30);
        base_col = mix(glass_day, in.color.rgb, glow_t * 0.92);
        emissive_col = in.color.rgb * (-in.mat.z) * (0.03 + 1.15 * glow_t);
        rough = 0.12;
    } else if (in.mat.z > 0.001) {
        emissive_col = in.color.rgb * in.mat.z * (0.55 + 0.90 * night);
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
    let raw_shadow = sample_shadow_pcf(in.world_pos, n, l);
    let shadow = select(raw_shadow, 1.0, is_glass);
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
    let spec_scale = select(1.0, 1.8, is_glass);
    var lit = diffuse_col * (ambient + u.sun_color_int.rgb * (diff * shadow) + u.hemi_sky_int.rgb * fill_diff)
            + u.sun_color_int.rgb * (direct_spec * spec_scale * shadow)
            + env_spec * spec_scale
            + emissive_col;
    let excess = max(lit - vec3<f32>(0.88), vec3<f32>(0.0));
    lit = min(lit, vec3<f32>(0.88)) + excess / (vec3<f32>(1.0) + excess * 1.6);

    var fog_factor: f32;
    if (u.cam_fwd.w < 0.5) {
        let haze = smoothstep(25000.0, 160000.0, dist) * 0.34;
        let horizon_blend = smoothstep(140000.0, 290000.0, dist);
        fog_factor = clamp(max(haze, horizon_blend) * u.fog_color.w, 0.0, 1.0);
    } else {
        fog_factor = clamp((dist - 12000.0) / 45000.0, 0.0, u.fog_color.w);
    }
    let final_col = mix(lit, u.fog_color.rgb, fog_factor);
    if (is_glass) {
        let glass_fresnel = 0.08 + 0.58 * pow(clamp(1.0 - ndotv, 0.0, 1.0), 3.0);
        let out_alpha = clamp(base_alpha + glass_fresnel * 0.40, 0.14, 0.78);
        return vec4<f32>(final_col, out_alpha);
    }
    return vec4<f32>(final_col, 1.0);
}
"#;

pub(super) const SHADOW_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_view_proj: mat4x4<f32>,
    sun_dir: vec4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    sun_color_int: vec4<f32>,
    hemi_sky_int: vec4<f32>,
    hemi_ground_pad: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    cam_fwd: vec4<f32>,
    sky_zenith: vec4<f32>,
    shadow_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

@vertex
fn vs_shadow(
    @location(0) pos: vec3<f32>,
    @location(4) inst_pos_rot: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let c = cos(inst_pos_rot.w);
    let s = sin(inst_pos_rot.w);
    let rx = pos.x * c - pos.z * s;
    let rz = pos.x * s + pos.z * c;
    let wp = vec3<f32>(
        inst_pos_rot.x + rx,
        inst_pos_rot.y + pos.y,
        inst_pos_rot.z + rz,
    );
    return u.sun_view_proj * vec4<f32>(wp, 1.0);
}

@vertex
fn vs_shadow_tex(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.sun_view_proj * vec4<f32>(pos, 1.0);
}
"#;

pub(super) const TEXTURED_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_view_proj: mat4x4<f32>,
    sun_dir: vec4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    sun_color_int: vec4<f32>,
    hemi_sky_int: vec4<f32>,
    hemi_ground_pad: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    cam_fwd: vec4<f32>,
    sky_zenith: vec4<f32>,
    shadow_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var t_shadow: texture_depth_2d;
@group(0) @binding(2)
var s_shadow: sampler_comparison;

@group(1) @binding(0)
var t_albedo: texture_2d<f32>;
@group(1) @binding(1)
var s_albedo: sampler;

fn sample_shadow_pcf(world_pos: vec3<f32>, normal: vec3<f32>, l: vec3<f32>) -> f32 {
    if (u.shadow_params.x <= 0.001) {
        return 1.0;
    }
    let ndotl = max(dot(normal, l), 0.0);
    let bias_mm = u.shadow_params.z * (1.0 - 0.65 * ndotl);
    let biased_pos = world_pos + normal * bias_mm;
    let light_clip = u.sun_view_proj * vec4<f32>(biased_pos, 1.0);
    let proj = light_clip.xyz / max(light_clip.w, 0.0001);
    let uv = vec2<f32>(proj.x * 0.5 + 0.5, 0.5 - proj.y * 0.5);
    let depth_ref = clamp(proj.z - u.shadow_params.w, 0.0, 1.0);
    let texel = u.shadow_params.y;

    var sum = 0.0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let suv = clamp(uv + vec2<f32>(f32(dx), f32(dy)) * texel, vec2<f32>(0.001), vec2<f32>(0.999));
            sum = sum + textureSampleCompare(t_shadow, s_shadow, suv, depth_ref);
        }
    }
    let pcf = sum / 9.0;
    let edge = max(abs(proj.x), abs(proj.y));
    let in_bounds = select(0.0, 1.0, proj.z >= 0.0 && proj.z <= 1.0);
    let fade = 1.0 - smoothstep(0.88, 1.0, edge);
    return mix(1.0, mix(1.0 - u.shadow_params.x, 1.0, pcf), fade * in_bounds);
}

struct TexVertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) mat: vec4<f32>,
};

struct TexVertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) mat: vec2<f32>,
};

@vertex
fn vs_tex(in: TexVertexIn) -> TexVertexOut {
    var out: TexVertexOut;
    out.clip_pos = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.world_pos = in.pos;
    out.normal = in.normal;
    out.uv = in.uv;
    out.color = in.color;
    out.mat = in.mat.xy;
    return out;
}

@fragment
fn fs_tex(in: TexVertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let l = normalize(u.sun_dir.xyz);
    let v_vec = u.cam_pos.xyz - in.world_pos;
    let dist = length(v_vec);
    let v = v_vec / max(dist, 0.001);

    // Sample textures strictly before any discard so control flow stays uniform for derivatives.
    let tex_col = textureSample(t_albedo, s_albedo, in.uv);
    let shadow = sample_shadow_pcf(in.world_pos, n, l);
    if (tex_col.a * in.color.a < 0.25) {
        discard;
    }

    let base_col = tex_col.rgb * in.color.rgb;
    let rough = clamp(in.mat.x, 0.04, 1.0);
    let metal = clamp(in.mat.y, 0.0, 1.0);
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
    var lit = diffuse_col * (ambient + u.sun_color_int.rgb * (diff * shadow) + u.hemi_sky_int.rgb * fill_diff)
            + u.sun_color_int.rgb * (direct_spec * shadow)
            + env_spec;
    let excess = max(lit - vec3<f32>(0.88), vec3<f32>(0.0));
    lit = min(lit, vec3<f32>(0.88)) + excess / (vec3<f32>(1.0) + excess * 1.6);

    var fog_factor: f32;
    if (u.cam_fwd.w < 0.5) {
        let haze = smoothstep(25000.0, 160000.0, dist) * 0.34;
        let horizon_blend = smoothstep(140000.0, 290000.0, dist);
        fog_factor = clamp(max(haze, horizon_blend) * u.fog_color.w, 0.0, 1.0);
    } else {
        fog_factor = clamp((dist - 12000.0) / 45000.0, 0.0, u.fog_color.w);
    }
    let final_col = mix(lit, u.fog_color.rgb, fog_factor);
    return vec4<f32>(final_col, 1.0);
}
"#;

pub(super) const SKY_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_view_proj: mat4x4<f32>,
    sun_dir: vec4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    sun_color_int: vec4<f32>,
    hemi_sky_int: vec4<f32>,
    hemi_ground_pad: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    cam_fwd: vec4<f32>,
    sky_zenith: vec4<f32>,
    shadow_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct SkyOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) idx: u32) -> SkyOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    let p = positions[idx];
    var out: SkyOut;
    out.pos = vec4<f32>(p, 0.9999, 1.0);
    out.ndc = p;
    return out;
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4<f32> {
    let aspect = u.cam_right.w;
    let tan_half_y = u.cam_up.w;
    let dir = normalize(
        u.cam_fwd.xyz
        + u.cam_right.xyz * (in.ndc.x * aspect * tan_half_y)
        + u.cam_up.xyz * (in.ndc.y * tan_half_y)
    );

    if (u.cam_fwd.w >= 0.5) {
        let screen_y = clamp(in.ndc.y * 0.5 + 0.5, 0.0, 1.0);
        let t_grad = smoothstep(0.0, 1.0, screen_y);
        let grad = mix(u.fog_color.rgb, u.sky_zenith.rgb, t_grad);
        let vig = 1.0 - 0.05 * clamp(dot(in.ndc, in.ndc) * 0.5, 0.0, 1.0);
        return vec4<f32>(grad * vig, 1.0);
    }

    let elev = clamp(dir.y, 0.0, 1.0);
    let horizon_mix = pow(1.0 - elev, 3.2);
    let zenith_mix = smoothstep(0.06, 0.72, elev);
    let mid_sky = mix(u.fog_color.rgb, u.sky_zenith.rgb * 1.25, 0.52);
    var col = mix(mid_sky, u.sky_zenith.rgb, zenith_mix);
    col = mix(col, u.fog_color.rgb, horizon_mix);

    let night = clamp(u.sky_zenith.w, 0.0, 1.0);
    let l_dir = normalize(u.sun_dir.xyz);
    let sun_dot = max(dot(dir, l_dir), 0.0);

    if (dir.y > -0.05) {
        let glow_col = mix(u.sun_color_int.rgb, vec3<f32>(0.75, 0.86, 1.0), night);
        let wide_halo = pow(sun_dot, 8.0) * mix(0.28, 0.08, night) * smoothstep(-0.04, 0.25, dir.y);
        let disc = smoothstep(0.99935, 0.99985, sun_dot) * mix(1.4, 0.65, night);
        col = col + glow_col * (wide_halo + disc);
    }

    if (night > 0.15 && dir.y > 0.02) {
        let star_cell = floor(dir * 260.0);
        let h = fract(sin(dot(star_cell, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
        let star = step(0.9965, h) * smoothstep(0.02, 0.28, dir.y) * smoothstep(0.15, 0.75, night);
        col = col + vec3<f32>(0.85, 0.90, 1.0) * star * 0.75;
    }

    return vec4<f32>(col, 1.0);
}
"#;

pub(super) const BLIT_SHADER_WGSL: &str = r#"
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

