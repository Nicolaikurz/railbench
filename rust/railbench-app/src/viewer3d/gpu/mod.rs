use super::*;

pub(super) struct OffscreenTarget {
    pub(super) size: [u32; 2],
    pub(super) msaa_tex: wgpu::Texture,
    pub(super) msaa_view: wgpu::TextureView,
    pub(super) color_tex: wgpu::Texture,
    pub(super) color_view: wgpu::TextureView,
    pub(super) depth_tex: wgpu::Texture,
    pub(super) depth_view: wgpu::TextureView,
    pub(super) blit_bg: wgpu::BindGroup,
}

impl OffscreenTarget {
    pub(super) fn destroy(self) {
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
    pub(super) index_offset: u32,
    pub(super) index_count: u32,
    pub(super) base_vertex: i32,
    pub(super) instance_range: std::ops::Range<u32>,
}

pub(super) struct GpuUploadedTexture {
    pub(super) tex: wgpu::Texture,
    pub(super) _view: wgpu::TextureView,
    pub(super) bind_group: wgpu::BindGroup,
}

impl Drop for GpuUploadedTexture {
    fn drop(&mut self) {
        self.tex.destroy();
    }
}

pub(super) struct GpuTexturedDraw3D {
    pub(super) texture_id: u64,
    pub(super) vb: wgpu::Buffer,
    pub(super) ib: wgpu::Buffer,
    pub(super) index_count: u32,
}

#[derive(Default)]
pub(super) struct GpuMeshSlot {
    pub(super) sig: Option<u64>,
    pub(super) static_vb: Option<wgpu::Buffer>,
    pub(super) static_ib: Option<wgpu::Buffer>,
    pub(super) static_index_count: u32,
    pub(super) glass_ib: Option<wgpu::Buffer>,
    pub(super) glass_index_count: u32,
    pub(super) proto_vb: Option<wgpu::Buffer>,
    pub(super) proto_ib: Option<wgpu::Buffer>,
    pub(super) instance_buf: Option<wgpu::Buffer>,
    pub(super) instanced_draws: Vec<GpuInstancedDraw3D>,
    pub(super) textured_draws: Vec<GpuTexturedDraw3D>,
}

impl GpuMeshSlot {
    pub(super) fn clear(&mut self) {
        if let Some(b) = self.static_vb.take() {
            b.destroy();
        }
        if let Some(b) = self.static_ib.take() {
            b.destroy();
        }
        if let Some(b) = self.glass_ib.take() {
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
        for td in self.textured_draws.drain(..) {
            td.vb.destroy();
            td.ib.destroy();
        }
        self.static_index_count = 0;
        self.glass_index_count = 0;
        self.instanced_draws.clear();
        self.sig = None;
    }

    pub(super) fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: Option<&(u64, Arc<SceneMesh3D>)>,
        label_prefix: &str,
        tex_ctx: Option<(
            &wgpu::BindGroupLayout,
            &wgpu::Sampler,
            &mut std::collections::HashMap<u64, GpuUploadedTexture>,
        )>,
    ) {
        let Some((new_sig, mesh)) = slot else {
            if self.sig.is_some() {
                self.clear();
            }
            if let Some((_, _, cache)) = tex_ctx {
                cache.clear();
            }
            return;
        };
        if self.sig == Some(*new_sig) {
            return;
        }
        self.clear();
        self.sig = Some(*new_sig);

        if !mesh.vertices.is_empty() && (!mesh.indices.is_empty() || !mesh.glass_indices.is_empty()) {
            let vb = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label_prefix),
                size: (mesh.vertices.len() * std::mem::size_of::<Vertex3D>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vb, 0, bytemuck::cast_slice(&mesh.vertices));
            self.static_vb = Some(vb);

            if !mesh.indices.is_empty() {
                let ib = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label_prefix),
                    size: (mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                queue.write_buffer(&ib, 0, bytemuck::cast_slice(&mesh.indices));
                self.static_ib = Some(ib);
                self.static_index_count = mesh.indices.len() as u32;
            }

            if !mesh.glass_indices.is_empty() {
                let gib = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label_prefix),
                    size: (mesh.glass_indices.len() * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                queue.write_buffer(&gib, 0, bytemuck::cast_slice(&mesh.glass_indices));
                self.glass_ib = Some(gib);
                self.glass_index_count = mesh.glass_indices.len() as u32;
            }
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

        if let Some((tex_bgl, tex_sampler, texture_cache)) = tex_ctx {
            if mesh.textured_meshes.is_empty() {
                texture_cache.clear();
            } else {
                let mut active_ids =
                    std::collections::HashSet::with_capacity(mesh.textured_meshes.len());
                for tm in &mesh.textured_meshes {
                    if tm.vertices.is_empty()
                        || tm.indices.is_empty()
                        || tm.tex_width == 0
                        || tm.tex_height == 0
                    {
                        continue;
                    }
                    active_ids.insert(tm.texture_id);
                    texture_cache.entry(tm.texture_id).or_insert_with(|| {
                        let size = wgpu::Extent3d {
                            width: tm.tex_width,
                            height: tm.tex_height,
                            depth_or_array_layers: 1,
                        };
                        let tex = device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("railbench_3d_custom_albedo"),
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });
                        queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &tex,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            &tm.tex_rgba,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * tm.tex_width),
                                rows_per_image: Some(tm.tex_height),
                            },
                            size,
                        );
                        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("railbench_3d_custom_albedo_bg"),
                            layout: tex_bgl,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::TextureView(&view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(tex_sampler),
                                },
                            ],
                        });
                        GpuUploadedTexture {
                            tex,
                            _view: view,
                            bind_group,
                        }
                    });

                    let vb = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("railbench_3d_tex_vb"),
                        size: (tm.vertices.len() * std::mem::size_of::<TexturedVertex3D>()) as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&vb, 0, bytemuck::cast_slice(&tm.vertices));
                    let ib = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("railbench_3d_tex_ib"),
                        size: (tm.indices.len() * std::mem::size_of::<u32>()) as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&ib, 0, bytemuck::cast_slice(&tm.indices));
                    self.textured_draws.push(GpuTexturedDraw3D {
                        texture_id: tm.texture_id,
                        vb,
                        ib,
                        index_count: tm.indices.len() as u32,
                    });
                }
                texture_cache.retain(|id, _| active_ids.contains(id));
            }
        }
    }

    pub(super) fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, identity_inst_buf: &'a wgpu::Buffer) {
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

    pub(super) fn draw_glass<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, identity_inst_buf: &'a wgpu::Buffer) {
        if self.glass_index_count > 0 {
            if let (Some(vb), Some(gib)) = (&self.static_vb, &self.glass_ib) {
                rpass.set_vertex_buffer(0, vb.slice(..));
                rpass.set_vertex_buffer(1, identity_inst_buf.slice(..));
                rpass.set_index_buffer(gib.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..self.glass_index_count, 0, 0..1);
            }
        }
    }

    pub(super) fn draw_textured<'a>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'a>,
        texture_cache: &'a std::collections::HashMap<u64, GpuUploadedTexture>,
    ) {
        for td in &self.textured_draws {
            let Some(gpu_tex) = texture_cache.get(&td.texture_id) else {
                continue;
            };
            rpass.set_bind_group(1, &gpu_tex.bind_group, &[]);
            rpass.set_vertex_buffer(0, td.vb.slice(..));
            rpass.set_index_buffer(td.ib.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..td.index_count, 0, 0..1);
        }
    }

    pub(super) fn draw_textured_shadow<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        for td in &self.textured_draws {
            rpass.set_vertex_buffer(0, td.vb.slice(..));
            rpass.set_index_buffer(td.ib.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..td.index_count, 0, 0..1);
        }
    }
}

pub(super) struct GpuCelestialSlot {
    pub(super) key: (u8, u16),
    pub(super) vb: Option<wgpu::Buffer>,
    pub(super) vb_cap: u64,
    pub(super) ib: Option<wgpu::Buffer>,
    pub(super) ib_cap: u64,
    pub(super) index_count: u32,
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
    pub(super) fn clear(&mut self) {
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

    pub(super) fn update(
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

    pub(super) fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, identity_inst_buf: &'a wgpu::Buffer) {
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


mod pipeline;
mod callback;

use pipeline::*;
pub use callback::{init_wgpu_3d_resources, release_wgpu_3d_resources};
pub(super) use callback::Viewport3DCallback;
