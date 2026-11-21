use super::*;

pub(super) fn is_instanceable_track_piece(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    e0: f64,
    e1: f64,
) -> bool {
    let Some(e) = entry else { return false };
    if matches!(e.kind.as_str(), "turntable" | "transfer-table") {
        return false;
    }
    if (e1 - e0).abs() > 0.01 {
        return false;
    }
    if piece.has_bridge == Some(true)
        || piece.has_tunnel == Some(true)
        || piece.has_catenary == Some(true)
        || piece.catenary_power_feed == Some(true)
        || piece.catenary_isolator == Some(true)
        || piece.has_lantern == Some(true)
        || piece.has_decoder == Some(true)
        || piece.has_signal == Some(true)
    {
        return false;
    }
    if piece.custom_offsets.as_ref().map_or(false, |o| !o.is_empty())
        || piece.custom_chord_dist.is_some()
        || piece.custom_length.is_some()
        || piece.custom_radius.is_some()
        || piece.custom_angle.is_some()
    {
        return false;
    }
    true
}

pub(super) fn compute_track_instance_tint(
    entry: &CatalogEntry,
    color_mode: ColorMode,
    layer_col: Option<[f32; 3]>,
) -> [f32; 4] {
    let base_ballast_col = [0.42_f32, 0.44, 0.48];
    let is_buf = crate::catalog::is_buffer_stop_entry(entry);
    match color_mode {
        ColorMode::Off => [base_ballast_col[0], base_ballast_col[1], base_ballast_col[2], 0.0],
        ColorMode::Types => {
            if entry.is_road == Some(true) {
                return [base_ballast_col[0], base_ballast_col[1], base_ballast_col[2], 0.0];
            }
            let tint = if is_buf {
                [0.94, 0.27, 0.27]
            } else {
                match entry.kind.as_str() {
                    "curve" => [0.22, 0.74, 0.97],
                    "turnout-left"
                    | "turnout-right"
                    | "curved-turnout-left"
                    | "curved-turnout-right"
                    | "three-way" => [0.96, 0.62, 0.04],
                    "crossing" | "double-slip" => [0.66, 0.33, 0.97],
                    "flex" => [0.06, 0.73, 0.51],
                    "buffer" => [0.94, 0.27, 0.27],
                    _ => [0.39, 0.45, 0.55],
                }
            };
            [
                base_ballast_col[0] * 0.48 + tint[0] * 0.52,
                base_ballast_col[1] * 0.48 + tint[1] * 0.52,
                base_ballast_col[2] * 0.48 + tint[2] * 0.52,
                1.0,
            ]
        }
        ColorMode::Layers => {
            if entry.is_road == Some(true) {
                return [base_ballast_col[0], base_ballast_col[1], base_ballast_col[2], 0.0];
            }
            let lc = layer_col.unwrap_or([0.22, 0.74, 0.97]);
            [
                base_ballast_col[0] * 0.42 + lc[0] * 0.58,
                base_ballast_col[1] * 0.42 + lc[1] * 0.58,
                base_ballast_col[2] * 0.42 + lc[2] * 0.58,
                1.0,
            ]
        }
    }
}

