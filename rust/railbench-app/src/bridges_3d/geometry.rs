use super::*;

pub(super) fn sample_polyline_at_dist(pts: &[[f32; 3]], cum: &[f32], target_d: f32) -> PathSample {
    let n = pts.len();
    if n < 2 {
        return PathSample {
            pos: pts.first().copied().unwrap_or([0.0; 3]),
            tangent: [1.0, 0.0],
            normal: [0.0, 1.0],
        };
    }
    let total = *cum.last().unwrap_or(&0.0);
    let d = target_d.clamp(0.0, total);
    let mut seg = 0usize;
    while seg + 1 < n - 1 && cum[seg + 1] < d {
        seg += 1;
    }
    let d0 = cum[seg];
    let d1 = cum[seg + 1];
    let span = (d1 - d0).max(1e-5);
    let t = ((d - d0) / span).clamp(0.0, 1.0);
    let a = pts[seg];
    let b = pts[seg + 1];
    let pos = [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ];
    let tan = seg_unit_dir(a, b);
    let norm = [-tan[1], tan[0]];
    PathSample {
        pos,
        tangent: tan,
        normal: norm,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_offset_box_polyline_mat_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pts: &[[f32; 3]],
    lateral_off: f32,
    y_off: f32,
    width: f32,
    height: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    let mut offset_pts = Vec::with_capacity(pts.len());
    for (i, &p) in pts.iter().enumerate() {
        let (nx, nz, mscale) = polyline_miter_normal(pts, i);
        offset_pts.push([
            p[0] + nx * lateral_off * mscale,
            p[1] + y_off,
            p[2] + nz * lateral_off * mscale,
        ]);
    }
    push_box_polyline_mat_3d(verts, idxs, &offset_pts, width, height, color, mat);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_vertical_offset_polyline_mat_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pts: &[[f32; 3]],
    y_off: f32,
    width: f32,
    height: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    let shifted: Vec<[f32; 3]> = pts.iter().map(|p| [p[0], p[1] + y_off, p[2]]).collect();
    push_box_polyline_mat_3d(verts, idxs, &shifted, width, height, color, mat);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_soffit_polyline_mat_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pts: &[[f32; 3]],
    y_off: f32,
    top_w: f32,
    bot_w: f32,
    height: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    if pts.len() < 2 {
        return;
    }
    let ht = top_w * 0.5;
    let hb = bot_w * 0.5;
    let cap_col = [color[0] * 0.85, color[1] * 0.85, color[2] * 0.85];
    let n = pts.len();
    let mut ring_tl = Vec::with_capacity(n);
    let mut ring_tr = Vec::with_capacity(n);
    let mut ring_bl = Vec::with_capacity(n);
    let mut ring_br = Vec::with_capacity(n);
    let mut ring_norm = Vec::with_capacity(n);

    for (i, &raw_p) in pts.iter().enumerate() {
        let (nx, nz, mscale) = polyline_miter_normal(pts, i);
        let tx = nz;
        let tz = -nx;
        let pad = if i == 0 {
            -0.25_f32
        } else if i + 1 == n {
            0.25_f32
        } else {
            0.0_f32
        };
        let px = raw_p[0] + tx * pad;
        let py = raw_p[1] + y_off;
        let pz = raw_p[2] + tz * pad;
        ring_tl.push([px + nx * ht * mscale, py + height, pz + nz * ht * mscale]);
        ring_tr.push([px - nx * ht * mscale, py + height, pz - nz * ht * mscale]);
        ring_bl.push([px + nx * hb * mscale, py, pz + nz * hb * mscale]);
        ring_br.push([px - nx * hb * mscale, py, pz - nz * hb * mscale]);
        ring_norm.push((nx, nz));
    }

    for i in 0..(n - 1) {
        let nx = 0.5 * (ring_norm[i].0 + ring_norm[i + 1].0);
        let nz = 0.5 * (ring_norm[i].1 + ring_norm[i + 1].1);
        push_quad_mat(verts, idxs, ring_tl[i], ring_tr[i], ring_tr[i + 1], ring_tl[i + 1], [0.0, 1.0, 0.0], color, mat);
        push_quad_mat(verts, idxs, ring_br[i], ring_bl[i], ring_bl[i + 1], ring_br[i + 1], [0.0, -1.0, 0.0], cap_col, mat);
        push_quad_mat(verts, idxs, ring_bl[i], ring_tl[i], ring_tl[i + 1], ring_bl[i + 1], normalize3([nx, -0.45, nz]), color, mat);
        push_quad_mat(verts, idxs, ring_tr[i], ring_br[i], ring_br[i + 1], ring_tr[i + 1], normalize3([-nx, -0.45, -nz]), color, mat);
    }

    let (nx0, nz0) = ring_norm[0];
    push_quad_mat(verts, idxs, ring_br[0], ring_tr[0], ring_tl[0], ring_bl[0], normalize3([-nz0, 0.0, nx0]), cap_col, mat);
    let (nxn, nzn) = ring_norm[n - 1];
    push_quad_mat(verts, idxs, ring_bl[n - 1], ring_tl[n - 1], ring_tr[n - 1], ring_br[n - 1], normalize3([nzn, 0.0, -nxn]), cap_col, mat);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_oriented_box_6_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    local_center: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
    mat: [f32; 3],
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

    push_quad_mat(verts, idxs, pt(-hx, hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 1.0, 0.0), color, mat);
    push_quad_mat(verts, idxs, pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(-hx, -hy, -hz), rn(0.0, -1.0, 0.0), color, mat);
    push_quad_mat(verts, idxs, pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 0.0, 1.0), color, mat);
    push_quad_mat(verts, idxs, pt(hx, -hy, -hz), pt(-hx, -hy, -hz), pt(-hx, hy, -hz), pt(hx, hy, -hz), rn(0.0, 0.0, -1.0), color, mat);
    push_quad_mat(verts, idxs, pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), rn(1.0, 0.0, 0.0), color, mat);
    push_quad_mat(verts, idxs, pt(-hx, -hy, -hz), pt(-hx, -hy, hz), pt(-hx, hy, hz), pt(-hx, hy, -hz), rn(-1.0, 0.0, 0.0), color, mat);
}

