use railbench_core::types::Model3D;

use crate::viewer3d::{normalize3, rot_xz, Vertex3D, MAT_DEFAULT};

/// Per-vertex PBR-like material descriptor packed into `Vertex3D`:
/// - `rough`: surface roughness `[0.04, 1.0]`
/// - `metal`: metallic factor `[0.0, 1.0]`
/// - `emissive`: `> 0.0` always-on self-illumination; `< 0.0` dusk/night window glow magnitude
#[derive(Clone, Copy)]
struct Mat {
    col: [f32; 3],
    rough: f32,
    metal: f32,
    emissive: f32,
}

impl Mat {
    const fn matte(rgb: u32, rough: f32) -> Self {
        Self {
            col: hex(rgb),
            rough,
            metal: 0.02,
            emissive: 0.0,
        }
    }

    const fn metal(rgb: u32, rough: f32, metal: f32) -> Self {
        Self {
            col: hex(rgb),
            rough,
            metal,
            emissive: 0.0,
        }
    }

    const fn glass(rgb: u32) -> Self {
        Self {
            col: hex(rgb),
            rough: 0.10,
            metal: 0.25,
            emissive: 0.0,
        }
    }

    /// Daytime dark reflective pane that glows warmly with `emissive = -glow` as night falls.
    const fn warm_window(rgb: u32, glow: f32) -> Self {
        Self {
            col: hex(rgb),
            rough: 0.14,
            metal: 0.18,
            emissive: -glow,
        }
    }

    const fn emissive(rgb: u32, strength: f32) -> Self {
        Self {
            col: hex(rgb),
            rough: 0.20,
            metal: 0.05,
            emissive: strength,
        }
    }

    fn with_col(self, col: [f32; 3]) -> Self {
        Self { col, ..self }
    }

    fn pack(self) -> [f32; 3] {
        [self.rough, self.metal, self.emissive]
    }
}

/// Convert `0xRRGGBB` to a linear-weighted color curve suitable for `Rgba8UnormSrgb` output.
const fn hex(rgb: u32) -> [f32; 3] {
    let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
    let b = (rgb & 0xFF) as f32 / 255.0;
    // Slight gamma darkening (x^1.32 = x * sqrt(sqrt(x)) approx via polynomial) keeps stone, brick & wood rich
    [
        r * (0.68 + 0.32 * r),
        g * (0.68 + 0.32 * g),
        b * (0.68 + 0.32 * b),
    ]
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn hash_u32(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x & 0x00FF_FFFF) as f32 / 16_777_215.0
}

#[inline]
fn rot_euler_xyz(p: [f32; 3], rx: f32, ry: f32, rz: f32) -> [f32; 3] {
    let [mut x, mut y, mut z] = p;
    if rx.abs() > 1e-6 {
        let (s, c) = rx.sin_cos();
        let ny = y * c - z * s;
        let nz = y * s + z * c;
        y = ny;
        z = nz;
    }
    if ry.abs() > 1e-6 {
        let (s, c) = ry.sin_cos();
        let nx = x * c + z * s;
        let nz = -x * s + z * c;
        x = nx;
        z = nz;
    }
    if rz.abs() > 1e-6 {
        let (s, c) = rz.sin_cos();
        let nx = x * c - y * s;
        let ny = x * s + y * c;
        x = nx;
        y = ny;
    }
    [x, y, z]
}

struct ModelBuilder<'a> {
    verts: &'a mut Vec<Vertex3D>,
    idxs: &'a mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    scale: [f32; 3],
    inv_scale: [f32; 3],
}

impl<'a> ModelBuilder<'a> {
    fn new(
        verts: &'a mut Vec<Vertex3D>,
        idxs: &'a mut Vec<u32>,
        origin: [f32; 3],
        rot_rad: f32,
        target_size: [f32; 3],
        nominal_size: [f32; 3],
    ) -> Self {
        let sx = (target_size[0] / nominal_size[0].max(1.0)).max(0.05);
        let sy = (target_size[1] / nominal_size[1].max(1.0)).max(0.05);
        let sz = (target_size[2] / nominal_size[2].max(1.0)).max(0.05);
        Self {
            verts,
            idxs,
            origin,
            rot_rad,
            scale: [sx, sy, sz],
            inv_scale: [1.0 / sx, 1.0 / sy, 1.0 / sz],
        }
    }

    #[inline]
    fn xform_pos(&self, p: [f32; 3]) -> [f32; 3] {
        let sx = p[0] * self.scale[0];
        let sy = p[1] * self.scale[1];
        let sz = p[2] * self.scale[2];
        let (rx, rz) = rot_xz(sx, sz, self.rot_rad);
        [self.origin[0] + rx, self.origin[1] + sy, self.origin[2] + rz]
    }

    #[inline]
    fn xform_norm(&self, n: [f32; 3]) -> [f32; 3] {
        let nx = n[0] * self.inv_scale[0];
        let ny = n[1] * self.inv_scale[1];
        let nz = n[2] * self.inv_scale[2];
        let (rx, rz) = rot_xz(nx, nz, self.rot_rad);
        normalize3([rx, ny, rz])
    }

