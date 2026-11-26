use super::*;

/// Computes the horizontal unit miter normal `(nx, nz)` and miter scale `1 / cos(half_angle)`
/// at vertex `i` of a 3D polyline so consecutive curved segments share vertices seamlessly without V-cuts.
pub(crate) fn polyline_miter_normal(pts: &[[f32; 3]], i: usize) -> (f32, f32, f32) {
    let seg_norm = |a: [f32; 3], b: [f32; 3]| -> (f32, f32) {
        let dx = b[0] - a[0];
        let dz = b[2] - a[2];
        let len = dx.hypot(dz).max(1e-5);
        (-dz / len, dx / len)
    };
    let n = pts.len();
    if n < 2 {
        return (1.0, 0.0, 1.0);
    }
    if i == 0 {
        let (nx, nz) = seg_norm(pts[0], pts[1]);
        return (nx, nz, 1.0);
    }
    if i >= n - 1 {
        let (nx, nz) = seg_norm(pts[n - 2], pts[n - 1]);
        return (nx, nz, 1.0);
    }
    let (n0x, n0z) = seg_norm(pts[i - 1], pts[i]);
    let (n1x, n1z) = seg_norm(pts[i], pts[i + 1]);
    let mx = n0x + n1x;
    let mz = n0z + n1z;
    let mlen = mx.hypot(mz);
    if mlen < 1e-4 {
        return (n1x, n1z, 1.0);
    }
    let ux = mx / mlen;
    let uz = mz / mlen;
    let cos_half = (ux * n1x + uz * n1z).clamp(0.45, 1.0);
    (ux, uz, 1.0 / cos_half)
}

