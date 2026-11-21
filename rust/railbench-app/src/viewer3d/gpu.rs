use super::*;

pub(super) struct OffscreenTarget {
    size: [u32; 2],
    msaa_tex: wgpu::Texture,
    msaa_view: wgpu::TextureView,
    color_tex: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_tex: wgpu::Texture,
    depth_view: wgpu::TextureView,
    blit_bg: wgpu::BindGroup,
}

impl OffscreenTarget {
    fn destroy(self) {
        drop(self.blit_bg);
        drop(self.msaa_view);
        drop(self.color_view);
        drop(self.depth_view);
        self.msaa_tex.destroy();
        self.color_tex.destroy();
        self.depth_tex.destroy();
    }
}

pub(super) struct GpuInstancedDraw3D {
    index_offset: u32,
    index_count: u32,
    base_vertex: i32,
    instance_range: std::ops::Range<u32>,
}

#[derive(Default)]
pub(super) struct GpuMeshSlot {
    sig: Option<u64>,
    static_vb: Option<wgpu::Buffer>,
    static_ib: Option<wgpu::Buffer>,
    static_index_count: u32,
    proto_vb: Option<wgpu::Buffer>,
    proto_ib: Option<wgpu::Buffer>,
    instance_buf: Option<wgpu::Buffer>,
    instanced_draws: Vec<GpuInstancedDraw3D>,
}

impl GpuMeshSlot {
    fn clear(&mut self) {
        if let Some(b) = self.static_vb.take() {
            b.destroy();
        }
        if let Some(b) = self.static_ib.take() {
            b.destroy();
        }
        if let Some(b) = self.proto_vb.take() {
            b.destroy();
        }
        if let Some(b) = self.proto_ib.take() {
            b.destroy();
        }
        if let Some(b) = self.instance_buf.take() {
            b.destroy();
        }
        self.static_index_count = 0;
        self.instanced_draws.clear();
        self.sig = None;
    }

    fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: Option<&(u64, Arc<SceneMesh3D>)>,
        label_prefix: &str,
    ) {
        let Some((new_sig, mesh)) = slot else {
            if self.sig.is_some() {
                self.clear();
            }
            return;
        };
        if self.sig == Some(*new_sig) {
            return;
        }
        self.clear();
        self.sig = Some(*new_sig);

        if !mesh.vertices.is_empty() && !mesh.indices.is_empty() {
            let vb = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label_prefix),
                size: (mesh.vertices.len() * std::mem::size_of::<Vertex3D>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vb, 0, bytemuck::cast_slice(&mesh.vertices));

            let ib = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label_prefix),
                size: (mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&ib, 0, bytemuck::cast_slice(&mesh.indices));

            self.static_vb = Some(vb);
            self.static_ib = Some(ib);
            self.static_index_count = mesh.indices.len() as u32;
        }

        if !mesh.instanced_tracks.is_empty() {
            let total_pv: usize = mesh.instanced_tracks.iter().map(|p| p.vertices.len()).sum();
            let total_pi: usize = mesh.instanced_tracks.iter().map(|p| p.indices.len()).sum();
            let total_inst: usize = mesh.instanced_tracks.iter().map(|p| p.instances.len()).sum();
            if total_pv > 0 && total_pi > 0 && total_inst > 0 {
                let mut pverts = Vec::with_capacity(total_pv);
                let mut pidxs = Vec::with_capacity(total_pi);
                let mut insts = Vec::with_capacity(total_inst);
                let mut draws = Vec::with_capacity(mesh.instanced_tracks.len());
                for proto in &mesh.instanced_tracks {
                    if proto.vertices.is_empty() || proto.indices.is_empty() || proto.instances.is_empty() {
                        continue;
                    }
                    let base_vertex = pverts.len() as i32;
                    let index_offset = pidxs.len() as u32;
                    let index_count = proto.indices.len() as u32;
                    let inst_start = insts.len() as u32;
                    let inst_end = inst_start + proto.instances.len() as u32;

                    pverts.extend_from_slice(&proto.vertices);
                    pidxs.extend_from_slice(&proto.indices);
                    insts.extend_from_slice(&proto.instances);

                    draws.push(GpuInstancedDraw3D {
                        index_offset,
                        index_count,
                        base_vertex,
                        instance_range: inst_start..inst_end,
                    });
                }
                if !draws.is_empty() {
                    let pvb = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("railbench_3d_proto_vb"),
                        size: (pverts.len() * std::mem::size_of::<Vertex3D>()) as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&pvb, 0, bytemuck::cast_slice(&pverts));

                    let pib = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("railbench_3d_proto_ib"),
                        size: (pidxs.len() * std::mem::size_of::<u32>()) as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&pib, 0, bytemuck::cast_slice(&pidxs));

                    let inst_b = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("railbench_3d_inst_b"),
                        size: (insts.len() * std::mem::size_of::<Instance3D>()) as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&inst_b, 0, bytemuck::cast_slice(&insts));

                    self.proto_vb = Some(pvb);
                    self.proto_ib = Some(pib);
                    self.instance_buf = Some(inst_b);
                    self.instanced_draws = draws;
                }
            }
        }
    }

    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, identity_inst_buf: &'a wgpu::Buffer) {
        if self.static_index_count > 0 {
            if let (Some(vb), Some(ib)) = (&self.static_vb, &self.static_ib) {
                rpass.set_vertex_buffer(0, vb.slice(..));
                rpass.set_vertex_buffer(1, identity_inst_buf.slice(..));
                rpass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..self.static_index_count, 0, 0..1);
            }
        }
        if !self.instanced_draws.is_empty() {
            if let (Some(pvb), Some(pib), Some(inst_b)) =
                (&self.proto_vb, &self.proto_ib, &self.instance_buf)
            {
                rpass.set_vertex_buffer(0, pvb.slice(..));
                rpass.set_vertex_buffer(1, inst_b.slice(..));
                rpass.set_index_buffer(pib.slice(..), wgpu::IndexFormat::Uint32);
                for draw in &self.instanced_draws {
                    rpass.draw_indexed(
                        draw.index_offset..(draw.index_offset + draw.index_count),
                        draw.base_vertex,
                        draw.instance_range.clone(),
                    );
                }
            }
        }
    }
}