pub(super) fn build_local_track_prototype_3d(
    entry: &CatalogEntry,
    is_road: bool,
    empty_layout: &Layout,
) -> (Arc<Vec<Vertex3D>>, Arc<Vec<u32>>) {
    let origin_piece = crate::catalog::create_piece(
        format!("proto-{}", entry.id),
        entry,
        railbench_core::types::Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let mut pv = Vec::with_capacity(256);
    let mut pi = Vec::with_capacity(512);
    let mut dummy_cat = CatenaryMast3DContext::default();
    push_track_piece_3d(
        &mut pv,
        &mut pi,
        &origin_piece,
        Some(entry),
        empty_layout,
        is_road,
        0.0,
        0.0,
        ColorMode::Off,
        None,
        &mut dummy_cat,
    );
    (Arc::new(pv), Arc::new(pi))
}

pub(super) fn compute_submesh_signatures(
    layout: &Layout,
    opts: &Scene3DOptions,
    boards: &[BaseboardConfig],
) -> (u64, u64, u64, u64) {
    let mut h_env = 0xcbf29ce484222325_u64;
    h_env = fnv1a_u64(h_env, &[opts.env_mode as u8, opts.dark_mode as u8]);
    for b in boards {
        h_env = hash_f64(h_env, b.width);
        h_env = hash_f64(h_env, b.depth);
        h_env = hash_f64(h_env, b.custom_x.unwrap_or(0.0));
        h_env = hash_f64(h_env, b.custom_y.unwrap_or(0.0));
        h_env = hash_f64(h_env, b.rotation.unwrap_or(0.0));
        h_env = fnv1a_u64(h_env, b.shape.as_deref().unwrap_or("rect").as_bytes());
    }

    let mut max_deck_elev = if boards.iter().any(|b| b.enabled) {
        0.0_f64
    } else {
        f64::NEG_INFINITY
    };
    let mut layer_h_map: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    let mut has_tight_plates = false;
    let mut h_boards = fnv1a_u64(h_env, &[opts.show_wood_plates as u8]);
    for b in boards {
        h_boards = fnv1a_u64(h_boards, b.style.as_bytes());
    }
    if let Some(layers) = &layout.layers {
        for l in layers {
            let lh = l.height_mm.unwrap_or(0.0);
            layer_h_map.insert(l.id.as_str(), lh);
            let mode_str = l.wood_plate_mode.as_deref().unwrap_or("none");
            if l.visible && opts.show_wood_plates {
                if mode_str == "table" || l.plates.as_ref().map_or(false, |ps| !ps.is_empty()) {
                    max_deck_elev = max_deck_elev.max(lh);
                }
                if mode_str == "tight" {
                    has_tight_plates = true;
                }
            }
            h_boards = fnv1a_u64(h_boards, l.id.as_bytes());
            h_boards = fnv1a_u64(h_boards, &[l.visible as u8]);
            h_boards = hash_f64(h_boards, lh);
            h_boards = hash_f64(h_boards, l.wood_plate_thickness_mm.unwrap_or(12.0));
            h_boards = fnv1a_u64(h_boards, mode_str.as_bytes());
            h_boards = fnv1a_u64(h_boards, l.wood_plate_style.as_deref().unwrap_or("").as_bytes());
            if let Some(plates) = &l.plates {
                for pl in plates {
                    h_boards = fnv1a_u64(h_boards, pl.id.as_bytes());
                    h_boards = hash_f64(h_boards, pl.width);
                    h_boards = hash_f64(h_boards, pl.depth);
                    h_boards = hash_f64(h_boards, pl.x.unwrap_or(0.0));
                    h_boards = hash_f64(h_boards, pl.y.unwrap_or(0.0));
                }
            }
        }
    }
    // Elevated, turntable/transfer, tight-plate bridged, or sub-deck catenary pieces affect board cutouts
    for p in &layout.pieces {
        let lh = p
            .layer_id
            .as_deref()
            .and_then(|lid| layer_h_map.get(lid).copied())
            .unwrap_or(0.0);
        let e0 = p.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = p.elevation.map(|e| e.end).unwrap_or(0.0);
        let sub_deck = (lh + e0.min(e1)) < max_deck_elev - 0.1;
        if e0.abs() > 1e-3
            || e1.abs() > 1e-3
            || p.connectors.len() >= 6
            || sub_deck
            || (has_tight_plates && p.has_bridge == Some(true))
            || opts.show_wood_plates
        {
            h_boards = fnv1a_u64(h_boards, p.id.as_bytes());
            h_boards = hash_f64(h_boards, p.transform.x);
            h_boards = hash_f64(h_boards, p.transform.y);
            h_boards = hash_f64(h_boards, p.transform.rotation);
            h_boards = hash_f64(h_boards, e0);
            h_boards = hash_f64(h_boards, e1);
            if sub_deck {
                let cutout_flags = ((p.has_catenary == Some(true)) as u8)
                    | (((p.catenary_power_feed == Some(true)) as u8) << 1)
                    | (((p.catenary_isolator == Some(true)) as u8) << 2);
                h_boards = fnv1a_u64(h_boards, &[cutout_flags]);
            }
            if has_tight_plates {
                h_boards = fnv1a_u64(h_boards, &[(p.has_bridge == Some(true)) as u8]);
            }
        }
    }

    let mut h_terrain = 0xcbf29ce484222325_u64;
    if let Some(layers) = &layout.layers {
        for l in layers {
            h_terrain = fnv1a_u64(h_terrain, &[l.visible as u8]);
            h_terrain = hash_f64(h_terrain, l.height_mm.unwrap_or(0.0));
        }
    }
    let mut has_terrain_models = false;
    let mut mountains: Vec<&Model3D> = Vec::new();
    if let Some(models) = &layout.models {
        for m in models {
            if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                has_terrain_models = true;
                if m.builtin_id.as_deref() == Some("mountain") {
                    mountains.push(m);
                }
                h_terrain = fnv1a_u64(h_terrain, m.id.as_bytes());
                h_terrain = hash_f64(h_terrain, m.transform.x);
                h_terrain = hash_f64(h_terrain, m.transform.y);
                h_terrain = hash_f64(h_terrain, m.transform.rotation);
                h_terrain = hash_f64(h_terrain, m.width);
                h_terrain = hash_f64(h_terrain, m.depth);
                h_terrain = hash_f64(h_terrain, m.height);
                h_terrain = hash_f64(h_terrain, m.elevation.unwrap_or(0.0));
                h_terrain = hash_f64(h_terrain, m.scale.unwrap_or(1.0));
                h_terrain = fnv1a_u64(h_terrain, m.mountain_style.as_deref().unwrap_or("").as_bytes());
                h_terrain = fnv1a_u64(h_terrain, m.ground_terrain_type.as_deref().unwrap_or("").as_bytes());
                if let Some(mp) = &m.mountain_path {
                    for pt in mp {
                        h_terrain = hash_f64(h_terrain, pt.x);
                        h_terrain = hash_f64(h_terrain, pt.y);
                    }
                }
            }
        }
    }
    if has_terrain_models {
        for p in &layout.pieces {
            h_terrain = fnv1a_u64(h_terrain, p.id.as_bytes());
            h_terrain = hash_f64(h_terrain, p.transform.x);
            h_terrain = hash_f64(h_terrain, p.transform.y);
            h_terrain = hash_f64(h_terrain, p.transform.rotation);
            let e0 = p.elevation.map(|e| e.start).unwrap_or(0.0);
            let e1 = p.elevation.map(|e| e.end).unwrap_or(0.0);
            h_terrain = hash_f64(h_terrain, e0);
            h_terrain = hash_f64(h_terrain, e1);
            let near_mountain = mountains.iter().any(|m| {
                let sc = m.scale.unwrap_or(1.0).max(0.05);
                let reach_r = (m.width.max(m.depth) * 0.5 * sc) + 65.0;
                (p.transform.x - m.transform.x).hypot(p.transform.y - m.transform.y) <= reach_r
                    || p.connectors
                        .iter()
                        .any(|c| (c.x - m.transform.x).hypot(c.y - m.transform.y) <= reach_r)
            });
            if near_mountain {
                let t_flags = ((p.has_catenary == Some(true)) as u8) | (((p.has_tunnel == Some(true)) as u8) << 1);
                h_terrain = fnv1a_u64(h_terrain, &[t_flags]);
            }
        }
    }

    let mut h_tracks = 0xcbf29ce484222325_u64;
    let cm_byte = match opts.color_mode {
        ColorMode::Off => 0u8,
        ColorMode::Types => 1u8,
        ColorMode::Layers => 2u8,
    };
    h_tracks = fnv1a_u64(h_tracks, &[cm_byte]);
    if let Some(layers) = &layout.layers {
        for l in layers {
            h_tracks = fnv1a_u64(h_tracks, l.id.as_bytes());
            h_tracks = fnv1a_u64(h_tracks, &[l.visible as u8]);
            h_tracks = hash_f64(h_tracks, l.height_mm.unwrap_or(0.0));
            h_tracks = fnv1a_u64(h_tracks, l.color.as_deref().unwrap_or("").as_bytes());
        }
    }
    for p in &layout.pieces {
        h_tracks = fnv1a_u64(h_tracks, p.id.as_bytes());
        h_tracks = fnv1a_u64(h_tracks, p.catalog_id.as_bytes());
        h_tracks = fnv1a_u64(h_tracks, p.layer_id.as_deref().unwrap_or("").as_bytes());
        h_tracks = hash_f64(h_tracks, p.transform.x);
        h_tracks = hash_f64(h_tracks, p.transform.y);
        h_tracks = hash_f64(h_tracks, p.transform.rotation);
        let e0 = p.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = p.elevation.map(|e| e.end).unwrap_or(0.0);
        h_tracks = hash_f64(h_tracks, e0);
        h_tracks = hash_f64(h_tracks, e1);
        h_tracks = hash_f64(h_tracks, p.custom_length.unwrap_or(0.0));
        h_tracks = hash_f64(h_tracks, p.custom_angle.unwrap_or(0.0));
        let flags = (p.has_catenary == Some(true)) as u8
            | (((p.catenary_power_feed == Some(true)) as u8) << 1)
            | (((p.catenary_isolator == Some(true)) as u8) << 2)
            | (((p.has_bridge == Some(true)) as u8) << 3)
            | (((p.has_tunnel == Some(true)) as u8) << 4)
            | (((p.has_lantern == Some(true)) as u8) << 5)
            | (((p.has_decoder == Some(true)) as u8) << 6)
            | (((p.has_signal == Some(true)) as u8) << 7);
        h_tracks = fnv1a_u64(h_tracks, &[flags, (p.power_feed == Some(true)) as u8, (p.is_isolator == Some(true)) as u8]);
        if p.has_bridge == Some(true) || p.has_tunnel == Some(true) {
            for c in &p.connectors {
                h_tracks = fnv1a_u64(
                    h_tracks,
                    c.connected_to.as_ref().map(|r| r.piece_id.as_str()).unwrap_or("").as_bytes(),
                );
            }
        }
    }
    if let Some(models) = &layout.models {
        for m in models {
            if !matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                h_tracks = fnv1a_u64(h_tracks, m.id.as_bytes());
                h_tracks = fnv1a_u64(h_tracks, m.builtin_id.as_deref().unwrap_or("").as_bytes());
                h_tracks = hash_f64(h_tracks, m.transform.x);
                h_tracks = hash_f64(h_tracks, m.transform.y);
                h_tracks = hash_f64(h_tracks, m.transform.rotation);
                h_tracks = hash_f64(h_tracks, m.width);
                h_tracks = hash_f64(h_tracks, m.depth);
                h_tracks = hash_f64(h_tracks, m.height);
                h_tracks = hash_f64(h_tracks, m.elevation.unwrap_or(0.0));
                h_tracks = hash_f64(h_tracks, m.scale.unwrap_or(1.0));
            }
        }
    }
    for sc in &opts.short_circuit_points {
        h_tracks = hash_f64(h_tracks, sc.x);
        h_tracks = hash_f64(h_tracks, sc.y);
    }

    (h_env, h_boards, h_terrain, h_tracks)
}