/// Extrudes a realistic multi-tier railway ballast roadbed (*Schotterbett*) along `pts`
/// with a wide sub-ballast shoulder, sloped crushed-stone embankment, beveled upper crown,
/// slightly recessed center crib shadow, and sealed start/end/bottom faces using `MAT_BALLAST`.
pub(super) fn push_ballast_bed_polyline_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pts: &[[f32; 3]],
    top_w: f32,
    bot_w: f32,
    height: f32,
    color: [f32; 3],
) {
    if pts.len() < 2 {
        return;
    }
    let ht = top_w * 0.5;
    let hm = (top_w * 0.56 + bot_w * 0.44) * 0.5;
    let hb = bot_w * 0.5;
    let h_mid = height * 0.68;

    let crown_col = color;
    let crib_col = [color[0] * 0.89, color[1] * 0.88, color[2] * 0.87];
    let upper_slope_col = [color[0] * 0.92, color[1] * 0.91, color[2] * 0.90];
    let lower_slope_col = [color[0] * 0.80, color[1] * 0.79, color[2] * 0.78];
    let cap_col = [color[0] * 0.74, color[1] * 0.73, color[2] * 0.72];

    let n = pts.len();
    let mut ring_cl = Vec::with_capacity(n); // inner crib left
    let mut ring_cr = Vec::with_capacity(n); // inner crib right
    let mut ring_tl = Vec::with_capacity(n); // top crown left
    let mut ring_tr = Vec::with_capacity(n); // top crown right
    let mut ring_ml = Vec::with_capacity(n); // mid shoulder left
    let mut ring_mr = Vec::with_capacity(n); // mid shoulder right
    let mut ring_bl = Vec::with_capacity(n); // bottom toe left
    let mut ring_br = Vec::with_capacity(n); // bottom toe right
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
        let py = raw_p[1];
        let pz = raw_p[2] + tz * pad;
        let eff_hc = ht * 0.55 * mscale;
        let eff_ht = ht * mscale;
        let eff_hm = hm * mscale;
        let eff_hb = hb * mscale;
        ring_cl.push([px + nx * eff_hc, py + height, pz + nz * eff_hc]);
        ring_cr.push([px - nx * eff_hc, py + height, pz - nz * eff_hc]);
        ring_tl.push([px + nx * eff_ht, py + height - 0.15, pz + nz * eff_ht]);
        ring_tr.push([px - nx * eff_ht, py + height - 0.15, pz - nz * eff_ht]);
        ring_ml.push([px + nx * eff_hm, py + h_mid, pz + nz * eff_hm]);
        ring_mr.push([px - nx * eff_hm, py + h_mid, pz - nz * eff_hm]);
        ring_bl.push([px + nx * eff_hb, py, pz + nz * eff_hb]);
        ring_br.push([px - nx * eff_hb, py, pz - nz * eff_hb]);
        ring_norm.push((nx, nz));
    }

    for i in 0..(n - 1) {
        let (nx0, nz0) = ring_norm[i];
        let (nx1, nz1) = ring_norm[i + 1];
        let nx = (nx0 + nx1) * 0.5;
        let nz = (nz0 + nz1) * 0.5;

        // Center ballast crib between rails (slightly darker oil/soot-weathered gravel)
        push_quad_mat(verts, idxs, ring_cl[i], ring_cr[i], ring_cr[i + 1], ring_cl[i + 1], [0.0, 1.0, 0.0], crib_col, MAT_BALLAST);
        // Left & right top ballast shoulders outside rails
        push_quad_mat(verts, idxs, ring_tl[i], ring_cl[i], ring_cl[i + 1], ring_tl[i + 1], normalize3([nx * 0.12, 1.0, nz * 0.12]), crown_col, MAT_BALLAST);
        push_quad_mat(verts, idxs, ring_cr[i], ring_tr[i], ring_tr[i + 1], ring_cr[i + 1], normalize3([-nx * 0.12, 1.0, -nz * 0.12]), crown_col, MAT_BALLAST);
        // Upper beveled crown slope
        push_quad_mat(verts, idxs, ring_ml[i], ring_tl[i], ring_tl[i + 1], ring_ml[i + 1], normalize3([nx, 0.85, nz]), upper_slope_col, MAT_BALLAST);
        push_quad_mat(verts, idxs, ring_tr[i], ring_mr[i], ring_mr[i + 1], ring_tr[i + 1], normalize3([-nx, 0.85, -nz]), upper_slope_col, MAT_BALLAST);
        // Lower sub-ballast embankment slope
        push_quad_mat(verts, idxs, ring_bl[i], ring_ml[i], ring_ml[i + 1], ring_bl[i + 1], normalize3([nx, 0.52, nz]), lower_slope_col, MAT_BALLAST);
        push_quad_mat(verts, idxs, ring_mr[i], ring_br[i], ring_br[i + 1], ring_mr[i + 1], normalize3([-nx, 0.52, -nz]), lower_slope_col, MAT_BALLAST);
        // Bottom base face
        push_quad_mat(verts, idxs, ring_br[i], ring_bl[i], ring_bl[i + 1], ring_br[i + 1], [0.0, -1.0, 0.0], cap_col, MAT_BALLAST);
    }

    // Sealed start & end caps
    let (nx0, nz0) = ring_norm[0];
    let n_start = normalize3([-nz0, 0.0, nx0]);
    push_quad_mat(verts, idxs, ring_br[0], ring_mr[0], ring_ml[0], ring_bl[0], n_start, cap_col, MAT_BALLAST);
    push_quad_mat(verts, idxs, ring_mr[0], ring_tr[0], ring_tl[0], ring_ml[0], n_start, cap_col, MAT_BALLAST);

    let (nxn, nzn) = ring_norm[n - 1];
    let n_end = normalize3([nzn, 0.0, -nxn]);
    push_quad_mat(verts, idxs, ring_bl[n - 1], ring_ml[n - 1], ring_mr[n - 1], ring_br[n - 1], n_end, cap_col, MAT_BALLAST);
    push_quad_mat(verts, idxs, ring_ml[n - 1], ring_tl[n - 1], ring_tr[n - 1], ring_mr[n - 1], n_end, cap_col, MAT_BALLAST);
}