    #[inline]
    fn push_raw_tri(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], n: [f32; 3], m: Mat) {
        let wn = self.xform_norm(n);
        let mp = m.pack();
        let base = self.verts.len() as u32;
        self.verts.push(Vertex3D { pos: self.xform_pos(p0), normal: wn, color: m.col, mat: mp });
        self.verts.push(Vertex3D { pos: self.xform_pos(p1), normal: wn, color: m.col, mat: mp });
        self.verts.push(Vertex3D { pos: self.xform_pos(p2), normal: wn, color: m.col, mat: mp });
        self.idxs.extend_from_slice(&[base, base + 1, base + 2]);
    }

    #[inline]
    fn push_raw_quad(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], n: [f32; 3], m: Mat) {
        let wn = self.xform_norm(n);
        let mp = m.pack();
        let base = self.verts.len() as u32;
        self.verts.push(Vertex3D { pos: self.xform_pos(p0), normal: wn, color: m.col, mat: mp });
        self.verts.push(Vertex3D { pos: self.xform_pos(p1), normal: wn, color: m.col, mat: mp });
        self.verts.push(Vertex3D { pos: self.xform_pos(p2), normal: wn, color: m.col, mat: mp });
        self.verts.push(Vertex3D { pos: self.xform_pos(p3), normal: wn, color: m.col, mat: mp });
        self.idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn quad_oriented(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], outward: [f32; 3], m: Mat) {
        let mut n = cross3(sub3(p1, p0), sub3(p2, p0));
        if dot3(n, n) < 1e-8 {
            n = outward;
        }
        if dot3(n, outward) < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        self.push_raw_quad(p0, p1, p2, p3, normalize3(n), m);
    }

    fn tri_oriented(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], outward: [f32; 3], m: Mat) {
        let mut n = cross3(sub3(p1, p0), sub3(p2, p0));
        if dot3(n, n) < 1e-8 {
            n = outward;
        }
        if dot3(n, outward) < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        self.push_raw_tri(p0, p1, p2, normalize3(n), m);
    }

    fn rbox(&mut self, center: [f32; 3], size: [f32; 3], m: Mat) {
        self.obox(center, size, [0.0, 0.0, 0.0], m);
    }

    fn obox(&mut self, center: [f32; 3], size: [f32; 3], euler: [f32; 3], m: Mat) {
        let hx = size[0] * 0.5;
        let hy = size[1] * 0.5;
        let hz = size[2] * 0.5;
        let [rx, ry, rz] = euler;
        let pt = |x: f32, y: f32, z: f32| -> [f32; 3] {
            let r = rot_euler_xyz([x, y, z], rx, ry, rz);
            [center[0] + r[0], center[1] + r[1], center[2] + r[2]]
        };
        let nm = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
            normalize3(rot_euler_xyz([nx, ny, nz], rx, ry, rz))
        };

        // Top (+Y) & Bottom (-Y)
        self.push_raw_quad(pt(-hx, hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), pt(-hx, hy, hz), nm(0.0, 1.0, 0.0), m);
        self.push_raw_quad(pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(-hx, -hy, -hz), nm(0.0, -1.0, 0.0), m);
        // Front (+Z) & Back (-Z)
        self.push_raw_quad(pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, hy, hz), pt(-hx, hy, hz), nm(0.0, 0.0, 1.0), m);
        self.push_raw_quad(pt(hx, -hy, -hz), pt(-hx, -hy, -hz), pt(-hx, hy, -hz), pt(hx, hy, -hz), nm(0.0, 0.0, -1.0), m);
        // Right (+X) & Left (-X)
        self.push_raw_quad(pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), nm(1.0, 0.0, 0.0), m);
        self.push_raw_quad(pt(-hx, -hy, -hz), pt(-hx, -hy, hz), pt(-hx, hy, hz), pt(-hx, hy, -hz), nm(-1.0, 0.0, 0.0), m);
    }

    fn strut_between(&mut self, p0: [f32; 3], p1: [f32; 3], width: f32, depth: f32, m: Mat) {
        let d = sub3(p1, p0);
        let len = dot3(d, d).sqrt().max(1e-4);
        let axis = [d[0] / len, d[1] / len, d[2] / len];
        let up_ref = if axis[1].abs() > 0.92 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
        let u = normalize3(cross3(axis, up_ref));
        let v = normalize3(cross3(u, axis));
        let hw = width * 0.5;
        let hd = depth * 0.5;

        let mk = |base: [f32; 3], su: f32, sv: f32| -> [f32; 3] {
            [
                base[0] + u[0] * su * hw + v[0] * sv * hd,
                base[1] + u[1] * su * hw + v[1] * sv * hd,
                base[2] + u[2] * su * hw + v[2] * sv * hd,
            ]
        };
        let a00 = mk(p0, -1.0, -1.0);
        let a10 = mk(p0, 1.0, -1.0);
        let a11 = mk(p0, 1.0, 1.0);
        let a01 = mk(p0, -1.0, 1.0);
        let b00 = mk(p1, -1.0, -1.0);
        let b10 = mk(p1, 1.0, -1.0);
        let b11 = mk(p1, 1.0, 1.0);
        let b01 = mk(p1, -1.0, 1.0);

        self.push_raw_quad(a10, a11, b11, b10, u, m);
        self.push_raw_quad(a01, a00, b00, b01, [-u[0], -u[1], -u[2]], m);
        self.push_raw_quad(a11, a01, b01, b11, v, m);
        self.push_raw_quad(a00, a10, b10, b00, [-v[0], -v[1], -v[2]], m);
        self.push_raw_quad(b00, b10, b11, b01, axis, m);
        self.push_raw_quad(a01, a11, a10, a00, [-axis[0], -axis[1], -axis[2]], m);
    }

    fn cyl_between(&mut self, p0: [f32; 3], p1: [f32; 3], r0: f32, r1: f32, segs: usize, m: Mat) {
        let segs = segs.max(5);
        let d = sub3(p1, p0);
        let len = dot3(d, d).sqrt().max(1e-4);
        let axis = [d[0] / len, d[1] / len, d[2] / len];
        let up_ref = if axis[1].abs() > 0.92 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
        let u = normalize3(cross3(axis, up_ref));
        let v = normalize3(cross3(axis, u));
        let dr = r0 - r1;

        for i in 0..segs {
            let a0 = (i as f32 / segs as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / segs as f32) * std::f32::consts::TAU;
            let (s0, c0) = a0.sin_cos();
            let (s1, c1) = a1.sin_cos();
            let am = 0.5 * (a0 + a1);
            let (sm, cm) = am.sin_cos();

            let dir0 = [u[0] * c0 + v[0] * s0, u[1] * c0 + v[1] * s0, u[2] * c0 + v[2] * s0];
            let dir1 = [u[0] * c1 + v[0] * s1, u[1] * c1 + v[1] * s1, u[2] * c1 + v[2] * s1];
            let dirm = [u[0] * cm + v[0] * sm, u[1] * cm + v[1] * sm, u[2] * cm + v[2] * sm];
            let n_side = normalize3([
                dirm[0] * len + axis[0] * dr,
                dirm[1] * len + axis[1] * dr,
                dirm[2] * len + axis[2] * dr,
            ]);

            let q0 = [p0[0] + dir0[0] * r0, p0[1] + dir0[1] * r0, p0[2] + dir0[2] * r0];
            let q1 = [p0[0] + dir1[0] * r0, p0[1] + dir1[1] * r0, p0[2] + dir1[2] * r0];
            let q2 = [p1[0] + dir1[0] * r1, p1[1] + dir1[1] * r1, p1[2] + dir1[2] * r1];
            let q3 = [p1[0] + dir0[0] * r1, p1[1] + dir0[1] * r1, p1[2] + dir0[2] * r1];

            if r0 > 1e-3 && r1 > 1e-3 {
                self.push_raw_quad(q0, q1, q2, q3, n_side, m);
            } else if r0 > 1e-3 {
                self.push_raw_tri(q0, q1, p1, n_side, m);
            }

            if r0 > 1e-3 {
                self.push_raw_tri(p0, q1, q0, [-axis[0], -axis[1], -axis[2]], m);
            }
            if r1 > 1e-3 {
                self.push_raw_tri(p1, q3, q2, axis, m);
            }
        }
    }

    fn cyl_y(&mut self, center: [f32; 3], r_top: f32, r_bot: f32, height: f32, segs: usize, m: Mat) {
        let p0 = [center[0], center[1] - height * 0.5, center[2]];
        let p1 = [center[0], center[1] + height * 0.5, center[2]];
        self.cyl_between(p0, p1, r_bot, r_top, segs, m);
    }

    fn cyl_x(&mut self, center: [f32; 3], r_neg: f32, r_pos: f32, length: f32, segs: usize, m: Mat) {
        let p0 = [center[0] - length * 0.5, center[1], center[2]];
        let p1 = [center[0] + length * 0.5, center[1], center[2]];
        self.cyl_between(p0, p1, r_neg, r_pos, segs, m);
    }

    fn cyl_z(&mut self, center: [f32; 3], r_neg: f32, r_pos: f32, length: f32, segs: usize, m: Mat) {
        let p0 = [center[0], center[1], center[2] - length * 0.5];
        let p1 = [center[0], center[1], center[2] + length * 0.5];
        self.cyl_between(p0, p1, r_neg, r_pos, segs, m);
    }

    fn cone_y(&mut self, base_center: [f32; 3], radius: f32, height: f32, segs: usize, yaw_offset: f32, m: Mat) {
        let segs = segs.max(4);
        let apex = [base_center[0], base_center[1] + height, base_center[2]];
        for i in 0..segs {
            let a0 = yaw_offset + (i as f32 / segs as f32) * std::f32::consts::TAU;
            let a1 = yaw_offset + ((i + 1) as f32 / segs as f32) * std::f32::consts::TAU;
            let p0 = [base_center[0] + a0.cos() * radius, base_center[1], base_center[2] + a0.sin() * radius];
            let p1 = [base_center[0] + a1.cos() * radius, base_center[1], base_center[2] + a1.sin() * radius];
            let am = 0.5 * (a0 + a1);
            let n = normalize3([am.cos() * height, radius, am.sin() * height]);
            self.push_raw_tri(p0, p1, apex, n, m);
            self.push_raw_tri(base_center, p1, p0, [0.0, -1.0, 0.0], m);
        }
    }

    fn pyramid_y(&mut self, base_center: [f32; 3], half_w: f32, half_d: f32, height: f32, m: Mat) {
        let [cx, by, cz] = base_center;
        let apex = [cx, by + height, cz];
        let c0 = [cx - half_w, by, cz + half_d];
        let c1 = [cx + half_w, by, cz + half_d];
        let c2 = [cx + half_w, by, cz - half_d];
        let c3 = [cx - half_w, by, cz - half_d];
        self.tri_oriented(c0, c1, apex, [0.0, 0.5, 1.0], m);
        self.tri_oriented(c1, c2, apex, [1.0, 0.5, 0.0], m);
        self.tri_oriented(c2, c3, apex, [0.0, 0.5, -1.0], m);
        self.tri_oriented(c3, c0, apex, [-1.0, 0.5, 0.0], m);
        self.push_raw_quad(c3, c2, c1, c0, [0.0, -1.0, 0.0], m);
    }

    fn sphere(&mut self, center: [f32; 3], radius: f32, stacks: usize, slices: usize, m: Mat) {
        self.ellipsoid_perturbed(center, [radius, radius, radius], stacks, slices, 0.0, 0, m);
    }

    fn hemi_cap_x(&mut self, center: [f32; 3], radius: f32, depth_x: f32, dir_x: f32, stacks: usize, slices: usize, m: Mat) {
        let stacks = stacks.max(4);
        let slices = slices.max(8);
        for st in 0..stacks {
            let t0 = (st as f32 / stacks as f32) * std::f32::consts::FRAC_PI_2;
            let t1 = ((st + 1) as f32 / stacks as f32) * std::f32::consts::FRAC_PI_2;
            let (r0, x0) = (t0.cos() * radius, t0.sin() * depth_x * dir_x);
            let (r1, x1) = (t1.cos() * radius, t1.sin() * depth_x * dir_x);
            for sl in 0..slices {
                let a0 = (sl as f32 / slices as f32) * std::f32::consts::TAU;
                let a1 = ((sl + 1) as f32 / slices as f32) * std::f32::consts::TAU;
                let p0 = [center[0] + x0, center[1] + a0.cos() * r0, center[2] + a0.sin() * r0];
                let p1 = [center[0] + x0, center[1] + a1.cos() * r0, center[2] + a1.sin() * r0];
                let p2 = [center[0] + x1, center[1] + a1.cos() * r1, center[2] + a1.sin() * r1];
                let p3 = [center[0] + x1, center[1] + a0.cos() * r1, center[2] + a0.sin() * r1];
                let out = [dir_x * 0.7, (a0.cos() + a1.cos()) * 0.5, (a0.sin() + a1.sin()) * 0.5];
                if st + 1 < stacks {
                    self.quad_oriented(p0, p1, p2, p3, out, m);
                } else {
                    self.tri_oriented(p0, p1, p2, out, m);
                }
            }
        }
    }

    fn ellipsoid_perturbed(
        &mut self,
        center: [f32; 3],
        radii: [f32; 3],
        stacks: usize,
        slices: usize,
        jitter: f32,
        seed: u32,
        m: Mat,
    ) {
        let stacks = stacks.max(4);
        let slices = slices.max(6);
        let pt_at = |st: usize, sl: usize| -> ([f32; 3], f32) {
            let phi = (st as f32 / stacks as f32) * std::f32::consts::PI;
            let theta = ((sl % slices) as f32 / slices as f32) * std::f32::consts::TAU;
            let h = if st == 0 || st == stacks || jitter <= 0.0 {
                1.0
            } else {
                let n = hash_u32(seed.wrapping_add((st as u32) * 31 + (sl % slices) as u32 * 97));
                1.0 + (n - 0.5) * 2.0 * jitter
            };
            let sp = phi.sin();
            let dir = [sp * theta.cos(), phi.cos(), sp * theta.sin()];
            (
                [
                    center[0] + dir[0] * radii[0] * h,
                    center[1] + dir[1] * radii[1] * h,
                    center[2] + dir[2] * radii[2] * h,
                ],
                dir[1],
            )
        };
        for st in 0..stacks {
            for sl in 0..slices {
                let (p0, y0) = pt_at(st, sl);
                let (p1, _) = pt_at(st, sl + 1);
                let (p2, y2) = pt_at(st + 1, sl + 1);
                let (p3, _) = pt_at(st + 1, sl);
                let shade = (0.86 + 0.16 * (0.5 * (y0 + y2) * 0.5 + 0.5)).clamp(0.74, 1.06);
                let cm = m.with_col([
                    (m.col[0] * shade).min(1.0),
                    (m.col[1] * shade).min(1.0),
                    (m.col[2] * shade).min(1.0),
                ]);
                let out = normalize3(sub3(
                    [
                        0.25 * (p0[0] + p1[0] + p2[0] + p3[0]),
                        0.25 * (p0[1] + p1[1] + p2[1] + p3[1]),
                        0.25 * (p0[2] + p1[2] + p2[2] + p3[2]),
                    ],
                    center,
                ));
                if st == 0 {
                    self.tri_oriented(p0, p2, p3, out, cm);
                } else if st + 1 == stacks {
                    self.tri_oriented(p0, p1, p2, out, cm);
                } else {
                    self.quad_oriented(p0, p1, p2, p3, out, cm);
                }
            }
        }
    }

    fn organic_foliage_cluster(&mut self, center: [f32; 3], rx: f32, ry: f32, rz: f32, seed: u32, m: Mat) {
        self.ellipsoid_perturbed(center, [rx, ry, rz], 6, 9, 0.16, seed, m);
    }

    fn pine_whorl(
        &mut self,
        base_center: [f32; 3],
        radius: f32,
        height: f32,
        segs: usize,
        yaw_offset: f32,
        seed: u32,
        m: Mat,
    ) {
        let segs = segs.max(7);
        let apex = [base_center[0], base_center[1] + height, base_center[2]];
        let dark = m.with_col([m.col[0] * 0.72, m.col[1] * 0.72, m.col[2] * 0.72]);
        for i in 0..segs {
            let a0 = yaw_offset + (i as f32 / segs as f32) * std::f32::consts::TAU;
            let a1 = yaw_offset + ((i + 1) as f32 / segs as f32) * std::f32::consts::TAU;
            let r0 = radius * (0.86 + 0.24 * hash_u32(seed.wrapping_add(i as u32 * 17)));
            let r1 = radius * (0.86 + 0.24 * hash_u32(seed.wrapping_add(((i + 1) % segs) as u32 * 17)));
            let droop0 = -height * 0.08 * hash_u32(seed.wrapping_add(i as u32 * 43 + 9));
            let droop1 = -height * 0.08 * hash_u32(seed.wrapping_add(((i + 1) % segs) as u32 * 43 + 9));
            let p0 = [base_center[0] + a0.cos() * r0, base_center[1] + droop0, base_center[2] + a0.sin() * r0];
            let p1 = [base_center[0] + a1.cos() * r1, base_center[1] + droop1, base_center[2] + a1.sin() * r1];
            self.tri_oriented(p0, p1, apex, [0.5 * (a0.cos() + a1.cos()), radius / height.max(1.0), 0.5 * (a0.sin() + a1.sin())], m);
            self.tri_oriented(base_center, p1, p0, [0.0, -1.0, 0.0], dark);
        }
    }

    fn curved_pipe(&mut self, pts: &[[f32; 3]], radius: f32, segs: usize, m: Mat) {
        if pts.len() < 2 {
            return;
        }
        for w in pts.windows(2) {
            self.cyl_between(w[0], w[1], radius, radius, segs, m);
        }
    }

    fn gable_roof_x(
        &mut self,
        base_center: [f32; 3],
        len_x: f32,
        span_z: f32,
        ridge_h: f32,
        overhang: f32,
        thick: f32,
        roof_m: Mat,
        gable_m: Mat,
        ridge_m: Mat,
    ) {
        let [cx, by, cz] = base_center;
        let hx = len_x * 0.5 + overhang;
        let hz = span_z * 0.5 + overhang;
        let wx = len_x * 0.5;
        let wz = span_z * 0.5;

        // Solid overhanging roof slabs (+Z and -Z slopes)
        self.quad_oriented([cx - hx, by, cz + hz], [cx + hx, by, cz + hz], [cx + hx, by + ridge_h, cz], [cx - hx, by + ridge_h, cz], [0.0, 1.0, 0.6], roof_m);
        self.quad_oriented([cx - hx, by - thick, cz + hz], [cx + hx, by - thick, cz + hz], [cx + hx, by + ridge_h - thick, cz], [cx - hx, by + ridge_h - thick, cz], [0.0, -1.0, -0.4], roof_m);
        self.quad_oriented([cx - hx, by - thick, cz + hz], [cx + hx, by - thick, cz + hz], [cx + hx, by, cz + hz], [cx - hx, by, cz + hz], [0.0, 0.0, 1.0], ridge_m);

        self.quad_oriented([cx + hx, by, cz - hz], [cx - hx, by, cz - hz], [cx - hx, by + ridge_h, cz], [cx + hx, by + ridge_h, cz], [0.0, 1.0, -0.6], roof_m);
        self.quad_oriented([cx + hx, by - thick, cz - hz], [cx - hx, by - thick, cz - hz], [cx - hx, by + ridge_h - thick, cz], [cx + hx, by + ridge_h - thick, cz], [0.0, -1.0, 0.4], roof_m);
        self.quad_oriented([cx + hx, by - thick, cz - hz], [cx - hx, by - thick, cz - hz], [cx - hx, by, cz - hz], [cx + hx, by, cz - hz], [0.0, 0.0, -1.0], ridge_m);

        // End bargeboards at ±hx
        for sx in [-1.0_f32, 1.0] {
            let ex = cx + sx * hx;
            self.quad_oriented([ex, by - thick, cz + hz], [ex, by, cz + hz], [ex, by + ridge_h, cz], [ex, by + ridge_h - thick, cz], [sx, 0.0, 0.0], ridge_m);
            self.quad_oriented([ex, by, cz - hz], [ex, by - thick, cz - hz], [ex, by + ridge_h - thick, cz], [ex, by + ridge_h, cz], [sx, 0.0, 0.0], ridge_m);
        }

        // Brick/plaster gable wall triangles recessed cleanly inside roof ends
        self.tri_oriented([cx - wx, by, cz - wz], [cx - wx, by, cz + wz], [cx - wx, by + ridge_h - 0.5, cz], [-1.0, 0.0, 0.0], gable_m);
        self.tri_oriented([cx + wx, by, cz + wz], [cx + wx, by, cz - wz], [cx + wx, by + ridge_h - 0.5, cz], [1.0, 0.0, 0.0], gable_m);

        // Ridge cap along X
        self.rbox([cx, by + ridge_h + 0.6, cz], [len_x + overhang * 2.0 + 1.0, 2.0, 3.0], ridge_m);
    }

    fn hip_roof(
        &mut self,
        base_center: [f32; 3],
        len_x: f32,
        span_z: f32,
        ridge_len_x: f32,
        ridge_h: f32,
        roof_m: Mat,
        ridge_m: Mat,
    ) {
        let [cx, by, cz] = base_center;
        let hx = len_x * 0.5;
        let hz = span_z * 0.5;
        let rlx = (ridge_len_x * 0.5).min(hx * 0.9);
        let ry = by + ridge_h;

        let fl = [cx - hx, by, cz + hz];
        let fr = [cx + hx, by, cz + hz];
        let br = [cx + hx, by, cz - hz];
        let bl = [cx - hx, by, cz - hz];
        let rl = [cx - rlx, ry, cz];
        let rr = [cx + rlx, ry, cz];

        self.quad_oriented(fl, fr, rr, rl, [0.0, 0.7, 1.0], roof_m);
        self.quad_oriented(br, bl, rl, rr, [0.0, 0.7, -1.0], roof_m);
        self.tri_oriented(bl, fl, rl, [-1.0, 0.7, 0.0], roof_m);
        self.tri_oriented(fr, br, rr, [1.0, 0.7, 0.0], roof_m);
        self.quad_oriented(bl, fl, fr, br, [0.0, -1.0, 0.0], roof_m);
        self.rbox([cx, ry + 0.5, cz], [rlx * 2.0 + 2.0, 2.0, 2.8], ridge_m);
    }

    fn sawtooth_roof_bay(
        &mut self,
        x0: f32,
        x1: f32,
        z0: f32,
        z1: f32,
        base_y: f32,
        peak_h: f32,
        roof_m: Mat,
        glass_m: Mat,
        frame_m: Mat,
    ) {
        // Steep glazed north/front face at z1, sloped deck down to z0
        let fl = [x0, base_y, z1];
        let fr = [x1, base_y, z1];
        let tl = [x0, base_y + peak_h, z1 - (z1 - z0) * 0.08];
        let tr = [x1, base_y + peak_h, z1 - (z1 - z0) * 0.08];
        let bl = [x0, base_y, z0];
        let br = [x1, base_y, z0];

        // Sloped solid metal roof back
        self.quad_oriented(bl, br, tr, tl, [0.0, 1.0, -0.5], roof_m);
        // Side gable cheeks
        self.tri_oriented(bl, fl, tl, [-1.0, 0.0, 0.0], frame_m);
        self.tri_oriented(fr, br, tr, [1.0, 0.0, 0.0], frame_m);
        // Glazed skylight strip & mullions
        self.quad_oriented(fl, fr, tr, tl, [0.0, 0.2, 1.0], glass_m);
        self.rbox([(x0 + x1) * 0.5, base_y + peak_h + 0.4, tl[2]], [x1 - x0 + 1.0, 1.6, 2.2], frame_m);
        let span = x1 - x0;
        let n_mul = ((span / 32.0).round() as i32).max(2);
        for k in 1..n_mul {
            let mx = x0 + (k as f32 / n_mul as f32) * span;
            self.strut_between([mx, base_y, z1 + 0.3], [mx, base_y + peak_h, tl[2] + 0.3], 1.4, 1.0, frame_m);
        }
    }

    fn window_z(
        &mut self,
        x: f32,
        y: f32,
        wall_z: f32,
        side: f32,
        win_w: f32,
        win_h: f32,
        glass_m: Mat,
        frame_m: Mat,
        sill_m: Mat,
        lintel_m: Mat,
    ) {
        self.rbox([x, y, wall_z + side * 0.55], [win_w + 2.2, win_h + 2.2, 1.1], frame_m);
        self.rbox([x, y, wall_z + side * 0.90], [win_w, win_h, 0.9], glass_m);
        self.rbox([x, y, wall_z + side * 1.15], [0.9, win_h, 0.6], frame_m);
        self.rbox([x, y + win_h * 0.14, wall_z + side * 1.15], [win_w, 0.9, 0.6], frame_m);
        self.rbox([x, y - win_h * 0.5 - 1.1, wall_z + side * 1.35], [win_w + 4.0, 1.6, 2.6], sill_m);
        self.rbox([x, y + win_h * 0.5 + 1.3, wall_z + side * 0.95], [win_w + 3.2, 2.2, 1.6], lintel_m);
    }

    fn window_x(
        &mut self,
        wall_x: f32,
        y: f32,
        z: f32,
        side: f32,
        win_d: f32,
        win_h: f32,
        glass_m: Mat,
        frame_m: Mat,
        sill_m: Mat,
        lintel_m: Mat,
    ) {
        self.rbox([wall_x + side * 0.55, y, z], [1.1, win_h + 2.2, win_d + 2.2], frame_m);
        self.rbox([wall_x + side * 0.90, y, z], [0.9, win_h, win_d], glass_m);
        self.rbox([wall_x + side * 1.15, y, z], [0.6, win_h, 0.9], frame_m);
        self.rbox([wall_x + side * 1.15, y + win_h * 0.14, z], [0.6, 0.9, win_d], frame_m);
        self.rbox([wall_x + side * 1.35, y - win_h * 0.5 - 1.1, z], [2.6, 1.6, win_d + 4.0], sill_m);
        self.rbox([wall_x + side * 0.95, y + win_h * 0.5 + 1.3, z], [1.6, 2.2, win_d + 3.2], lintel_m);
    }

    fn clock_face_z(&mut self, center: [f32; 3], side: f32, radius: f32, rim_m: Mat, dial_m: Mat, hand_m: Mat) {
        self.cyl_z([center[0], center[1], center[2] + side * 0.5], radius, radius, 1.2, 14, rim_m);
        self.cyl_z([center[0], center[1], center[2] + side * 1.1], radius * 0.84, radius * 0.84, 0.5, 14, dial_m);
        let zf = center[2] + side * 1.45;
        self.rbox([center[0] + radius * 0.26, center[1], zf], [radius * 0.52, radius * 0.13, 0.4], hand_m);
        self.rbox([center[0], center[1] + radius * 0.34, zf], [radius * 0.11, radius * 0.68, 0.4], hand_m);
    }

    fn clock_face_x(&mut self, center: [f32; 3], side: f32, radius: f32, rim_m: Mat, dial_m: Mat, hand_m: Mat) {
        self.cyl_x([center[0] + side * 0.5, center[1], center[2]], radius, radius, 1.2, 14, rim_m);
        self.cyl_x([center[0] + side * 1.1, center[1], center[2]], radius * 0.84, radius * 0.84, 0.5, 14, dial_m);
        let xf = center[0] + side * 1.45;
        self.rbox([xf, center[1], center[2] + radius * 0.26], [0.4, radius * 0.13, radius * 0.52], hand_m);
        self.rbox([xf, center[1] + radius * 0.34, center[2]], [0.4, radius * 0.68, radius * 0.11], hand_m);
    }
}

pub(crate) fn push_env_pine_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, x: f32, floor_y: f32, z: f32, h: f32) {
    let w = h * 0.44;
    let mut b = ModelBuilder::new(verts, idxs, [x, floor_y, z], 0.0, [w, h, w], [55.0, 135.0, 55.0]);
    let bark = Mat::matte(0x3f2e21, 0.94);
    b.cyl_y([0.0, 22.0, 0.0], 2.8, 5.2, 44.0, 7, bark);
    let tiers = [
        (24.0, 27.0, 36.0, Mat::matte(0x14532d, 0.95), 0.1),
        (46.0, 23.0, 34.0, Mat::matte(0x166534, 0.94), 0.6),
        (70.0, 18.0, 32.0, Mat::matte(0x15803d, 0.93), 1.1),
        (95.0, 12.0, 38.0, Mat::matte(0x16a34a, 0.91), 0.4),
    ];
    for (i, (y, r, th, col, yaw)) in tiers.into_iter().enumerate() {
        b.pine_whorl([0.0, y, 0.0], r, th, 8, yaw, 700 + i as u32 * 31, col);
    }
}