pub(super) struct GpuCelestialSlot {
    key: (u8, u16),
    vb: Option<wgpu::Buffer>,
    vb_cap: u64,
    ib: Option<wgpu::Buffer>,
    ib_cap: u64,
    index_count: u32,
}

impl Default for GpuCelestialSlot {
    fn default() -> Self {
        Self {
            key: (u8::MAX, u16::MAX),
            vb: None,
            vb_cap: 0,
            ib: None,
            ib_cap: 0,
            index_count: 0,
        }
    }
}

impl GpuCelestialSlot {
    fn clear(&mut self) {
        if let Some(b) = self.vb.take() {
            b.destroy();
        }
        if let Some(b) = self.ib.take() {
            b.destroy();
        }
        self.vb_cap = 0;
        self.ib_cap = 0;
        self.index_count = 0;
        self.key = (u8::MAX, u16::MAX);
    }

    fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        time_of_day: f32,
        env_mode: Viewer3DEnv,
    ) {
        let minute_bucket = (time_of_day.rem_euclid(24.0) * 60.0).round() as u16;
        let new_key = (env_mode as u8, minute_bucket);
        if self.key == new_key {
            return;
        }
        self.key = new_key;

        let mut verts = Vec::with_capacity(64);
        let mut idxs = Vec::with_capacity(128);
        push_celestial_bodies_3d(&mut verts, &mut idxs, time_of_day, env_mode);
        if verts.is_empty() || idxs.is_empty() {
            self.index_count = 0;
            return;
        }

        let vb_bytes = (verts.len() * std::mem::size_of::<Vertex3D>()) as u64;
        if self.vb.is_none() || self.vb_cap < vb_bytes {
            if let Some(old) = self.vb.take() {
                old.destroy();
            }
            let cap = vb_bytes.max(64 * std::mem::size_of::<Vertex3D>() as u64);
            self.vb = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("railbench_3d_celestial_vb"),
                size: cap,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vb_cap = cap;
        }

        let ib_bytes = (idxs.len() * std::mem::size_of::<u32>()) as u64;
        if self.ib.is_none() || self.ib_cap < ib_bytes {
            if let Some(old) = self.ib.take() {
                old.destroy();
            }
            let cap = ib_bytes.max(128 * std::mem::size_of::<u32>() as u64);
            self.ib = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("railbench_3d_celestial_ib"),
                size: cap,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.ib_cap = cap;
        }

        if let (Some(vb), Some(ib)) = (&self.vb, &self.ib) {
            queue.write_buffer(vb, 0, bytemuck::cast_slice(&verts));
            queue.write_buffer(ib, 0, bytemuck::cast_slice(&idxs));
            self.index_count = idxs.len() as u32;
        }
    }

    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, identity_inst_buf: &'a wgpu::Buffer) {
        if self.index_count > 0 {
            if let (Some(vb), Some(ib)) = (&self.vb, &self.ib) {
                rpass.set_vertex_buffer(0, vb.slice(..));
                rpass.set_vertex_buffer(1, identity_inst_buf.slice(..));
                rpass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }
    }
}

pub(super) struct Gpu3DState {
    scene_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    blit_bgl: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    scene_bg: wgpu::BindGroup,
    identity_instance_buf: wgpu::Buffer,
    slot_env: GpuMeshSlot,
    slot_boards: GpuMeshSlot,
    slot_terrain: GpuMeshSlot,
    slot_tracks: GpuMeshSlot,
    celestial_slot: GpuCelestialSlot,
    offscreen: Option<OffscreenTarget>,
}