/// Extrudes a continuous mitered trapezoidal prism (roadbed) along `pts` with sealed start, end, and bottom faces.
pub(super) fn push_trapezoid_polyline_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pts: &[[f32; 3]],
    top_w: f32,
    bot_w: f32,
    height: f32,
    color: [f32; 3],
) {
    if pts.len() < 2 {
        return;
    }
    let ht = top_w * 0.5;
    let hb = bot_w * 0.5;
    let slope_col = [color[0] * 0.86, color[1] * 0.86, color[2] * 0.86];
    let cap_col = [color[0] * 0.78, color[1] * 0.78, color[2] * 0.78];

    // Pad start and end outward by 0.25 mm along tangent so connected track joints have zero hairline seam
    let mut ring_tl = Vec::with_capacity(pts.len());
    let mut ring_tr = Vec::with_capacity(pts.len());
    let mut ring_bl = Vec::with_capacity(pts.len());
    let mut ring_br = Vec::with_capacity(pts.len());
    let mut ring_norm = Vec::with_capacity(pts.len());

    let n = pts.len();
    for (i, &raw_p) in pts.iter().enumerate() {
        let (nx, nz, mscale) = polyline_miter_normal(pts, i);
        // Tangent vector `(tx, tz)` is `(nz, -nx)` since `(nx, nz) = (-dz/len, dx/len)`
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
        let py = raw_p[1];
        let pz = raw_p[2] + tz * pad;
        let eff_ht = ht * mscale;
        let eff_hb = hb * mscale;
        ring_tl.push([px + nx * eff_ht, py + height, pz + nz * eff_ht]);
        ring_tr.push([px - nx * eff_ht, py + height, pz - nz * eff_ht]);
        ring_bl.push([px + nx * eff_hb, py, pz + nz * eff_hb]);
        ring_br.push([px - nx * eff_hb, py, pz - nz * eff_hb]);
        ring_norm.push((nx, nz));
    }

    for i in 0..(n - 1) {
        let (nx0, nz0) = ring_norm[i];
        let (nx1, nz1) = ring_norm[i + 1];
        let nx = (nx0 + nx1) * 0.5;
        let nz = (nz0 + nz1) * 0.5;
        // Top roadbed
        push_quad(verts, idxs, ring_tl[i], ring_tr[i], ring_tr[i + 1], ring_tl[i + 1], [0.0, 1.0, 0.0], color);
        // Bottom base face (prevents seeing through from underneath elevated tracks)
        push_quad(verts, idxs, ring_br[i], ring_bl[i], ring_bl[i + 1], ring_br[i + 1], [0.0, -1.0, 0.0], cap_col);
        // Left embankment slope
        push_quad(verts, idxs, ring_bl[i], ring_tl[i], ring_tl[i + 1], ring_bl[i + 1], normalize3([nx, 0.65, nz]), slope_col);
        // Right embankment slope
        push_quad(verts, idxs, ring_tr[i], ring_br[i], ring_br[i + 1], ring_tr[i + 1], normalize3([-nx, 0.65, -nz]), slope_col);
    }

    // Start end-cap (at i = 0, facing -tangent)
    let (nx0, nz0) = ring_norm[0];
    push_quad(
        verts,
        idxs,
        ring_br[0],
        ring_tr[0],
        ring_tl[0],
        ring_bl[0],
        normalize3([-nz0, 0.0, nx0]),
        cap_col,
    );
    // End end-cap (at i = n - 1, facing +tangent)
    let (nxn, nzn) = ring_norm[n - 1];
    push_quad(
        verts,
        idxs,
        ring_bl[n - 1],
        ring_tl[n - 1],
        ring_tr[n - 1],
        ring_br[n - 1],
        normalize3([nzn, 0.0, -nxn]),
        cap_col,
    );
}