pub(crate) fn push_builtin_model_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, m: &Model3D, y0: f32) {
    let scale = m.scale.unwrap_or(1.0) as f32;
    let w = m.width as f32 * scale;
    let d = m.depth as f32 * scale;
    let h = m.height as f32 * scale;
    let origin = [m.transform.x as f32, y0, m.transform.y as f32];
    let rot_rad = (m.transform.rotation as f32).to_radians();

    match m.builtin_id.as_deref() {
        Some("station") => build_station(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("signal-box") => build_signal_box(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("engine-shed") => build_engine_shed(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("turntable-trainhall") => build_turntable_trainhall(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("town-house") => build_town_house(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("water-tower") => build_water_tower(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("coaling-station") => build_coaling_station(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("diesel-fuel-station") => build_diesel_fuel_station(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("light-signal") => build_light_signal(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("buffer-stop") => build_buffer_stop(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("pine-tree") => build_pine_tree(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("deciduous-tree") => build_deciduous_tree(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("platform-lamp") => build_platform_lamp(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("platform") | Some("bahnsteig") => build_platform(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("skyscraper") => build_skyscraper(verts, idxs, origin, rot_rad, [w, h, d]),
        Some("industry-building") => build_industry_building(verts, idxs, origin, rot_rad, [w, h, d]),
        _ => build_generic_building(verts, idxs, origin, rot_rad, [w, h, d]),
    }
}

// ----------------------------------------------------------------------------
// 1. STATION (Bahnhof Neustadt — 3-wing Gründerzeit station + clock tower + glazed canopy)
// ----------------------------------------------------------------------------
fn build_station(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [320.0, 120.0, 140.0]);
    let stone = Mat::matte(0x64748b, 0.88);
    let platform = Mat::matte(0x94a3b8, 0.85);
    let forecourt = Mat::matte(0xcbd5e1, 0.84);
    let tactile = Mat::matte(0xfef08a, 0.72);
    let wall = Mat::matte(0xe5d6be, 0.78);
    let brick = Mat::matte(0x9a3412, 0.78);
    let cornice = Mat::matte(0xd6d3d1, 0.74);
    let roof = Mat::matte(0x991b1b, 0.68);
    let ridge = Mat::matte(0x7c2d12, 0.72);
    let copper = Mat::metal(0x2d6a56, 0.42, 0.35);
    let iron = Mat::metal(0x1e293b, 0.40, 0.72);
    let canopy_m = Mat::metal(0x334155, 0.50, 0.45);
    let skylight = Mat::glass(0x93c5fd);
    let win_dark = Mat::warm_window(0xfde68a, 0.38);
    let win_warm = Mat::warm_window(0xfde68a, 0.92);
    let frame = Mat::matte(0xf8fafc, 0.62);
    let door_w = Mat::matte(0x5c2e14, 0.74);
    let bench_w = Mat::matte(0x92400e, 0.78);
    let sign_blue = Mat::metal(0x1d4ed8, 0.45, 0.15);
    let sign_white = Mat::matte(0xf8fafc, 0.55);
    let brass = Mat::metal(0xd97706, 0.28, 0.82);
    let lamp_glow = Mat::emissive(0xfef08a, 0.95);
    let shrub_m = Mat::matte(0x15803d, 0.90);

    // 1. Plinth, house platform (+Z) & forecourt (-Z)
    b.rbox([0.0, 2.0, 0.0], [320.0, 4.0, 140.0], stone);
    b.rbox([0.0, 5.1, 48.0], [316.0, 2.2, 42.0], platform);
    b.rbox([0.0, 5.25, 68.5], [316.0, 2.5, 3.0], cornice);
    b.rbox([0.0, 6.45, 63.5], [312.0, 0.5, 2.2], tactile);
    b.rbox([0.0, 4.6, -60.0], [316.0, 1.2, 20.0], forecourt);

    // 2. Central main hall
    b.rbox([0.0, 8.0, -12.0], [142.0, 8.0, 88.0], stone);
    b.rbox([0.0, 41.0, -12.0], [140.0, 62.0, 86.0], wall);
    b.rbox([0.0, 40.0, -12.0], [143.0, 2.5, 89.0], cornice);
    b.rbox([0.0, 72.0, -12.0], [145.0, 3.0, 91.0], cornice);
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([sx * 68.5, 41.0, -12.0 + sz * 41.5], [4.5, 62.0, 4.5], brick);
        }
    }
    b.gable_roof_x([0.0, 73.5, -12.0], 144.0, 88.0, 26.0, 3.5, 2.0, roof, wall, ridge);

    // 4 roof dormers + brick chimney
    for dx in [-42.0_f32, 42.0] {
        for side in [-1.0_f32, 1.0] {
            let dz = -12.0 + side * 24.0;
            b.rbox([dx, 82.0, dz], [15.0, 10.0, 14.0], wall);
            b.rbox([dx, 87.8, dz], [17.5, 2.4, 16.5], roof);
            b.rbox([dx, 82.5, dz + side * 7.2], [7.5, 6.2, 0.9], win_warm);
        }
    }
    b.rbox([52.0, 94.0, -24.0], [9.5, 22.0, 9.5], brick);
    b.rbox([52.0, 105.6, -24.0], [12.0, 2.2, 12.0], cornice);

    // 3. Symmetrical left & right wings
    for side_x in [-1.0_f32, 1.0] {
        let wx = side_x * 112.0;
        b.rbox([wx, 7.5, -12.0], [88.0, 7.0, 76.0], stone);
        b.rbox([wx, 31.0, -12.0], [86.0, 44.0, 74.0], wall);
        b.rbox([wx, 53.0, -12.0], [90.0, 2.5, 78.0], cornice);
        for sz in [-1.0_f32, 1.0] {
            b.rbox([wx + side_x * 41.5, 31.0, -12.0 + sz * 35.5], [4.0, 44.0, 4.0], brick);
        }
        b.gable_roof_x([wx, 54.2, -12.0], 88.0, 76.0, 20.0, 3.0, 1.8, roof, wall, ridge);
        b.window_x(wx + side_x * 43.1, 33.0, -12.0, side_x, 11.0, 14.5, win_warm, frame, cornice, brick);
    }

    // 4. Central Clock Tower + 4 clock dials + green copper spire + brass finial
    b.rbox([0.0, 56.0, -12.0], [34.0, 104.0, 34.0], wall);
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([sx * 16.0, 90.0, -12.0 + sz * 16.0], [3.0, 38.0, 3.0], brick);
        }
    }
    b.rbox([0.0, 86.0, -12.0], [37.0, 2.4, 37.0], cornice);
    b.rbox([0.0, 108.5, -12.0], [39.0, 2.8, 39.0], cornice);
    for side in [-1.0_f32, 1.0] {
        b.clock_face_z([0.0, 98.0, -12.0 + side * 17.2], side, 8.2, iron, sign_white, iron);
        b.clock_face_x([side * 17.2, 98.0, -12.0], side, 8.2, iron, sign_white, iron);
    }
    b.pyramid_y([0.0, 110.0, -12.0], 20.5, 20.5, 30.0, copper);
    b.sphere([0.0, 141.0, -12.0], 2.0, 6, 8, brass);
    b.cyl_y([0.0, 147.0, -12.0], 0.4, 0.9, 12.0, 8, brass);

    // 5. Windows, grand portals, DB station boards & planters
    for side in [-1.0_f32, 1.0] {
        let wz = -12.0 + side * 43.0;
        for dx in [-46.0_f32, -26.0, 26.0, 46.0] {
            let wm = if dx.abs() < 30.0 { win_warm } else { win_dark };
            b.window_z(dx, 24.0, wz, side, 10.5, 16.0, wm, frame, cornice, brick);
        }
        for dx in [-46.0_f32, -23.0, 0.0, 23.0, 46.0] {
            let wm = if dx == 0.0 || dx.abs() > 40.0 { win_warm } else { win_dark };
            b.window_z(dx, 55.0, wz, side, 10.5, 15.5, wm, frame, cornice, brick);
        }
        // Main entrance portal
        for px in [-12.0_f32, 12.0] {
            b.rbox([px, 20.5, wz + side * 1.8], [3.8, 28.0, 4.5], cornice);
        }
        b.rbox([0.0, 35.5, wz + side * 2.2], [29.0, 4.2, 5.5], cornice);
        b.rbox([0.0, 20.0, wz + side * 0.5], [20.0, 27.5, 1.2], iron);
        for dx in [-4.5_f32, 4.5] {
            b.rbox([dx, 16.5, wz + side * 1.1], [8.0, 21.0, 1.1], door_w);
        }
        b.rbox([0.0, 30.2, wz + side * 1.1], [16.8, 4.6, 1.1], win_warm);
        b.rbox([0.0, 5.2, wz + side * 3.4], [28.0, 2.0, 6.8], cornice);

        // Wing windows
        let wing_wz = -12.0 + side * 37.0;
        for side_x in [-1.0_f32, 1.0] {
            let wx = side_x * 112.0;
            for ox in [-24.0_f32, 0.0, 24.0] {
                let wm = if ox == 0.0 { win_warm } else { win_dark };
                b.window_z(wx + ox, 27.0, wing_wz, side, 10.0, 15.0, wm, frame, cornice, brick);
            }
        }
    }
    b.rbox([0.0, 43.0, -56.4], [54.0, 7.0, 1.2], sign_blue);
    b.rbox([0.0, 43.0, -56.7], [42.0, 2.4, 1.2], sign_white);
    for px in [-24.0_f32, 24.0] {
        b.rbox([px, 7.6, -60.0], [10.0, 5.4, 10.0], stone);
        b.organic_foliage_cluster([px, 14.5, -60.0], 5.5, 5.2, 5.5, (px as i32 + 50) as u32, shrub_m);
    }

    // 6. Trackside glazed canopy (+Z) on cast-iron columns, benches, hanging signs & lamps
    b.obox([0.0, 40.8, 36.0], [298.0, 2.0, 13.0], [0.08, 0.0, 0.0], canopy_m);
    b.obox([0.0, 39.9, 47.5], [294.0, 1.2, 11.0], [0.08, 0.0, 0.0], skylight);
    b.obox([0.0, 38.9, 59.0], [298.0, 2.0, 13.0], [0.08, 0.0, 0.0], canopy_m);
    b.rbox([0.0, 37.8, 65.5], [298.0, 3.2, 1.5], iron);

    for i in 0..=5 {
        let x = -130.0 + (i as f32) * 52.0;
        b.rbox([x, 8.5, 58.0], [5.8, 5.0, 5.8], stone);
        b.cyl_y([x, 24.5, 58.0], 1.6, 2.0, 30.0, 10, iron);
        b.rbox([x, 38.0, 58.0], [4.8, 2.0, 4.8], iron);
        b.obox([x, 38.4, 46.0], [2.0, 2.2, 35.0], [0.08, 0.0, 0.0], iron);
    }
    for sx in [-78.0_f32, 0.0, 78.0] {
        b.rbox([sx, 33.8, 56.0], [28.0, 5.0, 1.2], sign_blue);
        b.rbox([sx, 33.8, 56.0], [22.0, 1.8, 1.5], sign_white);
    }
    for lx in [-104.0_f32, -52.0, 52.0, 104.0] {
        b.cyl_y([lx, 36.4, 49.0], 2.6, 3.6, 2.0, 10, iron);
        b.sphere([lx, 35.0, 49.0], 1.7, 6, 8, lamp_glow);
    }
    for bx in [-95.0_f32, -40.0, 40.0, 95.0] {
        b.rbox([bx, 10.2, 42.0], [22.0, 1.2, 5.5], bench_w);
        b.rbox([bx, 13.0, 39.5], [22.0, 4.2, 1.0], bench_w);
        for lx in [-8.5_f32, 8.5] {
            b.rbox([bx + lx, 8.4, 42.0], [1.2, 4.4, 5.0], iron);
        }
    }
    b.rbox([22.0, 18.0, 33.8], [13.5, 15.0, 2.2], iron);
    b.rbox([22.0, 18.0, 34.1], [11.0, 12.2, 2.2], sign_white);
}

// ----------------------------------------------------------------------------
// 2. SIGNAL BOX (Stellwerk W1 — brick lower, cantilevered glazed upper, hip roof & steel stairs)
// ----------------------------------------------------------------------------
fn build_signal_box(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [120.0, 110.0, 75.0]);
    let stone = Mat::matte(0x64748b, 0.88);
    let cornice = Mat::matte(0xd6d3d1, 0.75);
    let brick = Mat::matte(0x9a3412, 0.82);
    let upper_wall = Mat::matte(0xe5d6be, 0.76);
    let steel = Mat::metal(0x1e293b, 0.42, 0.72);
    let roof = Mat::matte(0x334155, 0.62);
    let ridge = Mat::matte(0x1e293b, 0.65);
    let win_dark = Mat::warm_window(0xfde68a, 0.35);
    let win_warm = Mat::warm_window(0xfde68a, 0.90);
    let frame = Mat::matte(0xf8fafc, 0.65);
    let door_w = Mat::matte(0x5c2e14, 0.78);
    let sign_blue = Mat::metal(0x1d4ed8, 0.45, 0.15);
    let sign_white = Mat::matte(0xf8fafc, 0.55);
    let desk = Mat::matte(0x475569, 0.60);
    let lever_red = Mat::metal(0xdc2626, 0.40, 0.40);
    let lever_blue = Mat::metal(0x2563eb, 0.40, 0.40);

    // 1. Concrete plinth & brick ground floor
    b.rbox([0.0, 2.5, 0.0], [118.0, 5.0, 73.0], stone);
    b.rbox([-4.0, 26.0, 0.0], [92.0, 42.0, 58.0], brick);
    b.rbox([-4.0, 48.0, 0.0], [108.0, 3.2, 70.0], cornice);
    for side in [-1.0_f32, 1.0] {
        for dx in [-32.0_f32, -8.0, 16.0] {
            let wm = if side > 0.0 && dx == -8.0 { win_warm } else { win_dark };
            b.window_z(dx, 27.0, side * 29.0, side, 8.6, 11.6, wm, frame, cornice, stone);
        }
    }
    b.rbox([42.1, 16.0, 0.0], [1.6, 22.0, 13.0], cornice);
    b.rbox([42.4, 15.5, 0.0], [1.6, 20.0, 10.5], door_w);

    // 2. Cantilevered interlocking room with panoramic windows + interior lever bank
    b.rbox([-4.0, 55.0, 0.0], [106.0, 11.0, 68.0], upper_wall);
    b.rbox([-4.0, 81.0, 0.0], [106.0, 6.0, 68.0], upper_wall);
    b.rbox([-4.0, 69.5, 0.0], [104.0, 18.0, 66.0], win_warm);
    b.rbox([-4.0, 64.0, 12.0], [66.0, 8.5, 9.0], desk);
    for lx in (-28..=20).step_by(8) {
        let lm = if lx % 16 == 0 { lever_red } else { lever_blue };
        b.obox([lx as f32, 70.0, 12.0], [1.2, 7.0, 1.2], [0.25, 0.0, 0.0], lm);
    }
    for side in [-1.0_f32, 1.0] {
        for mx in (-56..=48).step_by(13) {
            b.rbox([mx as f32, 69.5, side * 33.4], [2.2, 19.0, 2.2], steel);
        }
    }
    for side_x in [-56.0_f32, 48.0] {
        for k in 0..4 {
            let mz = -22.0 + (k as f32) * 14.5;
            b.rbox([side_x, 69.5, mz], [2.2, 19.0, 2.2], steel);
        }
    }
    for side in [-1.0_f32, 1.0] {
        b.rbox([-4.0, 55.0, side * 34.4], [34.0, 6.4, 1.2], sign_blue);
        b.rbox([-4.0, 55.0, side * 34.6], [26.0, 2.2, 1.2], sign_white);
    }

    // 3. Overhanging hip roof + vent + radio antenna
    b.rbox([-4.0, 85.5, 0.0], [116.0, 3.2, 78.0], roof);
    b.hip_roof([-4.0, 87.0, 0.0], 116.0, 78.0, 56.0, 17.5, roof, ridge);
    b.rbox([22.0, 99.0, -12.0], [9.5, 6.0, 9.5], stone);
    b.cyl_y([-36.0, 102.0, -14.0], 0.45, 0.85, 18.0, 8, steel);

    // 4. Exterior steel staircase on +X
    b.rbox([54.0, 49.0, -18.0], [13.0, 2.0, 18.0], steel);
    for i in 0..11 {
        let fi = i as f32;
        b.rbox([54.0, 7.0 + fi * 4.1, 24.0 - fi * 3.8], [11.0, 1.4, 4.0], steel);
    }
    b.strut_between([59.5, 7.0, 25.0], [59.5, 49.0, -16.0], 1.3, 2.4, steel);
    b.strut_between([59.5, 17.0, 25.0], [59.5, 59.0, -16.0], 1.0, 1.2, steel);
    b.strut_between([59.5, 7.0, 25.0], [59.5, 17.0, 25.0], 1.0, 1.0, steel);
    b.strut_between([59.5, 49.0, -16.0], [59.5, 59.0, -16.0], 1.0, 1.0, steel);
}

// ----------------------------------------------------------------------------
// 3. ENGINE SHED (Lokschuppen — open -X portal for snapped track, arched lintel, open doors, pit, vents)
// ----------------------------------------------------------------------------
fn build_engine_shed(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [240.0, 105.0, 72.0]);
    let brick = Mat::matte(0x852e16, 0.85);
    let trim = Mat::matte(0xc4b5a5, 0.80);
    let concrete = Mat::matte(0x52525b, 0.90);
    let pit = Mat::matte(0x18181b, 0.95);
    let rail = Mat::metal(0xb0bcc9, 0.24, 0.85);
    let roof = Mat::matte(0x334155, 0.65);
    let skylight = Mat::glass(0x93c5fd);
    let win = Mat::warm_window(0xfde68a, 0.65);
    let frame = Mat::metal(0x1e293b, 0.55, 0.35);
    let door_w = Mat::matte(0x78350f, 0.85);
    let iron = Mat::metal(0x0f172a, 0.55, 0.55);
    let buf_red = Mat::matte(0xb91c1c, 0.70);
    let lamp_glow = Mat::emissive(0xfef08a, 0.95);

    // 1. Concrete foundation + inspection pit + interior H0 rails + rear buffer stop
    b.rbox([0.0, 1.5, 0.0], [242.0, 3.0, 72.0], concrete);
    b.rbox([10.0, 3.1, 0.0], [190.0, 1.0, 20.0], pit);
    b.rbox([-10.0, 3.9, -8.25], [220.0, 1.8, 1.5], rail);
    b.rbox([-10.0, 3.9, 8.25], [220.0, 1.8, 1.5], rail);
    b.rbox([95.0, 10.0, 0.0], [4.0, 5.5, 22.0], buf_red);
    b.rbox([95.0, 6.5, -8.0], [3.0, 7.0, 3.0], iron);
    b.rbox([95.0, 6.5, 8.0], [3.0, 7.0, 3.0], iron);

    // 2. Brick side & rear walls (clear opening at -X from Z=-22..+22, Y=3..82)
    let wall_h = 79.0_f32;
    let wall_y = 3.0 + wall_h * 0.5;
    b.rbox([0.0, wall_y, -34.0], [240.0, wall_h, 3.0], brick);
    b.rbox([0.0, wall_y, 34.0], [240.0, wall_h, 3.0], brick);
    b.rbox([118.5, wall_y, 0.0], [3.0, wall_h, 70.0], brick);
    // Masonry pilasters along sides
    for px in [-96.0_f32, -48.0, 0.0, 48.0, 96.0] {
        b.rbox([px, wall_y, -35.8], [4.2, wall_h, 1.4], brick);
        b.rbox([px, wall_y, 35.8], [4.2, wall_h, 1.4], brick);
    }
    // 4 industrial workshop windows per side
    for sx in [-72.0_f32, -24.0, 24.0, 72.0] {
        for z_side in [-1.0_f32, 1.0] {
            b.window_z(sx, 42.0, z_side * 34.8, z_side, 22.0, 36.0, win, frame, trim, trim);
        }
    }

    // 3. Front -X portal piers, sandstone arch lintel, lantern & open double doors
    let portal_w = 44.0_f32;
    let portal_h = 72.0_f32;
    let pillar_w = (70.0 - portal_w) * 0.5;
    b.rbox([-118.5, wall_y, -35.0 + pillar_w * 0.5], [3.8, wall_h, pillar_w], brick);
    b.rbox([-118.5, wall_y, 35.0 - pillar_w * 0.5], [3.8, wall_h, pillar_w], brick);
    b.rbox([-118.5, 3.0 + portal_h + (wall_h - portal_h) * 0.5, 0.0], [4.4, wall_h - portal_h, 70.0], trim);
    b.rbox([-120.5, 3.0 + portal_h + 1.5, 0.0], [2.4, 3.2, 5.0], trim); // Keystone
    b.rbox([-121.0, 3.0 + portal_h + 4.5, 0.0], [4.0, 1.8, 1.8], iron);
    b.sphere([-122.6, 3.0 + portal_h + 3.6, 0.0], 2.1, 6, 8, lamp_glow);

    let leaf_w = portal_w * 0.5;
    let leaf_h = portal_h - 1.5;
    let leaf_cy = 3.0 + leaf_h * 0.5;
    for &(side, yaw) in &[(-1.0_f32, 1.40_f32), (1.0_f32, -1.40_f32)] {
        let hinge_z = side * (portal_w * 0.5);
        let dir_z = -side;
        let (dx, dz) = rot_xz(0.0, dir_z * leaf_w * 0.5, yaw);
        b.obox([-120.0 + dx, leaf_cy, hinge_z + dz], [2.0, leaf_h, leaf_w], [0.0, -yaw, 0.0], door_w);
        b.obox([-120.0 + dx, leaf_cy + leaf_h * 0.28, hinge_z + dz], [2.6, 13.0, 8.0], [0.0, -yaw, 0.0], win);
        for hy in [-leaf_h * 0.36, 0.0, leaf_h * 0.36] {
            b.obox([-120.0 + dx * 0.9, leaf_cy + hy, hinge_z + dz * 0.9], [2.5, 2.2, leaf_w * 0.85], [0.0, -yaw, 0.0], iron);
        }
    }

    // 4. Pitched roof, glazed skylight ridge monitor & 3 locomotive smoke jacks
    b.gable_roof_x([0.0, 82.0, 0.0], 242.0, 72.0, 21.0, 2.5, 2.0, roof, brick, trim);
    b.rbox([0.0, 104.0, 0.0], [150.0, 4.5, 11.5], skylight);
    b.rbox([0.0, 106.5, 0.0], [152.0, 1.6, 2.0], iron);
    for cx in [-75.0_f32, 0.0, 75.0] {
        b.cyl_y([cx, 109.0, 0.0], 4.2, 5.2, 11.0, 12, iron);
        b.cone_y([cx, 114.5, 0.0], 6.8, 4.0, 12, 0.0, iron);
    }
}

// ----------------------------------------------------------------------------
// 4. TURNTABLE TRAINHALL (Ringlokschuppen — true radial 3-stall fan around -Z turntable)
// ----------------------------------------------------------------------------
fn build_turntable_trainhall(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [320.0, 115.0, 310.0]);
    const TT_Z: f32 = -350.0;
    const R_FRONT: f32 = 195.0;
    const R_REAR: f32 = 505.0;
    let deg2rad = std::f32::consts::PI / 180.0;
    let stall_centers = [-12.0 * deg2rad, 0.0, 12.0 * deg2rad];
    let part_angles = [-18.0 * deg2rad, -6.0 * deg2rad, 6.0 * deg2rad, 18.0 * deg2rad];

    let y_ground = 0.0_f32;
    let y_floor = 3.5_f32;
    let y_eave = 92.0_f32;
    let y_ridge = 110.0_f32;

    let radial_pt = |r: f32, ang: f32, y: f32| -> [f32; 3] { [r * ang.sin(), y, r * ang.cos() + TT_Z] };

    let ground_m = Mat::matte(0x475569, 0.90);
    let brick_m = Mat::matte(0x8b3a2b, 0.83);
    let trim_m = Mat::matte(0xd4b996, 0.76);
    let roof_m = Mat::matte(0x1e293b, 0.68);
    let glass_m = Mat::warm_window(0xfde68a, 0.55);
    let skylight_m = Mat::glass(0x38bdf8);
    let door_m = Mat::matte(0x1e3a5f, 0.80);
    let steel_m = Mat::metal(0x334155, 0.42, 0.70);
    let rail_m = Mat::metal(0xb0bcc9, 0.24, 0.85);
    let chim_m = Mat::metal(0x1c1917, 0.58, 0.35);
    let lamp_m = Mat::emissive(0xfef08a, 0.95);
    let pit_m = Mat::matte(0x1f2937, 0.95);
    let stop_m = Mat::matte(0x78350f, 0.82);

    // 1. Curved radial concrete floor slab (16 segments)
    let n_steps = 16usize;
    for j in 0..n_steps {
        let a1 = -18.0 * deg2rad + (36.0 * deg2rad * j as f32) / n_steps as f32;
        let a2 = -18.0 * deg2rad + (36.0 * deg2rad * (j + 1) as f32) / n_steps as f32;
        let am = 0.5 * (a1 + a2);
        let t1 = radial_pt(R_FRONT, a1, y_floor);
        let t2 = radial_pt(R_FRONT, a2, y_floor);
        let t3 = radial_pt(R_REAR, a2, y_floor);
        let t4 = radial_pt(R_REAR, a1, y_floor);
        b.quad_oriented(t1, t2, t3, t4, [0.0, 1.0, 0.0], ground_m);
        let b1 = radial_pt(R_FRONT, a1, y_ground);
        let b2 = radial_pt(R_FRONT, a2, y_ground);
        b.quad_oriented(b1, t1, t2, b2, [-am.sin(), 0.0, -am.cos()], ground_m);
        let b4 = radial_pt(R_REAR, a1, y_ground);
        let b3 = radial_pt(R_REAR, a2, y_ground);
        b.quad_oriented(b3, t3, t4, b4, [am.sin(), 0.0, am.cos()], ground_m);
    }

    // 2. Interior stall tracks, pits & buffer stops
    let track_len = R_REAR - R_FRONT - 25.0;
    let wall_h = y_eave - y_floor;
    let flank_len = R_REAR - R_FRONT;

    for &ang in &stall_centers {
        let rc = R_FRONT + track_len * 0.5;
        let p_c = radial_pt(rc, ang, y_floor + 1.0);
        let p_l = radial_pt(rc, ang, y_floor + 1.0);
        let (dx_l, dz_l) = (ang.cos() * (-8.25), -ang.sin() * (-8.25));
        let (dx_r, dz_r) = (ang.cos() * 8.25, -ang.sin() * 8.25);
        b.obox([p_l[0] + dx_l, y_floor + 1.0, p_l[2] + dz_l], [1.6, 2.0, track_len], [0.0, -ang, 0.0], rail_m);
        b.obox([p_l[0] + dx_r, y_floor + 1.0, p_l[2] + dz_r], [1.6, 2.0, track_len], [0.0, -ang, 0.0], rail_m);
        b.obox([p_c[0], y_floor + 0.3, p_c[2]], [13.2, 0.6, track_len - 28.0], [0.0, -ang, 0.0], pit_m);
        let bs = radial_pt(R_FRONT + track_len - 6.0, ang, y_floor + 3.5);
        b.obox(bs, [22.0, 6.5, 4.0], [0.0, -ang, 0.0], stop_m);
    }

    // 3. Side flank walls (-18 deg and +18 deg) with workshop windows
    for &side in &[-1.0_f32, 1.0] {
        let ang = side * 18.0 * deg2rad;
        let mid = radial_pt(R_FRONT + flank_len * 0.5, ang, y_floor + wall_h * 0.5);
        b.obox(mid, [4.0, wall_h, flank_len], [0.0, -ang, 0.0], brick_m);
        for k in 0..4 {
            let fz = R_FRONT + 45.0 + k as f32 * 58.0;
            let wp = radial_pt(fz, ang, y_floor + 44.0);
            b.obox(wp, [4.6, 36.0, 28.0], [0.0, -ang, 0.0], glass_m);
            b.obox([wp[0], y_floor + 24.5, wp[2]], [5.4, 2.4, 32.0], [0.0, -ang, 0.0], trim_m);
        }
    }

    // 4. Segmented curved rear brick wall with windows & sandstone pilasters
    let rear_seg_w = R_REAR * 12.0 * deg2rad;
    for &ang in &stall_centers {
        let rw = radial_pt(R_REAR - 2.0, ang, y_floor + wall_h * 0.5);
        b.obox(rw, [rear_seg_w + 2.0, wall_h, 4.0], [0.0, -ang, 0.0], brick_m);
        let rwin = radial_pt(R_REAR - 2.0, ang, y_floor + 46.0);
        b.obox(rwin, [rear_seg_w * 0.64, 42.0, 4.6], [0.0, -ang, 0.0], glass_m);
    }
    for &pang in &part_angles {
        let pp = radial_pt(R_REAR - 1.0, pang, y_floor + wall_h * 0.5 + 0.5);
        b.obox(pp, [5.0, wall_h + 1.5, 6.0], [0.0, -pang, 0.0], trim_m);
    }

    // 5. Front locomotive portals (clear opening 34 x 80 mm at R_FRONT facing turntable -Z) with open doors
    let portal_w = 34.0_f32;
    let portal_h = 80.0_f32;
    let front_seg_w = R_FRONT * 12.0 * deg2rad + 2.0;
    let lintel_h = wall_h - portal_h;
    let side_pw = (front_seg_w - portal_w) * 0.5;
    let leaf_w = portal_w * 0.5 - 0.5;
    let door_h = portal_h - 1.0;

    for &ang in &stall_centers {
        let dir_x = ang.sin();
        let dir_z = ang.cos();
        let right_x = ang.cos();
        let right_z = -ang.sin();

        let fc = [dir_x * (R_FRONT + 2.0), y_floor + portal_h + lintel_h * 0.5, TT_Z + dir_z * (R_FRONT + 2.0)];
        b.obox(fc, [front_seg_w, lintel_h, 4.0], [0.0, -ang, 0.0], brick_m);

        for s in [-1.0_f32, 1.0] {
            let off = s * (front_seg_w * 0.5 - side_pw * 0.5);
            b.obox(
                [dir_x * (R_FRONT + 2.0) + right_x * off, y_floor + wall_h * 0.5, TT_Z + dir_z * (R_FRONT + 2.0) + right_z * off],
                [side_pw + 1.2, wall_h, 4.6],
                [0.0, -ang, 0.0],
                trim_m,
            );
        }
        b.obox(
            [dir_x * (R_FRONT + 2.0), y_floor + portal_h + 1.6, TT_Z + dir_z * (R_FRONT + 2.0)],
            [portal_w + 4.0, 3.2, 5.0],
            [0.0, -ang, 0.0],
            trim_m,
        );

        // Hinged open stall doors swinging outward toward turntable
        for s in [-1.0_f32, 1.0] {
            let hx = dir_x * (R_FRONT + 2.0) + right_x * (s * portal_w * 0.5);
            let hz = TT_Z + dir_z * (R_FRONT + 2.0) + right_z * (s * portal_w * 0.5);
            let swing = s * (-1.35);
            let leaf_ang = -ang + swing;
            let (ox, oz) = rot_xz(-s * leaf_w * 0.5, 0.0, -leaf_ang);
            b.obox([hx + ox, y_floor + door_h * 0.5, hz + oz], [leaf_w, door_h, 1.8], [0.0, leaf_ang, 0.0], door_m);
            b.obox([hx + ox, y_floor + door_h * 0.72, hz + oz], [8.5, 18.0, 2.3], [0.0, leaf_ang, 0.0], glass_m);
        }

        // Portal lantern facing turntable
        let lamp_p = radial_pt(R_FRONT - 1.0, ang, y_floor + portal_h + 6.0);
        b.sphere(lamp_p, 2.1, 6, 8, lamp_m);
    }

    // 6. True radial fan 3-gable pitched slate roof, brick gables, skylights & 3 smoke stacks
    let r_roof_f = R_FRONT - 3.0;
    let r_roof_r = R_REAR + 5.0;
    for s in 0..3 {
        let a_l = part_angles[s];
        let a_c = stall_centers[s];
        let a_r = part_angles[s + 1];

        let fl = radial_pt(r_roof_f, a_l, y_eave);
        let fc = radial_pt(r_roof_f, a_c, y_ridge);
        let fr = radial_pt(r_roof_f, a_r, y_eave);
        let rl = radial_pt(r_roof_r, a_l, y_eave);
        let rc = radial_pt(r_roof_r, a_c, y_ridge);
        let rr = radial_pt(r_roof_r, a_r, y_eave);

        b.quad_oriented(fl, rl, rc, fc, [-a_c.cos(), 1.0, a_c.sin()], roof_m);
        b.quad_oriented(fc, rc, rr, fr, [a_c.cos(), 1.0, -a_c.sin()], roof_m);

        let flw = radial_pt(R_FRONT + 2.0, a_l, y_eave);
        let fcw = radial_pt(R_FRONT + 2.0, a_c, y_ridge);
        let frw = radial_pt(R_FRONT + 2.0, a_r, y_eave);
        b.tri_oriented(flw, fcw, frw, [-a_c.sin(), 0.0, -a_c.cos()], brick_m);

        let rlw = radial_pt(R_REAR - 2.0, a_l, y_eave);
        let rcw = radial_pt(R_REAR - 2.0, a_c, y_ridge);
        let rrw = radial_pt(R_REAR - 2.0, a_r, y_eave);
        b.tri_oriented(rrw, rcw, rlw, [a_c.sin(), 0.0, a_c.cos()], brick_m);

        // Ridge skylight & locomotive chimney
        let mid_r = 0.5 * (r_roof_f + r_roof_r);
        let ridge_len = r_roof_r - r_roof_f - 40.0;
        b.obox(radial_pt(mid_r, a_c, y_ridge + 2.4), [9.5, 4.8, ridge_len], [0.0, -a_c, 0.0], skylight_m);
        b.obox(radial_pt(mid_r, a_c, y_ridge + 5.0), [10.8, 1.2, ridge_len + 2.0], [0.0, -a_c, 0.0], steel_m);

        let ch_p = radial_pt(R_FRONT + 140.0, a_c, y_ridge + 9.0);
        b.cyl_y(ch_p, 4.8, 5.8, 18.0, 12, chim_m);
        b.cone_y([ch_p[0], ch_p[1] + 9.0, ch_p[2]], 7.8, 4.8, 12, 0.0, chim_m);
    }

    // Interior bay support columns at ±6 deg
    for &pang in &[-6.0 * deg2rad, 6.0 * deg2rad] {
        for pr in [R_FRONT + 75.0, R_FRONT + 155.0, R_FRONT + 235.0] {
            b.obox(radial_pt(pr, pang, y_floor + wall_h * 0.5), [3.5, wall_h, 3.5], [0.0, -pang, 0.0], steel_m);
        }
    }
}

// ----------------------------------------------------------------------------
// 5. TOWN HOUSE (Fachwerkhaus — half-timbered beams & diagonal braces, shutters, dormer, chimney)
// ----------------------------------------------------------------------------
fn build_town_house(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [110.0, 125.0, 90.0]);
    let stone = Mat::matte(0x64748b, 0.90);
    let plaster = Mat::matte(0xf3ead8, 0.82);
    let beam = Mat::matte(0x451a03, 0.88);
    let roof = Mat::matte(0x991b1b, 0.68);
    let ridge = Mat::matte(0x7c2d12, 0.72);
    let brick = Mat::matte(0x9a3412, 0.82);
    let door_m = Mat::matte(0x5c2e14, 0.78);
    let frame = Mat::matte(0xf8fafc, 0.65);
    let shutter = Mat::matte(0x166534, 0.78);
    let win_dark = Mat::warm_window(0xfde68a, 0.35);
    let win_warm = Mat::warm_window(0xfde68a, 0.92);
    let fbox = Mat::matte(0x78350f, 0.85);
    let flowers = Mat::matte(0xdc2626, 0.70);

    // 1. Stone plinth + ground floor + jettied upper floor
    b.rbox([0.0, 4.5, 0.0], [108.0, 9.0, 86.0], stone);
    b.rbox([0.0, 25.5, 0.0], [102.0, 33.0, 78.0], plaster);
    b.rbox([0.0, 59.0, 0.0], [106.0, 35.0, 82.0], plaster);

    // 2. Half-timbering horizontal rails, posts & diagonal corner braces
    for by in [42.5_f32, 59.0, 75.2] {
        b.rbox([0.0, by, 0.0], [107.6, 2.5, 83.6], beam);
    }
    for sz in [-1.0_f32, 1.0] {
        for bx in [-52.2_f32, -26.0, 0.0, 26.0, 52.2] {
            b.rbox([bx, 59.0, sz * 40.5], [2.5, 35.0, 2.5], beam);
        }
        for bx in [-50.2_f32, -25.0, 25.0, 50.2] {
            b.rbox([bx, 25.5, sz * 38.5], [2.5, 33.0, 2.5], beam);
        }
        for sx in [-1.0_f32, 1.0] {
            b.strut_between([sx * 50.5, 43.0, sz * 40.9], [sx * 27.5, 74.5, sz * 40.9], 2.0, 1.8, beam);
        }
    }

    // 3. Steep gable roof + dormer + brick chimney
    b.gable_roof_x([0.0, 76.5, 0.0], 106.0, 82.0, 44.0, 3.5, 2.0, roof, plaster, ridge);
    b.rbox([0.0, 91.0, 22.0], [20.0, 14.0, 18.0], plaster);
    b.rbox([0.0, 99.0, 22.0], [23.0, 3.0, 21.0], roof);
    b.rbox([0.0, 91.5, 31.2], [10.0, 8.5, 1.2], win_warm);
    b.rbox([28.0, 112.0, -14.0], [10.0, 24.0, 10.0], brick);
    b.rbox([28.0, 124.8, -14.0], [12.0, 2.2, 12.0], stone);

    // 4. Mullioned windows with green shutters & flower boxes
    let mut add_win = |wx: f32, wy: f32, wall_z: f32, side: f32, warm: bool, with_flowers: bool| {
        let wm = if warm { win_warm } else { win_dark };
        b.window_z(wx, wy, wall_z, side, 10.0, 13.0, wm, frame, fbox, beam);
        for sx in [-1.0_f32, 1.0] {
            b.rbox([wx + sx * 8.4, wy, wall_z + side * 0.7], [4.0, 14.8, 1.1], shutter);
        }
        if with_flowers {
            b.rbox([wx, wy - 8.8, wall_z + side * 2.2], [12.8, 2.6, 3.0], fbox);
            b.rbox([wx, wy - 7.0, wall_z + side * 2.2], [11.4, 2.2, 2.2], flowers);
        }
    };
    for side in [1.0_f32, -1.0] {
        for dx in [-36.0_f32, 0.0, 36.0] {
            add_win(dx, 59.0, side * 41.0, side, dx == 0.0 || (side > 0.0 && dx < 0.0), side > 0.0);
        }
        for dx in [-36.0_f32, 36.0] {
            add_win(dx, 26.0, side * 39.0, side, side > 0.0 && dx > 0.0, false);
        }
    }
    for sx in [-1.0_f32, 1.0] {
        b.window_x(sx * 53.0, 90.0, 0.0, sx, 8.8, 10.5, win_warm, frame, stone, beam);
    }

    // 5. Front entrance door & stone stoop (+Z)
    b.rbox([0.0, 21.5, 39.2], [16.0, 25.0, 1.6], beam);
    b.rbox([0.0, 20.5, 39.6], [13.0, 22.5, 1.6], door_m);
    b.rbox([0.0, 7.6, 42.0], [20.0, 3.2, 6.0], stone);
}

// ----------------------------------------------------------------------------
// 6. WATER TOWER (Wasserturm — tapered cylindrical shaft, corbel brackets, tank, conical roof, cupola)
// ----------------------------------------------------------------------------
fn build_water_tower(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [85.0, 170.0, 85.0]);
    let stone = Mat::matte(0x64748b, 0.90);
    let cornice = Mat::matte(0xcbd5e1, 0.78);
    let brick = Mat::matte(0x9a3412, 0.82);
    let tank_wall = Mat::metal(0x475569, 0.56, 0.35);
    let tank_band = Mat::metal(0x1e293b, 0.40, 0.72);
    let roof = Mat::matte(0x334155, 0.55);
    let door_m = Mat::matte(0x5c2e14, 0.80);
    let win_warm = Mat::warm_window(0xfde68a, 0.88);
    let win_dark = Mat::warm_window(0xfde68a, 0.32);
    let brass = Mat::metal(0xd97706, 0.28, 0.82);
    let gauge_bg = Mat::matte(0xf8fafc, 0.50);
    let gauge_red = Mat::matte(0xdc2626, 0.45);

    // 1. Stepped cylindrical stone plinth & tapered brick shaft
    b.cyl_y([0.0, 3.0, 0.0], 39.0, 41.0, 6.0, 16, stone);
    b.cyl_y([0.0, 13.0, 0.0], 35.0, 37.0, 14.0, 16, stone);
    b.cyl_y([0.0, 21.2, 0.0], 36.5, 36.5, 2.4, 16, cornice);
    b.cyl_y([0.0, 52.0, 0.0], 28.0, 34.0, 60.0, 16, brick);

    // Entry portal (+Z) & 8 shaft windows
    b.rbox([0.0, 16.0, 34.5], [14.0, 22.0, 4.0], cornice);
    b.rbox([0.0, 15.0, 35.0], [10.4, 19.0, 4.2], door_m);
    for i in 0..4 {
        let ang = (i as f32) * std::f32::consts::FRAC_PI_2;
        for &(wy, r, warm) in &[(44.0_f32, 31.5_f32, false), (66.0, 29.4, i % 2 == 0)] {
            let cx = ang.sin() * r;
            let cz = ang.cos() * r;
            b.obox([cx, wy, cz], [6.4, 10.5, 2.2], [0.0, -ang, 0.0], cornice);
            b.obox([cx + ang.sin() * 0.4, wy, cz + ang.cos() * 0.4], [4.4, 8.5, 2.2], [0.0, -ang, 0.0], if warm { win_warm } else { win_dark });
        }
    }

    // 2. Flared corbel ring & 12 stone brackets supporting overhanging reservoir tank
    b.cyl_y([0.0, 86.5, 0.0], 38.5, 29.0, 9.0, 16, cornice);
    for i in 0..12 {
        let ang = (i as f32 / 12.0) * std::f32::consts::TAU;
        b.obox([ang.sin() * 32.5, 84.5, ang.cos() * 32.5], [3.2, 9.5, 8.5], [0.0, -ang, 0.0], stone);
    }

    // 3. Elevated cylindrical reservoir with steel bands, control windows & level gauge
    b.cyl_y([0.0, 111.0, 0.0], 39.5, 39.0, 40.0, 20, tank_wall);
    for by in [95.0_f32, 107.0, 119.0, 129.0] {
        b.cyl_y([0.0, by, 0.0], 40.2, 40.2, 1.8, 20, tank_band);
    }
    for i in 0..8 {
        let ang = (i as f32 / 8.0) * std::f32::consts::TAU + std::f32::consts::FRAC_PI_8;
        let cx = ang.sin() * 39.7;
        let cz = ang.cos() * 39.7;
        b.obox([cx, 113.0, cz], [6.8, 7.6, 1.5], [0.0, -ang, 0.0], if i % 3 == 0 { win_warm } else { win_dark });
    }
    b.rbox([0.0, 111.0, 40.2], [4.5, 31.0, 1.6], gauge_bg);
    b.rbox([0.0, 118.0, 40.8], [5.2, 3.0, 1.8], gauge_red);

    // 4. Conical slate roof, lit cupola lantern & brass finial
    b.cone_y([0.0, 131.0, 0.0], 43.0, 26.0, 20, 0.0, roof);
    b.cyl_y([0.0, 158.0, 0.0], 9.2, 9.8, 8.5, 12, tank_band);
    b.cyl_y([0.0, 158.5, 0.0], 8.6, 8.6, 5.2, 12, win_warm);
    b.cone_y([0.0, 162.2, 0.0], 11.5, 9.0, 12, 0.0, roof);
    b.cyl_y([0.0, 174.0, 0.0], 0.5, 1.0, 10.0, 8, brass);

    // 5. Rear climbing ladder (-Z) & trackside water-crane pipe (+X)
    for lx in [-2.4_f32, 2.4] {
        b.cyl_y([lx, 68.0, -35.5], 0.6, 0.6, 110.0, 6, tank_band);
    }
    for ry in (18..=120).step_by(8) {
        b.rbox([0.0, ry as f32, -35.5], [5.2, 0.8, 0.8], tank_band);
    }
    b.cyl_y([35.5, 43.0, 0.0], 3.0, 3.6, 80.0, 10, tank_band);
    b.cyl_x([47.0, 80.0, 0.0], 2.6, 2.4, 24.0, 10, tank_band);
    b.cyl_y([58.0, 72.0, 0.0], 2.6, 1.8, 16.0, 10, tank_band);
}

// ----------------------------------------------------------------------------
// 7. COALING STATION (Bekohlungsanlage — bunker, coal mounds, hopper, slewing crane & water crane)
// ----------------------------------------------------------------------------
fn build_coaling_station(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [180.0, 125.0, 75.0]);
    let concrete = Mat::matte(0x64748b, 0.90);
    let dark_conc = Mat::matte(0x475569, 0.92);
    let coal = Mat::matte(0x16171b, 0.68);
    let steel = Mat::metal(0x334155, 0.44, 0.68);
    let dark_steel = Mat::metal(0x1e293b, 0.40, 0.76);
    let crane_cab = Mat::metal(0x1e3a5f, 0.48, 0.40);
    let rust = Mat::matte(0x7c2d12, 0.84);
    let warning = Mat::matte(0xea580c, 0.50);
    let cab_glass = Mat::warm_window(0xfef08a, 0.72);

    // 1. Concrete foundation plate
    b.rbox([0.0, 2.0, 0.0], [180.0, 4.0, 75.0], dark_conc);

    // 2. Concrete coal bunker with 10 buttresses & jagged coal mound heap
    let (bx, bz, bw, bd, bh, wt) = (-46.0_f32, -9.0_f32, 76.0_f32, 46.0_f32, 22.0_f32, 3.5_f32);
    b.rbox([bx, 4.0 + bh * 0.5, bz - bd * 0.5 + wt * 0.5], [bw, bh, wt], concrete);
    b.rbox([bx, 4.0 + bh * 0.41, bz + bd * 0.5 - wt * 0.5], [bw, bh * 0.82, wt], concrete);
    b.rbox([-84.0 + wt * 0.5, 4.0 + bh * 0.5, bz], [wt, bh, bd], concrete);
    b.rbox([-8.0 - wt * 0.5, 4.0 + bh * 0.5, bz], [wt, bh, bd], concrete);
    for i in 0..=4 {
        let px = -80.0 + (i as f32) * ((bw - 8.0) / 4.0);
        b.rbox([px, 4.0 + bh * 0.5, bz - bd * 0.5 - 1.0], [2.8, bh + 1.0, 3.5], dark_conc);
        b.rbox([px, 4.0 + bh * 0.41, bz + bd * 0.5 + 1.0], [2.8, bh * 0.82 + 1.0, 3.5], dark_conc);
    }
    b.rbox([bx, 10.5, bz], [bw - 8.0, 13.0, bd - 8.0], coal);
    for (idx, (mx, mz, mr, mh)) in [
        (-64.0_f32, -12.0_f32, 14.0_f32, 15.0_f32),
        (-46.0, -9.0, 16.0, 19.0),
        (-28.0, -11.0, 14.0, 16.0),
        (-56.0, -2.0, 11.0, 12.0),
        (-34.0, -2.0, 11.0, 13.0),
    ].into_iter().enumerate() {
        b.cone_y([mx, 15.0, mz], mr, mh, 7, (idx as f32) * 0.42, coal);
    }

    // 3. Elevated steel weigh hopper & chute (+X = 54)
    let (tx, tz, span_x, span_z) = (54.0_f32, -4.0_f32, 34.0_f32, 32.0_f32);
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            let lx = tx + sx * span_x * 0.5;
            let lz = tz + sz * span_z * 0.5;
            b.rbox([lx, 7.0, lz], [6.0, 6.0, 6.0], concrete);
            b.rbox([lx, 38.0, lz], [3.2, 56.0, 3.2], steel);
        }
    }
    // Cross-bracing struts
    for sz in [-1.0_f32, 1.0] {
        let lz = tz + sz * span_z * 0.5;
        b.strut_between([tx - 17.0, 14.0, lz], [tx + 17.0, 44.0, lz], 1.6, 1.6, steel);
        b.strut_between([tx + 17.0, 14.0, lz], [tx - 17.0, 44.0, lz], 1.6, 1.6, steel);
    }
    for y_lvl in [28.0_f32, 48.0, 64.0] {
        b.rbox([tx, y_lvl, tz - span_z * 0.5], [span_x + 4.0, 2.5, 2.5], steel);
        b.rbox([tx, y_lvl, tz + span_z * 0.5], [span_x + 4.0, 2.5, 2.5], steel);
        b.rbox([tx - span_x * 0.5, y_lvl, tz], [2.5, 2.5, span_z + 4.0], steel);
        b.rbox([tx + span_x * 0.5, y_lvl, tz], [2.5, 2.5, span_z + 4.0], steel);
    }
    b.cyl_y([tx, 56.0, tz], 22.0, 9.0, 20.0, 4, steel);
    b.rbox([tx, 67.0, tz], [36.0, 4.0, 34.0], dark_steel);
    b.rbox([tx, 76.0, tz - 15.9], [36.0, 22.0, 2.2], steel);
    b.rbox([tx, 76.0, tz + 15.9], [36.0, 22.0, 2.2], steel);
    b.rbox([tx - 16.9, 76.0, tz], [2.2, 22.0, 29.6], steel);
    b.rbox([tx + 16.9, 76.0, tz], [2.2, 22.0, 29.6], steel);
    b.rbox([tx, 74.0, tz], [30.8, 12.0, 28.8], coal);
    b.cone_y([tx, 80.0, tz], 12.0, 6.0, 7, 0.2, coal);
    b.obox([tx, 44.0, tz + 21.0], [12.0, 2.0, 24.0], [0.52, 0.0, 0.0], dark_steel);
    b.obox([tx - 5.5, 45.8, tz + 21.0], [1.5, 4.8, 24.0], [0.52, 0.0, 0.0], steel);
    b.obox([tx + 5.5, 45.8, tz + 21.0], [1.5, 4.8, 24.0], [0.52, 0.0, 0.0], steel);
    b.rbox([tx, 65.0, tz], [42.0, 2.0, 40.0], dark_steel);

    // 4. Slewing coaling crane cab, lattice boom, cable & clamshell grab (pointing toward -X coal heap)
    let (crx, crz) = (12.0_f32, -10.0_f32);
    b.cyl_y([crx, 25.0, crz], 8.5, 10.0, 42.0, 12, concrete);
    b.cyl_y([crx, 47.0, crz], 9.5, 9.5, 3.0, 14, dark_steel);
    b.rbox([crx + 1.0, 56.5, crz], [18.0, 16.0, 14.0], crane_cab);
    b.rbox([crx + 1.0, 65.5, crz], [20.0, 2.0, 16.0], dark_steel);
    b.rbox([crx - 7.5, 58.5, crz], [2.0, 7.0, 10.0], cab_glass);
    b.rbox([crx + 12.0, 54.5, crz], [6.0, 10.0, 12.0], dark_conc);

    let boom_start = [crx - 5.0, 59.5, crz];
    let boom_tip = [crx - 46.0, 100.0, crz];
    for bz in [-2.6_f32, 2.6] {
        b.cyl_between([boom_start[0], boom_start[1], crz + bz], [boom_tip[0], boom_tip[1], crz + bz], 1.2, 0.9, 6, steel);
    }
    for k in 1..6 {
        let t = k as f32 / 6.0;
        let mx = boom_start[0] + (boom_tip[0] - boom_start[0]) * t;
        let my = boom_start[1] + (boom_tip[1] - boom_start[1]) * t;
        b.rbox([mx, my, crz], [1.4, 2.6, 5.8], steel);
    }
    b.cyl_z(boom_tip, 2.8, 2.8, 2.4, 12, warning);
    b.cyl_y([boom_tip[0], boom_tip[1] - 18.0, crz], 0.45, 0.45, 36.0, 6, dark_steel);
    let grab_y = boom_tip[1] - 36.0;
    b.rbox([boom_tip[0], grab_y, crz], [4.5, 5.2, 4.5], dark_steel);
    b.obox([boom_tip[0] - 2.6, grab_y - 4.2, crz], [5.2, 6.2, 7.2], [0.0, 0.0, 0.25], rust);
    b.obox([boom_tip[0] + 2.6, grab_y - 4.2, crz], [5.2, 6.2, 7.2], [0.0, 0.0, -0.25], rust);
    b.organic_foliage_cluster([boom_tip[0], grab_y - 3.6, crz], 3.8, 3.4, 3.8, 99, coal);

    // 5. Trackside articulated water crane (X = -72, Z = +24)
    let (wcx, wcz) = (-72.0_f32, 24.0_f32);
    b.rbox([wcx + 18.0, 4.5, wcz], [14.0, 1.0, 14.0], dark_steel);
    b.cyl_y([wcx, 8.0, wcz], 3.8, 5.0, 8.0, 10, dark_steel);
    b.cyl_y([wcx, 31.0, wcz], 2.4, 2.8, 46.0, 10, dark_steel);
    b.cyl_x([wcx + 8.0, 53.0, wcz], 1.8, 1.8, 28.0, 8, dark_steel);
    b.cyl_y([wcx + 20.0, 48.0, wcz], 1.6, 2.0, 12.0, 8, dark_steel);
    b.sphere([wcx - 7.0, 55.0, wcz], 3.6, 8, 10, warning);
}

// ----------------------------------------------------------------------------
// 8. DIESEL FUEL STATION (Dieseltankstelle — cylindrical tank + dished caps, catwalk, pump house, dispensers)
// ----------------------------------------------------------------------------
fn build_diesel_fuel_station(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [170.0, 68.0, 65.0]);
    let concrete = Mat::matte(0x94a3b8, 0.88);
    let dark_conc = Mat::matte(0x64748b, 0.90);
    let grate = Mat::metal(0x334155, 0.55, 0.50);
    let tank_m = Mat::metal(0xdbeafe, 0.36, 0.38);
    let red_band = Mat::matte(0xdc2626, 0.45);
    let dark_steel = Mat::metal(0x334155, 0.45, 0.55);
    let safety = Mat::matte(0xfacc15, 0.50);
    let pump_wall = Mat::matte(0xe2e8f0, 0.85);
    let brick = Mat::matte(0x991b1b, 0.86);
    let canopy_m = Mat::metal(0x475569, 0.48, 0.42);
    let disp_blue = Mat::metal(0x0284c7, 0.38, 0.25);
    let pipe_m = Mat::metal(0x64748b, 0.32, 0.70);
    let hose_m = Mat::matte(0x111827, 0.85);
    let light_m = Mat::emissive(0xfef08a, 0.90);

    // 1. Apron, recessed spill pan, drainage grates & yellow curb
    b.rbox([0.0, 1.25, 0.0], [168.0, 2.5, 63.0], concrete);
    b.rbox([32.0, 2.7, 17.0], [84.0, 0.8, 18.0], dark_conc);
    for gx in [4.0_f32, 22.0, 40.0, 58.0] {
        b.rbox([gx, 3.15, 17.0], [14.0, 0.5, 12.0], grate);
    }
    b.rbox([32.0, 3.1, 26.5], [86.0, 1.4, 1.4], safety);

    // 2. Main horizontal cylindrical diesel tank with dished end-caps on 3 saddle piers
    let (tx, tz, ty, tr, tl) = (-40.0_f32, -8.0_f32, 23.5_f32, 14.5_f32, 68.0_f32);
    for sx in [-24.0_f32, 0.0, 24.0] {
        b.rbox([tx + sx, 7.0, tz], [4.5, 9.0, 24.0], dark_conc);
    }
    b.cyl_x([tx, ty, tz], tr, tr, tl, 24, tank_m);
    b.hemi_cap_x([tx - tl * 0.5, ty, tz], tr, 5.2, -1.0, 6, 18, tank_m);
    b.hemi_cap_x([tx + tl * 0.5, ty, tz], tr, 5.2, 1.0, 6, 18, tank_m);
    for &(bx, bw, bm) in &[(-18.0_f32, 2.2_f32, dark_steel), (0.0, 5.5, red_band), (18.0, 2.2, dark_steel)] {
        b.cyl_x([tx + bx, ty, tz], tr + 0.3, tr + 0.3, bw, 24, bm);
    }
    for dx in [-16.0_f32, 16.0] {
        b.cyl_y([tx + dx, ty + tr + 1.2, tz], 3.1, 3.5, 3.5, 12, dark_steel);
    }
    b.rbox([tx, ty + tr + 2.8, tz], [58.0, 0.8, 6.0], grate);
    b.rbox([tx, ty + tr + 9.5, tz - 2.8], [58.0, 0.6, 0.6], safety);
    for rx in [-28.0_f32, -14.0, 0.0, 14.0, 28.0] {
        b.rbox([tx + rx, ty + tr + 6.2, tz - 2.8], [0.6, 7.0, 0.6], safety);
    }
    let lad_h = ty - 2.5 + tr + 3.0;
    for lx in [-1.8_f32, 1.8] {
        b.rbox([tx + lx, 2.5 + lad_h * 0.5, tz + tr + 1.5], [0.6, lad_h, 0.6], safety);
    }
    for k in 0..8 {
        b.rbox([tx, 6.5 + k as f32 * 4.5, tz + tr + 1.5], [3.6, 0.5, 0.5], safety);
    }

    // 3. Brick/plaster pump house + pipe bridge
    let (px, pz) = (18.0_f32, -12.0_f32);
    b.rbox([px, 5.0, pz], [28.0, 5.0, 22.0], brick);
    b.rbox([px, 18.0, pz], [27.0, 21.0, 21.0], pump_wall);
    b.rbox([px, 29.6, pz], [30.0, 2.2, 24.0], canopy_m);
    b.rbox([px - 5.0, 13.5, pz + 10.6], [7.0, 17.0, 0.8], dark_steel);
    b.rbox([px + 6.0, 17.5, pz + 10.6], [8.0, 7.0, 0.8], light_m);
    for py in [12.5_f32, 16.5] {
        b.cyl_x([px - 20.0, py, pz + 2.0], 1.1, 1.1, 26.0, 10, pipe_m);
    }

    // 4. Fueling island, 2 locomotive dispensers with swivel booms/hoses & cantilevered canopy
    let (ix, iz) = (32.0_f32, 15.0_f32);
    b.rbox([ix, 3.6, iz], [72.0, 2.2, 8.0], concrete);
    let can_y = 54.5_f32;
    for cx in [-22.0_f32, 22.0] {
        b.rbox([ix + cx, 2.5 + 26.0, iz - 1.0], [3.2, 52.0, 3.2], dark_steel);
        b.rbox([ix + cx, can_y - 1.5, iz + 1.0], [2.8, 3.0, 24.0], dark_steel);
    }
    b.rbox([ix, can_y + 1.2, iz + 1.0], [78.0, 2.4, 26.0], canopy_m);
    b.rbox([ix, can_y + 1.3, iz + 1.0], [78.6, 1.8, 26.6], disp_blue);
    for lx in [-18.0_f32, 18.0] {
        b.rbox([ix + lx, can_y - 0.4, iz + 2.0], [16.0, 0.8, 3.0], light_m);
    }
    for dx in [-14.0_f32, 14.0] {
        b.rbox([ix + dx, 14.2, iz], [7.5, 19.0, 5.0], disp_blue);
        b.rbox([ix + dx, 17.7, iz], [5.2, 5.5, 5.4], pump_wall);
        b.cyl_y([ix + dx + 2.2, 28.7, iz], 0.9, 0.9, 18.0, 8, pipe_m);
        b.cyl_between([ix + dx + 2.2, 37.0, iz], [ix + dx + 2.2, 35.0, iz + 11.0], 0.75, 0.7, 8, safety);
        b.cyl_y([ix + dx + 2.2, 26.5, iz + 11.0], 0.45, 0.45, 17.0, 8, hose_m);
    }
    for bx in [-33.0_f32, 33.0] {
        for bz in [-2.5_f32, 2.5] {
            b.cyl_y([ix + bx, 7.5, iz + bz], 1.1, 1.1, 10.0, 10, safety);
        }
    }
}

