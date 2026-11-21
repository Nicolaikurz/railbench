use super::*;

pub(crate) fn push_tunnel_bore_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    path_xyz: &[f32],
    prof: &TunnelProfile,
) {
    let n_pts = path_xyz.len() / 3;
    if n_pts < 2 {
        return;
    }

    let arc_steps = 16usize;
    let section = tunnel_profile_points(prof, 0.8, arc_steps);
    let mut sec_flat = Vec::with_capacity(section.len() * 2);
    for [sx, sy] in &section {
        sec_flat.push(*sx);
        sec_flat.push(*sy);
    }
    let swept = build_tunnel_sweep_positions(path_xyz, &sec_flat);
    let n_sec = section.len();
    let n_ring = swept.len() / (n_sec * 3);

    let (base_col, crown_col, mat) = if prof.is_road {
        ([0.44, 0.46, 0.49], [0.27, 0.29, 0.32], MAT_CONCRETE)
    } else {
        ([0.26, 0.26, 0.28], [0.12, 0.13, 0.15], MAT_STONE)
    };

    for r_i in 0..n_ring.saturating_sub(1) {
        let mid_c = [
            0.5 * (path_xyz[r_i * 3] + path_xyz[(r_i + 1) * 3]),
            0.5 * (path_xyz[r_i * 3 + 1] + path_xyz[(r_i + 1) * 3 + 1]) + prof.wall_h * 0.45,
            0.5 * (path_xyz[r_i * 3 + 2] + path_xyz[(r_i + 1) * 3 + 2]),
        ];
        for s_j in 0..(n_sec - 1) {
            let b0 = (r_i * n_sec + s_j) * 3;
            let b1 = (r_i * n_sec + s_j + 1) * 3;
            let b2 = ((r_i + 1) * n_sec + s_j + 1) * 3;
            let b3 = ((r_i + 1) * n_sec + s_j) * 3;
            let p0 = [swept[b0], swept[b0 + 1], swept[b0 + 2]];
            let p1 = [swept[b1], swept[b1 + 1], swept[b1 + 2]];
            let p2 = [swept[b2], swept[b2 + 1], swept[b2 + 2]];
            let p3 = [swept[b3], swept[b3 + 1], swept[b3 + 2]];
            let h_t = (0.5 * (section[s_j][1] + section[s_j + 1][1]) / prof.total_h.max(1.0)).clamp(0.0, 1.0);
            let seg_col = [
                base_col[0] + (crown_col[0] - base_col[0]) * h_t,
                base_col[1] + (crown_col[1] - base_col[1]) * h_t,
                base_col[2] + (crown_col[2] - base_col[2]) * h_t,
            ];
            push_oriented_quad_toward(verts, idxs, p0, p1, p2, p3, mid_c, true, seg_col, mat);
        }
    }

    // Side safety inspection walkways / cable ducts (Fluchtweg) outside ballast bed
    let c = (prof.inner_width * 0.5 - 0.8).max(8.0);
    let sc = (prof.pier / 16.0).clamp(0.5, 2.2);
    let walk_w = (4.6 * sc).min(c * 0.22);
    let walk_h = 3.8 * sc;
    let walk_col = if prof.is_road { CONCRETE_TRIM } else { [0.46, 0.45, 0.43] };
    for s in [-1.0_f32, 1.0] {
        let x_wall = s * c;
        let x_edge = s * (c - walk_w);
        let walk_sec = [
            x_edge, 0.0,
            x_edge, walk_h,
            x_wall, walk_h,
        ];
        let walk_swept = build_tunnel_sweep_positions(path_xyz, &walk_sec);
        let nw_sec = 3usize;
        let nw_ring = walk_swept.len() / (nw_sec * 3);
        for r_i in 0..nw_ring.saturating_sub(1) {
            let cy = 0.5 * (path_xyz[r_i * 3 + 1] + path_xyz[(r_i + 1) * 3 + 1]) + walk_h * 0.5;
            let track_mid = [
                0.5 * (path_xyz[r_i * 3] + path_xyz[(r_i + 1) * 3]),
                cy,
                0.5 * (path_xyz[r_i * 3 + 2] + path_xyz[(r_i + 1) * 3 + 2]),
            ];
            for s_j in 0..(nw_sec - 1) {
                let b0 = (r_i * nw_sec + s_j) * 3;
                let b1 = (r_i * nw_sec + s_j + 1) * 3;
                let b2 = ((r_i + 1) * nw_sec + s_j + 1) * 3;
                let b3 = ((r_i + 1) * nw_sec + s_j) * 3;
                let p0 = [walk_swept[b0], walk_swept[b0 + 1], walk_swept[b0 + 2]];
                let p1 = [walk_swept[b1], walk_swept[b1 + 1], walk_swept[b1 + 2]];
                let p2 = [walk_swept[b2], walk_swept[b2 + 1], walk_swept[b2 + 2]];
                let p3 = [walk_swept[b3], walk_swept[b3 + 1], walk_swept[b3 + 2]];
                let force_n = if s_j == 1 {
                    [0.0, 1.0, 0.0]
                } else {
                    let mut n = normalize3(cross3(
                        [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]],
                        [p3[0] - p0[0], p3[1] - p0[1], p3[2] - p0[2]],
                    ));
                    let fc = [
                        0.25 * (p0[0] + p1[0] + p2[0] + p3[0]),
                        0.25 * (p0[1] + p1[1] + p2[1] + p3[1]),
                        0.25 * (p0[2] + p1[2] + p2[2] + p3[2]),
                    ];
                    if dot3(n, [track_mid[0] - fc[0], track_mid[1] - fc[1], track_mid[2] - fc[2]]) < 0.0 {
                        n = [-n[0], -n[1], -n[2]];
                    }
                    n
                };
                push_quad_mat(verts, idxs, p0, p1, p2, p3, force_n, walk_col, MAT_CONCRETE);
            }
        }
    }

    // Stations along the bore for structural arch ribs, sconces / ceiling LEDs, and catenary hangers
    let mut cum = vec![0.0_f32; n_pts];
    for i in 1..n_pts {
        let dx = path_xyz[i * 3] - path_xyz[(i - 1) * 3];
        let dz = path_xyz[i * 3 + 2] - path_xyz[(i - 1) * 3 + 2];
        cum[i] = cum[i - 1] + dx.hypot(dz);
    }
    let total_len = *cum.last().unwrap_or(&0.0);
    if total_len < 36.0 {
        return;
    }

    // Interior structural arch ribs (Gurtbögen)
    let rib_step = 48.0 * sc;
    let rib_count = ((total_len - 28.0) / rib_step).floor().max(0.0) as usize;
    let rib_pts = tunnel_profile_points(prof, 1.8, 12);
    let rib_col = if prof.is_road { CONCRETE_DARK } else { STONE_DARK };
    for k in 0..rib_count {
        let dist = 18.0 + (k as f32 + 0.5) * ((total_len - 36.0) / (rib_count as f32).max(1.0));
        let (pos, fwd) = sample_path_xyz(path_xyz, &cum, dist);
        let f = PortalFrame::new(pos, fwd);
        for w in rib_pts.windows(2) {
            let ax = w[0][0];
            let ay = w[0][1];
            let bx = w[1][0];
            let by = w[1][1];
            let seg_len = (bx - ax).hypot(by - ay);
            if seg_len < 0.5 {
                continue;
            }
            let mx = 0.5 * (ax + bx);
            let my = 0.5 * (ay + by);
            let mut in_nx = -(by - ay) / seg_len;
            let mut in_ny = (bx - ax) / seg_len;
            if in_nx * (-mx) + in_ny * (prof.wall_h * 0.45 - my) < 0.0 {
                in_nx = -in_nx;
                in_ny = -in_ny;
            }
            let thick = 2.6 * sc;
            let p_a = [ax + in_nx * thick, ay + in_ny * thick];
            let p_b = [bx + in_nx * thick, by + in_ny * thick];
            let hw_z = 1.7 * sc;
            let pa_f = f.to_world(p_a[0], p_a[1], -hw_z);
            let pb_f = f.to_world(p_b[0], p_b[1], -hw_z);
            let pb_b = f.to_world(p_b[0], p_b[1], hw_z);
            let pa_b = f.to_world(p_a[0], p_a[1], hw_z);
            push_quad_mat(verts, idxs, pa_f, pb_f, pb_b, pa_b, f.to_norm(in_nx, in_ny, 0.0), rib_col, mat);
            push_quad_mat(verts, idxs, f.to_world(ax, ay, -hw_z), f.to_world(bx, by, -hw_z), pb_f, pa_f, f.to_norm(0.0, 0.0, -1.0), rib_col, mat);
            push_quad_mat(verts, idxs, pa_b, pb_b, f.to_world(bx, by, hw_z), f.to_world(ax, ay, hw_z), f.to_norm(0.0, 0.0, 1.0), rib_col, mat);
        }
    }

    // Tunnel illumination (night sconces for rail, overhead LED strip fixtures for road)
    let light_step = 66.0 * sc;
    let light_count = ((total_len - 24.0) / light_step).floor().max(1.0) as usize;
    for k in 0..light_count {
        let dist = 16.0 + (k as f32 + 0.5) * ((total_len - 32.0) / light_count as f32);
        let (pos, fwd) = sample_path_xyz(path_xyz, &cum, dist);
        let f = PortalFrame::new(pos, fwd);
        if prof.is_road {
            f.push_box(verts, idxs, -4.5, 4.5, prof.total_h - 3.2, prof.total_h - 1.2, -5.0, 5.0, [0.92, 0.96, 1.0], MAT_ROAD_LED);
        } else {
            let sconce_y = prof.wall_h * 0.72;
            for s in [-1.0_f32, 1.0] {
                f.push_box(verts, idxs, s * (c - 1.6), s * c, sconce_y - 1.8 * sc, sconce_y + 1.8 * sc, -2.0, 2.0, [1.0, 0.85, 0.45], MAT_WARM_SCONCE);
            }
        }
    }

    // Overhead ceiling catenary hangers (Tunnelausleger) when electrified
    if !prof.is_road && prof.has_catenary {
        let hanger_step = 64.0 * sc;
        let hanger_count = ((total_len - 24.0) / hanger_step).floor().max(1.0) as usize;
        let msg_y = 85.0_f32.min(prof.total_h - 5.0);
        let cw_y = 72.0_f32.min(prof.wall_h - 2.0);
        let n_trk = prof.num_tracks.max(1);
        let track_spacing = if n_trk > 1 {
            ((prof.inner_width - 74.0 * sc) / (n_trk - 1) as f32).max(46.0 * sc)
        } else {
            0.0
        };

        for k in 0..hanger_count {
            let dist = 16.0 + (k as f32 + 0.5) * ((total_len - 32.0) / hanger_count as f32);
            let (pos, fwd) = sample_path_xyz(path_xyz, &cum, dist);
            let f = PortalFrame::new(pos, fwd);

            for ti in 0..n_trk {
                let lx = if n_trk == 1 {
                    0.0
                } else {
                    (ti as f32 - 0.5 * (n_trk - 1) as f32) * track_spacing
                };
                let ceil_y = tunnel_ceiling_y_at_x(prof, lx).max(msg_y + 3.0);
                // Ceiling anchor plate & vertical yoke beam
                f.push_box(verts, idxs, lx - 3.0, lx + 3.0, ceil_y - 1.5, ceil_y + 0.8, -2.0, 2.0, [0.22, 0.25, 0.30], MAT_STEEL_DARK);
                f.push_box(verts, idxs, lx + 3.2, lx + 4.8, cw_y + 1.5, ceil_y, -0.8, 0.8, [0.24, 0.28, 0.34], MAT_STEEL_DARK);
                // Upper cantilever arm + brown porcelain insulator + messenger wire clamp
                f.push_box(verts, idxs, lx - 0.8, lx + 4.8, msg_y + 1.0, msg_y + 2.4, -0.7, 0.7, [0.24, 0.28, 0.34], MAT_STEEL_DARK);
                f.push_box(verts, idxs, lx + 1.5, lx + 3.6, msg_y - 0.3, msg_y + 2.6, -1.0, 1.0, [0.52, 0.30, 0.08], MAT_PORCELAIN);
                f.push_box(verts, idxs, lx - 0.7, lx + 0.7, msg_y - 0.8, msg_y + 1.6, -0.8, 0.8, [0.58, 0.64, 0.72], MAT_STEEL_DARK);
                // Lower steady-arm (Seitenhalter) + porcelain insulator + contact wire clamp
                f.push_box(verts, idxs, lx, lx + 4.8, cw_y + 1.2, cw_y + 2.5, -0.6, 0.6, [0.28, 0.32, 0.38], MAT_STEEL_DARK);
                f.push_box(verts, idxs, lx + 1.5, lx + 3.4, cw_y + 0.4, cw_y + 2.8, -0.9, 0.9, [0.52, 0.30, 0.08], MAT_PORCELAIN);
                f.push_box(verts, idxs, lx - 0.5, lx + 0.5, cw_y - 0.5, cw_y + 1.4, -0.7, 0.7, [0.85, 0.47, 0.04], MAT_STEEL_DARK);
            }
        }
    }
}

// ============================================================================
//  Public Entry Point 3: Standalone Piece Tunnels (Outside Mountains)
// ============================================================================