/// Extrudes a continuous mitered rectangular prism (rail profile, bridge deck, road stripe) along `pts` with sealed start, end, and bottom faces.
pub(crate) fn push_box_polyline_mat_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pts: &[[f32; 3]],
    width: f32,
    height: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    if pts.len() < 2 {
        return;
    }
    let hw = width * 0.5;
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
        let py = raw_p[1];
        let pz = raw_p[2] + tz * pad;
        let eff_hw = hw * mscale;
        ring_tl.push([px + nx * eff_hw, py + height, pz + nz * eff_hw]);
        ring_tr.push([px - nx * eff_hw, py + height, pz - nz * eff_hw]);
        ring_bl.push([px + nx * eff_hw, py, pz + nz * eff_hw]);
        ring_br.push([px - nx * eff_hw, py, pz - nz * eff_hw]);
        ring_norm.push((nx, nz));
    }

    for i in 0..(n - 1) {
        let (nx0, nz0) = ring_norm[i];
        let (nx1, nz1) = ring_norm[i + 1];
        let nx = (nx0 + nx1) * 0.5;
        let nz = (nz0 + nz1) * 0.5;
        push_quad_mat(verts, idxs, ring_tl[i], ring_tr[i], ring_tr[i + 1], ring_tl[i + 1], [0.0, 1.0, 0.0], color, mat);
        push_quad_mat(verts, idxs, ring_br[i], ring_bl[i], ring_bl[i + 1], ring_br[i + 1], [0.0, -1.0, 0.0], color, mat);
        push_quad_mat(verts, idxs, ring_bl[i], ring_tl[i], ring_tl[i + 1], ring_bl[i + 1], normalize3([nx, 0.0, nz]), color, mat);
        push_quad_mat(verts, idxs, ring_tr[i], ring_br[i], ring_br[i + 1], ring_tr[i + 1], normalize3([-nx, 0.0, -nz]), color, mat);
    }

    let (nx0, nz0) = ring_norm[0];
    push_quad_mat(
        verts,
        idxs,
        ring_br[0],
        ring_tr[0],
        ring_tl[0],
        ring_bl[0],
        normalize3([-nz0, 0.0, nx0]),
        color,
        mat,
    );
    let (nxn, nzn) = ring_norm[n - 1];
    push_quad_mat(
        verts,
        idxs,
        ring_bl[n - 1],
        ring_tl[n - 1],
        ring_tr[n - 1],
        ring_br[n - 1],
        normalize3([nzn, 0.0, -nxn]),
        color,
        mat,
    );
}

#[allow(dead_code, clippy::too_many_arguments)]
pub(super) fn push_trapezoid_segment(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    top_w: f32,
    bot_w: f32,
    height: f32,
    color: [f32; 3],
) {
    push_trapezoid_polyline_3d(verts, idxs, &[a, b], top_w, bot_w, height, color);
}

pub(super) fn push_box_segment(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    width: f32,
    height: f32,
    color: [f32; 3],
) {
    push_box_segment_mat(verts, idxs, a, b, width, height, color, MAT_DEFAULT);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_box_segment_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    width: f32,
    height: f32,
    color: [f32; 3],
    mat: [f32; 3],
) {
    let dx = b[0] - a[0];
    let dz = b[2] - a[2];
    let len = dx.hypot(dz).max(1e-4);
    let nx = -dz / len;
    let nz = dx / len;
    let tx = nz;
    let tz = -nx;
    let hw = width * 0.5;

    let atl = [a[0] + nx * hw, a[1] + height, a[2] + nz * hw];
    let atr = [a[0] - nx * hw, a[1] + height, a[2] - nz * hw];
    let btl = [b[0] + nx * hw, b[1] + height, b[2] + nz * hw];
    let btr = [b[0] - nx * hw, b[1] + height, b[2] - nz * hw];

    let abl = [a[0] + nx * hw, a[1], a[2] + nz * hw];
    let abr = [a[0] - nx * hw, a[1], a[2] - nz * hw];
    let bbl = [b[0] + nx * hw, b[1], b[2] + nz * hw];
    let bbr = [b[0] - nx * hw, b[1], b[2] - nz * hw];

    push_quad_mat(verts, idxs, atl, atr, btr, btl, [0.0, 1.0, 0.0], color, mat);
    push_quad_mat(verts, idxs, abr, abl, bbl, bbr, [0.0, -1.0, 0.0], color, mat);
    push_quad_mat(verts, idxs, abl, atl, btl, bbl, [nx, 0.0, nz], color, mat);
    push_quad_mat(verts, idxs, atr, abr, bbr, btr, [-nx, 0.0, -nz], color, mat);
    push_quad_mat(verts, idxs, abr, atr, atl, abl, [-tx, 0.0, -tz], color, mat);
    push_quad_mat(verts, idxs, bbl, btl, btr, bbr, [tx, 0.0, tz], color, mat);
}