// ----------------------------------------------------------------------------
// 9. LIGHT SIGNAL (DB Lichthauptsignal — plinth, cabinet, tapered mast, ladder, hooded optics)
// ----------------------------------------------------------------------------
fn build_light_signal(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [25.0, 85.0, 25.0]);
    let concrete = Mat::matte(0x64748b, 0.90);
    let mast_m = Mat::metal(0x334155, 0.42, 0.72);
    let cab_m = Mat::metal(0x94a3b8, 0.55, 0.45);
    let shield_m = Mat::matte(0x0f172a, 0.65);
    let white_m = Mat::matte(0xf8fafc, 0.50);
    let red_m = Mat::matte(0xdc2626, 0.50);
    let green_glow = Mat::emissive(0x22c55e, 1.15);
    let red_glow = Mat::emissive(0xef4444, 0.95);
    let yellow_glow = Mat::emissive(0xf59e0b, 0.65);

    b.rbox([0.0, 2.2, 0.0], [18.0, 4.4, 18.0], concrete);
    b.rbox([0.0, 5.1, 0.0], [9.0, 1.4, 9.0], mast_m);
    b.cyl_y([0.0, 41.0, 0.0], 1.65, 2.15, 72.0, 10, mast_m);
    b.rbox([0.0, 13.0, -1.5], [9.5, 14.0, 7.5], cab_m);
    b.rbox([0.0, 20.5, -1.5], [10.5, 1.2, 8.5], mast_m);

    // White-red-white DB mast plate (+Z)
    b.rbox([0.0, 38.0, 2.3], [4.2, 22.0, 0.8], white_m);
    b.rbox([0.0, 38.0, 2.6], [2.2, 21.5, 0.8], red_m);

    // Rear ladder & maintenance platform (-Z)
    for lx in [-1.8_f32, 1.8] {
        b.cyl_y([lx, 44.0, -3.5], 0.35, 0.35, 60.0, 6, mast_m);
    }
    for ry in (22..=72).step_by(6) {
        b.rbox([0.0, ry as f32, -3.5], [3.8, 0.5, 0.5], mast_m);
    }
    b.rbox([0.0, 53.0, -2.5], [11.0, 1.0, 7.0], mast_m);

    // Signal head shield & 3 hooded optical lenses
    b.rbox([0.0, 69.0, 2.8], [13.2, 26.8, 1.5], white_m);
    b.rbox([0.0, 69.0, 3.2], [11.8, 25.4, 2.1], shield_m);
    for (ay, lm) in [(76.0_f32, green_glow), (69.0, red_glow), (62.0, yellow_glow)] {
        b.cyl_z([0.0, ay, 4.3], 1.85, 1.85, 1.6, 12, lm);
        b.rbox([0.0, ay + 2.1, 5.6], [4.8, 0.7, 4.2], shield_m);
        for sx in [-2.1_f32, 2.1] {
            b.rbox([sx, ay + 1.0, 5.3], [0.6, 2.4, 3.6], shield_m);
        }
    }
}

