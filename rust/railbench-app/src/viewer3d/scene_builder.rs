use super::*;

pub fn update_scene_3d_cache(
    layout: &Layout,
    catalogs: &[Catalog],
    opts: &Scene3DOptions,
    cache: &mut Scene3DCache,
) -> Scene3DBundle {
    // Layer height, visibility & color lookup
    let mut layer_info: std::collections::HashMap<&str, (f64, bool)> = std::collections::HashMap::new();
    let mut layer_colors: std::collections::HashMap<&str, [f32; 3]> = std::collections::HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_info.insert(l.id.as_str(), (l.height_mm.unwrap_or(0.0), l.visible));
            if let Some(col_str) = l.color.as_deref() {
                if let Some(rgb) = parse_hex_color_f32(col_str) {
                    layer_colors.insert(l.id.as_str(), rgb);
                }
            }
        }
    }

    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();

    let (sig_env, sig_boards, sig_terrain, sig_tracks) = compute_submesh_signatures(layout, opts, &boards);
    let need_env = cache.env.as_ref().map_or(true, |(s, _)| *s != sig_env);
    let need_boards = cache.boards.as_ref().map_or(true, |(s, _)| *s != sig_boards);
    let need_terrain = cache.terrain.as_ref().map_or(true, |(s, _)| *s != sig_terrain);
    let need_tracks = cache.tracks_and_models.as_ref().map_or(true, |(s, _)| *s != sig_tracks);

    let hw_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    cache.last_build_threads = hw_threads;

    let boards_ref = &boards;
    let layer_info_ref = &layer_info;
    let layer_colors_ref = &layer_colors;

    // Build all dirty 3D stages (Environment, Baseboards/Wood Plates, Terrain/Mountains, and Tracks/Buildings) concurrently across CPU threads!
    let (new_env, new_boards, new_terrain, new_tracks) = std::thread::scope(|s| {
        let env_handle = if need_env {
            Some(s.spawn(move || {
                let mut verts = Vec::with_capacity(2048);
                let mut idxs = Vec::with_capacity(4096);
                match opts.env_mode {
                    Viewer3DEnv::HallLandscape => {
                        push_hall_landscape_env_3d(&mut verts, &mut idxs, boards_ref, opts.dark_mode)
                    }
                    Viewer3DEnv::Minimal => {}
                }
                SceneMesh3D { vertices: verts, indices: idxs, instanced_tracks: Vec::new() }
            }))
        } else {
            None
        };

        let boards_handle = if need_boards {
            Some(s.spawn(move || {
                let mut verts = Vec::with_capacity(2048);
                let mut idxs = Vec::with_capacity(4096);
                let table_layer = layout.layers.as_ref().and_then(|ls| {
                    ls.iter().find(|l| l.height_mm.unwrap_or(0.0).abs() < 1e-6)
                });
                let table_visible = table_layer.map_or(true, |l| l.visible);
                if table_visible {
                    for b in boards_ref {
                        let poly = board_tools::get_baseboard_polygon(b);
                        if poly.len() < 3 {
                            continue;
                        }
                        let matching_plate = table_layer.and_then(|tl| {
                            tl.plates.as_ref().and_then(|ps| {
                                ps.iter().find(|pl| {
                                    pl.baseboard_id.as_deref().or_else(|| boards_ref.first().and_then(|fb| fb.id.as_deref()))
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
                        holes.extend(collect_turntable_holes(layout, catalogs, &poly, 0.0, layer_info_ref));

                        push_extruded_polygon_3d(&mut verts, &mut idxs, &poly, &holes, 0.0, thick, top_col, side_col);

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
                                            .and_then(|bid| boards_ref.iter().find(|b| b.id.as_deref() == Some(bid)))
                                            .or_else(|| boards_ref.first());
                                        let poly = board_tools::get_layer_plate_polygon(plate, target_board);
                                        if poly.len() < 3 {
                                            continue;
                                        }
                                        let pbb = board_tools::layer_plate_as_baseboard(plate, target_board);
                                        let pthick = plate.thickness_mm.unwrap_or(lthick);
                                        let cutouts = board_tools::compute_table_track_cutouts(layout, catalogs, &pbb, lh, pthick);
                                        let mut holes: Vec<Vec<Vec2>> = cutouts.iter().map(|c| c.points.clone()).collect();
                                        holes.extend(collect_turntable_holes(layout, catalogs, &poly, lh, layer_info_ref));
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
                                for b in boards_ref {
                                    let poly = board_tools::get_baseboard_polygon(b);
                                    let cutouts = board_tools::compute_table_track_cutouts(layout, catalogs, b, lh, lthick);
                                    let mut holes: Vec<Vec<Vec2>> = cutouts.iter().map(|c| c.points.clone()).collect();
                                    holes.extend(collect_turntable_holes(layout, catalogs, &poly, lh, layer_info_ref));
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
                                    let entry = find_entry(catalogs, &piece.catalog_id);
                                    for br in sample_track_branches_3d(piece, entry, e0, e1) {
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
                SceneMesh3D { vertices: verts, indices: idxs, instanced_tracks: Vec::new() }
            }))
        } else {
            None
        };

        let terrain_handle = if need_terrain {
            Some(s.spawn(move || {
                let mut terrain_mesh = SceneMesh3D::default();
                if let Some(models) = &layout.models {
                    let terrain_tasks: Vec<(&Model3D, f32, bool)> = models
                        .iter()
                        .filter_map(|m| {
                            let is_mtn = match m.builtin_id.as_deref() {
                                Some("mountain") => true,
                                Some("ground-terrain") => false,
                                _ => return None,
                            };
                            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
                            let (lh, vis) = layer_info_ref.get(lid).copied().unwrap_or((0.0, true));
                            if !vis {
                                return None;
                            }
                            let y0 = (lh + m.elevation.unwrap_or(0.0)) as f32;
                            Some((m, y0, is_mtn))
                        })
                        .collect();

                    if !terrain_tasks.is_empty() {
                        let chains = build_connected_corridor_chains(layout, catalogs, layer_info_ref);
                        let all_mountains: Vec<&Model3D> = models
                            .iter()
                            .filter(|m| m.builtin_id.as_deref() == Some("mountain"))
                            .collect();

                        if terrain_tasks.len() >= 2 {
                            let sub_meshes: Vec<SceneMesh3D> = std::thread::scope(|s_terr| {
                                let handles: Vec<_> = terrain_tasks
                                    .iter()
                                    .map(|&(m, y0, is_mtn)| {
                                        let chains_ref = &chains;
                                        let all_mtns_ref = &all_mountains;
                                        s_terr.spawn(move || {
                                            let mut v = Vec::with_capacity(4096);
                                            let mut idx = Vec::with_capacity(16384);
                                            if is_mtn {
                                                push_mountain_with_tunnels_3d(
                                                    &mut v,
                                                    &mut idx,
                                                    m,
                                                    y0,
                                                    layout,
                                                    boards_ref,
                                                    chains_ref,
                                                    all_mtns_ref,
                                                    layer_info_ref,
                                                );
                                            } else {
                                                push_ground_terrain_3d(
                                                    &mut v,
                                                    &mut idx,
                                                    m,
                                                    y0,
                                                    layout,
                                                    layer_info_ref,
                                                    boards_ref,
                                                    chains_ref,
                                                );
                                            }
                                            SceneMesh3D { vertices: v, indices: idx, instanced_tracks: Vec::new() }
                                        })
                                    })
                                    .collect();
                                handles.into_iter().filter_map(|h| h.join().ok()).collect()
                            });
                            for sm in &sub_meshes {
                                terrain_mesh.append_from(sm);
                            }
                        } else {
                            let (m, y0, is_mtn) = terrain_tasks[0];
                            if is_mtn {
                                push_mountain_with_tunnels_3d(
                                    &mut terrain_mesh.vertices,
                                    &mut terrain_mesh.indices,
                                    m,
                                    y0,
                                    layout,
                                    boards_ref,
                                    &chains,
                                    &all_mountains,
                                    layer_info_ref,
                                );
                            } else {
                                push_ground_terrain_3d(
                                    &mut terrain_mesh.vertices,
                                    &mut terrain_mesh.indices,
                                    m,
                                    y0,
                                    layout,
                                    layer_info_ref,
                                    boards_ref,
                                    &chains,
                                );
                            }
                        }
                    }
                }
                terrain_mesh
            }))
        } else {
            None
        };

        let new_tracks = if need_tracks {
            let est_verts = layout.models.as_ref().map_or(0, |ms| ms.len() * 220) + 1024;
            let mut verts = Vec::with_capacity(est_verts);
            let mut idxs = Vec::with_capacity(est_verts * 2);

            let visible_models: Vec<(&Model3D, f32)> = layout
                .models
                .as_ref()
                .map(|models| {
                    models
                        .iter()
                        .filter_map(|m| {
                            if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                                return None;
                            }
                            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
                            let (lh, vis) = layer_info_ref.get(lid).copied().unwrap_or((0.0, true));
                            if !vis {
                                return None;
                            }
                            Some((m, (lh + m.elevation.unwrap_or(0.0)) as f32))
                        })
                        .collect()
                })
                .unwrap_or_default();

            let piece_map: std::collections::HashMap<&str, &TrackPiece> = layout
                .pieces
                .iter()
                .map(|p| (p.id.as_str(), p))
                .collect();
            let piece_map_ref = &piece_map;

            // Partition visible pieces into:
            // 1. Instanceable flat standard tracks/roads/turnouts/buffers: ONE shared 3D prototype mesh per unique catalog_id + per-instance transform buffer!
            // 2. Non-catenary custom/sloped/bridged/tunneled/signaled pieces (baked in parallel chunks)
            // 3. Catenary pieces (share CatenaryMast3DContext for joint mast deduplication)
            let mut inst_order: Vec<(&str, &CatalogEntry, bool)> = Vec::new();
            let mut inst_groups: std::collections::HashMap<&str, Vec<Instance3D>> =
                std::collections::HashMap::new();
            let mut non_cat_pieces: Vec<(&TrackPiece, Option<&CatalogEntry>, bool, f64, f64, Option<[f32; 3]>)> =
                Vec::new();
            let mut cat_pieces: Vec<(&TrackPiece, Option<&CatalogEntry>, bool, f64, f64, Option<[f32; 3]>)> =
                Vec::new();

            for piece in &layout.pieces {
                let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
                let (lh, vis) = layer_info_ref.get(lid).copied().unwrap_or((0.0, true));
                if !vis {
                    continue;
                }
                let e0 = lh + piece.elevation.map(|e| e.start).unwrap_or(0.0);
                let e1 = lh + piece.elevation.map(|e| e.end).unwrap_or(0.0);
                let entry = find_entry(catalogs, &piece.catalog_id);
                let is_road = is_road_id(catalogs, &piece.catalog_id);
                let layer_col = layer_colors_ref.get(lid).copied();

                if is_instanceable_track_piece(piece, entry, e0, e1) {
                    let e = entry.unwrap();
                    let cid = e.id.as_str();
                    let list = inst_groups.entry(cid).or_insert_with(|| {
                        inst_order.push((cid, e, is_road));
                        Vec::new()
                    });
                    list.push(Instance3D {
                        pos_rot: [
                            piece.transform.x as f32,
                            e0 as f32,
                            piece.transform.y as f32,
                            (piece.transform.rotation as f32).to_radians(),
                        ],
                        tint_w: compute_track_instance_tint(e, opts.color_mode, layer_col),
                    });
                } else if piece.has_catenary == Some(true) {
                    cat_pieces.push((piece, entry, is_road, e0, e1, layer_col));
                } else {
                    non_cat_pieces.push((piece, entry, is_road, e0, e1, layer_col));
                }
            }

            // Build any missing unique track prototypes in parallel (1 mesh per track type!)
            let missing_protos: Vec<(&str, &CatalogEntry, bool)> = inst_order
                .iter()
                .copied()
                .filter(|&(cid, _, _)| !cache.track_prototypes.contains_key(cid))
                .collect();
            if !missing_protos.is_empty() {
                if hw_threads > 1 && missing_protos.len() >= 2 {
                    let built: Vec<(String, (Arc<Vec<Vertex3D>>, Arc<Vec<u32>>))> = std::thread::scope(|s_p| {
                        let handles: Vec<_> = missing_protos
                            .iter()
                            .map(|&(cid, entry, is_road)| {
                                s_p.spawn(move || {
                                    (
                                        cid.to_string(),
                                        build_local_track_prototype_3d(entry, is_road, layout),
                                    )
                                })
                            })
                            .collect();
                        handles.into_iter().filter_map(|h| h.join().ok()).collect()
                    });
                    for (cid, proto_mesh) in built {
                        cache.track_prototypes.insert(cid, proto_mesh);
                    }
                } else {
                    for (cid, entry, is_road) in missing_protos {
                        let proto_mesh = build_local_track_prototype_3d(entry, is_road, layout);
                        cache.track_prototypes.insert(cid.to_string(), proto_mesh);
                    }
                }
            }

            let mut instanced_tracks: Vec<InstancedTrackPrototype3D> =
                Vec::with_capacity(inst_order.len());
            let mut active_proto_ids: std::collections::HashSet<String> =
                std::collections::HashSet::with_capacity(inst_order.len());
            for (cid, _, _) in inst_order {
                if let (Some((pv, pi)), Some(instances)) =
                    (cache.track_prototypes.get(cid), inst_groups.remove(cid))
                {
                    if !instances.is_empty() && !pv.is_empty() && !pi.is_empty() {
                        active_proto_ids.insert(cid.to_string());
                        instanced_tracks.push(InstancedTrackPrototype3D {
                            catalog_id: cid.to_string(),
                            vertices: Arc::clone(pv),
                            indices: Arc::clone(pi),
                            instances,
                        });
                    }
                }
            }
            cache
                .track_prototypes
                .retain(|cid, _| active_proto_ids.contains(cid));

            // Parallelize non-terrain 3D building/model meshes alongside chunked non-catenary custom/sloped track meshes
            std::thread::scope(|s_trk| {
                let model_chunks_handle = if hw_threads > 1 && visible_models.len() >= 2 {
                    let chunk_sz = (visible_models.len() + hw_threads - 1) / hw_threads;
                    let handles: Vec<_> = visible_models
                        .chunks(chunk_sz)
                        .map(|chunk| {
                            s_trk.spawn(move || {
                                let mut mv = Vec::with_capacity(chunk.len() * 320);
                                let mut mi = Vec::with_capacity(chunk.len() * 640);
                                for &(m, y0) in chunk {
                                    push_model_3d(&mut mv, &mut mi, m, y0);
                                }
                                SceneMesh3D { vertices: mv, indices: mi, instanced_tracks: Vec::new() }
                            })
                        })
                        .collect();
                    Some(handles)
                } else {
                    None
                };

                let track_chunks_handle = if hw_threads > 1 && non_cat_pieces.len() >= 16 {
                    let chunk_sz = (non_cat_pieces.len() + hw_threads - 1) / hw_threads;
                    let handles: Vec<_> = non_cat_pieces
                        .chunks(chunk_sz)
                        .map(|chunk| {
                            s_trk.spawn(move || {
                                let mut tv = Vec::with_capacity(chunk.len() * 160);
                                let mut ti = Vec::with_capacity(chunk.len() * 320);
                                let mut dummy_cat = CatenaryMast3DContext::default();
                                for &(piece, entry, is_road, e0, e1, layer_col) in chunk {
                                    push_track_piece_3d(
                                        &mut tv,
                                        &mut ti,
                                        piece,
                                        entry,
                                        layout,
                                        is_road,
                                        e0,
                                        e1,
                                        opts.color_mode,
                                        layer_col,
                                        opts.turnout_states.get(&piece.id).copied().unwrap_or(0),
                                        opts.turntable_angles_deg.get(&piece.id).copied().unwrap_or(0.0),
                                        opts.transfer_offsets_mm.get(&piece.id).copied(),
                                        &mut dummy_cat,
                                    );
                                    if piece.has_tunnel == Some(true) && piece.connectors.len() < 6 {
                                        crate::tunnels_3d::push_piece_tunnel_3d(
                                            &mut tv,
                                            &mut ti,
                                            piece,
                                            entry,
                                            is_road,
                                            e0,
                                            e1,
                                            layout,
                                            layer_info_ref,
                                            piece_map_ref,
                                        );
                                    }
                                }
                                SceneMesh3D { vertices: tv, indices: ti, instanced_tracks: Vec::new() }
                            })
                        })
                        .collect();
                    Some(handles)
                } else {
                    None
                };

                if let Some(handles) = model_chunks_handle {
                    for h in handles {
                        if let Ok(sm) = h.join() {
                            if !sm.vertices.is_empty() {
                                let base = verts.len() as u32;
                                verts.extend_from_slice(&sm.vertices);
                                idxs.extend(sm.indices.into_iter().map(|i| base + i));
                            }
                        }
                    }
                } else {
                    for &(m, y0) in &visible_models {
                        push_model_3d(&mut verts, &mut idxs, m, y0);
                    }
                }

                if let Some(handles) = track_chunks_handle {
                    for h in handles {
                        if let Ok(sm) = h.join() {
                            if !sm.vertices.is_empty() {
                                let base = verts.len() as u32;
                                verts.extend_from_slice(&sm.vertices);
                                idxs.extend(sm.indices.into_iter().map(|i| base + i));
                            }
                        }
                    }
                } else {
                    let mut dummy_cat = CatenaryMast3DContext::default();
                    for &(piece, entry, is_road, e0, e1, layer_col) in &non_cat_pieces {
                        push_track_piece_3d(
                            &mut verts,
                            &mut idxs,
                            piece,
                            entry,
                            layout,
                            is_road,
                            e0,
                            e1,
                            opts.color_mode,
                            layer_col,
                            opts.turnout_states.get(&piece.id).copied().unwrap_or(0),
                            opts.turntable_angles_deg.get(&piece.id).copied().unwrap_or(0.0),
                            opts.transfer_offsets_mm.get(&piece.id).copied(),
                            &mut dummy_cat,
                        );
                        if piece.has_tunnel == Some(true) && piece.connectors.len() < 6 {
                            crate::tunnels_3d::push_piece_tunnel_3d(
                                &mut verts,
                                &mut idxs,
                                piece,
                                entry,
                                is_road,
                                e0,
                                e1,
                                layout,
                                layer_info_ref,
                                piece_map_ref,
                            );
                        }
                    }
                }

                // Catenary pieces share CatenaryMast3DContext so masts at shared joints are deduplicated
                let mut cat_ctx = CatenaryMast3DContext::default();
                for (piece, entry, is_road, e0, e1, layer_col) in cat_pieces {
                    push_track_piece_3d(
                        &mut verts,
                        &mut idxs,
                        piece,
                        entry,
                        layout,
                        is_road,
                        e0,
                        e1,
                        opts.color_mode,
                        layer_col,
                        opts.turnout_states.get(&piece.id).copied().unwrap_or(0),
                        opts.turntable_angles_deg.get(&piece.id).copied().unwrap_or(0.0),
                        opts.transfer_offsets_mm.get(&piece.id).copied(),
                        &mut cat_ctx,
                    );
                    if piece.has_tunnel == Some(true) && piece.connectors.len() < 6 {
                        crate::tunnels_3d::push_piece_tunnel_3d(
                            &mut verts,
                            &mut idxs,
                            piece,
                            entry,
                            is_road,
                            e0,
                            e1,
                            layout,
                            layer_info_ref,
                            piece_map_ref,
                        );
                    }
                }
            });

            for sc in &opts.short_circuit_points {
                push_box(&mut verts, &mut idxs, [sc.x as f32, 34.0, sc.y as f32], [6.0, 68.0, 6.0], [0.95, 0.15, 0.15]);
                push_box(&mut verts, &mut idxs, [sc.x as f32, 74.0, sc.y as f32], [20.0, 20.0, 20.0], [1.0, 0.30, 0.12]);
            }

            Some(SceneMesh3D {
                vertices: verts,
                indices: idxs,
                instanced_tracks,
            })
        } else {
            None
        };

        (
            env_handle.and_then(|h| h.join().ok()),
            boards_handle.and_then(|h| h.join().ok()),
            terrain_handle.and_then(|h| h.join().ok()),
            new_tracks,
        )
    });

    if let Some(m) = new_env {
        cache.env = Some((sig_env, Arc::new(m)));
        cache.env_rebuilds += 1;
    }
    if let Some(m) = new_boards {
        cache.boards = Some((sig_boards, Arc::new(m)));
        cache.boards_rebuilds += 1;
    }
    if let Some(m) = new_terrain {
        cache.terrain = Some((sig_terrain, Arc::new(m)));
        cache.terrain_rebuilds += 1;
    }
    if let Some(m) = new_tracks {
        cache.tracks_and_models = Some((sig_tracks, Arc::new(m)));
        cache.tracks_rebuilds += 1;
    }

    Scene3DBundle {
        env: cache.env.as_ref().map(|(s, m)| (*s, Arc::clone(m))),
        boards: cache.boards.as_ref().map(|(s, m)| (*s, Arc::clone(m))),
        terrain: cache.terrain.as_ref().map(|(s, m)| (*s, Arc::clone(m))),
        tracks: cache.tracks_and_models.as_ref().map(|(s, m)| (*s, Arc::clone(m))),
    }
}

