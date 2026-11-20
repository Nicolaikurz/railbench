use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke};
use eframe::egui_wgpu::{self, wgpu};
use railbench_core::geometry::{
    build_tunnel_sweep_positions, rotate, sample_arc_between_connectors, sample_flex_spline_points, smoothstep,
};
use railbench_core::terrain::{
    carve_ground_terrain_grid, evaluate_mountain_heightfield, evaluate_mountain_track_samples, mountain_noise,
};
use railbench_core::types::{BaseboardConfig, Catalog, CatalogEntry, Layout, Model3D, TrackPiece, Vec2};

use crate::catalog::{find_entry, is_road_id};

// ============================================================================
//  WGSL Shaders: 3D Lit Scene (with Depth Buffer + Fog) & Fullscreen Blit
// ============================================================================

const SCENE_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_dir: vec4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip_pos = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.world_pos = in.pos;
    out.normal = in.normal;
    out.color = in.color;
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

    // Key directional sun + subtle cool fill from opposite side
    let diff = max(dot(n, l), 0.0);
    let fill_l = normalize(vec3<f32>(-u.sun_dir.x, 0.45, -u.sun_dir.z));
    let fill_diff = max(dot(n, fill_l), 0.0) * 0.22;

    // Hemisphere sky/ground ambient
    let hemi = n.y * 0.5 + 0.5;
    let ambient = mix(vec3<f32>(0.24, 0.25, 0.28), vec3<f32>(0.48, 0.52, 0.58), hemi);
    let spec = pow(max(dot(n, h), 0.0), 42.0) * 0.16;

    let lit = in.color * (ambient + vec3<f32>(0.82, 0.79, 0.73) * diff + vec3<f32>(0.55, 0.65, 0.80) * fill_diff)
            + vec3<f32>(spec);

    // Soft atmospheric distance fog
    let fog_factor = clamp((dist - 5500.0) / 16000.0, 0.0, 0.65);
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

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex3D {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms3D {
    view_proj: [[f32; 4]; 4],
    sun_dir: [f32; 4],
    cam_pos: [f32; 4],
    fog_color: [f32; 4],
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
        let mut boards = Vec::new();
        if let Some(list) = &layout.baseboards {
            boards.extend(list.iter().filter(|b| b.enabled));
        } else if let Some(b) = &layout.baseboard {
            if b.enabled {
                boards.push(b);
            }
        }
        for b in boards {
            let (x0, y0) = baseboard_origin(b);
            min_x = min_x.min(x0 as f32);
            max_x = max_x.max((x0 + b.width) as f32);
            min_z = min_z.min(y0 as f32);
            max_z = max_z.max((y0 + b.depth) as f32);
        }

        if !min_x.is_finite() {
            *self = Self::default();
            return;
        }
        self.target = [(min_x + max_x) * 0.5, 25.0, (min_z + max_z) * 0.5];
        let span = (max_x - min_x).max(max_z - min_z).max(600.0);
        self.distance = (span * 1.15).clamp(400.0, 18000.0);
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

pub fn draw_viewport_3d(
    ui: &mut egui::Ui,
    camera: &mut Camera3D,
    cached_mesh: &Option<Arc<SceneMesh3D>>,
    mesh_version: u64,
    dark_mode: bool,
) {
    let avail = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

    // Orbit (left drag)
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        camera.yaw -= d.x * 0.0065;
        camera.pitch = (camera.pitch + d.y * 0.0065).clamp(0.06, 1.50);
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
            camera.distance = (camera.distance * factor).clamp(100.0, 25000.0);
        }
    }

    let ppp = ui.ctx().pixels_per_point();
    let px_w = ((rect.width() * ppp).round() as u32).max(16);
    let px_h = ((rect.height() * ppp).round() as u32).max(16);
    let aspect = rect.width() / rect.height().max(1.0);
    let eye = camera.eye();

    // Linear clear color + fog color matching TS Three.js 3D studio sky
    let (clear_color, fog_color) = if dark_mode {
        ([0.014, 0.024, 0.055], [0.06, 0.09, 0.16, 1.0])
    } else {
        ([0.52, 0.68, 0.84], [0.74, 0.84, 0.93, 1.0])
    };

    let uniforms = Uniforms3D {
        view_proj: camera.view_proj(aspect),
        sun_dir: [0.55, 0.85, 0.45, 0.0],
        cam_pos: [eye[0], eye[1], eye[2], 1.0],
        fog_color,
    };

    let cb = egui_wgpu::Callback::new_paint_callback(
        rect,
        Viewport3DCallback {
            uniforms,
            mesh: cached_mesh.clone(),
            mesh_version,
            viewport_px: [px_w, px_h],
            clear_color,
        },
    );
    ui.painter().add(cb);

    // Floating Camera Preset HUD (matches TS 3D camera controls)
    egui::Area::new(egui::Id::new("hud_3d_presets"))
        .fixed_pos(rect.min + egui::vec2(14.0, 12.0))
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 210))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(8, 5))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 65)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("3D").strong().size(11.5).color(Color32::from_rgb(56, 189, 248)));
                        if ui.small_button("Iso").clicked() {
                            camera.yaw = 0.62;
                            camera.pitch = 0.52;
                        }
                        if ui.small_button("Top").clicked() {
                            camera.pitch = 1.48;
                        }
                        if ui.small_button("Front").clicked() {
                            camera.yaw = 0.0;
                            camera.pitch = 0.22;
                        }
                        if ui.small_button("Side").clicked() {
                            camera.yaw = std::f32::consts::FRAC_PI_2;
                            camera.pitch = 0.22;
                        }
                        ui.label(
                            RichText::new("LMB: Orbit • RMB: Pan • Wheel: Zoom")
                                .size(10.5)
                                .color(Color32::from_rgb(148, 163, 184)),
                        );
                    });
                });
        });
}