/// Emits a closed 6-faced 3D structural strut between arbitrary 3D endpoints `a` and `b`.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_strut_3d_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    width: f32,
    depth: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    let diff = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    if dot3(diff, diff) < 1e-4 {
        return;
    }
    let dir = normalize3(diff);
    let ref_up = if dir[1].abs() > 0.96 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let right = normalize3(cross3(dir, ref_up));
    let up = normalize3(cross3(right, dir));
    let hw = width * 0.5;
    let hd = depth * 0.5;

    let corner = |end: [f32; 3], r_s: f32, u_s: f32| -> [f32; 3] {
        [
            end[0] + right[0] * (r_s * hw) + up[0] * (u_s * hd),
            end[1] + right[1] * (r_s * hw) + up[1] * (u_s * hd),
            end[2] + right[2] * (r_s * hw) + up[2] * (u_s * hd),
        ]
    };

    let a_tl = corner(a, -1.0, 1.0);
    let a_tr = corner(a, 1.0, 1.0);
    let a_bl = corner(a, -1.0, -1.0);
    let a_br = corner(a, 1.0, -1.0);
    let b_tl = corner(b, -1.0, 1.0);
    let b_tr = corner(b, 1.0, 1.0);
    let b_bl = corner(b, -1.0, -1.0);
    let b_br = corner(b, 1.0, -1.0);

    let neg_right = [-right[0], -right[1], -right[2]];
    let neg_up = [-up[0], -up[1], -up[2]];
    let neg_dir = [-dir[0], -dir[1], -dir[2]];

    push_quad_mat(verts, idxs, a_tl, a_tr, b_tr, b_tl, up, color, mat);
    push_quad_mat(verts, idxs, a_br, a_bl, b_bl, b_br, neg_up, color, mat);
    push_quad_mat(verts, idxs, a_tr, a_br, b_br, b_tr, right, color, mat);
    push_quad_mat(verts, idxs, a_bl, a_tl, b_tl, b_bl, neg_right, color, mat);
    push_quad_mat(verts, idxs, a_br, a_tr, a_tl, a_bl, neg_dir, color, mat);
    push_quad_mat(verts, idxs, b_bl, b_tl, b_tr, b_br, dir, color, mat);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_tapered_prism_y_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    yaw: f32,
    y_bot: f32,
    y_top: f32,
    bot_lx: f32,
    bot_lz: f32,
    top_lx: f32,
    top_lz: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    if y_top <= y_bot + 0.05 {
        return;
    }
    let bhx = bot_lx * 0.5;
    let bhz = bot_lz * 0.5;
    let thx = top_lx * 0.5;
    let thz = top_lz * 0.5;
    let to_w = |x: f32, y: f32, z: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(x, z, yaw);
        [origin[0] + rx, origin[1] + y, origin[2] + rz]
    };
    let rn = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(nx, nz, yaw);
        [rx, ny, rz]
    };
    let center = to_w(0.0, 0.5 * (y_bot + y_top), 0.0);

    let b0 = to_w(-bhx, y_bot, -bhz);
    let b1 = to_w(bhx, y_bot, -bhz);
    let b2 = to_w(bhx, y_bot, bhz);
    let b3 = to_w(-bhx, y_bot, bhz);
    let t0 = to_w(-thx, y_top, -thz);
    let t1 = to_w(thx, y_top, -thz);
    let t2 = to_w(thx, y_top, thz);
    let t3 = to_w(-thx, y_top, thz);

    push_quad_mat(verts, idxs, t0, t1, t2, t3, rn(0.0, 1.0, 0.0), color, mat);
    push_quad_mat(verts, idxs, b3, b2, b1, b0, rn(0.0, -1.0, 0.0), color, mat);
    push_outward_quad(verts, idxs, b3, b2, t2, t3, center, color, mat);
    push_outward_quad(verts, idxs, b1, b0, t0, t1, center, color, mat);
    push_outward_quad(verts, idxs, b2, b1, t1, t2, center, color, mat);
    push_outward_quad(verts, idxs, b0, b3, t3, t0, center, color, mat);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_cutwater_prism_y_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    yaw: f32,
    y_bot: f32,
    y_top: f32,
    half_w_bot: f32,
    half_w_top: f32,
    base_z_bot: f32,
    base_z_top: f32,
    tip_z_bot: f32,
    tip_z_top: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    if y_top <= y_bot + 0.05 {
        return;
    }
    let to_w = |x: f32, y: f32, z: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(x, z, yaw);
        [origin[0] + rx, origin[1] + y, origin[2] + rz]
    };

    let bl = to_w(-half_w_bot, y_bot, base_z_bot);
    let br = to_w(half_w_bot, y_bot, base_z_bot);
    let bt = to_w(0.0, y_bot, tip_z_bot);
    let tl = to_w(-half_w_top, y_top, base_z_top);
    let tr = to_w(half_w_top, y_top, base_z_top);
    let tt = to_w(0.0, y_top, tip_z_top);

    let center = [
        (bl[0] + br[0] + bt[0] + tl[0] + tr[0] + tt[0]) / 6.0,
        (bl[1] + br[1] + bt[1] + tl[1] + tr[1] + tt[1]) / 6.0,
        (bl[2] + br[2] + bt[2] + tl[2] + tr[2] + tt[2]) / 6.0,
    ];

    push_tri_mat(verts, idxs, tl, tr, tt, [0.0, 1.0, 0.0], color, mat);
    push_tri_mat(verts, idxs, bl, bt, br, [0.0, -1.0, 0.0], color, mat);
    push_outward_quad(verts, idxs, bl, tl, tt, bt, center, color, mat);
    push_outward_quad(verts, idxs, bt, tt, tr, br, center, color, mat);
    push_outward_quad(verts, idxs, br, tr, tl, bl, center, color, mat);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_outward_quad(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    interior: [f32; 3],
    color: [f32; 3],
    mat: [f32; 3],
) {
    let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let mut n = normalize3(cross3(e1, e2));
    let fc = [
        0.25 * (p0[0] + p1[0] + p2[0] + p3[0]),
        0.25 * (p0[1] + p1[1] + p2[1] + p3[1]),
        0.25 * (p0[2] + p1[2] + p2[2] + p3[2]),
    ];
    let to_out = [fc[0] - interior[0], fc[1] - interior[1], fc[2] - interior[2]];
    if dot3(n, to_out) < 0.0 {
        n = [-n[0], -n[1], -n[2]];
    }
    push_quad_mat(verts, idxs, p0, p1, p2, p3, n, color, mat);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_tri_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    mat: [f32; 3],
) {
    let base = verts.len() as u32;
    verts.push(Vertex3D::new(p0, normal, color, mat));
    verts.push(Vertex3D::new(p1, normal, color, mat));
    verts.push(Vertex3D::new(p2, normal, color, mat));
    idxs.extend_from_slice(&[base, base + 1, base + 2]);
}

// ============================================================================
//  Unit Tests
// ============================================================================