// ----------------------------------------------------------------------------
// 10. BUFFER STOP (Prellbock — open steel A-frame, rail clamps, oak buffer beam, twin round buffers, Sh2)
// ----------------------------------------------------------------------------
fn build_buffer_stop(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [25.0, 25.0, 35.0]);
    let steel = Mat::metal(0x334155, 0.52, 0.65);
    let dark_steel = Mat::metal(0x1e293b, 0.44, 0.76);
    let rust = Mat::matte(0x4b3832, 0.80);
    let wood = Mat::matte(0x6b3e1e, 0.88);
    let white_m = Mat::matte(0xf8fafc, 0.45);
    let sh2_red = Mat::emissive(0xdc2626, 0.35);

    let rail_half = 8.25_f32;
    let beam_y = 12.6_f32;
    let beam_z = -8.0_f32;
    for side in [-1.0_f32, 1.0] {
        let rx = side * rail_half;
        b.rbox([rx, 0.9, 1.2], [3.2, 1.8, 25.5], dark_steel);
        for cz in [-10.0_f32, 2.0, 11.8] {
            b.rbox([rx, 1.1, cz], [4.2, 2.2, 2.4], rust);
        }
        b.strut_between([rx, 1.4, -10.5], [rx, beam_y + 1.2, beam_z + 0.5], 2.1, 2.3, steel);
        b.strut_between([rx, beam_y + 0.8, beam_z + 1.2], [rx, 1.4, 12.5], 2.1, 2.3, steel);
        b.strut_between([rx, 6.2, -8.8], [rx, 6.2, 4.2], 1.5, 1.6, dark_steel);
    }
    b.rbox([0.0, 2.2, 10.5], [rail_half * 2.0 + 2.4, 2.0, 2.2], dark_steel);
    b.rbox([0.0, beam_y, beam_z + 2.4], [rail_half * 2.0 + 4.0, 3.2, 2.4], steel);
    b.rbox([0.0, beam_y, beam_z - 0.5], [27.5, 4.8, 3.8], wood);
    for side in [-1.0_f32, 1.0] {
        b.rbox([side * 13.4, beam_y, beam_z - 0.5], [1.4, 5.2, 4.2], dark_steel);
        let px = side * 9.8;
        b.rbox([px, beam_y, beam_z - 2.6], [4.2, 4.2, 0.8], dark_steel);
        b.cyl_z([px, beam_y, beam_z - 4.2], 1.85, 1.45, 2.8, 10, steel);
        b.cyl_z([px, beam_y, beam_z - 5.8], 2.5, 2.5, 0.8, 14, dark_steel);
    }
    b.rbox([0.0, beam_y + 4.8, beam_z - 0.2], [1.3, 5.6, 1.3], dark_steel);
    b.rbox([0.0, beam_y + 8.6, beam_z - 0.3], [8.4, 6.2, 1.4], dark_steel);
    b.rbox([0.0, beam_y + 8.6, beam_z - 1.05], [7.8, 5.6, 0.35], white_m);
    b.rbox([0.0, beam_y + 8.6, beam_z - 1.25], [6.0, 3.8, 0.35], sh2_red);
}