// ============================================================================
//  Batched 3D Scene Mesh Builder (Baseboard + Detailed Tracks + Models + Terrain)
// ============================================================================

fn baseboard_origin(b: &railbench_core::types::BaseboardConfig) -> (f64, f64) {
    match b.position_mode.as_str() {
        "center" => (-b.width * 0.5, -b.depth * 0.5),
        "top-left" => (0.0, 0.0),
        _ => (b.custom_x.unwrap_or(0.0), b.custom_y.unwrap_or(0.0)),
    }
}

struct CorridorBranch {
    pts_xze: Vec<[f32; 3]>, // [world_x, world_z, world_elev]
    half_w: f32,
    is_road: bool,
    has_catenary: bool,
    gauge: f32,
    is_turntable: bool,
}

fn collect_visible_corridors(
    layout: &Layout,
    catalogs: &[Catalog],
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
) -> Vec<CorridorBranch> {
    let mut out = Vec::new();
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
        let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
        let half_w = if is_road {
            entry.and_then(|e| e.road_width).unwrap_or(60.0) as f32 * 0.5
        } else {
            (gauge / 16.5) * 20.0
        };
        let has_catenary = piece.has_catenary == Some(true);
        let is_turntable = piece.connectors.len() >= 6;

        for branch in sample_track_branches_3d(piece, e0, e1) {
            if branch.len() < 2 {
                continue;
            }
            // Subdivide dense points every ~15 mm for accurate tunnel portal intersection & terrain carving
            let mut dense = Vec::new();
            for (si, w) in branch.windows(2).enumerate() {
                let a = w[0];
                let b = w[1];
                let seg_len = (b[0] - a[0]).hypot(b[2] - a[2]);
                let sub = ((seg_len / 15.0).ceil() as usize).max(1);
                let start_k = if si == 0 { 0 } else { 1 };
                for k in start_k..=sub {
                    let u = k as f32 / sub as f32;
                    dense.push([
                        a[0] + (b[0] - a[0]) * u,
                        a[2] + (b[2] - a[2]) * u,
                        a[1] + (b[1] - a[1]) * u,
                    ]);
                }
            }
            out.push(CorridorBranch {
                pts_xze: dense,
                half_w,
                is_road,
                has_catenary,
                gauge,
                is_turntable,
            });
        }
    }
    out
}

pub fn build_scene_mesh_3d(layout: &Layout, catalogs: &[Catalog]) -> SceneMesh3D {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    // 1. Baseboards with wood skirt & 4 legs
    let mut boards: Vec<&BaseboardConfig> = Vec::new();
    if let Some(list) = &layout.baseboards {
        boards.extend(list.iter().filter(|b| b.enabled));
    } else if let Some(b) = &layout.baseboard {
        if b.enabled {
            boards.push(b);
        }
    }
    for b in &boards {
        let (x0, y0) = baseboard_origin(b);
        let cx = (x0 + b.width * 0.5) as f32;
        let cz = (y0 + b.depth * 0.5) as f32;
        let w = b.width as f32;
        let d = b.depth as f32;
        let top_col = match b.style.as_str() {
            "grass" => [0.22, 0.42, 0.14],
            "dark-wood" => [0.34, 0.22, 0.14],
            "slate" => [0.22, 0.28, 0.36],
            "white" => [0.88, 0.90, 0.92],
            _ => [0.68, 0.54, 0.36],
        };
        let frame_col = [0.42, 0.29, 0.18];
        push_box(&mut verts, &mut idxs, [cx, -10.0, cz], [w, 20.0, d], top_col);
        push_box(&mut verts, &mut idxs, [cx, -32.0, cz], [w - 28.0, 26.0, d - 28.0], frame_col);
        let leg_col = [0.35, 0.24, 0.15];
        for sx in [-1.0_f32, 1.0] {
            for sz in [-1.0_f32, 1.0] {
                push_box(
                    &mut verts,
                    &mut idxs,
                    [cx + sx * (w * 0.46), -175.0, cz + sz * (d * 0.46)],
                    [34.0, 290.0, 34.0],
                    leg_col,
                );
            }
        }
    }

    // Layer height & visibility lookup
    let mut layer_info: std::collections::HashMap<&str, (f64, bool)> = std::collections::HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_info.insert(l.id.as_str(), (l.height_mm.unwrap_or(0.0), l.visible));
        }
    }

    let corridors = collect_visible_corridors(layout, catalogs, &layer_info);

    // 2. Mountains (with Tunnel Carving, Portals & Swept Tube), Carved Ground Terrain & 3D Models
    if let Some(models) = &layout.models {
        for m in models {
            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
            let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
            if !vis {
                continue;
            }
            let y0 = (lh + m.elevation.unwrap_or(0.0)) as f32;
            match m.builtin_id.as_deref() {
                Some("mountain") => push_mountain_with_tunnels_3d(&mut verts, &mut idxs, m, y0, &corridors),
                Some("ground-terrain") => {
                    push_ground_terrain_3d(&mut verts, &mut idxs, m, y0, layout, &layer_info, &boards, &corridors);
                }
                _ => push_model_3d(&mut verts, &mut idxs, m, y0),
            }
        }
    }

    // 3. Detailed 3D Tracks & Roads (trapezoidal ballast, 3D sleepers, 3D rails, bridges, catenary)
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
                push_box_segment(verts, idxs, ra, rb, rail_w, rail_h, rail_col);
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