pub(super) fn push_rotated_box(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    local_center: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
) {
    push_rotated_box_mat(verts, idxs, origin, rot_rad, local_center, size, color, MAT_DEFAULT);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_rotated_box_mat(
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
    push_quad_mat(verts, idxs, pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, hy, hz), pt(-hx, hy, hz), rn(0.0, 0.0, 1.0), color, mat);
    push_quad_mat(verts, idxs, pt(hx, -hy, -hz), pt(-hx, -hy, -hz), pt(-hx, hy, -hz), pt(hx, hy, -hz), rn(0.0, 0.0, -1.0), color, mat);
    push_quad_mat(verts, idxs, pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), rn(1.0, 0.0, 0.0), color, mat);
    push_quad_mat(verts, idxs, pt(-hx, -hy, -hz), pt(-hx, -hy, hz), pt(-hx, hy, hz), pt(-hx, hy, -hz), rn(-1.0, 0.0, 0.0), color, mat);
}

pub(super) fn push_box(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    center: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
) {
    let hx = size[0] * 0.5;
    let hy = size[1] * 0.5;
    let hz = size[2] * 0.5;
    let [cx, cy, cz] = center;

    push_quad(verts, idxs, [cx - hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz + hz], [cx - hx, cy + hy, cz + hz], [0.0, 1.0, 0.0], color);
    push_quad(verts, idxs, [cx - hx, cy - hy, cz + hz], [cx + hx, cy - hy, cz + hz], [cx + hx, cy + hy, cz + hz], [cx - hx, cy + hy, cz + hz], [0.0, 0.0, 1.0], color);
    push_quad(verts, idxs, [cx + hx, cy - hy, cz - hz], [cx - hx, cy - hy, cz - hz], [cx - hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz - hz], [0.0, 0.0, -1.0], color);
    push_quad(verts, idxs, [cx + hx, cy - hy, cz + hz], [cx + hx, cy - hy, cz - hz], [cx + hx, cy + hy, cz - hz], [cx + hx, cy + hy, cz + hz], [1.0, 0.0, 0.0], color);
    push_quad(verts, idxs, [cx - hx, cy - hy, cz - hz], [cx - hx, cy - hy, cz + hz], [cx - hx, cy + hy, cz + hz], [cx - hx, cy + hy, cz - hz], [-1.0, 0.0, 0.0], color);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_quad(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
) {
    push_quad_mat(verts, idxs, p0, p1, p2, p3, normal, color, MAT_DEFAULT);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_quad_mat(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    mat: [f32; 3],
) {
    let base = verts.len() as u32;
    verts.push(Vertex3D::new(p0, normal, color, mat));
    verts.push(Vertex3D::new(p1, normal, color, mat));
    verts.push(Vertex3D::new(p2, normal, color, mat));
    verts.push(Vertex3D::new(p3, normal, color, mat));
    idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_glass_quad(
    verts: &mut Vec<Vertex3D>,
    glass_idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    alpha: f32,
    mat: [f32; 3],
) {
    let base = verts.len() as u32;
    verts.push(Vertex3D::new_alpha(p0, normal, color, alpha, mat));
    verts.push(Vertex3D::new_alpha(p1, normal, color, alpha, mat));
    verts.push(Vertex3D::new_alpha(p2, normal, color, alpha, mat));
    verts.push(Vertex3D::new_alpha(p3, normal, color, alpha, mat));
    glass_idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_tri(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
) {
    let base = verts.len() as u32;
    verts.push(Vertex3D::new(p0, normal, color, MAT_DEFAULT));
    verts.push(Vertex3D::new(p1, normal, color, MAT_DEFAULT));
    verts.push(Vertex3D::new(p2, normal, color, MAT_DEFAULT));
    idxs.extend_from_slice(&[base, base + 1, base + 2]);
}

// ============================================================================
//  Minimal 4x4 Column-Major Matrix Math
// ============================================================================

pub(super) fn mat4_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let rl = (right - left).max(1e-4);
    let tb = (top - bottom).max(1e-4);
    let fn_ = (far - near).max(1e-4);
    [
        [2.0 / rl, 0.0, 0.0, 0.0],
        [0.0, 2.0 / tb, 0.0, 0.0],
        [0.0, 0.0, -1.0 / fn_, 0.0],
        [-(right + left) / rl, -(top + bottom) / tb, -near / fn_, 1.0],
    ]
}

pub(super) fn compute_shadow_bounds(layout: &Layout, boards: &[BaseboardConfig]) -> ([f32; 3], f32) {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    let mut max_y = 60.0_f32;

    for b in boards {
        if !b.enabled {
            continue;
        }
        for pt in board_tools::get_baseboard_polygon(b) {
            min_x = min_x.min(pt.x as f32);
            max_x = max_x.max(pt.x as f32);
            min_z = min_z.min(pt.y as f32);
            max_z = max_z.max(pt.y as f32);
        }
    }
    for p in &layout.pieces {
        let e_max = p
            .elevation
            .map(|e| e.start.max(e.end) as f32)
            .unwrap_or(0.0);
        max_y = max_y.max(e_max + 80.0);
        for c in &p.connectors {
            min_x = min_x.min(c.x as f32);
            max_x = max_x.max(c.x as f32);
            min_z = min_z.min(c.y as f32);
            max_z = max_z.max(c.y as f32);
        }
    }
    if let Some(models) = &layout.models {
        for m in models {
            if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                max_y = max_y.max(m.height as f32 * m.scale.unwrap_or(1.0) as f32);
                continue;
            }
            let sc = m.scale.unwrap_or(1.0) as f32;
            let hw = (m.width.max(m.depth) as f32 * sc) * 0.5;
            let mx = m.transform.x as f32;
            let mz = m.transform.y as f32;
            min_x = min_x.min(mx - hw);
            max_x = max_x.max(mx + hw);
            min_z = min_z.min(mz - hw);
            max_z = max_z.max(mz + hw);
            max_y = max_y.max(m.elevation.unwrap_or(0.0) as f32 + m.height as f32 * sc);
        }
    }

    if !min_x.is_finite() {
        return ([0.0, 40.0, 0.0], 1600.0);
    }
    let center = [
        (min_x + max_x) * 0.5,
        (max_y * 0.35).clamp(20.0, 220.0),
        (min_z + max_z) * 0.5,
    ];
    let half_extent = ((max_x - min_x).max(max_z - min_z) * 0.5 + 320.0).clamp(650.0, 6500.0);
    (center, half_extent)
}

pub(super) fn compute_sun_view_proj(
    sun_dir: [f32; 3],
    center: [f32; 3],
    half_extent: f32,
) -> [[f32; 4]; 4] {
    let dir = normalize3(sun_dir);
    let up = if dir[1].abs() > 0.98 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let dist = (half_extent * 2.5).max(3000.0);
    let light_eye = [
        center[0] + dir[0] * dist,
        center[1] + dir[1] * dist,
        center[2] + dir[2] * dist,
    ];
    let near = 100.0;
    let far = dist * 2.0 + half_extent * 1.5;
    let view = mat4_look_at(light_eye, center, up);
    let proj = mat4_ortho(-half_extent, half_extent, -half_extent, half_extent, near, far);
    mat4_mul(proj, view)
}

pub(super) fn mat4_perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y * 0.5).tan();
    let nf = 1.0 / (near - far);
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far * nf, -1.0],
        [0.0, 0.0, far * near * nf, 0.0],
    ]
}

pub(super) fn mat4_look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize3([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    let s = normalize3(cross3(f, up));
    let u = cross3(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot3(s, eye), -dot3(u, eye), dot3(f, eye), 1.0],
    ]
}

pub(super) fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0_f32; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] = a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

pub(crate) fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = dot3(v, v).sqrt().max(1e-6);
    [v[0] / len, v[1] / len, v[2] / len]
}

