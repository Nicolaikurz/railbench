use super::*;

pub(super) fn shade_top_view_vertex(
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    mat: [f32; 3],
    min_y: f32,
    max_y: f32,
) -> Color32 {
    let nx = normal[0];
    let ny = normal[1];
    let nz = normal[2];
    let n_len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-5);
    let (nx, ny, nz) = (nx / n_len, ny / n_len, nz / n_len);

    // Same primary sun & fill lighting directions as the 3D viewer (`viewer3d.rs`),
    // evaluated from directly overhead (`view_dir = (0, 1, 0)`).
    let sun_dir = [0.42_f32, 0.78, -0.46];
    let fill_dir = [-0.55_f32, 0.48, 0.68];
    let ndotl = (nx * sun_dir[0] + ny * sun_dir[1] + nz * sun_dir[2]).max(0.0);
    let ndotfill = (nx * fill_dir[0] + ny * fill_dir[1] + nz * fill_dir[2]).max(0.0);

    let hemi = (ny * 0.5 + 0.5).clamp(0.0, 1.0);
    let ambient = 0.34 + 0.22 * hemi;
    let diffuse = ndotl * 0.62 + ndotfill * 0.16;

    // Overhead half-vector between sun_dir and +Y (0, 1, 0)
    let hx = sun_dir[0];
    let hy = sun_dir[1] + 1.0;
    let hz = sun_dir[2];
    let h_len = (hx * hx + hy * hy + hz * hz).sqrt().max(1e-5);
    let ndoth = ((nx * hx + ny * hy + nz * hz) / h_len).max(0.0);
    let roughness = mat[0].clamp(0.08, 1.0);
    let metallic = mat[1].clamp(0.0, 1.0);
    let emissive = mat[2].clamp(0.0, 1.0);
    let spec_pow = (2.0 / (roughness * roughness).max(0.02)).clamp(4.0, 64.0);
    let spec = ndoth.powf(spec_pow) * (0.08 + 0.28 * metallic) * (1.0 - roughness * 0.65);

    // Subtle elevation ambient occlusion so recessed pits & lower plinths sit cleanly beneath coping/bridge/roofs
    let h_ratio = ((pos[1] - min_y) / (max_y - min_y).max(1.0)).clamp(0.0, 1.0);
    let ao = if min_y < -0.5 {
        0.78 + 0.28 * h_ratio
    } else {
        0.86 + 0.18 * h_ratio
    };

    let light = (ambient + diffuse) * ao;
    let r = (color[0] * (light + emissive * 0.85) + spec).clamp(0.0, 1.0);
    let g = (color[1] * (light + emissive * 0.85) + spec).clamp(0.0, 1.0);
    let b = (color[2] * (light + emissive * 0.85) + spec).clamp(0.0, 1.0);

    Color32::from_rgb(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

pub(super) fn project_3d_mesh_to_top_view_2d(
    verts: &[crate::viewer3d::Vertex3D],
    idxs: &[u32],
    min_y: f32,
    max_y: f32,
) -> CachedModelTopMesh2D {
    if idxs.len() < 3 {
        return CachedModelTopMesh2D::default();
    }

    // Collect visible triangles (skip faces pointing downward or nearly vertical invisible slivers)
    // and sort back-to-front by elevation (`y`) so coping, bridges, cabins & arches render cleanly over pits.
    let mut raw_tris: Vec<(f32, [[f32; 2]; 3], [Color32; 3])> = Vec::with_capacity(idxs.len() / 3);
    for tri in idxs.chunks_exact(3) {
        let (v0, v1, v2) = (
            &verts[tri[0] as usize],
            &verts[tri[1] as usize],
            &verts[tri[2] as usize],
        );
        let (n0, n1, n2) = (v0.normal_f32(), v1.normal_f32(), v2.normal_f32());
        // Compute geometric face normal Y component in 3D `(X, Y, Z)`
        let ux = v1.pos[0] - v0.pos[0];
        let uz = v1.pos[2] - v0.pos[2];
        let vx = v2.pos[0] - v0.pos[0];
        let vz = v2.pos[2] - v0.pos[2];
        let fn_y = uz * vx - ux * vz;
        let avg_ny = (n0[1] + n1[1] + n2[1]) / 3.0;
        // Cull strictly downward-facing underside triangles
        if fn_y < -1e-4 && avg_ny < -0.05 {
            continue;
        }
        // Check 2D projected area in XZ plane to skip zero-area vertical edge slivers
        let area2_xz = (ux * vz - uz * vx).abs();
        if area2_xz < 0.02 {
            continue;
        }
        let sort_y = (v0.pos[1] + v1.pos[1] + v2.pos[1]) / 3.0
            + v0.pos[1].max(v1.pos[1]).max(v2.pos[1]) * 0.35;
        let pts = [
            [v0.pos[0], v0.pos[2]],
            [v1.pos[0], v1.pos[2]],
            [v2.pos[0], v2.pos[2]],
        ];
        let cols = [
            shade_top_view_vertex(v0.pos, n0, v0.color_f32(), v0.mat_f32(), min_y, max_y),
            shade_top_view_vertex(v1.pos, n1, v1.color_f32(), v1.mat_f32(), min_y, max_y),
            shade_top_view_vertex(v2.pos, n2, v2.color_f32(), v2.mat_f32(), min_y, max_y),
        ];
        raw_tris.push((sort_y, pts, cols));
    }

    raw_tris.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let tris = raw_tris.into_iter().map(|(_, pts, cols)| (pts, cols)).collect();
    CachedModelTopMesh2D { tris }
}

pub(crate) fn build_model_top_view_mesh_2d(m: &Model3D) -> CachedModelTopMesh2D {
    let is_mtn = m.builtin_id.as_deref() == Some("mountain");
    let is_ground = m.builtin_id.as_deref() == Some("ground-terrain");
    if is_mtn || is_ground {
        return CachedModelTopMesh2D::default();
    }

    let scale = m.scale.unwrap_or(1.0) as f32;
    let w = (m.width as f32 * scale).max(4.0);
    let h = (m.height as f32 * scale).max(4.0);
    let d = (m.depth as f32 * scale).max(4.0);

    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    if m.builtin_id.is_some() {
        let mut local_m = m.clone();
        local_m.transform.x = 0.0;
        local_m.transform.y = 0.0;
        local_m.transform.rotation = 0.0;
        if local_m.builtin_id.as_deref() == Some("bahnsteig") {
            local_m.builtin_id = Some("platform".into());
        }
        crate::models3d_mesh::push_builtin_model_3d(&mut verts, &mut idxs, &local_m, 0.0);
    } else if m.file_data.is_some() || matches!(m.model_type.as_str(), "glb" | "gltf") {
        if let Some(parsed) = crate::model_tools::decode_custom_model_mesh(m) {
            let sx = (parsed.max[0] - parsed.min[0]).max(1e-4);
            let sy = (parsed.max[1] - parsed.min[1]).max(1e-4);
            let sz = (parsed.max[2] - parsed.min[2]).max(1e-4);
            let cx = (parsed.min[0] + parsed.max[0]) * 0.5;
            let cz = (parsed.min[2] + parsed.max[2]) * 0.5;
            let min_y = parsed.min[1];
            for tri in &parsed.triangles {
                let base = verts.len() as u32;
                for k in 0..3 {
                    let p = tri.positions[k];
                    let lx = ((p[0] - cx) / sx) * w;
                    let ly = ((p[1] - min_y) / sy) * h;
                    let lz = ((p[2] - cz) / sz) * d;
                    verts.push(crate::viewer3d::Vertex3D::new(
                        [lx, ly, lz],
                        tri.normals[k],
                        tri.color,
                        [0.65, 0.1, 0.0],
                    ));
                }
                idxs.extend_from_slice(&[base, base + 1, base + 2]);
            }
        }
    }

    project_3d_mesh_to_top_view_2d(&verts, &idxs, 0.0, h)
}

pub(crate) fn build_special_piece_top_view_mesh_2d(
    piece: &TrackPiece,
    entry: &CatalogEntry,
) -> CachedModelTopMesh2D {
    let mut local_p = piece.clone();
    local_p.transform = Transform {
        x: 0.0,
        y: 0.0,
        rotation: 0.0,
    };
    local_p.elevation = None;
    local_p.has_catenary = None;

    let mut verts = Vec::new();
    let mut idxs = Vec::new();
    match entry.kind.as_str() {
        "turntable" => {
            crate::turntables_3d::push_turntable_3d(&mut verts, &mut idxs, &local_p, Some(entry), 0.0);
        }
        "transfer-table" => {
            let empty_layout = Layout {
                name: String::new(),
                scale: String::new(),
                pieces: Vec::new(),
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
            crate::turntables_3d::push_transfer_table_3d(
                &mut verts,
                &mut idxs,
                &local_p,
                Some(entry),
                &empty_layout,
                0.0,
            );
        }
        _ => return CachedModelTopMesh2D::default(),
    }

    let min_y = verts.iter().map(|v| v.pos[1]).fold(0.0_f32, f32::min);
    let max_y = verts.iter().map(|v| v.pos[1]).fold(1.0_f32, f32::max);
    project_3d_mesh_to_top_view_2d(&verts, &idxs, min_y, max_y)
}

#[derive(Default)]
pub struct Canvas2DCache {
    pub(super) version: u64,
    pub(super) sig: u64,
    pub(super) initialized: bool,
    pub(super) pieces: Vec<CachedPieceGeom>,
    pub(super) piece_id_to_idx: HashMap<String, usize>,
    pub(super) model_bounds: Vec<(f64, f64, f64, f64)>,
    pub(super) model_top_meshes: Vec<Arc<CachedModelTopMesh2D>>,
    pub(super) model_proto_cache: HashMap<ModelTopMeshKey, Arc<CachedModelTopMesh2D>>,
    pub(super) piece_top_proto_cache: HashMap<(String, bool), Arc<CachedModelTopMesh2D>>,
    pub(super) layer_piece_indices: HashMap<String, Vec<usize>>,
    pub(super) layer_ground_models: HashMap<String, Vec<usize>>,
    pub(super) layer_top_models: HashMap<String, Vec<usize>>,
    pub(super) layer_flags: HashMap<String, (bool, bool)>,
    pub(super) layer_heights: HashMap<String, f64>,
    pub(super) level_mismatch_conns: HashSet<(usize, usize)>,
    pub(super) door_positions: Vec<(f64, f64, f64)>,
    pub(super) visible_doors: Vec<(f64, f64, u8)>,
    pub(super) baseboards: Vec<CachedBaseboard2D>,
    pub(super) layer_plates_built: bool,
    pub(super) layer_plates: HashMap<String, Vec<CachedLayerPlate2D>>,
}

pub(super) fn compute_layout_cache_sig(layout: &Layout, layout_version: u64) -> u64 {
    let mut h = layout_version.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= (layout.pieces.len() as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= (layout.models.as_ref().map_or(0, |m| m.len()) as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    if let Some(ls) = &layout.layers {
        for (i, l) in ls.iter().enumerate() {
            let bits = ((l.visible as u64) | ((l.locked as u64) << 1))
                ^ l.height_mm.map_or(0, |v| v.to_bits());
            h = h.rotate_left(7) ^ bits.wrapping_add(i as u64);
        }
    }
    for (i, b) in board_tools::get_baseboards_slice(layout).iter().enumerate() {
        if b.enabled {
            let bits = b.width.to_bits()
                ^ b.depth.to_bits().rotate_left(13)
                ^ b.custom_x.map_or(0, |v| v.to_bits().rotate_left(29))
                ^ b.custom_y.map_or(0, |v| v.to_bits().rotate_left(41));
            h = h.rotate_left(5) ^ bits.wrapping_add(i as u64);
        }
    }
    h
}

impl Canvas2DCache {
    pub(crate) fn model_top_mesh(&self, idx: usize) -> Option<&CachedModelTopMesh2D> {
        self.model_top_meshes.get(idx).map(|arc| arc.as_ref())
    }

    pub fn clear_proto_caches(&mut self) {
        self.model_proto_cache.clear();
        self.piece_top_proto_cache.clear();
        self.model_top_meshes.clear();
        self.initialized = false;
    }

    pub fn ensure_fresh(&mut self, layout: &Layout, catalogs: &[Catalog], layout_version: u64) {
        let sig = compute_layout_cache_sig(layout, layout_version);
        if self.initialized
            && self.version == layout_version
            && self.sig == sig
            && self.pieces.len() == layout.pieces.len()
        {
            return;
        }
        self.version = layout_version;
        self.sig = sig;
        self.initialized = true;
        self.layer_plates_built = false;
        self.layer_plates.clear();

        // 1. Layer flags & layer heights
        self.layer_flags.clear();
        self.layer_heights.clear();
        if let Some(ls) = &layout.layers {
            for l in ls {
                self.layer_flags.insert(l.id.clone(), (l.visible, l.visible && !l.locked));
                self.layer_heights.insert(l.id.clone(), l.height_mm.unwrap_or(0.0));
            }
        }

        // 2. Pieces geometry, AABBs, ID map & per-layer sorted indices (Parallelized across CPU threads)
        self.pieces.clear();
        self.pieces.reserve(layout.pieces.len());
        self.piece_id_to_idx.clear();
        self.piece_id_to_idx.reserve(layout.pieces.len());
        self.layer_piece_indices.clear();

        let eval_piece_geom = |piece: &TrackPiece| -> CachedPieceGeom {
            let entry = find_entry(catalogs, &piece.catalog_id);
            let branches = piece_branches_world(piece, entry);
            let (mut min_x, mut min_y, mut max_x, mut max_y) =
                (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for br in &branches {
                for p in br {
                    min_x = min_x.min(p.x);
                    min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x);
                    max_y = max_y.max(p.y);
                }
            }
            for c in &piece.connectors {
                min_x = min_x.min(c.x);
                min_y = min_y.min(c.y);
                max_x = max_x.max(c.x);
                max_y = max_y.max(c.y);
            }
            let hit_shape = match entry.map(|e| e.kind.as_str()) {
                Some("turntable") => {
                    let outer_r = crate::turntables_3d::turntable_outer_radius(piece, entry);
                    min_x = min_x.min(piece.transform.x - outer_r);
                    max_x = max_x.max(piece.transform.x + outer_r);
                    min_y = min_y.min(piece.transform.y - outer_r);
                    max_y = max_y.max(piece.transform.y + outer_r);
                    PieceHitShape::Circle {
                        cx: piece.transform.x,
                        cy: piece.transform.y,
                        radius: outer_r,
                    }
                }
                Some("transfer-table") => {
                    for corner in crate::turntables_3d::transfer_table_outer_world_corners(piece, entry) {
                        min_x = min_x.min(corner.x);
                        max_x = max_x.max(corner.x);
                        min_y = min_y.min(corner.y);
                        max_y = max_y.max(corner.y);
                    }
                    let (lx0, lx1, ly0, ly1) = crate::turntables_3d::transfer_table_local_bounds(entry);
                    PieceHitShape::OrientedRect {
                        origin: Vec2 {
                            x: piece.transform.x,
                            y: piece.transform.y,
                        },
                        rot_deg: piece.transform.rotation,
                        min_x: lx0,
                        max_x: lx1,
                        min_y: ly0,
                        max_y: ly1,
                    }
                }
                _ => PieceHitShape::BranchesOnly,
            };
            if !min_x.is_finite() {
                min_x = piece.transform.x;
                max_x = piece.transform.x;
                min_y = piece.transform.y;
                max_y = piece.transform.y;
            }
            let pad = if entry.and_then(|e| e.is_road).unwrap_or(false) {
                entry.and_then(|e| e.road_width).unwrap_or(68.0) * 0.5 + 36.0
            } else {
                (entry.and_then(|e| e.gauge).unwrap_or(16.5) * 1.5).max(36.0) + 36.0
            };
            CachedPieceGeom {
                branches,
                hit_shape,
                min_x: min_x - pad,
                min_y: min_y - pad,
                max_x: max_x + pad,
                max_y: max_y + pad,
            }
        };

        let hw_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        if hw_threads > 1 && layout.pieces.len() >= 64 {
            let chunk_size = (layout.pieces.len() + hw_threads - 1) / hw_threads;
            let chunk_vecs: Vec<Vec<CachedPieceGeom>> = std::thread::scope(|s| {
                let handles: Vec<_> = layout
                    .pieces
                    .chunks(chunk_size)
                    .map(|slice| {
                        s.spawn(move || slice.iter().map(eval_piece_geom).collect::<Vec<_>>())
                    })
                    .collect();
                handles.into_iter().filter_map(|h| h.join().ok()).collect()
            });
            for cv in chunk_vecs {
                self.pieces.extend(cv);
            }
        } else {
            for piece in &layout.pieces {
                self.pieces.push(eval_piece_geom(piece));
            }
        }

        let mut active_piece_keys: HashSet<(String, bool)> = HashSet::new();
        for piece in &layout.pieces {
            if let Some(entry) = find_entry(catalogs, &piece.catalog_id) {
                if matches!(entry.kind.as_str(), "turntable" | "transfer-table") {
                    let key = (piece.catalog_id.clone(), piece.has_tunnel == Some(true));
                    active_piece_keys.insert(key.clone());
                    self.piece_top_proto_cache
                        .entry(key)
                        .or_insert_with(|| Arc::new(build_special_piece_top_view_mesh_2d(piece, entry)));
                }
            }
        }
        self.piece_top_proto_cache
            .retain(|k, _| active_piece_keys.contains(k));

        for (idx, piece) in layout.pieces.iter().enumerate() {
            self.piece_id_to_idx.insert(piece.id.clone(), idx);
            let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
            if let Some(list) = self.layer_piece_indices.get_mut(lid) {
                list.push(idx);
            } else {
                self.layer_piece_indices.insert(lid.to_string(), vec![idx]);
            }
        }
        for indices in self.layer_piece_indices.values_mut() {
            indices.sort_by(|&ai, &bi| {
                let ea = layout.pieces[ai]
                    .elevation
                    .map(|e| (e.start + e.end) * 0.5)
                    .unwrap_or(0.0);
                let eb = layout.pieces[bi]
                    .elevation
                    .map(|e| (e.start + e.end) * 0.5)
                    .unwrap_or(0.0);
                ea.partial_cmp(&eb).unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // 3. Models AABBs, top-down 3D orthographic meshes, layer groups & building doors
        self.model_bounds.clear();
        self.model_top_meshes.clear();
        self.layer_ground_models.clear();
        self.layer_top_models.clear();
        self.door_positions.clear();
        self.visible_doors.clear();

        let mut active_model_keys: HashSet<ModelTopMeshKey> = HashSet::new();
        if let Some(models) = &layout.models {
            self.model_bounds.reserve(models.len());
            self.model_top_meshes.reserve(models.len());
            let mut m_doors = Vec::with_capacity(4);
            for (mi, m) in models.iter().enumerate() {
                let scale = m.scale.unwrap_or(1.0);
                let proto_key = ModelTopMeshKey {
                    kind_key: m
                        .builtin_id
                        .clone()
                        .unwrap_or_else(|| custom_model_content_key(m)),
                    w_mm10: (m.width * scale * 10.0).round() as i32,
                    d_mm10: (m.depth * scale * 10.0).round() as i32,
                    h_mm10: (m.height * scale * 10.0).round() as i32,
                };
                active_model_keys.insert(proto_key.clone());
                let top_mesh = Arc::clone(
                    self.model_proto_cache
                        .entry(proto_key)
                        .or_insert_with(|| Arc::new(build_model_top_view_mesh_2d(m))),
                );
                self.model_top_meshes.push(top_mesh);
                let hw = m.width * scale * 0.5;
                let hd = m.depth * scale * 0.5;
                let rot = m.transform.rotation;
                let (mut min_x, mut min_y, mut max_x, mut max_y) =
                    (m.transform.x, m.transform.y, m.transform.x, m.transform.y);
                for c in [
                    Vec2 { x: -hw, y: -hd },
                    Vec2 { x: hw, y: -hd },
                    Vec2 { x: hw, y: hd },
                    Vec2 { x: -hw, y: hd },
                ] {
                    let rc = rotate(c, rot);
                    let wx = m.transform.x + rc.x;
                    let wy = m.transform.y + rc.y;
                    min_x = min_x.min(wx);
                    min_y = min_y.min(wy);
                    max_x = max_x.max(wx);
                    max_y = max_y.max(wy);
                }
                if let Some(nodes) = &m.mountain_path {
                    for n in nodes {
                        let rp = rotate(
                            Vec2 {
                                x: n.x * scale,
                                y: n.y * scale,
                            },
                            rot,
                        );
                        let wx = m.transform.x + rp.x;
                        let wy = m.transform.y + rp.y;
                        let r = n.radius * scale;
                        min_x = min_x.min(wx - r);
                        max_x = max_x.max(wx + r);
                        min_y = min_y.min(wy - r);
                        max_y = max_y.max(wy + r);
                    }
                }
                self.model_bounds
                    .push((min_x - 32.0, min_y - 32.0, max_x + 32.0, max_y + 32.0));

                let lid = m.layer_id.as_deref().unwrap_or("layer-1");
                let target_map = if m.builtin_id.as_deref() == Some("ground-terrain") {
                    &mut self.layer_ground_models
                } else {
                    &mut self.layer_top_models
                };
                if let Some(list) = target_map.get_mut(lid) {
                    list.push(mi);
                } else {
                    target_map.insert(lid.to_string(), vec![mi]);
                }

                m_doors.clear();
                crate::building_snap::collect_model_door_positions(layout, m, &mut m_doors);
                if !m_doors.is_empty() {
                    let vis = self.layer_flags.get(lid).map_or(true, |&(v, _)| v);
                    for &(dx, dy, dh) in &m_doors {
                        self.door_positions.push((dx, dy, dh));
                        if vis {
                            let mut door_state = 0_u8;
                            for (pi, pg) in self.pieces.iter().enumerate() {
                                let piece = &layout.pieces[pi];
                                let p_lid = piece.layer_id.as_deref().unwrap_or("layer-1");
                                if !self.layer_flags.get(p_lid).map_or(true, |&(v, _)| v) {
                                    continue;
                                }
                                if crate::catalog::is_road_id(catalogs, &piece.catalog_id) {
                                    continue;
                                }
                                if !point_in_padded_aabb((pg.min_x, pg.min_y, pg.max_x, pg.max_y), dx, dy, 18.0) {
                                    continue;
                                }
                                let p_lh = self.layer_heights.get(p_lid).copied().unwrap_or(0.0);
                                for (ci, c) in piece.connectors.iter().enumerate() {
                                    let ddx = c.x - dx;
                                    let ddy = c.y - dy;
                                    let d_sq = ddx * ddx + ddy * ddy;
                                    if d_sq <= 324.0 {
                                        let ch = railbench_core::snapping::piece_connector_height(piece, ci, p_lh);
                                        if d_sq <= 30.25 && (ch - dh).abs() <= 1.0 {
                                            door_state = 1;
                                            break;
                                        } else if (ch - dh).abs() > 1.0 && door_state == 0 {
                                            door_state = 2;
                                        }
                                    }
                                }
                                if door_state == 1 {
                                    break;
                                }
                            }
                            self.visible_doors.push((dx, dy, door_state));
                        }
                    }
                }
            }
        }
        self.model_proto_cache
            .retain(|k, _| active_model_keys.contains(k));

        // 3.5. Detect open connectors near another track connector, overhead track body, or building door
        //      at a different vertical level (> 1.0 mm apart) so they can be marked yellow in 2D.
        self.level_mismatch_conns.clear();
        if !layout.pieces.is_empty() {
            let cell_sz = 24.0_f64;
            let inv_c = 1.0 / cell_sz;
            // Grid entry: (x, y, h, piece_idx, is_road)
            let mut all_conn_grid: HashMap<(i32, i32), Vec<(f64, f64, f64, usize, bool)>> =
                HashMap::with_capacity(layout.pieces.len() * 2);
            let mut piece_vis = vec![false; layout.pieces.len()];
            let mut piece_is_road = vec![false; layout.pieces.len()];
            let mut piece_max_h = vec![f64::NEG_INFINITY; layout.pieces.len()];

            for (pidx, p) in layout.pieces.iter().enumerate() {
                let lid = p.layer_id.as_deref().unwrap_or("layer-1");
                let vis = self.layer_flags.get(lid).map_or(true, |&(v, _)| v);
                piece_vis[pidx] = vis;
                let is_road = crate::catalog::is_road_id(catalogs, &p.catalog_id);
                piece_is_road[pidx] = is_road;
                let lh = self.layer_heights.get(lid).copied().unwrap_or(0.0);
                let mut max_h = lh + p.elevation.map(|e| e.start.max(e.end)).unwrap_or(0.0);
                if vis {
                    for (cidx, c) in p.connectors.iter().enumerate() {
                        let h = railbench_core::snapping::piece_connector_height(p, cidx, lh);
                        if h > max_h {
                            max_h = h;
                        }
                        let gx = (c.x * inv_c).floor() as i32;
                        let gy = (c.y * inv_c).floor() as i32;
                        all_conn_grid
                            .entry((gx, gy))
                            .or_default()
                            .push((c.x, c.y, h, pidx, is_road));
                    }
                }
                piece_max_h[pidx] = max_h;
            }

            for (p1, piece) in layout.pieces.iter().enumerate() {
                if !piece_vis[p1] {
                    continue;
                }
                let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
                let lh = self.layer_heights.get(lid).copied().unwrap_or(0.0);
                let r1 = piece_is_road[p1];

                for (c1, conn1) in piece.connectors.iter().enumerate() {
                    if conn1.connected_to.is_some() {
                        continue;
                    }
                    let x1 = conn1.x;
                    let y1 = conn1.y;
                    let h1 = railbench_core::snapping::piece_connector_height(piece, c1, lh);

                    // 1. Check nearby connectors (connected or open) at different height
                    let gx = (x1 * inv_c).floor() as i32;
                    let gy = (y1 * inv_c).floor() as i32;
                    let mut mismatch = false;
                    'conn_search: for ox in -1..=1 {
                        for oy in -1..=1 {
                            let Some(nb) = all_conn_grid.get(&(gx + ox, gy + oy)) else {
                                continue;
                            };
                            for &(x2, y2, h2, p2, r2) in nb {
                                if p2 == p1 || r2 != r1 {
                                    continue;
                                }
                                let dx = x1 - x2;
                                let dy = y1 - y2;
                                if dx * dx + dy * dy <= 324.0 && (h1 - h2).abs() > 1.0 {
                                    mismatch = true;
                                    break 'conn_search;
                                }
                            }
                        }
                    }

                    // 2. Else check overhead track segments passing above this open connector
                    if !mismatch {
                        let wp = Vec2 { x: x1, y: y1 };
                        for (p2, pg2) in self.pieces.iter().enumerate() {
                            if p2 == p1 || !piece_vis[p2] || piece_is_road[p2] != r1 {
                                continue;
                            }
                            if piece_max_h[p2] <= h1 + 1.0 {
                                continue;
                            }
                            if !point_in_padded_aabb((pg2.min_x, pg2.min_y, pg2.max_x, pg2.max_y), x1, y1, 20.0) {
                                continue;
                            }
                            if pg2
                                .branches
                                .iter()
                                .any(|br| br.windows(2).any(|w| point_to_segment_dist_sq(wp, w[0], w[1]) <= 400.0))
                            {
                                mismatch = true;
                                break;
                            }
                        }
                    }

                    // 3. Else check building doors at a different height
                    if !mismatch && !r1 {
                        if self.door_positions.iter().any(|&(dx, dy, dh)| {
                            let ddx = x1 - dx;
                            let ddy = y1 - dy;
                            ddx * ddx + ddy * ddy <= 324.0 && (h1 - dh).abs() > 1.0
                        }) {
                            mismatch = true;
                        }
                    }

                    if mismatch {
                        self.level_mismatch_conns.insert((p1, c1));
                    }
                }
            }
        }

        // 4. Baseboard polygons, track cutouts & ear-clipped triangulations (Parallelized when multiple boards exist)
        self.baseboards.clear();
        let enabled_boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
            .into_iter()
            .filter(|b| b.enabled)
            .collect();
        let build_cached_bb = |b: BaseboardConfig| -> Option<CachedBaseboard2D> {
            let poly = board_tools::get_baseboard_polygon(&b);
            if poly.len() < 3 {
                return None;
            }
            let bounds = board_tools::get_baseboard_bounds(&b);
            let cutouts: Vec<TrackCutoutPolygon> =
                board_tools::compute_table_track_cutouts(layout, catalogs, &b, 0.0, 20.0);
            let holes: Vec<Vec<Vec2>> = cutouts.into_iter().map(|c| c.points).collect();
            let tris = board_tools::triangulate_polygon_with_holes(&poly, &holes);
            let hole_tris: Vec<Vec<[Vec2; 3]>> = holes
                .iter()
                .map(|h| board_tools::triangulate_polygon_with_holes(h, &[]))
                .collect();
            let anchor = poly
                .iter()
                .min_by(|a, b_pt| {
                    (a.y + a.x * 0.25)
                        .partial_cmp(&(b_pt.y + b_pt.x * 0.25))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(Vec2 { x: bounds.x, y: bounds.y });
            Some(CachedBaseboard2D {
                id: b.id.unwrap_or_else(|| "board-1".into()),
                name: b.name,
                style: b.style,
                show_dimensions: b.show_dimensions,
                bounds,
                anchor,
                mesh: CachedTriMesh {
                    outer: poly,
                    holes,
                    tris,
                },
                hole_tris,
            })
        };
        if hw_threads > 1 && enabled_boards.len() >= 2 {
            let built: Vec<Option<CachedBaseboard2D>> = std::thread::scope(|s| {
                let handles: Vec<_> = enabled_boards
                    .into_iter()
                    .map(|b| s.spawn(move || build_cached_bb(b)))
                    .collect();
                handles.into_iter().filter_map(|h| h.join().ok()).collect()
            });
            self.baseboards.extend(built.into_iter().flatten());
        } else {
            for b in enabled_boards {
                if let Some(cb) = build_cached_bb(b) {
                    self.baseboards.push(cb);
                }
            }
        }
    }

    pub(super) fn ensure_layer_plates(&mut self, layout: &Layout, catalogs: &[Catalog]) {
        if self.layer_plates_built {
            return;
        }
        self.layer_plates_built = true;
        self.layer_plates.clear();

        let Some(layers) = &layout.layers else {
            return;
        };
        let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
            .into_iter()
            .filter(|b| b.enabled)
            .collect();
        if boards.is_empty() {
            return;
        }

        for layer in layers {
            if !layer.visible {
                continue;
            }
            let lh = layer.height_mm.unwrap_or(0.0);
            let mut cached_list = Vec::new();
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
                        let thick = plate
                            .thickness_mm
                            .or(layer.wood_plate_thickness_mm)
                            .unwrap_or(12.0);
                        let holes: Vec<Vec<Vec2>> =
                            board_tools::compute_table_track_cutouts(layout, catalogs, &pbb, lh, thick)
                                .into_iter()
                                .map(|c| c.points)
                                .collect();
                        let tris = board_tools::triangulate_polygon_with_holes(&poly, &holes);
                        let centroid = poly_centroid(&poly);
                        cached_list.push(CachedLayerPlate2D {
                            style: plate.style.as_deref().unwrap_or("plywood").to_string(),
                            name: plate.name.clone(),
                            centroid,
                            is_table_mode: false,
                            mesh: CachedTriMesh {
                                outer: poly,
                                holes,
                                tris,
                            },
                        });
                    }
                    self.layer_plates.insert(layer.id.clone(), cached_list);
                    continue;
                }
            }

            let mode = layer.wood_plate_mode.as_deref().unwrap_or("none");
            if mode == "table" && lh.abs() >= 1e-6 {
                let thick = layer.wood_plate_thickness_mm.unwrap_or(12.0);
                let style = layer
                    .wood_plate_style
                    .as_deref()
                    .unwrap_or("plywood")
                    .to_string();
                for b in &boards {
                    let poly = board_tools::get_baseboard_polygon(b);
                    let holes: Vec<Vec<Vec2>> =
                        board_tools::compute_table_track_cutouts(layout, catalogs, b, lh, thick)
                            .into_iter()
                            .map(|c| c.points)
                            .collect();
                    let tris = board_tools::triangulate_polygon_with_holes(&poly, &holes);
                    let centroid = poly_centroid(&poly);
                    cached_list.push(CachedLayerPlate2D {
                        style: style.clone(),
                        name: None,
                        centroid,
                        is_table_mode: true,
                        mesh: CachedTriMesh {
                            outer: poly,
                            holes,
                            tris,
                        },
                    });
                }
                self.layer_plates.insert(layer.id.clone(), cached_list);
            }
        }
    }
}