fn eval_brush_local(lx: f64, lz: f64, m: &Model3D) -> (f64, f64) {
    let nodes = match &m.mountain_path {
        Some(p) if !p.is_empty() => p,
        _ => return (2.0, 0.0),
    };
    let mut best_norm_r = f64::INFINITY;
    let mut best_h = 0.0_f64;
    for n in nodes {
        let r = n.radius.max(20.0);
        let nr = (lx - n.x).hypot(lz - n.y) / r;
        if nr < best_norm_r {
            best_norm_r = nr;
        }
        if nr < 1.0 {
            let prof = (nr * std::f64::consts::FRAC_PI_2).cos().powf(1.35);
            let noise = mountain_noise(lx / r, lz / r);
            let ch = (n.height * prof * (0.88 + 0.16 * noise)).max(0.0);
            if ch > best_h {
                best_h = ch;
            }
        }
    }
    (best_norm_r, best_h)
}

fn push_mountain_with_tunnels_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    m: &Model3D,
    y0: f32,
    corridors: &[CorridorBranch],
) {
    let scale = m.scale.unwrap_or(1.0) as f32;
    let w = (m.width as f32 * scale).max(60.0);
    let d = (m.depth as f32 * scale).max(60.0);
    let h = (m.height as f32 * scale).max(20.0);
    let cx = m.transform.x as f32;
    let cz = m.transform.y as f32;
    let rot_rad = (m.transform.rotation as f32).to_radians();
    let cos_inv = (-rot_rad as f64).cos();
    let sin_inv = (-rot_rad as f64).sin();

    let mut strokes: Vec<f32> = Vec::new();
    if let Some(nodes) = &m.mountain_path {
        for n in nodes {
            strokes.push((n.x as f32) * scale);
            strokes.push((n.y as f32) * scale);
            strokes.push((n.radius as f32) * scale);
            strokes.push((n.height as f32) * scale);
        }
    }

    let nx = 34usize;
    let nz = 34usize;
    let mut heights = evaluate_mountain_heightfield(nx, nz, (w * 0.5) as f64, (d * 0.5) as f64, h as f64, &strokes);
    let cols = nx + 1;
    let rows = nz + 1;
    let mut suppress_vertex = vec![false; cols * rows];

    // Check track & road corridors passing through or cutting this mountain
    let bound_r = w.max(d) * 0.65;
    struct HitSample {
        wx: f32,
        wz: f32,
        rel_elev: f32,
        half_w: f32,
        total_h: f32,
        norm_r: f32,
    }
    let mut underground_hits: Vec<HitSample> = Vec::new();

    for br in corridors {
        let near = br
            .pts_xze
            .iter()
            .any(|p| (p[0] - cx).hypot(p[1] - cz) <= bound_r + 120.0);
        if !near {
            continue;
        }
        let g_scale = (br.gauge / 16.5).clamp(0.4, 3.0);
        let tunnel_half_w = if br.is_road {
            40.0
        } else if br.has_catenary {
            35.0 * g_scale
        } else {
            30.0 * g_scale
        };
        let tunnel_total_h = if br.is_road {
            72.0
        } else if br.has_catenary {
            104.0 * g_scale
        } else {
            84.0 * g_scale
        };

        let mut flat_xze = Vec::with_capacity(br.pts_xze.len() * 3);
        for p in &br.pts_xze {
            flat_xze.push(p[0]);
            flat_xze.push(p[1]);
            flat_xze.push(p[2] - y0);
        }

        let eval = if strokes.is_empty() {
            evaluate_mountain_track_samples(
                &flat_xze,
                cx as f64,
                cz as f64,
                cos_inv,
                sin_inv,
                (w * 0.5) as f64,
                (d * 0.5) as f64,
                h as f64,
            )
        } else {
            let mut out = vec![0.0_f32; br.pts_xze.len() * 2];
            for (k, p) in br.pts_xze.iter().enumerate() {
                let dx = p[0] as f64 - cx as f64;
                let dz = p[1] as f64 - cz as f64;
                let lx = dx * cos_inv - dz * sin_inv;
                let lz = dx * sin_inv + dz * cos_inv;
                let (nr, sh) = eval_brush_local(lx / scale as f64, lz / scale as f64, m);
                out[k * 2] = nr as f32;
                out[k * 2 + 1] = (sh as f32) * scale;
            }
            out
        };

        let mut is_tunnel_pt = vec![false; br.pts_xze.len()];
        for k in 0..br.pts_xze.len() {
            let nr = eval[k * 2];
            let sh = eval[k * 2 + 1];
            let rel_elev = flat_xze[k * 3 + 2];
            if rel_elev >= -35.0 && rel_elev <= h - 10.0 && nr < 1.05 {
                if sh > rel_elev + 16.0 && nr < 0.90 {
                    is_tunnel_pt[k] = true;
                }
                underground_hits.push(HitSample {
                    wx: br.pts_xze[k][0],
                    wz: br.pts_xze[k][1],
                    rel_elev,
                    half_w: tunnel_half_w,
                    total_h: tunnel_total_h,
                    norm_r: nr,
                });
            }
        }

        // Find contiguous tunnel spans [start..=end] -> emit 3D stone portals & swept dark tunnel interior!
        let mut idx = 0;
        while idx < is_tunnel_pt.len() {
            if !is_tunnel_pt[idx] {
                idx += 1;
                continue;
            }
            let start = idx;
            while idx + 1 < is_tunnel_pt.len() && is_tunnel_pt[idx + 1] {
                idx += 1;
            }
            let end = idx;
            idx += 1;
            if end <= start {
                continue;
            }

            // Entry portal
            let entry_pt = br.pts_xze[start];
            let next_pt = br.pts_xze[(start + 1).min(end)];
            let (fdx, fdz) = (next_pt[0] - entry_pt[0], next_pt[1] - entry_pt[1]);
            if fdx.hypot(fdz) > 1e-3 {
                push_tunnel_portal_3d(
                    verts,
                    idxs,
                    [entry_pt[0], entry_pt[2], entry_pt[1]],
                    [-fdx, -fdz],
                    tunnel_half_w,
                    tunnel_total_h,
                );
            }

            // Exit portal
            let exit_pt = br.pts_xze[end];
            let prev_pt = br.pts_xze[end.saturating_sub(1)];
            let (bdx, bdz) = (exit_pt[0] - prev_pt[0], exit_pt[1] - prev_pt[1]);
            if bdx.hypot(bdz) > 1e-3 {
                push_tunnel_portal_3d(
                    verts,
                    idxs,
                    [exit_pt[0], exit_pt[2], exit_pt[1]],
                    [bdx, bdz],
                    tunnel_half_w,
                    tunnel_total_h,
                );
            }

            // Swept interior tunnel lining via railbench_core::geometry::build_tunnel_sweep_positions
            let mut path_xyz = Vec::with_capacity((end - start + 1) * 3);
            for k in start..=end {
                let p = br.pts_xze[k];
                path_xyz.push(p[0]);
                path_xyz.push(p[2]);
                path_xyz.push(p[1]);
            }
            let hw = tunnel_half_w;
            let wall_h = tunnel_total_h * 0.68;
            let arch_r = tunnel_total_h - wall_h;
            let mut section_xy = Vec::new();
            section_xy.extend_from_slice(&[-hw, 0.0, -hw, wall_h]);
            for a_step in 1..6 {
                let ang = std::f32::consts::PI - (a_step as f32 / 6.0) * std::f32::consts::PI;
                section_xy.push(hw * ang.cos());
                section_xy.push(wall_h + arch_r * ang.sin());
            }
            section_xy.extend_from_slice(&[hw, wall_h, hw, 0.0]);

            let swept = build_tunnel_sweep_positions(&path_xyz, &section_xy);
            let n_sec = section_xy.len() / 2;
            let n_ring = swept.len() / (n_sec * 3);
            let tube_col = [0.11, 0.12, 0.14];
            let inner_n = [0.0, -1.0, 0.0];
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
                    push_quad(verts, idxs, p0, p1, p2, p3, inner_n, tube_col);
                }
            }
        }
    }

    // Carve mountain approach slopes & clear tunnel portal openings in heightfield
    if !underground_hits.is_empty() {
        for iz in 0..rows {
            let lz = (iz as f32 / nz as f32 - 0.5) * d;
            for ix in 0..cols {
                let lx = (ix as f32 / nx as f32 - 0.5) * w;
                let (rx, rz) = rot_xz(lx, lz, rot_rad);
                let wx = cx + rx;
                let wz = cz + rz;
                let v_idx = iz * cols + ix;
                let mh = heights[v_idx];

                for hit in &underground_hits {
                    let dist = (wx - hit.wx).hypot(wz - hit.wz);
                    if dist <= hit.half_w + 34.0 {
                        if hit.norm_r >= 0.78 && hit.norm_r <= 1.04 && dist <= hit.half_w + 8.0 {
                            // Clear vertices blocking portal opening
                            suppress_vertex[v_idx] = true;
                        } else if hit.norm_r >= 0.80 {
                            // Carve gentle approach cutting outside portal
                            let t = 1.0 - smoothstep(hit.half_w as f64, (hit.half_w + 34.0) as f64, dist as f64) as f32;
                            heights[v_idx] = mh * (1.0 - t) + (hit.rel_elev - 2.0).max(0.0) * t;
                        } else if dist <= hit.half_w + 10.0 && mh > hit.rel_elev + 10.0 {
                            // Guarantee headroom above internal tunnel crown
                            heights[v_idx] = mh.max(hit.rel_elev + hit.total_h + 10.0);
                        }
                    }
                }
            }
        }
    }

    let step_x = w / nx as f32;
    let step_z = d / nz as f32;
    let base_v = verts.len() as u32;

    for iz in 0..rows {
        let lz = (iz as f32 / nz as f32 - 0.5) * d;
        for ix in 0..cols {
            let lx = (ix as f32 / nx as f32 - 0.5) * w;
            let mh = heights[iz * cols + ix];
            let hl = heights[iz * cols + ix.saturating_sub(1)];
            let hr = heights[iz * cols + (ix + 1).min(nx)];
            let hd = heights[iz.saturating_sub(1) * cols + ix];
            let hu = heights[(iz + 1).min(nz) * cols + ix];
            let normal = normalize3([(hl - hr) / (2.0 * step_x), 1.0, (hd - hu) / (2.0 * step_z)]);

            let ratio = (mh / h.max(1.0)).clamp(0.0, 1.0);
            let col = if ratio > 0.76 {
                [0.93, 0.95, 0.98]
            } else if ratio > 0.42 {
                [0.46, 0.45, 0.44]
            } else {
                [0.22, 0.44, 0.18]
            };
            let (rx, rz) = rot_xz(lx, lz, rot_rad);
            let (rnx, rnz) = rot_xz(normal[0], normal[2], rot_rad);
            verts.push(Vertex3D {
                pos: [cx + rx, y0 + mh, cz + rz],
                normal: [rnx, normal[1], rnz],
                color: col,
            });
        }
    }
    for iz in 0..nz {
        for ix in 0..nx {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            if suppress_vertex[i_a] || suppress_vertex[i_b] || suppress_vertex[i_c] || suppress_vertex[i_d] {
                continue;
            }
            if heights[i_a] > 0.2 || heights[i_b] > 0.2 || heights[i_c] > 0.2 || heights[i_d] > 0.2 {
                let a = base_v + i_a as u32;
                let b = base_v + i_b as u32;
                let c = base_v + i_c as u32;
                let d_i = base_v + i_d as u32;
                idxs.extend_from_slice(&[a, c, b, b, c, d_i]);
            }
        }
    }
}

