use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_arched_tunnel_portal_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    world_pos: [f32; 3],
    inward_dir_xz: [f32; 2],
    elev_for_found: f32,
    prof: &TunnelProfile,
) {
    let frame = PortalFrame::new(world_pos, inward_dir_xz);
    let c = prof.inner_width * 0.5;
    let outer_hw = c + prof.pier;
    let is_multi = prof.num_tracks > 1;
    let sc = (prof.pier / 16.0).clamp(0.55, 2.2);

    let lintel = if prof.is_road { 12.0 } else if is_multi { 18.0 } else { 15.0 };
    let headwall_h = if prof.is_road { 18.0 } else if is_multi { 28.0 } else { 24.0 };
    let face_depth = if is_multi { 18.0 } else { 15.0 };
    let collar_depth = if is_multi { 74.0 } else { 68.0 };
    let face_h = prof.total_h + lintel + headwall_h;

    let arch_pts = tunnel_profile_points(prof, 0.0, 16);
    let wall_h = prof.wall_h;

    let (face_col, reveal_col, mat) = if prof.is_road {
        (CONCRETE_PORTAL, LINING_REVEAL_ROAD, MAT_CONCRETE)
    } else {
        (STONE_MAIN, LINING_REVEAL_RAIL, MAT_STONE)
    };

    let push_arch_wall_block =
        |verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, z0: f32, z1: f32, hw_out: f32, h_out: f32, col: [f32; 3]| {
            let h_out = h_out.max(prof.total_h + 6.0);
            let n = arch_pts.len();
            if n < 5 {
                return;
            }
            let fn_front = frame.to_norm(0.0, 0.0, -1.0);
            let fn_back = frame.to_norm(0.0, 0.0, 1.0);

            for &(z, norm) in &[(z0, fn_front), (z1, fn_back)] {
                // Left pier
                push_quad_mat(verts, idxs, frame.to_world(-hw_out, 0.0, z), frame.to_world(-c, 0.0, z), frame.to_world(-c, wall_h, z), frame.to_world(-hw_out, wall_h, z), norm, col, mat);
                // Right pier
                push_quad_mat(verts, idxs, frame.to_world(c, 0.0, z), frame.to_world(hw_out, 0.0, z), frame.to_world(hw_out, wall_h, z), frame.to_world(c, wall_h, z), norm, col, mat);
                // Corner spandrel triangles
                push_tri(verts, idxs, frame.to_world(-hw_out, wall_h, z), frame.to_world(-c, wall_h, z), frame.to_world(-hw_out, h_out, z), norm, col);
                push_tri(verts, idxs, frame.to_world(c, wall_h, z), frame.to_world(hw_out, wall_h, z), frame.to_world(hw_out, h_out, z), norm, col);
                // Spandrel quads over arch curve
                for k in 1..(n - 2) {
                    let p_a = arch_pts[k];
                    let p_b = arch_pts[k + 1];
                    let tx_a = (p_a[0] / c) * hw_out;
                    let tx_b = (p_b[0] / c) * hw_out;
                    push_quad_mat(verts, idxs, frame.to_world(p_a[0], p_a[1], z), frame.to_world(p_b[0], p_b[1], z), frame.to_world(tx_b, h_out, z), frame.to_world(tx_a, h_out, z), norm, col, mat);
                }
            }

            // Outer box perimeter walls
            push_quad_mat(verts, idxs, frame.to_world(-hw_out, 0.0, z1), frame.to_world(-hw_out, 0.0, z0), frame.to_world(-hw_out, h_out, z0), frame.to_world(-hw_out, h_out, z1), frame.to_norm(-1.0, 0.0, 0.0), col, mat);
            push_quad_mat(verts, idxs, frame.to_world(hw_out, 0.0, z0), frame.to_world(hw_out, 0.0, z1), frame.to_world(hw_out, h_out, z1), frame.to_world(hw_out, h_out, z0), frame.to_norm(1.0, 0.0, 0.0), col, mat);
            push_quad_mat(verts, idxs, frame.to_world(-hw_out, h_out, z0), frame.to_world(hw_out, h_out, z0), frame.to_world(hw_out, h_out, z1), frame.to_world(-hw_out, h_out, z1), frame.to_norm(0.0, 1.0, 0.0), col, mat);

            // Inner arch reveal walls along opening
            let interior_ref = frame.to_world(0.0, wall_h * 0.45, 0.5 * (z0 + z1));
            for k in 0..(n - 1) {
                let p_a = arch_pts[k];
                let p_b = arch_pts[k + 1];
                let pa0 = frame.to_world(p_a[0], p_a[1], z0);
                let pb0 = frame.to_world(p_b[0], p_b[1], z0);
                let pb1 = frame.to_world(p_b[0], p_b[1], z1);
                let pa1 = frame.to_world(p_a[0], p_a[1], z1);
                push_oriented_quad_toward(verts, idxs, pa0, pb0, pb1, pa1, interior_ref, true, reveal_col, mat);
            }
        };

    // 1. Main headwall + deep slope-sealing arch collar
    push_arch_wall_block(verts, idxs, 0.0, face_depth, outer_hw, face_h, face_col);
    push_arch_wall_block(verts, idxs, face_depth, face_depth + collar_depth, outer_hw - 3.5, prof.total_h + 15.0, face_col);

    if prof.is_road {
        push_road_portal_details(verts, idxs, &frame, prof, &arch_pts, c, outer_hw, face_depth, face_h, sc);
    } else {
        push_railway_portal_details(verts, idxs, &frame, prof, &arch_pts, c, outer_hw, face_depth, face_h, sc);
    }

    // Stepped & splayed wing retaining walls flaring outward into approach (-Z)
    let wing_len = if is_multi { 60.0 } else { 52.0 };
    let wing_thick = if prof.is_road { 8.5 } else { 9.5 * sc };
    let wing_ang = 26.0_f32.to_radians();
    push_portal_wing_walls(verts, idxs, &frame, outer_hw, wing_len, wing_thick, wing_ang, prof.total_h + 4.0, prof.is_road);

    // Twin masonry/concrete foundations strictly outside track bed (|x| >= c)
    let found_h = (elev_for_found + 16.0).max(24.0);
    let side_w = outer_hw - c + wing_len * wing_ang.sin() + 6.0;
    let z_min = -wing_len * wing_ang.cos() - 2.0;
    let z_max = face_depth + 8.0;
    let found_col = if prof.is_road { CONCRETE_DARK } else { STONE_DARK };
    for s in [-1.0_f32, 1.0] {
        let fx0 = s * (c + side_w);
        let fx1 = s * c;
        frame.push_box(verts, idxs, fx0, fx1, -found_h, 0.0, z_min, z_max, found_col, mat);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_railway_portal_details(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    frame: &PortalFrame,
    prof: &TunnelProfile,
    arch_pts: &[[f32; 2]],
    c: f32,
    outer_hw: f32,
    face_depth: f32,
    face_h: f32,
    sc: f32,
) {
    let n = arch_pts.len();
    // 1. Protruding radial Voussoir arch stones (Keilsteine)
    if n >= 5 {
        let arch_center = [0.0_f32, prof.wall_h * 0.45];
        for k in 1..(n - 2) {
            let a = arch_pts[k];
            let b = arch_pts[k + 1];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let slen = dx.hypot(dy).max(1e-4);
            let mut rx = -dy / slen;
            let mut ry = dx / slen;
            let mid_x = 0.5 * (a[0] + b[0]);
            let mid_y = 0.5 * (a[1] + b[1]);
            if rx * (mid_x - arch_center[0]) + ry * (mid_y - arch_center[1]) < 0.0 {
                rx = -rx;
                ry = -ry;
            }
            let r_thick = 7.4 * sc;
            let oa = [a[0] + rx * r_thick, a[1] + ry * r_thick];
            let ob = [b[0] + rx * r_thick, b[1] + ry * r_thick];
            let z_f = -2.4;
            let z_b = 1.4;
            let col = if k % 2 == 0 { STONE_VOUSSOIR_A } else { STONE_VOUSSOIR_B };

            let af = frame.to_world(a[0], a[1], z_f);
            let bf = frame.to_world(b[0], b[1], z_f);
            let obf = frame.to_world(ob[0], ob[1], z_f);
            let oaf = frame.to_world(oa[0], oa[1], z_f);
            let ab = frame.to_world(a[0], a[1], z_b);
            let bb = frame.to_world(b[0], b[1], z_b);
            let obb = frame.to_world(ob[0], ob[1], z_b);
            let oab = frame.to_world(oa[0], oa[1], z_b);

            let v_center = frame.to_world(0.5 * (mid_x + 0.5 * (oa[0] + ob[0])), 0.5 * (mid_y + 0.5 * (oa[1] + ob[1])), 0.5 * (z_f + z_b));
            push_quad_mat(verts, idxs, af, bf, obf, oaf, frame.to_norm(0.0, 0.0, -1.0), col, MAT_STONE);
            push_quad_mat(verts, idxs, oaf, obf, obb, oab, frame.to_norm(rx, ry, 0.0), STONE_LIGHT, MAT_STONE);
            push_oriented_quad_toward(verts, idxs, af, bf, bb, ab, v_center, false, LINING_REVEAL_RAIL, MAT_STONE);
            push_oriented_quad_toward(verts, idxs, ab, bb, obb, oab, v_center, false, col, MAT_STONE);
            push_oriented_quad_toward(verts, idxs, bf, bb, obb, obf, v_center, false, col, MAT_STONE);
            push_oriented_quad_toward(verts, idxs, af, oaf, oab, ab, v_center, false, col, MAT_STONE);
        }
    }

    // 2. Oversized Protruding Keystone (Schlussstein) at crown
    frame.push_box(
        verts,
        idxs,
        -4.8 * sc,
        4.8 * sc,
        prof.total_h - 1.5 * sc,
        prof.total_h + 11.0 * sc,
        -3.8,
        2.0,
        STONE_LIGHT,
        MAT_STONE,
    );

    // 3. Springline Impost Blocks (Kämpfersteine), Flanking Pilasters & Alternating Quoins
    for s in [-1.0_f32, 1.0] {
        // Impost cap at arch springline
        frame.push_box(
            verts,
            idxs,
            s * c,
            s * (outer_hw + 1.2 * sc),
            prof.wall_h - 4.0 * sc,
            prof.wall_h + 1.5 * sc,
            -2.4,
            2.2,
            STONE_LIGHT,
            MAT_STONE,
        );
        // Pilaster plinth at ground level
        frame.push_box(
            verts,
            idxs,
            s * (outer_hw - 7.5 * sc),
            s * (outer_hw + 1.8 * sc),
            0.0,
            11.0 * sc,
            -2.6,
            face_depth * 0.7,
            STONE_DARK,
            MAT_STONE,
        );
        // Alternating rustic corner quoins up the portal flank
        let n_quoins = ((face_h / (12.0 * sc)).floor() as usize).clamp(4, 12);
        let step_h = (face_h - 12.0 * sc) / n_quoins as f32;
        for q in 0..n_quoins {
            let y0 = 11.5 * sc + q as f32 * step_h;
            let y1 = y0 + step_h * 0.88;
            let w_in = if q % 2 == 0 { 7.5 * sc } else { 5.2 * sc };
            let col = if q % 2 == 0 { STONE_LIGHT } else { STONE_VOUSSOIR_B };
            frame.push_box(
                verts,
                idxs,
                s * (outer_hw - w_in),
                s * (outer_hw + 1.0 * sc),
                y0,
                y1,
                -1.8,
                face_depth * 0.5,
                col,
                MAT_STONE,
            );
        }
        // Entrance inspection wall sconce just inside portal mouth
        frame.push_box(
            verts,
            idxs,
            s * (c - 1.5 * sc),
            s * c,
            prof.wall_h * 0.72 - 2.0 * sc,
            prof.wall_h * 0.72 + 2.0 * sc,
            3.5,
            7.5,
            [1.0, 0.84, 0.45],
            MAT_WARM_SCONCE,
        );
    }

    // 4. Two-Tier Overhanging Stone Cornice (Kranzgesims) + Upper Stone Parapet (Brüstung)
    let cw = outer_hw + 3.5;
    frame.push_box(verts, idxs, -cw, cw, face_h - 2.5, face_h + 1.5, -3.0, face_depth + 2.5, STONE_DARK, MAT_STONE);
    frame.push_box(verts, idxs, -(cw + 2.5), cw + 2.5, face_h + 1.5, face_h + 6.0, -4.6, face_depth + 4.5, STONE_LIGHT, MAT_STONE);
    frame.push_box(verts, idxs, -outer_hw, outer_hw, face_h + 6.0, face_h + 12.0, 0.0, face_depth, STONE_MAIN, MAT_STONE);
    frame.push_box(verts, idxs, -(outer_hw + 1.5), outer_hw + 1.5, face_h + 12.0, face_h + 14.2, -1.2, face_depth + 1.2, STONE_LIGHT, MAT_STONE);

    // 5. Recessed Granite Name / Year Plaque above Keystone
    let p_cy = (face_h - 10.5 * sc).max(prof.total_h + 14.5 * sc);
    if p_cy + 4.0 * sc < face_h - 2.5 {
        frame.push_box(verts, idxs, -13.0 * sc, 13.0 * sc, p_cy - 4.8 * sc, p_cy + 4.8 * sc, -1.6, 0.2, STONE_LIGHT, MAT_STONE);
        frame.push_box(verts, idxs, -11.2 * sc, 11.2 * sc, p_cy - 3.4 * sc, p_cy + 3.4 * sc, -2.1, -1.2, STONE_PLAQUE, MAT_STONE);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_road_portal_details(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    frame: &PortalFrame,
    prof: &TunnelProfile,
    arch_pts: &[[f32; 2]],
    c: f32,
    outer_hw: f32,
    face_depth: f32,
    face_h: f32,
    sc: f32,
) {
    // 1. Protruding Precast Concrete Portal Arch Frame
    let n = arch_pts.len();
    if n >= 5 {
        let arch_center = [0.0_f32, prof.wall_h * 0.4];
        for k in 1..(n - 2) {
            let a = arch_pts[k];
            let b = arch_pts[k + 1];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let slen = dx.hypot(dy).max(1e-4);
            let mut rx = -dy / slen;
            let mut ry = dx / slen;
            let mid_x = 0.5 * (a[0] + b[0]);
            let mid_y = 0.5 * (a[1] + b[1]);
            if rx * (mid_x - arch_center[0]) + ry * (mid_y - arch_center[1]) < 0.0 {
                rx = -rx;
                ry = -ry;
            }
            let r_thick = 5.5 * sc;
            let oa = [a[0] + rx * r_thick, a[1] + ry * r_thick];
            let ob = [b[0] + rx * r_thick, b[1] + ry * r_thick];
            let af = frame.to_world(a[0], a[1], -2.6);
            let bf = frame.to_world(b[0], b[1], -2.6);
            let obf = frame.to_world(ob[0], ob[1], -2.6);
            let oaf = frame.to_world(oa[0], oa[1], -2.6);
            let ab = frame.to_world(a[0], a[1], 1.2);
            let bb = frame.to_world(b[0], b[1], 1.2);
            let obb = frame.to_world(ob[0], ob[1], 1.2);
            let oab = frame.to_world(oa[0], oa[1], 1.2);
            let center = frame.to_world(mid_x + rx * 2.5, mid_y + ry * 2.5, -0.7);
            push_quad_mat(verts, idxs, af, bf, obf, oaf, frame.to_norm(0.0, 0.0, -1.0), CONCRETE_TRIM, MAT_CONCRETE);
            push_quad_mat(verts, idxs, oaf, obf, obb, oab, frame.to_norm(rx, ry, 0.0), CONCRETE_TRIM, MAT_CONCRETE);
            push_oriented_quad_toward(verts, idxs, af, bf, bb, ab, center, false, LINING_REVEAL_ROAD, MAT_CONCRETE);
            push_oriented_quad_toward(verts, idxs, ab, bb, obb, oab, center, false, CONCRETE_TRIM, MAT_CONCRETE);
            push_oriented_quad_toward(verts, idxs, bf, bb, obb, obf, center, false, CONCRETE_TRIM, MAT_CONCRETE);
            push_oriented_quad_toward(verts, idxs, af, oaf, oab, ab, center, false, CONCRETE_TRIM, MAT_CONCRETE);
        }
    }

    // 2. Cantilevered Architectural Concrete Fascia Canopy
    let cw = outer_hw + 4.0;
    frame.push_box(verts, idxs, -cw, cw, face_h - 1.5, face_h + 6.0, -5.0, face_depth + 3.0, CONCRETE_TRIM, MAT_CONCRETE);
    frame.push_box(verts, idxs, -outer_hw, outer_hw, face_h + 6.0, face_h + 10.5, -1.0, face_depth, CONCRETE_PORTAL, MAT_CONCRETE);

    // 3. Alternating Black / Safety-Yellow Portal Jamb Clearance Chevrons
    for s in [-1.0_f32, 1.0] {
        for j in 0..4 {
            let y0 = 6.0 + j as f32 * 8.5;
            let col = if j % 2 == 0 { [0.95, 0.75, 0.08] } else { [0.10, 0.12, 0.14] };
            frame.push_box(
                verts,
                idxs,
                s * (c + 1.0),
                s * (c + 4.8),
                y0,
                y0 + 7.2,
                -0.8,
                0.5,
                col,
                MAT_DEFAULT,
            );
        }
    }

    // 4. Overhead Highway Portal Entrance LED Strip
    frame.push_box(
        verts,
        idxs,
        -(c * 0.45),
        c * 0.45,
        prof.total_h - 2.6,
        prof.total_h - 0.6,
        4.0,
        11.0,
        [0.90, 0.96, 1.0],
        MAT_ROAD_LED,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_portal_wing_walls(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    frame: &PortalFrame,
    outer_hw: f32,
    wing_len: f32,
    wing_thick: f32,
    splay_rad: f32,
    top_h_near: f32,
    is_road: bool,
) {
    let steps = 4usize;
    let dx_s = splay_rad.sin();
    let dz_s = splay_rad.cos();
    let (wall_col, cap_col, mat) = if is_road {
        (CONCRETE_PORTAL, CONCRETE_TRIM, MAT_CONCRETE)
    } else {
        (STONE_MAIN, STONE_LIGHT, MAT_STONE)
    };
    let top_h_far = 11.0_f32;

    for s in [-1.0_f32, 1.0] {
        // Splay outward into -Z: as u advances toward -Z, x moves toward s * (outer_hw + dx_s * d)
        // With local_yaw: world/portal-local X axis rotates by -s * splay_rad when extending along -Z.
        let wall_yaw = -s * splay_rad;
        for step in 0..steps {
            let u_mid = (step as f32 + 0.5) / steps as f32;
            let seg_len = wing_len / steps as f32 + 1.5;
            let top_y = top_h_near + (top_h_far - top_h_near) * u_mid;
            let lx = s * (outer_hw + dx_s * (u_mid * wing_len) - wing_thick * 0.25);
            let lz = -dz_s * (u_mid * wing_len) + 2.0;

            frame.push_angled_box(
                verts,
                idxs,
                lx,
                top_y * 0.5,
                lz,
                wall_yaw,
                [wing_thick, top_y, seg_len],
                wall_col,
                mat,
            );
            frame.push_angled_box(
                verts,
                idxs,
                lx,
                top_y + 1.1,
                lz,
                wall_yaw,
                [wing_thick + 1.8, 2.2, seg_len + 0.4],
                cap_col,
                mat,
            );
            if is_road {
                frame.push_angled_box(
                    verts,
                    idxs,
                    lx,
                    top_y + 5.6,
                    lz,
                    wall_yaw,
                    [1.2, 6.8, 1.2],
                    [0.58, 0.62, 0.68],
                    MAT_STEEL_DARK,
                );
                frame.push_angled_box(
                    verts,
                    idxs,
                    lx,
                    top_y + 9.2,
                    lz,
                    wall_yaw,
                    [1.2, 1.2, seg_len],
                    [0.64, 0.68, 0.74],
                    MAT_STEEL_DARK,
                );
            }
        }
    }
}

// ============================================================================
//  Public Entry Point 2: Shared Vaulted Tunnel Bore Lining + Ribs + Lighting + Catenary Hangers
// ============================================================================