// ----------------------------------------------------------------------------
// 11. PINE TREE (Fichte / Tanne — root flare, tapered trunk & 7 organic drooping whorls)
// ----------------------------------------------------------------------------
fn build_pine_tree(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [55.0, 135.0, 55.0]);
    let bark = Mat::matte(0x3f2e21, 0.94);
    b.cyl_y([0.0, 5.0, 0.0], 5.5, 8.8, 10.0, 9, bark);
    b.cyl_y([0.0, 36.0, 0.0], 2.4, 5.6, 56.0, 9, bark);

    let tiers = [
        (22.0_f32, 28.5_f32, 34.0_f32, 0.0_f32, Mat::matte(0x134e2a, 0.95)),
        (39.0, 25.5, 32.0, 0.55, Mat::matte(0x14532d, 0.94)),
        (56.0, 22.0, 30.0, 1.15, Mat::matte(0x166534, 0.94)),
        (72.0, 18.5, 28.0, 0.30, Mat::matte(0x15803d, 0.93)),
        (88.0, 14.5, 25.0, 0.92, Mat::matte(0x166534, 0.92)),
        (103.0, 10.5, 22.0, 1.48, Mat::matte(0x16a34a, 0.91)),
        (117.0, 6.4, 21.0, 0.72, Mat::matte(0x22c55e, 0.90)),
    ];
    for (idx, (y, r, th, yaw, mat)) in tiers.into_iter().enumerate() {
        b.pine_whorl([0.0, y, 0.0], r, th, 11, yaw, 100 + idx as u32 * 53, mat);
    }
}