fn push_tunnel_portal_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pos: [f32; 3],
    outward_xz: [f32; 2],
    half_w: f32,
    total_h: f32,
) {
    let yaw = outward_xz[0].atan2(outward_xz[1]);
    let stone = [0.46, 0.45, 0.42];
    let capstone = [0.56, 0.55, 0.51];
    let void_col = [0.06, 0.07, 0.09];
    let pier_w = (half_w * 0.45).clamp(12.0, 24.0);
    let wall_h = total_h * 0.72;
    let lintel_h = (total_h - wall_h).max(16.0);
    let portal_thick = 18.0_f32;

    // Left & right stone piers + crown lintel + capstone parapet + dark portal throat
    push_rotated_box(verts, idxs, pos, yaw, [-(half_w + pier_w * 0.5), wall_h * 0.5, 0.0], [pier_w, wall_h, portal_thick], stone);
    push_rotated_box(verts, idxs, pos, yaw, [half_w + pier_w * 0.5, wall_h * 0.5, 0.0], [pier_w, wall_h, portal_thick], stone);
    push_rotated_box(verts, idxs, pos, yaw, [0.0, wall_h + lintel_h * 0.5, 0.0], [(half_w + pier_w) * 2.0, lintel_h, portal_thick], stone);
    push_rotated_box(verts, idxs, pos, yaw, [0.0, wall_h + lintel_h + 4.0, 0.0], [(half_w + pier_w) * 2.1, 8.0, portal_thick + 5.0], capstone);
    push_rotated_box(verts, idxs, pos, yaw, [-(half_w + pier_w * 0.8), wall_h * 0.35, 14.0], [8.0, wall_h * 0.7, 22.0], stone);
    push_rotated_box(verts, idxs, pos, yaw, [half_w + pier_w * 0.8, wall_h * 0.35, 14.0], [8.0, wall_h * 0.7, 22.0], stone);
    push_rotated_box(verts, idxs, pos, yaw, [0.0, wall_h * 0.5, -10.0], [half_w * 1.9, wall_h, 6.0], void_col);
}