#[allow(dead_code)]
pub fn build_scene_mesh_3d(layout: &Layout, catalogs: &[Catalog], opts: &Scene3DOptions) -> SceneMesh3D {
    let mut cache = Scene3DCache::default();
    build_scene_mesh_3d_cached(layout, catalogs, opts, &mut cache)
}

#[allow(dead_code)]
pub fn build_scene_mesh_3d_cached(
    layout: &Layout,
    catalogs: &[Catalog],
    opts: &Scene3DOptions,
    cache: &mut Scene3DCache,
) -> SceneMesh3D {
    let bundle = update_scene_3d_cache(layout, catalogs, opts, cache);
    let total_verts = bundle.env.as_ref().map_or(0, |(_, m)| m.vertices.len())
        + bundle.boards.as_ref().map_or(0, |(_, m)| m.vertices.len())
        + bundle.terrain.as_ref().map_or(0, |(_, m)| m.vertices.len())
        + bundle.tracks.as_ref().map_or(0, |(_, m)| m.vertices.len())
        + 96;
    let total_idxs = bundle.env.as_ref().map_or(0, |(_, m)| m.indices.len())
        + bundle.boards.as_ref().map_or(0, |(_, m)| m.indices.len())
        + bundle.terrain.as_ref().map_or(0, |(_, m)| m.indices.len())
        + bundle.tracks.as_ref().map_or(0, |(_, m)| m.indices.len())
        + 192;
    let mut combined = SceneMesh3D {
        vertices: Vec::with_capacity(total_verts),
        indices: Vec::with_capacity(total_idxs),
        instanced_tracks: Vec::new(),
    };
    if let Some((_, ref m)) = bundle.env {
        combined.append_from(m);
    }
    if let Some((_, ref m)) = bundle.boards {
        combined.append_from(m);
    }
    if let Some((_, ref m)) = bundle.terrain {
        combined.append_from(m);
    }
    if let Some((_, ref m)) = bundle.tracks {
        combined.append_from(m);
    }
    push_celestial_bodies_3d(&mut combined.vertices, &mut combined.indices, opts.time_of_day, opts.env_mode);
    combined
}