impl Gpu3DState {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat, surface_msaa_samples: u32) -> Self {
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
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex3D>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                            wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Snorm8x4 },
                            wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Unorm8x4 },
                            wgpu::VertexAttribute { offset: 20, shader_location: 3, format: wgpu::VertexFormat::Snorm8x4 },
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance3D>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute { offset: 0, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
                            wgpu::VertexAttribute { offset: 16, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
                        ],
                    },
                ],
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
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
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
            multisample: wgpu::MultisampleState {
                count: surface_msaa_samples.max(1),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let identity_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("railbench_3d_identity_instance"),
            size: std::mem::size_of::<Instance3D>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        identity_instance_buf
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::bytes_of(&Instance3D::IDENTITY));
        identity_instance_buf.unmap();

        Self {
            scene_pipeline,
            blit_pipeline,
            blit_bgl,
            blit_sampler,
            uniform_buf,
            scene_bg,
            identity_instance_buf,
            slot_env: GpuMeshSlot::default(),
            slot_boards: GpuMeshSlot::default(),
            slot_terrain: GpuMeshSlot::default(),
            slot_tracks: GpuMeshSlot::default(),
            celestial_slot: GpuCelestialSlot::default(),
            offscreen: None,
        }
    }

    fn release_buffers_and_offscreen(&mut self) {
        self.slot_env.clear();
        self.slot_boards.clear();
        self.slot_terrain.clear();
        self.slot_tracks.clear();
        self.celestial_slot.clear();
        if let Some(old) = self.offscreen.take() {
            old.destroy();
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
        if let Some(old) = self.offscreen.take() {
            old.destroy();
        }
        let msaa_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("railbench_3d_msaa_color_tex"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: MSAA_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_tex.create_view(&wgpu::TextureViewDescriptor::default());

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
            sample_count: MSAA_SAMPLE_COUNT,
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
            msaa_tex,
            msaa_view,
            color_tex,
            color_view,
            depth_tex,
            depth_view,
            blit_bg,
        });
    }
}

pub(super) struct Viewport3DCallback {
    pub(super) uniforms: Uniforms3D,
    pub(super) bundle: Option<Scene3DBundle>,
    pub(super) time_of_day: f32,
    pub(super) env_mode: Viewer3DEnv,
    pub(super) viewport_px: [u32; 2],
    pub(super) clear_color: [f64; 3],
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
            .or_insert_with(|| Gpu3DState::new(device, wgpu::TextureFormat::Bgra8Unorm, WINDOW_MSAA_SAMPLES));

        state.ensure_offscreen(device, self.viewport_px[0], self.viewport_px[1]);
        queue.write_buffer(&state.uniform_buf, 0, bytemuck::bytes_of(&self.uniforms));

        if let Some(bundle) = &self.bundle {
            state.slot_env.sync(device, queue, bundle.env.as_ref(), "railbench_3d_env");
            state.slot_boards.sync(device, queue, bundle.boards.as_ref(), "railbench_3d_boards");
            state.slot_terrain.sync(device, queue, bundle.terrain.as_ref(), "railbench_3d_terrain");
            state.slot_tracks.sync(device, queue, bundle.tracks.as_ref(), "railbench_3d_tracks");
        }
        state.celestial_slot.update(device, queue, self.time_of_day, self.env_mode);

        // Render the 3D scene into our 4x MSAA Color + Depth32Float buffer and resolve into color_view
        if let Some(ref target) = state.offscreen {
            let mut rpass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("railbench_3d_offscreen_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.msaa_view,
                    resolve_target: Some(&target.color_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color[0],
                            g: self.clear_color[1],
                            b: self.clear_color[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Discard,
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

            rpass.set_pipeline(&state.scene_pipeline);
            rpass.set_bind_group(0, &state.scene_bg, &[]);
            state.slot_env.draw(&mut rpass, &state.identity_instance_buf);
            state.slot_boards.draw(&mut rpass, &state.identity_instance_buf);
            state.slot_terrain.draw(&mut rpass, &state.identity_instance_buf);
            state.slot_tracks.draw(&mut rpass, &state.identity_instance_buf);
            state.celestial_slot.draw(&mut rpass, &state.identity_instance_buf);
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

pub fn init_wgpu_3d_resources(cc: &eframe::CreationContext<'_>, surface_msaa_samples: u32) {
    if let Some(render_state) = &cc.wgpu_render_state {
        let mut res = render_state.renderer.write();
        let state = Gpu3DState::new(
            &render_state.device,
            render_state.target_format,
            surface_msaa_samples,
        );
        res.callback_resources.insert(state);
    }
}

pub fn release_wgpu_3d_resources(render_state: Option<&egui_wgpu::RenderState>) {
    if let Some(rs) = render_state {
        let mut res = rs.renderer.write();
        if let Some(state) = res.callback_resources.get_mut::<Gpu3DState>() {
            state.release_buffers_and_offscreen();
        }
    }
}