#[allow(clippy::too_many_arguments)]
fn push_ground_terrain_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    m: &Model3D,
    y0: f32,
    layout: &Layout,
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
    boards: &[&BaseboardConfig],
    corridors: &[CorridorBranch],
) {
    let scale = m.scale.unwrap_or(1.0) as f64;
    let width = (m.width * scale).max(60.0);
    let depth = (m.depth * scale).max(60.0);
    let ttype = m.ground_terrain_type.as_deref().unwrap_or("hill");
    let is_water = ttype == "water";
    let max_height = m.height.max(2.0) * scale;
    let mx = m.transform.x;
    let mz = m.transform.y;
    let rot_rad = m.transform.rotation.to_radians();
    let cos_m = rot_rad.cos();
    let sin_m = rot_rad.sin();

    let segs_x = 28usize;
    let segs_z = 28usize;
    let cols = segs_x + 1;
    let rows = segs_z + 1;
    let mut r_dist_grid = Vec::with_capacity(cols * rows);
    for iz in 0..rows {
        let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
        let nz = lz / (depth * 0.5);
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
            let nx = lx / (width * 0.5);
            r_dist_grid.push(nx.hypot(nz) as f32);
        }
    }

    // Support polygon from enabled baseboards
    let mut poly_offsets: Vec<u32> = Vec::new();
    let mut poly_coords: Vec<f32> = Vec::new();
    if !boards.is_empty() {
        poly_offsets.push(0);
        for b in boards {
            let (x0, y_b) = baseboard_origin(b);
            for (px, py) in [
                (x0, y_b),
                (x0 + b.width, y_b),
                (x0 + b.width, y_b + b.depth),
                (x0, y_b + b.depth),
            ] {
                poly_coords.push(px as f32);
                poly_coords.push(py as f32);
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
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
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
            let col = if is_water {
                if is_w {
                    [0.14, 0.38, 0.56]
                } else {
                    [0.72, 0.65, 0.48]
                }
            } else {
                [0.24 + h_ratio * 0.08 - flat * 0.03, 0.44 + h_ratio * 0.11, 0.16 + h_ratio * 0.04]
            };

            let wx = mx + (lx * cos_m - lz * sin_m);
            let wz = mz + (lx * sin_m + lz * cos_m);
            verts.push(Vertex3D {
                pos: [wx as f32, y0 + gh + 0.3, wz as f32],
                normal: [rnx, norm[1], rnz],
                color: col,
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
            let edge_vis = |vi: usize| r_dist_grid[vi] <= 1.02;
            if (in_poly(i_a) && in_poly(i_b) && in_poly(i_c) && in_poly(i_d))
                && (edge_vis(i_a) || edge_vis(i_b) || edge_vis(i_c) || edge_vis(i_d))
            {
                let a = base_v + i_a as u32;
                let b = base_v + i_b as u32;
                let c = base_v + i_c as u32;
                let d_i = base_v + i_d as u32;
                idxs.extend_from_slice(&[a, c, b, b, c, d_i]);
            }
        }
    }
}

fn push_model_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, m: &Model3D, y0: f32) {
    let scale = m.scale.unwrap_or(1.0) as f32;
    let w = m.width as f32 * scale;
    let d = m.depth as f32 * scale;
    let h = m.height as f32 * scale;
    let cx = m.transform.x as f32;
    let cz = m.transform.y as f32;
    let rot_rad = (m.transform.rotation as f32).to_radians();

    struct ModelCtx<'a> {
        verts: &'a mut Vec<Vertex3D>,
        idxs: &'a mut Vec<u32>,
        origin: [f32; 3],
        rot_rad: f32,
    }
    impl<'a> ModelCtx<'a> {
        fn rbox(&mut self, lx: f32, ly: f32, lz: f32, sx: f32, sy: f32, sz: f32, col: [f32; 3]) {
            push_rotated_box(self.verts, self.idxs, self.origin, self.rot_rad, [lx, ly, lz], [sx, sy, sz], col);
        }
        fn rgable(&mut self, lx: f32, ly: f32, lz: f32, sx: f32, sy: f32, sz: f32, col: [f32; 3]) {
            push_gable_roof(self.verts, self.idxs, self.origin, self.rot_rad, [lx, ly, lz], [sx, sy, sz], col);
        }
    }
    let mut b = ModelCtx {
        verts,
        idxs,
        origin: [cx, y0, cz],
        rot_rad,
    };

    match m.builtin_id.as_deref() {
        Some("station") => {
            let plaster = [0.90, 0.85, 0.75];
            let plinth = [0.54, 0.52, 0.49];
            let roof = [0.58, 0.16, 0.14];
            let window = [0.16, 0.24, 0.34];
            b.rbox(0.0, h * 0.05, 0.0, w * 0.96, h * 0.10, d * 0.84, plinth);
            b.rbox(0.0, h * 0.34, 0.0, w * 0.44, h * 0.48, d * 0.62, plaster);
            b.rgable(0.0, h * 0.58, 0.0, w * 0.48, h * 0.22, d * 0.68, roof);
            b.rbox(-w * 0.33, h * 0.28, 0.0, w * 0.26, h * 0.36, d * 0.54, plaster);
            b.rgable(-w * 0.33, h * 0.46, 0.0, w * 0.28, h * 0.16, d * 0.60, roof);
            b.rbox(w * 0.33, h * 0.28, 0.0, w * 0.26, h * 0.36, d * 0.54, plaster);
            b.rgable(w * 0.33, h * 0.46, 0.0, w * 0.28, h * 0.16, d * 0.60, roof);
            b.rbox(w * 0.20, h * 0.52, -d * 0.10, w * 0.12, h * 0.84, d * 0.22, plaster);
            b.rgable(w * 0.20, h * 0.94, -d * 0.10, w * 0.14, h * 0.14, d * 0.24, roof);
            for i in -2..=2 {
                let wx = (i as f32) * w * 0.16;
                b.rbox(wx, h * 0.32, d * 0.315, w * 0.06, h * 0.14, 2.0, window);
                b.rbox(wx, h * 0.32, -d * 0.315, w * 0.06, h * 0.14, 2.0, window);
            }
        }
        Some("town-house") => {
            let plaster = [0.87, 0.80, 0.68];
            let base = [0.50, 0.48, 0.46];
            let roof = [0.62, 0.22, 0.16];
            let win = [0.18, 0.26, 0.36];
            b.rbox(0.0, h * 0.06, 0.0, w * 0.94, h * 0.12, d * 0.90, base);
            b.rbox(0.0, h * 0.38, 0.0, w * 0.90, h * 0.54, d * 0.84, plaster);
            b.rgable(0.0, h * 0.65, 0.0, w * 0.96, h * 0.32, d * 0.92, roof);
            for floor in [h * 0.28, h * 0.50] {
                for sx in [-0.26_f32, 0.0, 0.26] {
                    b.rbox(w * sx, floor, d * 0.425, w * 0.14, h * 0.12, 2.0, win);
                    b.rbox(w * sx, floor, -d * 0.425, w * 0.14, h * 0.12, 2.0, win);
                }
            }
        }
        Some("signal-box") => {
            let brick = [0.58, 0.24, 0.18];
            let timber = [0.32, 0.22, 0.15];
            let glass = [0.22, 0.36, 0.48];
            let roof = [0.26, 0.28, 0.32];
            b.rbox(0.0, h * 0.28, 0.0, w * 0.82, h * 0.56, d * 0.78, brick);
            b.rbox(0.0, h * 0.68, 0.0, w * 0.96, h * 0.26, d * 0.92, timber);
            b.rbox(0.0, h * 0.70, 0.0, w * 0.98, h * 0.14, d * 0.94, glass);
            b.rgable(0.0, h * 0.81, 0.0, w * 1.04, h * 0.19, d * 1.0, roof);
        }
        Some("water-tower") => {
            let stone = [0.56, 0.52, 0.46];
            let tank = [0.36, 0.24, 0.16];
            let roof = [0.26, 0.29, 0.33];
            b.rbox(0.0, h * 0.28, 0.0, w * 0.54, h * 0.56, d * 0.54, stone);
            b.rbox(0.0, h * 0.68, 0.0, w * 0.82, h * 0.28, d * 0.82, tank);
            b.rgable(0.0, h * 0.82, 0.0, w * 0.90, h * 0.18, d * 0.90, roof);
        }
        Some("pine-tree") => {
            b.rbox(0.0, h * 0.16, 0.0, w * 0.16, h * 0.32, d * 0.16, [0.32, 0.21, 0.13]);
            let tiers = [
                (0.36, 0.88, [0.10, 0.34, 0.16]),
                (0.54, 0.68, [0.12, 0.40, 0.18]),
                (0.71, 0.48, [0.14, 0.45, 0.21]),
                (0.86, 0.26, [0.16, 0.50, 0.24]),
            ];
            for (yf, wf, col) in tiers {
                b.rbox(0.0, h * yf, 0.0, w * wf, h * 0.20, d * wf, col);
            }
        }
        Some("deciduous-tree") => {
            b.rbox(0.0, h * 0.22, 0.0, w * 0.18, h * 0.44, d * 0.18, [0.34, 0.22, 0.14]);
            b.rbox(0.0, h * 0.62, 0.0, w * 0.72, h * 0.42, d * 0.72, [0.18, 0.50, 0.20]);
            b.rbox(w * 0.20, h * 0.70, d * 0.14, w * 0.46, h * 0.34, d * 0.46, [0.22, 0.56, 0.24]);
            b.rbox(-w * 0.18, h * 0.66, -d * 0.16, w * 0.44, h * 0.32, d * 0.44, [0.15, 0.44, 0.18]);
        }
        Some("light-signal") => {
            b.rbox(0.0, h * 0.06, 0.0, w * 0.8, h * 0.12, d * 0.8, [0.55, 0.57, 0.60]);
            b.rbox(0.0, h * 0.48, 0.0, w * 0.22, h * 0.80, d * 0.22, [0.28, 0.32, 0.36]);
            b.rbox(0.0, h * 0.84, 0.0, w * 0.52, h * 0.28, d * 0.32, [0.12, 0.14, 0.16]);
            b.rbox(0.0, h * 0.88, d * 0.18, w * 0.24, h * 0.08, 1.5, [0.15, 0.92, 0.32]);
        }
        Some("platform-lamp") => {
            b.rbox(0.0, h * 0.48, 0.0, w * 0.20, h * 0.94, d * 0.20, [0.22, 0.26, 0.30]);
            b.rbox(0.0, h * 0.94, 0.0, w * 0.70, h * 0.08, d * 0.70, [0.98, 0.94, 0.72]);
        }
        _ => {
            b.rbox(0.0, h * 0.5, 0.0, w, h, d, [0.76, 0.68, 0.58]);
        }
    }
}