// ----------------------------------------------------------------------------
// 12. DECIDUOUS TREE (Laubbaum / Eiche — roots, forked limbs & 9 sculpted foliage clusters)
// ----------------------------------------------------------------------------
fn build_deciduous_tree(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [80.0, 120.0, 80.0]);
    let bark = Mat::matte(0x4b3621, 0.93);
    let leaf_dark = Mat::matte(0x14532d, 0.92);
    let leaf_mid = Mat::matte(0x15803d, 0.90);
    let leaf_light = Mat::matte(0x22c55e, 0.88);
    let leaf_sun = Mat::matte(0x4ade80, 0.86);

    b.cyl_y([0.0, 5.5, 0.0], 7.2, 12.0, 11.0, 10, bark);
    b.cyl_y([0.0, 31.0, 0.0], 4.4, 7.4, 46.0, 10, bark);
    for &(tx, ty, tz) in &[
        (18.0_f32, 72.0_f32, 11.0_f32),
        (-18.0, 74.0, -10.0),
        (5.0, 76.0, -17.0),
        (-6.0, 78.0, 16.0),
    ] {
        b.cyl_between([0.0, 49.0, 0.0], [tx, ty, tz], 3.4, 1.8, 7, bark);
    }

    let clusters = [
        (0.0_f32, 82.0_f32, 0.0_f32, 28.0_f32, 23.0_f32, 27.0_f32, 11_u32, leaf_mid),
        (18.0, 73.0, 12.0, 22.0, 19.0, 22.0, 23, leaf_light),
        (-18.0, 75.0, -12.0, 22.0, 20.0, 21.0, 37, leaf_dark),
        (-15.0, 86.0, 14.0, 20.0, 18.0, 20.0, 49, leaf_light),
        (16.0, 85.0, -15.0, 19.0, 17.0, 19.0, 61, leaf_mid),
        (2.0, 99.0, 2.0, 20.0, 17.0, 19.0, 73, leaf_sun),
        (-22.0, 68.0, 6.0, 17.0, 15.0, 17.0, 83, leaf_dark),
        (21.0, 69.0, -9.0, 17.0, 15.0, 16.0, 97, leaf_mid),
        (0.0, 72.0, 19.0, 17.0, 15.0, 16.0, 109, leaf_light),
    ];
    for (cx, cy, cz, rx, ry, rz, seed, cm) in clusters {
        b.organic_foliage_cluster([cx, cy, cz], rx, ry, rz, seed, cm);
    }
}

// ----------------------------------------------------------------------------
// 13. PLATFORM LAMP (Bahnsteigleuchte — cast-iron plinth, tapered post, curved swan-neck, shade & bulb)
// ----------------------------------------------------------------------------
fn build_platform_lamp(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [20.0, 70.0, 20.0]);
    let iron = Mat::metal(0x1e293b, 0.36, 0.78);
    let shade = Mat::metal(0x334155, 0.44, 0.45);
    let reflector = Mat::matte(0xf8fafc, 0.40);
    let glow = Mat::emissive(0xfef08a, 1.35);

    b.rbox([0.0, 1.1, 0.0], [7.5, 2.2, 7.5], iron);
    b.cyl_y([0.0, 7.5, 0.0], 2.4, 3.6, 11.0, 10, iron);
    b.cyl_y([0.0, 13.5, 0.0], 3.0, 3.0, 1.6, 10, iron);
    b.cyl_y([0.0, 37.5, 0.0], 1.1, 1.8, 46.5, 10, iron);
    b.cyl_y([0.0, 60.0, 0.0], 1.9, 1.9, 1.4, 10, iron);

    let mut neck = Vec::with_capacity(11);
    for i in 0..=10 {
        let a = std::f32::consts::PI - (i as f32 / 10.0) * std::f32::consts::FRAC_PI_2;
        neck.push([7.8 + a.cos() * 7.8, 60.0 + a.sin() * 7.8, 0.0]);
    }
    b.curved_pipe(&neck, 0.95, 8, iron);
    b.cone_y([15.6, 62.6, 0.0], 5.6, 3.2, 16, 0.0, shade);
    b.cyl_y([15.6, 62.3, 0.0], 5.1, 5.1, 0.6, 16, reflector);
    b.sphere([15.6, 61.0, 0.0], 2.2, 8, 12, glow);
}

// ----------------------------------------------------------------------------
// 14. PLATFORM (Bahnsteig — sloped ramps, curbs, stairs, butterfly glass canopy, clock, ZZA, benches)
// ----------------------------------------------------------------------------
fn build_platform(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [320.0, 55.0, 54.0]);
    let deck = Mat::matte(0x94a3b8, 0.88);
    let curb = Mat::matte(0x475569, 0.90);
    let safety_w = Mat::matte(0xf8fafc, 0.50);
    let warn_y = Mat::matte(0xeab308, 0.60);
    let steel = Mat::metal(0x1e293b, 0.44, 0.68);
    let sheet = Mat::metal(0x334155, 0.54, 0.42);
    let glass = Mat::glass(0x7dd3fc);
    let rail_glass = Mat::glass(0x38bdf8);
    let chrome = Mat::metal(0x94a3b8, 0.25, 0.85);
    let wood = Mat::matte(0xb45309, 0.76);
    let db_blue = Mat::metal(0x0369a1, 0.48, 0.18);
    let zza_glow = Mat::emissive(0x38bdf8, 0.85);
    let sos_red = Mat::matte(0xdc2626, 0.50);
    let lamp_glow = Mat::emissive(0xfef08a, 1.10);
    let dark_pit = Mat::matte(0x0f172a, 0.95);

    // 1. Concrete platform deck & sloped end ramps (-160..-140, +140..+160)
    b.rbox([0.0, 5.0, 0.0], [280.0, 10.0, 52.0], deck);
    // Left ramp wedge
    for (z0, z1, nz) in [(-26.0_f32, -26.0, -1.0_f32), (26.0, 26.0, 1.0)] {
        b.quad_oriented([-160.0, 0.0, z0], [-140.0, 0.0, z0], [-140.0, 10.0, z1], [-160.0, 1.2, z1], [0.0, 0.0, nz], deck);
    }
    b.quad_oriented([-160.0, 1.2, -26.0], [-140.0, 10.0, -26.0], [-140.0, 10.0, 26.0], [-160.0, 1.2, 26.0], [-0.4, 0.9, 0.0], deck);
    // Right ramp wedge
    for (z0, z1, nz) in [(-26.0_f32, -26.0, -1.0_f32), (26.0, 26.0, 1.0)] {
        b.quad_oriented([140.0, 0.0, z0], [160.0, 0.0, z0], [160.0, 1.2, z1], [140.0, 10.0, z1], [0.0, 0.0, nz], deck);
    }
    b.quad_oriented([140.0, 10.0, -26.0], [160.0, 1.2, -26.0], [160.0, 1.2, 26.0], [140.0, 10.0, 26.0], [0.4, 0.9, 0.0], deck);

    for rx in [-139.5_f32, 139.5] {
        b.rbox([rx, 10.2, 0.0], [2.0, 0.4, 52.0], warn_y);
    }
    for z_side in [-26.2_f32, 26.2] {
        b.rbox([0.0, 5.15, z_side], [320.0, 10.3, 2.0], curb);
    }
    for z_side in [-22.5_f32, 22.5] {
        b.rbox([0.0, 10.22, z_side], [300.0, 0.4, 1.8], safety_w);
    }

    // 2. Underpass stairwell + steps + chrome railings & glass balustrades
    b.rbox([0.0, 9.75, 0.0], [34.0, 0.8, 18.0], dark_pit);
    for s in 0..7 {
        b.rbox([12.0 - s as f32 * 3.8, 10.0 - s as f32 * 0.45, 0.0], [3.6, 1.2, 16.0], curb);
    }
    for &(px, pz) in &[
        (-17.0_f32, -9.0_f32), (-6.0, -9.0), (6.0, -9.0), (17.0, -9.0),
        (-17.0, 9.0), (-6.0, 9.0), (6.0, 9.0), (17.0, 9.0), (-17.0, 0.0),
    ] {
        b.cyl_y([px, 15.0, pz], 0.5, 0.5, 10.0, 8, chrome);
    }
    for pz in [-9.0_f32, 9.0] {
        b.rbox([0.0, 20.0, pz], [34.0, 0.8, 0.8], chrome);
        b.rbox([0.0, 14.5, pz], [32.0, 8.0, 0.4], rail_glass);
    }
    b.rbox([-17.0, 20.0, 0.0], [0.8, 0.8, 18.0], chrome);
    b.rbox([-17.0, 14.5, 0.0], [0.4, 8.0, 16.0], rail_glass);

    // 3. Steel-and-glass V butterfly canopy (Y-arms + cylindrical struts)
    let (can_len, roof_y, slope_ang) = (210.0_f32, 40.0_f32, 0.12_f32);
    let slope_tan = slope_ang.tan();
    let y_ridge = roof_y + 0.6;
    for px in [-80.0_f32, -32.0, 32.0, 80.0] {
        b.rbox([px, 10.75, 0.0], [5.0, 1.5, 5.0], steel);
        b.rbox([px, 25.0, 0.0], [3.5, 30.0, 3.5], steel);
        for z_dir in [-1.0_f32, 1.0] {
            let arm_len = 24.0 / slope_ang.cos();
            let arm_y = (roof_y - 0.8) + 12.0 * slope_tan;
            b.obox([px, arm_y, z_dir * 12.0], [2.5, 2.2, arm_len], [-z_dir * slope_ang, 0.0, 0.0], steel);
            b.cyl_between([px, 31.0, 0.0], [px, roof_y - 0.8 + 11.0 * slope_tan, z_dir * 11.0], 0.85, 0.85, 8, steel);
        }
    }
    b.rbox([0.0, roof_y, 0.0], [can_len, 3.0, 3.2], steel);
    for z_side in [-24.2_f32, 24.2] {
        b.rbox([0.0, y_ridge + 24.2 * slope_tan - 0.6, z_side], [can_len, 2.6, 2.0], steel);
    }
    for z_dir in [-1.0_f32, 1.0] {
        b.obox([0.0, y_ridge + 5.0 * slope_tan, z_dir * 5.0], [can_len - 4.0, 1.2, 7.4], [-z_dir * slope_ang, 0.0, 0.0], glass);
        b.obox([0.0, y_ridge + 16.5 * slope_tan, z_dir * 16.5], [can_len, 1.5, 16.6], [-z_dir * slope_ang, 0.0, 0.0], sheet);
    }
    for x_end in [-can_len * 0.5, can_len * 0.5] {
        for z_dir in [-1.0_f32, 1.0] {
            b.obox([x_end, y_ridge + 12.25 * slope_tan - 0.3, z_dir * 12.25], [1.5, 2.8, 25.0], [-z_dir * slope_ang, 0.0, 0.0], steel);
        }
    }

    // 4. Hanging ZZA boards, station sign, double-sided clock, benches, SOS pillar & end lamps
    for zx in [-45.0_f32, 45.0] {
        b.rbox([zx, roof_y - 5.0, 0.0], [18.0, 6.5, 7.0], db_blue);
        for hx in [-6.0_f32, 6.0] {
            b.cyl_y([zx + hx, roof_y - 1.0, 0.0], 0.4, 0.4, 4.0, 6, steel);
        }
        for z_dir in [-1.0_f32, 1.0] {
            b.rbox([zx, roof_y - 5.0, z_dir * 3.6], [16.0, 4.5, 0.5], zza_glow);
        }
    }
    b.rbox([0.0, roof_y - 5.5, 0.0], [32.0, 5.5, 1.6], db_blue);
    b.rbox([0.0, roof_y - 5.5, 0.0], [28.0, 3.2, 2.0], safety_w);
    b.cyl_y([-11.0, roof_y - 1.5, 0.0], 0.4, 0.4, 4.5, 6, steel);
    b.cyl_y([11.0, roof_y - 1.5, 0.0], 0.4, 0.4, 4.5, 6, steel);
    b.cyl_y([16.0, roof_y - 1.0, 0.0], 0.4, 0.4, 3.5, 6, steel);
    for side in [-1.0_f32, 1.0] {
        b.clock_face_x([16.0 + side * 0.2, roof_y - 4.5, 0.0], side, 3.5, steel, safety_w, steel);
    }
    for bx in [-58.0_f32, 58.0] {
        for fx in [-7.0_f32, 7.0] {
            b.rbox([bx + fx, 12.25, 0.0], [1.2, 4.5, 7.5], steel);
        }
        b.rbox([bx, 14.5, 0.0], [18.0, 0.8, 7.5], wood);
        b.rbox([bx, 17.0, 0.0], [18.0, 3.5, 0.8], wood);
    }
    b.rbox([-32.0, 16.0, 0.0], [1.2, 12.0, 14.0], steel);
    b.rbox([-32.0, 16.0, 0.0], [1.6, 9.5, 11.0], warn_y);
    b.rbox([32.0, 17.0, 4.0], [2.5, 14.0, 2.5], sos_red);
    b.rbox([32.0, 23.5, 4.0], [3.2, 2.5, 3.2], safety_w);
    for px in [-80.0_f32, 80.0] {
        b.cyl_y([px, 14.5, 3.5], 1.5, 1.3, 4.0, 10, chrome);
    }
    for lx in [-132.0_f32, 132.0] {
        b.cyl_y([lx, 25.0, 0.0], 1.0, 1.4, 30.0, 8, steel);
        b.rbox([lx, 40.5, 0.0], [8.0, 1.2, 4.0], steel);
        b.rbox([lx, 39.8, 0.0], [6.5, 0.7, 2.8], lamp_glow);
    }
}

