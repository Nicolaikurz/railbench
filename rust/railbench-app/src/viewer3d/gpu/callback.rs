use super::super::*;
use super::*;

pub(in crate::viewer3d) struct Viewport3DCallback {
    pub(in crate::viewer3d) uniforms: Uniforms3D,
    pub(in crate::viewer3d) bundle: Option<Scene3DBundle>,
    pub(in crate::viewer3d) vehicles_mesh: Option<Arc<SceneMesh3D>>,
    pub(in crate::viewer3d) time_of_day: f32,
    pub(in crate::viewer3d) env_mode: Viewer3DEnv,
    pub(in crate::viewer3d) viewport_px: [u32; 2],
    pub(in crate::viewer3d) clear_color: [f64; 3],
    pub(in crate::viewer3d) is_cab_view: bool,
    pub(in crate::viewer3d) shadows_enabled: bool,
    pub(in crate::viewer3d) day_night_active: bool,
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

        state.ensure_offscreen(device, self.viewport_px[0], self.viewport_px[1], self.is_cab_view);
        queue.write_buffer(&state.uniform_buf, 0, bytemuck::bytes_of(&self.uniforms));

        if let Some(bundle) = &self.bundle {
            state.slot_env.sync(device, queue, bundle.env.as_ref(), "railbench_3d_env", None);
            state.slot_boards.sync(device, queue, bundle.boards.as_ref(), "railbench_3d_boards", None);
            state.slot_terrain.sync(device, queue, bundle.terrain.as_ref(), "railbench_3d_terrain", None);
            state.slot_tracks.sync(
                device,
                queue,
                bundle.tracks.as_ref(),
                "railbench_3d_tracks",
                Some((&state.tex_bgl, &state.tex_sampler, &mut state.texture_cache)),
            );
        }
        if let Some(vmesh) = &self.vehicles_mesh {
            static VEHICLE_MESH_SEQ: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(1);
            let ver = VEHICLE_MESH_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let slot_data = (ver, Arc::clone(vmesh));
            state
                .slot_vehicles
                .sync(device, queue, Some(&slot_data), "railbench_3d_vehicles", None);
        } else {
            state
                .slot_vehicles
                .sync(device, queue, None, "railbench_3d_vehicles", None);
        }
        state.celestial_slot.update(device, queue, self.time_of_day, self.env_mode);

        // Render directional sun depth shadow map (2048x2048), throttled to ~2 Hz during continuous day/night playback
        let sun_bucket = (self.time_of_day.rem_euclid(24.0) * 60.0).round() as i32;
        let geo_sigs = (
            self.bundle.as_ref().and_then(|b| b.boards.as_ref().map(|(s, _)| *s)),
            self.bundle.as_ref().and_then(|b| b.terrain.as_ref().map(|(s, _)| *s)),
            self.bundle.as_ref().and_then(|b| b.tracks.as_ref().map(|(s, _)| *s)),
            self.vehicles_mesh.as_ref().map(|m| Arc::as_ptr(m) as usize as u64),
        );
        let new_shadow_key = (geo_sigs.0, geo_sigs.1, geo_sigs.2, geo_sigs.3, sun_bucket, self.env_mode as u8);
        if self.shadows_enabled {
            let need_shadow = match state.last_shadow_key {
                None => true,
                Some(prev) => {
                    let geom_changed = (prev.0, prev.1, prev.2, prev.3) != geo_sigs;
                    if geom_changed {
                        true
                    } else if prev.4 != sun_bucket || prev.5 != (self.env_mode as u8) {
                        if !self.day_night_active {
                            true
                        } else {
                            state
                                .last_shadow_time
                                .map_or(true, |t| t.elapsed() >= std::time::Duration::from_millis(500))
                        }
                    } else {
                        false
                    }
                }
            };
            if need_shadow {
                let mut spass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("railbench_3d_shadow_pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &state.shadow_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                spass.set_pipeline(&state.shadow_pipeline);
                spass.set_bind_group(0, &state.shadow_bg, &[]);
                state.slot_boards.draw(&mut spass, &state.identity_instance_buf);
                state.slot_terrain.draw(&mut spass, &state.identity_instance_buf);
                state.slot_tracks.draw(&mut spass, &state.identity_instance_buf);
                state.slot_vehicles.draw(&mut spass, &state.identity_instance_buf);
                if !state.slot_tracks.textured_draws.is_empty() {
                    spass.set_pipeline(&state.shadow_textured_pipeline);
                    spass.set_bind_group(0, &state.shadow_bg, &[]);
                    state.slot_tracks.draw_textured_shadow(&mut spass);
                }
                drop(spass);
                state.last_shadow_key = Some(new_shadow_key);
                state.last_shadow_time = Some(std::time::Instant::now());
            }
        }

        // Render the 3D scene into our 4x MSAA Color + Depth32Float buffer and resolve into color_view
        let offscreen_ref = if self.is_cab_view { &state.offscreen_cab } else { &state.offscreen };
        if let Some(ref target) = *offscreen_ref {
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

            rpass.set_pipeline(&state.sky_pipeline);
            rpass.set_bind_group(0, &state.shadow_bg, &[]);
            rpass.draw(0..3, 0..1);

            rpass.set_pipeline(&state.scene_pipeline);
            rpass.set_bind_group(0, &state.scene_bg, &[]);
            if self.uniforms.cam_pos[1] >= -700.0 || self.env_mode == Viewer3DEnv::Minimal {
                state.slot_env.draw(&mut rpass, &state.identity_instance_buf);
            }
            state.slot_boards.draw(&mut rpass, &state.identity_instance_buf);
            state.slot_terrain.draw(&mut rpass, &state.identity_instance_buf);
            state.slot_tracks.draw(&mut rpass, &state.identity_instance_buf);
            state.slot_vehicles.draw(&mut rpass, &state.identity_instance_buf);
            state.celestial_slot.draw(&mut rpass, &state.identity_instance_buf);

            if !state.slot_tracks.textured_draws.is_empty() {
                rpass.set_pipeline(&state.textured_pipeline);
                rpass.set_bind_group(0, &state.scene_bg, &[]);
                state.slot_tracks.draw_textured(&mut rpass, &state.texture_cache);
            }

            if self.uniforms.cam_pos[1] >= -700.0 || self.env_mode == Viewer3DEnv::Minimal {
                rpass.set_pipeline(&state.glass_pipeline);
                rpass.set_bind_group(0, &state.scene_bg, &[]);
                state.slot_env.draw_glass(&mut rpass, &state.identity_instance_buf);
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
            let offscreen_ref = if self.is_cab_view { &state.offscreen_cab } else { &state.offscreen };
            if let Some(ref target) = *offscreen_ref {
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