// ============================================================================
//  3D Geometry Primitives (Trapezoidal Ballast, Box Segments, Rotated Boxes, Roofs)
// ============================================================================

fn rot_xz(x: f32, z: f32, rad: f32) -> (f32, f32) {
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

    push_quad(verts, idxs, atl, atr, btr, btl, [0.0, 1.0, 0.0], color);
    push_quad(verts, idxs, abl, atl, btl, bbl, [nx, 0.0, nz], color);
    push_quad(verts, idxs, atr, abr, bbr, btr, [-nx, 0.0, -nz], color);
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

    push_quad(verts, idxs, pt(-hx, hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 1.0, 0.0), color);
    push_quad(verts, idxs, pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 0.0, 1.0), color);
    push_quad(verts, idxs, pt(hx, -hy, -hz), pt(-hx, -hy, -hz), pt(-hx, hy, -hz), pt(hx, hy, -hz), rn(0.0, 0.0, -1.0), color);
    push_quad(verts, idxs, pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), rn(1.0, 0.0, 0.0), color);
    push_quad(verts, idxs, pt(-hx, -hy, -hz), pt(-hx, -hy, hz), pt(-hx, hy, hz), pt(-hx, hy, -hz), rn(-1.0, 0.0, 0.0), color);
}

fn push_gable_roof(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    base_center: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
) {
    let hx = size[0] * 0.5;
    let h = size[1];
    let hz = size[2] * 0.5;
    let pt = |x: f32, y: f32, z: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(base_center[0] + x, base_center[2] + z, rot_rad);
        [origin[0] + rx, origin[1] + base_center[1] + y, origin[2] + rz]
    };
    let rn = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(nx, nz, rot_rad);
        normalize3([rx, ny, rz])
    };

    // Front & back roof slopes
    push_quad(verts, idxs, pt(-hx, 0.0, hz), pt(hx, 0.0, hz), pt(hx, h, 0.0), pt(-hx, h, 0.0), rn(0.0, hz, h), color);
    push_quad(verts, idxs, pt(hx, 0.0, -hz), pt(-hx, 0.0, -hz), pt(-hx, h, 0.0), pt(hx, h, 0.0), rn(0.0, hz, -h), color);
    // Left & right gable triangles
    let g_col = [color[0] * 0.85, color[1] * 0.85, color[2] * 0.85];
    let b1 = verts.len() as u32;
    let nl = rn(-1.0, 0.0, 0.0);
    verts.push(Vertex3D { pos: pt(-hx, 0.0, -hz), normal: nl, color: g_col });
    verts.push(Vertex3D { pos: pt(-hx, 0.0, hz), normal: nl, color: g_col });
    verts.push(Vertex3D { pos: pt(-hx, h, 0.0), normal: nl, color: g_col });
    idxs.extend_from_slice(&[b1, b1 + 1, b1 + 2]);

    let b2 = verts.len() as u32;
    let nr = rn(1.0, 0.0, 0.0);
    verts.push(Vertex3D { pos: pt(hx, 0.0, hz), normal: nr, color: g_col });
    verts.push(Vertex3D { pos: pt(hx, 0.0, -hz), normal: nr, color: g_col });
    verts.push(Vertex3D { pos: pt(hx, h, 0.0), normal: nr, color: g_col });
    idxs.extend_from_slice(&[b2, b2 + 1, b2 + 2]);
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
    let base = verts.len() as u32;
    verts.push(Vertex3D { pos: p0, normal, color });
    verts.push(Vertex3D { pos: p1, normal, color });
    verts.push(Vertex3D { pos: p2, normal, color });
    verts.push(Vertex3D { pos: p3, normal, color });
    idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
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

fn normalize3(v: [f32; 3]) -> [f32; 3] {
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

        let mesh_base = build_scene_mesh_3d(&layout_only_mtn, &catalogs);
        let mesh_tunnel = build_scene_mesh_3d(&layout_with_tunnel, &catalogs);
        assert!(mesh_tunnel.vertices.len() > mesh_base.vertices.len() + 100);
        assert!(mesh_tunnel.vertices.iter().all(|v| v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite()));
    }
}