// ----------------------------------------------------------------------------
// 15. SKYSCRAPER (Wolkenkratzer — colonnade lobby, glazed curtain wall + mullions, setback terrace, spire & beacon)
// ----------------------------------------------------------------------------
fn build_skyscraper(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [130.0, 280.0, 110.0]);
    let plaza = Mat::matte(0x334155, 0.82);
    let curb = Mat::matte(0x64748b, 0.74);
    let bush = Mat::matte(0x15803d, 0.90);
    let steel = Mat::metal(0x1e293b, 0.38, 0.72);
    let alum = Mat::metal(0x94a3b8, 0.32, 0.70);
    let glass = Mat::warm_window(0xfde68a, 0.42);
    let warm_office = Mat::warm_window(0xfde68a, 0.95);
    let dark_glass = Mat::warm_window(0xbae6fd, 0.36);
    let roof = Mat::matte(0x1e293b, 0.75);
    let beacon = Mat::emissive(0xef4444, 1.30);
    let sign_glow = Mat::emissive(0xfef08a, 1.00);

    // 1. Plaza, steps & ornamental planter bushes
    b.rbox([0.0, 2.0, 0.0], [130.0, 4.0, 110.0], plaza);
    b.rbox([0.0, 3.0, 50.0], [60.0, 2.0, 14.0], curb);
    for px in [-42.0_f32, 42.0] {
        b.rbox([px, 5.0, 45.0], [14.0, 5.5, 14.0], curb);
        b.organic_foliage_cluster([px, 11.5, 45.0], 6.4, 5.8, 6.4, (px as i32 + 99) as u32, bush);
    }

    // 2. Lobby podium with colonnade & lit entrance awning
    let lobby_h = 24.0_f32;
    b.rbox([0.0, 4.0 + lobby_h * 0.5, 0.0], [102.0, lobby_h, 82.0], warm_office);
    b.rbox([0.0, 22.0, 47.0], [50.0, 2.4, 18.0], steel);
    b.rbox([0.0, 24.8, 48.0], [36.0, 3.6, 1.5], sign_glow);
    for cx in [-49.0_f32, -24.5, 0.0, 24.5, 49.0] {
        b.rbox([cx, 4.0 + lobby_h * 0.5, 41.0], [3.6, lobby_h, 3.6], steel);
    }
    b.rbox([0.0, 4.0 + lobby_h + 1.5, 0.0], [106.0, 3.0, 86.0], steel);

    // 3. Main curtain-wall shaft + 10 spandrel bands + perimeter mullions + lit office window accents
    let t1_h = 136.0_f32;
    let t1_base = 31.0_f32;
    let t1_y = t1_base + t1_h * 0.5;
    b.rbox([0.0, t1_y, 0.0], [98.0, t1_h, 78.0], glass);
    for f in 1..=10 {
        let fy = t1_base + (t1_h * f as f32) / 11.0;
        b.rbox([0.0, fy, 0.0], [99.4, 2.1, 79.4], alum);
        if f % 2 == 1 {
            b.rbox([((f as f32 - 5.5) * 6.0).clamp(-32.0, 32.0), fy - 5.5, 39.4], [24.0, 7.2, 0.8], warm_office);
            b.rbox([((5.5 - f as f32) * 5.0).clamp(-30.0, 30.0), fy - 5.5, -39.4], [22.0, 7.2, 0.8], warm_office);
        }
    }
    for mx in (-44..=44).step_by(14) {
        b.rbox([mx as f32, t1_y, 39.4], [1.5, t1_h, 1.8], steel);
        b.rbox([mx as f32, t1_y, -39.4], [1.5, t1_h, 1.8], steel);
    }
    for mz in (-32..=32).step_by(16) {
        b.rbox([-49.4, t1_y, mz as f32], [1.8, t1_h, 1.5], steel);
        b.rbox([49.4, t1_y, mz as f32], [1.8, t1_h, 1.5], steel);
    }

    // 4. Setback upper tower with observation terrace & safety rail
    let setback_y = t1_base + t1_h;
    b.rbox([0.0, setback_y + 1.5, 0.0], [102.0, 3.0, 82.0], steel);
    for sx in [-49.0_f32, 49.0] {
        b.rbox([sx, setback_y + 5.0, 0.0], [1.6, 5.2, 80.0], alum);
    }
    let t2_h = 62.0_f32;
    let t2_y = setback_y + 3.0 + t2_h * 0.5;
    b.rbox([0.0, t2_y, 0.0], [76.0, t2_h, 60.0], dark_glass);
    for f in 1..=5 {
        b.rbox([0.0, setback_y + 3.0 + (t2_h * f as f32) / 6.0, 0.0], [77.2, 1.9, 61.2], alum);
    }
    for mx in [-30.0_f32, -10.0, 10.0, 30.0] {
        b.rbox([mx, t2_y, 30.3], [1.4, t2_h, 1.5], steel);
        b.rbox([mx, t2_y, -30.3], [1.4, t2_h, 1.5], steel);
    }

    // 5. Roof mechanical penthouse, HVAC units, tapered cylindrical antenna spire & aviation beacon
    let roof_y = setback_y + 3.0 + t2_h;
    b.rbox([0.0, roof_y + 1.5, 0.0], [78.0, 3.0, 62.0], steel);
    b.rbox([-7.0, roof_y + 9.5, 0.0], [34.0, 14.0, 28.0], roof);
    for acz in [-9.0_f32, 9.0] {
        b.rbox([20.0, roof_y + 5.8, acz], [14.0, 7.2, 10.0], curb);
    }
    b.cyl_y([-7.0, roof_y + 31.0, 0.0], 1.1, 2.3, 30.0, 10, steel);
    b.cyl_y([-7.0, roof_y + 52.0, 0.0], 0.4, 1.0, 13.0, 8, alum);
    b.sphere([-7.0, roof_y + 59.0, 0.0], 2.0, 8, 10, beacon);
}

// ----------------------------------------------------------------------------
// 16. INDUSTRY BUILDING (Industriehalle — sawtooth roofs, dock & ramp, roll doors, cylindrical silo & stack)
// ----------------------------------------------------------------------------
fn build_industry_building(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [260.0, 110.0, 160.0]);
    let ground = Mat::matte(0x475569, 0.90);
    let ramp = Mat::matte(0x64748b, 0.85);
    let brick = Mat::matte(0x78350f, 0.87);
    let metal_wall = Mat::metal(0x52525b, 0.56, 0.38);
    let trim = Mat::metal(0x334155, 0.48, 0.55);
    let glass = Mat::glass(0x38bdf8);
    let win_warm = Mat::warm_window(0xfde68a, 0.72);
    let gate_blue = Mat::metal(0x0369a1, 0.40, 0.50);
    let silo_m = Mat::metal(0x94a3b8, 0.28, 0.82);
    let pipe_m = Mat::metal(0x1e293b, 0.36, 0.76);
    let hazard = Mat::matte(0xeab308, 0.52);
    let chim_m = Mat::matte(0x28211e, 0.76);

    // 1. Concrete yard apron, loading dock, sloped forklift wedge & dock bumpers
    b.rbox([0.0, 2.0, 0.0], [260.0, 4.0, 160.0], ground);
    b.rbox([-35.0, 9.0, 62.0], [126.0, 10.0, 24.0], ramp);
    for (z0, nz) in [(50.0_f32, -1.0_f32), (74.0, 1.0)] {
        b.quad_oriented([28.0, 4.0, z0], [50.0, 4.0, z0], [28.0, 14.0, z0], [28.0, 14.0, z0], [0.0, 0.0, nz], ramp);
    }
    b.quad_oriented([28.0, 14.0, 50.0], [50.0, 4.0, 50.0], [50.0, 4.0, 74.0], [28.0, 14.0, 74.0], [0.4, 0.9, 0.0], ramp);
    for bx in [-82.0_f32, -59.0, -36.0, -13.0, 10.0] {
        b.rbox([bx, 10.0, 75.5], [5.5, 5.5, 3.0], pipe_m);
    }

    // 2. Two-tone hall body (brick lower + steel panel upper + pilasters + loading canopy & sectional doors)
    let (hall_w, hall_d, wall_h, hall_cx) = (182.0_f32, 110.0_f32, 52.0_f32, -20.0_f32);
    b.rbox([hall_cx, 12.0, 0.0], [hall_w, 16.0, hall_d], brick);
    b.rbox([hall_cx, 20.0 + (wall_h - 16.0) * 0.5, 0.0], [hall_w, wall_h - 16.0, hall_d], metal_wall);
    for k in 0..=5 {
        let cx = hall_cx - hall_w * 0.5 + (k as f32) * (hall_w / 5.0);
        b.rbox([cx, 4.0 + wall_h * 0.5, hall_d * 0.5], [3.0, wall_h, 3.0], trim);
    }
    b.rbox([-35.0, 40.0, 65.0], [134.0, 2.4, 22.0], trim);
    b.rbox([-35.0, 39.4, 76.2], [134.0, 1.2, 0.6], hazard);
    for gx in [-62.0_f32, -10.0] {
        b.rbox([gx, 26.5, 55.2], [30.0, 25.0, 1.8], trim);
        b.rbox([gx, 26.0, 55.7], [26.0, 23.0, 1.5], gate_blue);
    }
    b.rbox([hall_cx, 20.0, -55.5], [26.0, 32.0, 1.5], gate_blue);
    for fz in [-32.0_f32, 0.0, 32.0] {
        b.rbox([hall_cx - hall_w * 0.5 - 0.6, 35.0, fz], [1.4, 13.0, 19.0], win_warm);
    }

    // 3. True asymmetrical sawtooth (shed) roofs with north-lit skylight glass bays
    let z_step = hall_d / 3.0;
    for i in 0..3 {
        let z0 = -hall_d * 0.5 + i as f32 * z_step;
        let z1 = z0 + z_step;
        b.sawtooth_roof_bay(
            hall_cx - hall_w * 0.5,
            hall_cx + hall_w * 0.5,
            z0,
            z1,
            4.0 + wall_h,
            21.0,
            trim,
            glass,
            pipe_m,
        );
    }

    // 4. Cylindrical brushed-steel silo (+X = 98) with hopper, cone roof, ladder & overhead pipe
    let (sx, sz, sr, sh) = (98.0_f32, 4.0_f32, 17.5_f32, 76.0_f32);
    b.cyl_y([sx, 16.0 + sh * 0.5, sz], sr, sr, sh, 20, silo_m);
    b.cone_y([sx, 16.0 + sh, sz], sr + 1.5, 11.0, 20, 0.0, silo_m);
    b.cyl_y([sx, 10.0, sz], sr, 6.0, 12.0, 16, trim);
    for k in 0..4 {
        let a = (k as f32) * std::f32::consts::FRAC_PI_2;
        b.rbox([sx + (sr - 2.0) * a.cos(), 13.0, sz + (sr - 2.0) * a.sin()], [2.8, 18.0, 2.8], pipe_m);
    }
    b.rbox([sx, 48.0, sz + sr + 1.0], [3.6, 76.0, 1.5], pipe_m);
    b.cyl_x([sx - 20.0, 48.0, sz], 2.4, 2.4, 28.0, 10, pipe_m);

    // 5. Tall tapered brick/masonry factory chimney (+X = 82, -Z = 42)
    b.cyl_y([82.0, 56.0, -42.0], 4.8, 7.2, 100.0, 14, chim_m);
    for by in [36.0_f32, 62.0, 88.0] {
        let r = 7.2 - ((by - 6.0) / 100.0) * 2.4;
        b.cyl_y([82.0, by, -42.0], r + 0.35, r + 0.35, 1.8, 14, pipe_m);
    }
    b.cone_y([82.0, 106.0, -42.0], 9.2, 5.2, 14, 0.0, pipe_m);
}

// ----------------------------------------------------------------------------
// GENERIC FALLBACK BUILDING (architectural plinth, plaster, framed windows, overhang roof)
// ----------------------------------------------------------------------------
fn build_generic_building(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [80.0, 60.0, 60.0]);
    let stone = Mat::matte(0x52525b, 0.88);
    let wall = Mat::matte(0xdcd3c4, 0.80);
    let roof = Mat::matte(0x7c2d12, 0.70);
    let cornice = Mat::matte(0xcbd5e1, 0.75);
    let frame = Mat::matte(0xf8fafc, 0.65);
    let win = Mat::warm_window(0xfde68a, 0.72);
    let door = Mat::matte(0x5c2e14, 0.78);

    b.rbox([0.0, 2.5, 0.0], [78.0, 5.0, 58.0], stone);
    b.rbox([0.0, 23.0, 0.0], [74.0, 36.0, 54.0], wall);
    b.rbox([0.0, 41.5, 0.0], [76.0, 1.8, 56.0], cornice);
    b.gable_roof_x([0.0, 42.4, 0.0], 74.0, 54.0, 16.0, 2.4, 1.8, roof, wall, roof);
    b.rbox([0.0, 15.0, 27.2], [11.0, 20.0, 1.2], door);
    for side in [-1.0_f32, 1.0] {
        for dx in [-22.0_f32, 22.0] {
            b.window_z(dx, 26.0, side * 27.0, side, 9.0, 12.0, win, frame, cornice, stone);
        }
    }
    let _ = MAT_DEFAULT;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_tools::BUILTIN_MODELS;
    use railbench_core::types::Transform;

    fn mk_model(id: Option<&str>, w: f64, d: f64, h: f64) -> Model3D {
        Model3D {
            id: "test-m".into(),
            name: id.unwrap_or("generic").into(),
            category: "railway".into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: w,
            depth: d,
            height: h,
            model_type: "builtin".into(),
            builtin_id: id.map(|s| s.to_string()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        }
    }

    #[test]
    fn all_16_builtin_models_generate_valid_detailed_meshes() {
        for def in BUILTIN_MODELS {
            let m = mk_model(Some(def.id), def.width, def.depth, def.height);
            let mut verts = Vec::new();
            let mut idxs = Vec::new();
            push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);

            assert!(verts.len() >= 100, "{} vertex count too small: {}", def.id, verts.len());
            assert!(idxs.len() >= 150 && idxs.len() % 3 == 0, "{} index count invalid: {}", def.id, idxs.len());
            let max_i = *idxs.iter().max().unwrap_or(&0) as usize;
            assert!(max_i < verts.len(), "{} out-of-bounds index {} >= {}", def.id, max_i, verts.len());

            for v in &verts {
                assert!(v.pos.iter().all(|x| x.is_finite()), "{} non-finite pos", def.id);
                assert!(v.normal.iter().all(|x| x.is_finite()), "{} non-finite normal", def.id);
                assert!(v.color.iter().all(|x| x.is_finite() && *x >= 0.0 && *x <= 1.0), "{} invalid color", def.id);
                let nlen = (v.normal[0] * v.normal[0] + v.normal[1] * v.normal[1] + v.normal[2] * v.normal[2]).sqrt();
                assert!((0.95..=1.05).contains(&nlen), "{} unnormalized normal len {}", def.id, nlen);
                assert!((0.04..=1.0).contains(&v.mat[0]), "{} bad roughness {}", def.id, v.mat[0]);
                assert!((0.0..=1.0).contains(&v.mat[1]), "{} bad metallic {}", def.id, v.mat[1]);
            }
        }

        // Alias & fallback check
        for id in [Some("bahnsteig"), None] {
            let m = mk_model(id, 100.0, 60.0, 80.0);
            let mut verts = Vec::new();
            let mut idxs = Vec::new();
            push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);
            assert!(verts.len() >= 100 && !idxs.is_empty());
        }
    }

    #[test]
    fn lit_models_emit_emissive_and_night_window_materials() {
        let ids = [
            "station",
            "town-house",
            "signal-box",
            "platform-lamp",
            "light-signal",
            "skyscraper",
            "platform",
            "diesel-fuel-station",
            "engine-shed",
            "turntable-trainhall",
        ];
        for id in ids {
            let def = crate::model_tools::find_builtin_def(id).unwrap();
            let m = mk_model(Some(id), def.width, def.depth, def.height);
            let mut verts = Vec::new();
            let mut idxs = Vec::new();
            push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);
            assert!(
                verts.iter().any(|v| v.mat[2].abs() > 0.01),
                "{} must emit emissive or night-glow window vertices",
                id
            );
        }
    }

    #[test]
    fn curved_models_have_non_axis_aligned_normals() {
        for id in ["water-tower", "pine-tree", "deciduous-tree", "diesel-fuel-station", "platform-lamp", "industry-building"] {
            let def = crate::model_tools::find_builtin_def(id).unwrap();
            let m = mk_model(Some(id), def.width, def.depth, def.height);
            let mut verts = Vec::new();
            let mut idxs = Vec::new();
            push_builtin_model_3d(&mut verts, &mut idxs, &m, 0.0);
            let oblique_normals = verts
                .iter()
                .filter(|v| {
                    let ax = v.normal[0].abs();
                    let az = v.normal[2].abs();
                    ax > 0.15 && az > 0.15
                })
                .count();
            assert!(oblique_normals >= 20, "{} should have smooth curved normals, found {}", id, oblique_normals);
        }
    }

    #[test]
    fn engine_shed_and_roundhouse_preserve_open_track_doorways() {
        // Engine shed: clear opening at x = -120 across y in [15..65], z in [-14..14]
        let shed = mk_model(Some("engine-shed"), 240.0, 72.0, 105.0);
        let mut s_verts = Vec::new();
        let mut s_idxs = Vec::new();
        push_builtin_model_3d(&mut s_verts, &mut s_idxs, &shed, 0.0);
        for tri in s_idxs.chunks_exact(3) {
            let p = [s_verts[tri[0] as usize].pos, s_verts[tri[1] as usize].pos, s_verts[tri[2] as usize].pos];
            let cx = (p[0][0] + p[1][0] + p[2][0]) / 3.0;
            let cy = (p[0][1] + p[1][1] + p[2][1]) / 3.0;
            let cz = (p[0][2] + p[1][2] + p[2][2]) / 3.0;
            assert!(
                !(cx < -114.0 && (12.0..=68.0).contains(&cy) && cz.abs() < 15.0),
                "engine-shed doorway blocked at ({cx:.1}, {cy:.1}, {cz:.1})"
            );
        }

        // Turntable trainhall: 3 open portals at (-40.54, -159.26), (0, -155), (+40.54, -159.26)
        let hall = mk_model(Some("turntable-trainhall"), 320.0, 310.0, 115.0);
        let mut h_verts = Vec::new();
        let mut h_idxs = Vec::new();
        push_builtin_model_3d(&mut h_verts, &mut h_idxs, &hall, 0.0);
        let min_z = h_verts.iter().map(|v| v.pos[2]).fold(f32::INFINITY, f32::min);
        let max_z = h_verts.iter().map(|v| v.pos[2]).fold(f32::NEG_INFINITY, f32::max);
        assert!(min_z <= -155.0 && max_z >= 150.0, "unexpected roundhouse z span: {min_z}..{max_z}");
        for &(px, pz) in &[(-40.54_f32, -159.26_f32), (0.0, -155.0), (40.54, -159.26)] {
            for tri in h_idxs.chunks_exact(3) {
                let p = [h_verts[tri[0] as usize].pos, h_verts[tri[1] as usize].pos, h_verts[tri[2] as usize].pos];
                let cx = (p[0][0] + p[1][0] + p[2][0]) / 3.0;
                let cy = (p[0][1] + p[1][1] + p[2][1]) / 3.0;
                let cz = (p[0][2] + p[1][2] + p[2][2]) / 3.0;
                assert!(
                    !((cx - px).abs() < 8.0 && (cz - pz).abs() < 6.0 && (12.0..=72.0).contains(&cy)),
                    "roundhouse stall ({px}, {pz}) doorway blocked at ({cx:.1}, {cy:.1}, {cz:.1})"
                );
            }
        }
    }
}
