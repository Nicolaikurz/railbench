use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn push_ground_terrain_3d(
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
    let clip_to_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = crate::terrain_tools::pack_support_polygons(&support_polys);

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

    let mut wx_g = vec![0.0_f64; cols * rows];
    let mut wz_g = vec![0.0_f64; cols * rows];
    let mut gh_g = vec![0.0_f32; cols * rows];
    let mut in_ok = vec![false; cols * rows];
    let mut edge_clamped = vec![false; cols * rows];
    let mut is_w_g = vec![false; cols * rows];
    let mut flat_g = vec![0.0_f32; cols * rows];

    for iz in 0..rows {
        let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
            let v_idx = iz * cols + ix;
            wx_g[v_idx] = mx + (lx * cos_m - lz * sin_m);
            wz_g[v_idx] = mz + (lx * sin_m + lz * cos_m);
            gh_g[v_idx] = grid[v_idx * 4];
            in_ok[v_idx] = grid[v_idx * 4 + 1] > 0.5;
            is_w_g[v_idx] = grid[v_idx * 4 + 2] > 0.5;
            flat_g[v_idx] = grid[v_idx * 4 + 3];
        }
    }

    if clip_to_support {
        for iz in 0..rows {
            for ix in 0..cols {
                let v_idx = iz * cols + ix;
                if in_ok[v_idx] {
                    continue;
                }
                let mut sum_h = 0.0_f32;
                let mut sum_flat = 0.0_f32;
                let mut w_votes = 0usize;
                let mut nb_cnt = 0usize;
                for dz in -1isize..=1 {
                    for dx in -1isize..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let nxi = ix as isize + dx;
                        let nzi = iz as isize + dz;
                        if nxi >= 0 && nxi < cols as isize && nzi >= 0 && nzi < rows as isize {
                            let ni = nzi as usize * cols + nxi as usize;
                            if in_ok[ni] {
                                sum_h += gh_g[ni];
                                sum_flat += flat_g[ni];
                                if is_w_g[ni] {
                                    w_votes += 1;
                                }
                                nb_cnt += 1;
                            }
                        }
                    }
                }
                if nb_cnt > 0 {
                    let clamped = crate::terrain_tools::clamp_point_to_support_polygons(
                        Vec2 { x: wx_g[v_idx], y: wz_g[v_idx] },
                        &support_polys,
                        &poly_offsets,
                        &poly_coords,
                    );
                    wx_g[v_idx] = clamped.x;
                    wz_g[v_idx] = clamped.y;
                    gh_g[v_idx] = sum_h / nb_cnt as f32;
                    flat_g[v_idx] = sum_flat / nb_cnt as f32;
                    is_w_g[v_idx] = w_votes > 0;
                    edge_clamped[v_idx] = true;
                }
            }
        }
    }

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
            let gh = gh_g[v_idx];
            let is_w = is_w_g[v_idx];
            let flat = flat_g[v_idx];

            let hl = gh_g[iz * cols + ix.saturating_sub(1)];
            let hr = gh_g[iz * cols + (ix + 1).min(segs_x)];
            let hd = gh_g[iz.saturating_sub(1) * cols + ix];
            let hu = gh_g[(iz + 1).min(segs_z) * cols + ix];
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

            verts.push(Vertex3D::new(
                [wx_g[v_idx] as f32, y0 + gh + 0.3, wz_g[v_idx] as f32],
                [rnx, norm[1], rnz],
                col,
                MAT_TERRAIN,
            ));
        }
    }

    let valid_gnd_tri = |ia: usize, ib: usize, ic: usize| -> bool {
        if !(in_ok[ia] || edge_clamped[ia])
            || !(in_ok[ib] || edge_clamped[ib])
            || !(in_ok[ic] || edge_clamped[ic])
        {
            return false;
        }
        if !clip_to_support {
            return true;
        }
        let tri_cx = (wx_g[ia] + wx_g[ib] + wx_g[ic]) / 3.0;
        let tri_cz = (wz_g[ia] + wz_g[ib] + wz_g[ic]) / 3.0;
        is_point_in_any_polygon(tri_cx, tri_cz, &poly_offsets, &poly_coords)
    };

    for iz in 0..segs_z {
        for ix in 0..segs_x {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            let vis_h = |vi: usize| gh_g[vi] > 0.08;
            if (vis_h(i_a) || vis_h(i_b) || vis_h(i_c)) && valid_gnd_tri(i_a, i_c, i_b) {
                idxs.extend_from_slice(&[base_v + i_a as u32, base_v + i_c as u32, base_v + i_b as u32]);
                if !is_water {
                    for &(u, v) in &[(i_a, i_c), (i_c, i_b), (i_b, i_a)] {
                        if edge_clamped[u] && edge_clamped[v] && gh_g[u].max(gh_g[v]) > 0.5 {
                            push_diorama_skirt_quad(
                                verts,
                                idxs,
                                wx_g[u] as f32,
                                wz_g[u] as f32,
                                y0 + gh_g[u] + 0.3,
                                wx_g[v] as f32,
                                wz_g[v] as f32,
                                y0 + gh_g[v] + 0.3,
                                y0,
                            );
                        }
                    }
                }
            }
            if (vis_h(i_b) || vis_h(i_c) || vis_h(i_d)) && valid_gnd_tri(i_b, i_c, i_d) {
                idxs.extend_from_slice(&[base_v + i_b as u32, base_v + i_c as u32, base_v + i_d as u32]);
                if !is_water {
                    for &(u, v) in &[(i_b, i_c), (i_c, i_d), (i_d, i_b)] {
                        if edge_clamped[u] && edge_clamped[v] && gh_g[u].max(gh_g[v]) > 0.5 {
                            push_diorama_skirt_quad(
                                verts,
                                idxs,
                                wx_g[u] as f32,
                                wz_g[u] as f32,
                                y0 + gh_g[u] + 0.3,
                                wx_g[v] as f32,
                                wz_g[v] as f32,
                                y0 + gh_g[v] + 0.3,
                                y0,
                            );
                        }
                    }
                }
            }
        }
    }

    // Reflective cyan water surface over water basin cells (strictly clipped to table/plate)
    if is_water {
        let water_base_v = verts.len() as u32;
        let water_y = y0 + 1.85;
        let water_col = [0.06, 0.56, 0.84];
        for v_idx in 0..(cols * rows) {
            verts.push(Vertex3D::new(
                [wx_g[v_idx] as f32, water_y, wz_g[v_idx] as f32],
                [0.0, 1.0, 0.0],
                water_col,
                MAT_WATER,
            ));
        }
        let is_wat = |vi: usize| (in_ok[vi] || edge_clamped[vi]) && is_w_g[vi];
        for iz in 0..segs_z {
            for ix in 0..segs_x {
                let i_a = iz * cols + ix;
                let i_b = i_a + 1;
                let i_c = (iz + 1) * cols + ix;
                let i_d = i_c + 1;
                if is_wat(i_a) && is_wat(i_b) && is_wat(i_c) && valid_gnd_tri(i_a, i_c, i_b) {
                    idxs.extend_from_slice(&[
                        water_base_v + i_a as u32,
                        water_base_v + i_c as u32,
                        water_base_v + i_b as u32,
                    ]);
                }
                if is_wat(i_b) && is_wat(i_c) && is_wat(i_d) && valid_gnd_tri(i_b, i_c, i_d) {
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

pub(super) fn push_model_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    textured_out: &mut Vec<TexturedSubmesh3D>,
    m: &Model3D,
    y0: f32,
) {
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
            let mut local_tex_submeshes: Vec<Option<usize>> = vec![None; parsed.textures.len()];
            let mut staged_tex: Vec<TexturedSubmesh3D> = Vec::new();
            let xform_vert = |p: [f32; 3], n: [f32; 3]| -> ([f32; 3], [f32; 3]) {
                let lx = (p[0] - cx_mesh) * scale;
                let ly = (p[1] - base_y) * scale;
                let lz = (p[2] - cz_mesh) * scale;
                let (rx, rz) = rot_xz(lx, lz, rot_rad);
                let (nx, nz) = rot_xz(n[0], n[2], rot_rad);
                ([cx + rx, y0 + ly, cz + rz], normalize3([nx, n[1], nz]))
            };

            for tri in &parsed.triangles {
                if let Some(ti) = tri.texture_idx.filter(|&i| i < parsed.textures.len()) {
                    let slot_idx = match local_tex_submeshes[ti] {
                        Some(si) => si,
                        None => {
                            let tex = &parsed.textures[ti];
                            let si = staged_tex.len();
                            staged_tex.push(TexturedSubmesh3D {
                                texture_id: tex.id,
                                tex_width: tex.width,
                                tex_height: tex.height,
                                tex_rgba: Arc::clone(&tex.rgba),
                                vertices: Vec::new(),
                                indices: Vec::new(),
                            });
                            local_tex_submeshes[ti] = Some(si);
                            si
                        }
                    };
                    let sub = &mut staged_tex[slot_idx];
                    let base_v = sub.vertices.len() as u32;
                    for k in 0..3 {
                        let (wp, wn) = xform_vert(tri.positions[k], tri.normals[k]);
                        sub.vertices.push(TexturedVertex3D {
                            pos: wp,
                            normal: wn,
                            uv: tri.uvs[k],
                            color: [tri.color[0], tri.color[1], tri.color[2], 1.0],
                            mat: [tri.roughness, tri.metallic, 0.0, 0.0],
                        });
                    }
                    sub.indices.extend_from_slice(&[base_v, base_v + 1, base_v + 2]);
                    continue;
                }
                let base_v = verts.len() as u32;
                for k in 0..3 {
                    let (wp, wn) = xform_vert(tri.positions[k], tri.normals[k]);
                    verts.push(Vertex3D::new(
                        wp,
                        wn,
                        tri.color,
                        [tri.roughness, tri.metallic, 0.0],
                    ));
                }
                idxs.extend_from_slice(&[base_v, base_v + 1, base_v + 2]);
            }
            for sub in staged_tex {
                merge_textured_submesh(textured_out, sub);
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

