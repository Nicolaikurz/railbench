use railbench_core::types::Model3D;

use crate::viewer3d::{normalize3, rot_xz, Vertex3D, MAT_DEFAULT};

/// Per-vertex PBR-like material descriptor packed into `Vertex3D`:
/// - `rough`: surface roughness `[0.04, 1.0]`
/// - `metal`: metallic factor `[0.0, 1.0]`
/// - `emissive`: `> 0.0` always-on self-illumination; `< 0.0` dusk/night window glow magnitude
#[derive(Clone, Copy)]
pub(crate) struct Mat {
    pub(crate) col: [f32; 3],
    pub(crate) rough: f32,
    pub(crate) metal: f32,
    pub(crate) emissive: f32,
}

impl Mat {
    pub(crate) const fn matte(rgb: u32, rough: f32) -> Self {
        Self {
            col: hex(rgb),
            rough,
            metal: 0.02,
            emissive: 0.0,
        }
    }

    pub(crate) const fn metal(rgb: u32, rough: f32, metal: f32) -> Self {
        Self {
            col: hex(rgb),
            rough,
            metal,
            emissive: 0.0,
        }
    }

    pub(crate) const fn glass(rgb: u32) -> Self {
        Self {
            col: hex(rgb),
            rough: 0.10,
            metal: 0.25,
            emissive: 0.0,
        }
    }

    /// Daytime dark reflective pane that glows warmly with `emissive = -glow` as night falls.
    pub(crate) const fn warm_window(rgb: u32, glow: f32) -> Self {
        Self {
            col: hex(rgb),
            rough: 0.14,
            metal: 0.18,
            emissive: -glow,
        }
    }

    pub(crate) const fn emissive(rgb: u32, strength: f32) -> Self {
        Self {
            col: hex(rgb),
            rough: 0.20,
            metal: 0.05,
            emissive: strength,
        }
    }

    pub(crate) fn with_col(self, col: [f32; 3]) -> Self {
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

pub(crate) struct ModelBuilder<'a> {
    verts: &'a mut Vec<Vertex3D>,
    idxs: &'a mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    scale: [f32; 3],
    inv_scale: [f32; 3],
}

impl<'a> ModelBuilder<'a> {
    pub(crate) fn new(
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
            pitch_rad: 0.0,
            scale: [sx, sy, sz],
            inv_scale: [1.0 / sx, 1.0 / sy, 1.0 / sz],
        }
    }

    pub(crate) fn with_pitch(mut self, pitch_rad: f32) -> Self {
        self.pitch_rad = pitch_rad;
        self
    }

    #[inline]
    fn xform_pos(&self, p: [f32; 3]) -> [f32; 3] {
        let mut sx = p[0] * self.scale[0];
        let mut sy = p[1] * self.scale[1];
        let sz = p[2] * self.scale[2];
        if self.pitch_rad.abs() > 1e-5 {
            let (sp, cp) = self.pitch_rad.sin_cos();
            let nx = sx * cp - sy * sp;
            let ny = sx * sp + sy * cp;
            sx = nx;
            sy = ny;
        }
        let (rx, rz) = rot_xz(sx, sz, self.rot_rad);
        [self.origin[0] + rx, self.origin[1] + sy, self.origin[2] + rz]
    }

    #[inline]
    fn xform_norm(&self, n: [f32; 3]) -> [f32; 3] {
        let mut nx = n[0] * self.inv_scale[0];
        let mut ny = n[1] * self.inv_scale[1];
        let nz = n[2] * self.inv_scale[2];
        if self.pitch_rad.abs() > 1e-5 {
            let (sp, cp) = self.pitch_rad.sin_cos();
            let rnx = nx * cp - ny * sp;
            let rny = nx * sp + ny * cp;
            nx = rnx;
            ny = rny;
        }
        let (rx, rz) = rot_xz(nx, nz, self.rot_rad);
        normalize3([rx, ny, rz])
    }

    #[inline]
    pub(crate) fn push_raw_tri(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], n: [f32; 3], m: Mat) {
        let wn = self.xform_norm(n);
        let mp = m.pack();
        let base = self.verts.len() as u32;
        self.verts.push(Vertex3D::new(self.xform_pos(p0), wn, m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p1), wn, m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p2), wn, m.col, mp));
        self.idxs.extend_from_slice(&[base, base + 1, base + 2]);
    }

    #[inline]
    pub(crate) fn push_raw_quad(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], n: [f32; 3], m: Mat) {
        let wn = self.xform_norm(n);
        let mp = m.pack();
        let base = self.verts.len() as u32;
        self.verts.push(Vertex3D::new(self.xform_pos(p0), wn, m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p1), wn, m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p2), wn, m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p3), wn, m.col, mp));
        self.idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub(crate) fn quad_oriented(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], outward: [f32; 3], m: Mat) {
        let mut n = cross3(sub3(p1, p0), sub3(p2, p0));
        if dot3(n, n) < 1e-8 {
            n = outward;
        }
        if dot3(n, outward) < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        self.push_raw_quad(p0, p1, p2, p3, normalize3(n), m);
    }

    pub(crate) fn tri_oriented(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], outward: [f32; 3], m: Mat) {
        let mut n = cross3(sub3(p1, p0), sub3(p2, p0));
        if dot3(n, n) < 1e-8 {
            n = outward;
        }
        if dot3(n, outward) < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        self.push_raw_tri(p0, p1, p2, normalize3(n), m);
    }

    pub(crate) fn rbox(&mut self, center: [f32; 3], size: [f32; 3], m: Mat) {
        self.obox(center, size, [0.0, 0.0, 0.0], m);
    }

    pub(crate) fn obox(&mut self, center: [f32; 3], size: [f32; 3], euler: [f32; 3], m: Mat) {
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

    pub(crate) fn strut_between(&mut self, p0: [f32; 3], p1: [f32; 3], width: f32, depth: f32, m: Mat) {
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

    #[inline]
    pub(crate) fn push_smooth_quad(
        &mut self,
        p0: [f32; 3],
        p1: [f32; 3],
        p2: [f32; 3],
        p3: [f32; 3],
        n0: [f32; 3],
        n1: [f32; 3],
        n2: [f32; 3],
        n3: [f32; 3],
        m: Mat,
    ) {
        let mp = m.pack();
        let base = self.verts.len() as u32;
        self.verts.push(Vertex3D::new(self.xform_pos(p0), self.xform_norm(n0), m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p1), self.xform_norm(n1), m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p2), self.xform_norm(n2), m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p3), self.xform_norm(n3), m.col, mp));
        self.idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    #[inline]
    pub(crate) fn push_smooth_tri(
        &mut self,
        p0: [f32; 3],
        p1: [f32; 3],
        p2: [f32; 3],
        n0: [f32; 3],
        n1: [f32; 3],
        n2: [f32; 3],
        m: Mat,
    ) {
        let mp = m.pack();
        let base = self.verts.len() as u32;
        self.verts.push(Vertex3D::new(self.xform_pos(p0), self.xform_norm(n0), m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p1), self.xform_norm(n1), m.col, mp));
        self.verts.push(Vertex3D::new(self.xform_pos(p2), self.xform_norm(n2), m.col, mp));
        self.idxs.extend_from_slice(&[base, base + 1, base + 2]);
    }

    pub(crate) fn cyl_between(&mut self, p0: [f32; 3], p1: [f32; 3], r0: f32, r1: f32, segs: usize, m: Mat) {
        let segs = segs.max(8);
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

            let dir0 = [u[0] * c0 + v[0] * s0, u[1] * c0 + v[1] * s0, u[2] * c0 + v[2] * s0];
            let dir1 = [u[0] * c1 + v[0] * s1, u[1] * c1 + v[1] * s1, u[2] * c1 + v[2] * s1];
            let n0 = normalize3([
                dir0[0] * len + axis[0] * dr,
                dir0[1] * len + axis[1] * dr,
                dir0[2] * len + axis[2] * dr,
            ]);
            let n1 = normalize3([
                dir1[0] * len + axis[0] * dr,
                dir1[1] * len + axis[1] * dr,
                dir1[2] * len + axis[2] * dr,
            ]);

            let q0 = [p0[0] + dir0[0] * r0, p0[1] + dir0[1] * r0, p0[2] + dir0[2] * r0];
            let q1 = [p0[0] + dir1[0] * r0, p0[1] + dir1[1] * r0, p0[2] + dir1[2] * r0];
            let q2 = [p1[0] + dir1[0] * r1, p1[1] + dir1[1] * r1, p1[2] + dir1[2] * r1];
            let q3 = [p1[0] + dir0[0] * r1, p1[1] + dir0[1] * r1, p1[2] + dir0[2] * r1];

            if r0 > 1e-3 && r1 > 1e-3 {
                self.push_smooth_quad(q0, q1, q2, q3, n0, n1, n1, n0, m);
            } else if r0 > 1e-3 {
                let nm = normalize3([n0[0] + n1[0], n0[1] + n1[1], n0[2] + n1[2]]);
                self.push_smooth_tri(q0, q1, p1, n0, n1, nm, m);
            }

            if r0 > 1e-3 {
                self.push_raw_tri(p0, q1, q0, [-axis[0], -axis[1], -axis[2]], m);
            }
            if r1 > 1e-3 {
                self.push_raw_tri(p1, q3, q2, axis, m);
            }
        }
    }

    pub(crate) fn cyl_y(&mut self, center: [f32; 3], r_top: f32, r_bot: f32, height: f32, segs: usize, m: Mat) {
        let p0 = [center[0], center[1] - height * 0.5, center[2]];
        let p1 = [center[0], center[1] + height * 0.5, center[2]];
        self.cyl_between(p0, p1, r_bot, r_top, segs, m);
    }

    pub(crate) fn cyl_x(&mut self, center: [f32; 3], r_neg: f32, r_pos: f32, length: f32, segs: usize, m: Mat) {
        let p0 = [center[0] - length * 0.5, center[1], center[2]];
        let p1 = [center[0] + length * 0.5, center[1], center[2]];
        self.cyl_between(p0, p1, r_neg, r_pos, segs, m);
    }

    pub(crate) fn cyl_z(&mut self, center: [f32; 3], r_neg: f32, r_pos: f32, length: f32, segs: usize, m: Mat) {
        let p0 = [center[0], center[1], center[2] - length * 0.5];
        let p1 = [center[0], center[1], center[2] + length * 0.5];
        self.cyl_between(p0, p1, r_neg, r_pos, segs, m);
    }

    /// Closed annular ring centered at `[center[0], center[1], center[2]]` around the Y axis,
    /// with vertical inner wall at `r_in` and outer wall tapering from `r_out_bot` to `r_out_top`.
    pub(crate) fn annular_ring_y(
        &mut self,
        center: [f32; 3],
        r_in: f32,
        r_out_bot: f32,
        r_out_top: f32,
        y_bot: f32,
        y_top: f32,
        segs: usize,
        m: Mat,
    ) {
        let segs = segs.max(12);
        let [cx, cy, cz] = center;
        let yb = cy + y_bot;
        let yt = cy + y_top;
        let h = (y_top - y_bot).max(1e-4);
        let slope_dr = r_out_bot - r_out_top;
        for i in 0..segs {
            let a0 = (i as f32 / segs as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / segs as f32) * std::f32::consts::TAU;
            let (s0, c0) = a0.sin_cos();
            let (s1, c1) = a1.sin_cos();

            let ib0 = [cx + r_in * c0, yb, cz + r_in * s0];
            let ib1 = [cx + r_in * c1, yb, cz + r_in * s1];
            let it0 = [cx + r_in * c0, yt, cz + r_in * s0];
            let it1 = [cx + r_in * c1, yt, cz + r_in * s1];

            let ob0 = [cx + r_out_bot * c0, yb, cz + r_out_bot * s0];
            let ob1 = [cx + r_out_bot * c1, yb, cz + r_out_bot * s1];
            let ot0 = [cx + r_out_top * c0, yt, cz + r_out_top * s0];
            let ot1 = [cx + r_out_top * c1, yt, cz + r_out_top * s1];

            let ni0 = [-c0, 0.0, -s0];
            let ni1 = [-c1, 0.0, -s1];
            self.push_smooth_quad(ib0, it0, it1, ib1, ni0, ni0, ni1, ni1, m);
            let no0 = normalize3([c0 * h, slope_dr, s0 * h]);
            let no1 = normalize3([c1 * h, slope_dr, s1 * h]);
            self.push_smooth_quad(ob0, ob1, ot1, ot0, no0, no1, no1, no0, m);
            self.quad_oriented(it0, ot0, ot1, it1, [0.0, 1.0, 0.0], m);
            self.quad_oriented(ib0, ib1, ob1, ob0, [0.0, -1.0, 0.0], m);
        }
    }

    /// Horizontal torus lying in the XZ plane around `center`.
    pub(crate) fn torus_y(
        &mut self,
        center: [f32; 3],
        major_r: f32,
        minor_r: f32,
        major_segs: usize,
        minor_segs: usize,
        m: Mat,
    ) {
        let major_segs = major_segs.max(16);
        let minor_segs = minor_segs.max(8);
        let [cx, cy, cz] = center;
        let pt = |u: f32, v: f32| -> ([f32; 3], [f32; 3]) {
            let (su, cu) = u.sin_cos();
            let (sv, cv) = v.sin_cos();
            let rr = major_r + minor_r * cv;
            (
                [cx + rr * cu, cy + minor_r * sv, cz + rr * su],
                normalize3([cv * cu, sv, cv * su]),
            )
        };
        for i in 0..major_segs {
            let u0 = (i as f32 / major_segs as f32) * std::f32::consts::TAU;
            let u1 = ((i + 1) as f32 / major_segs as f32) * std::f32::consts::TAU;
            for j in 0..minor_segs {
                let v0 = (j as f32 / minor_segs as f32) * std::f32::consts::TAU;
                let v1 = ((j + 1) as f32 / minor_segs as f32) * std::f32::consts::TAU;
                let (p00, n00) = pt(u0, v0);
                let (p10, n10) = pt(u1, v0);
                let (p11, n11) = pt(u1, v1);
                let (p01, n01) = pt(u0, v1);
                self.push_smooth_quad(p00, p01, p11, p10, n00, n01, n11, n10, m);
            }
        }
    }

    fn cone_y(&mut self, base_center: [f32; 3], radius: f32, height: f32, segs: usize, yaw_offset: f32, m: Mat) {
        let segs = segs.max(8);
        let apex = [base_center[0], base_center[1] + height, base_center[2]];
        for i in 0..segs {
            let a0 = yaw_offset + (i as f32 / segs as f32) * std::f32::consts::TAU;
            let a1 = yaw_offset + ((i + 1) as f32 / segs as f32) * std::f32::consts::TAU;
            let p0 = [base_center[0] + a0.cos() * radius, base_center[1], base_center[2] + a0.sin() * radius];
            let p1 = [base_center[0] + a1.cos() * radius, base_center[1], base_center[2] + a1.sin() * radius];
            let am = 0.5 * (a0 + a1);
            let n0 = normalize3([a0.cos() * height, radius, a0.sin() * height]);
            let n1 = normalize3([a1.cos() * height, radius, a1.sin() * height]);
            let nm = normalize3([am.cos() * height, radius, am.sin() * height]);
            self.push_smooth_tri(p0, p1, apex, n0, n1, nm, m);
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

    fn frustum_rect_y(
        &mut self,
        center: [f32; 3],
        top_w: f32,
        top_d: f32,
        bot_w: f32,
        bot_d: f32,
        height: f32,
        m: Mat,
    ) {
        let [cx, cy, cz] = center;
        let hy = height * 0.5;
        let (htw, htd) = (top_w * 0.5, top_d * 0.5);
        let (hbw, hbd) = (bot_w * 0.5, bot_d * 0.5);
        let y_top = cy + hy;
        let y_bot = cy - hy;
        let t0 = [cx - htw, y_top, cz + htd];
        let t1 = [cx + htw, y_top, cz + htd];
        let t2 = [cx + htw, y_top, cz - htd];
        let t3 = [cx - htw, y_top, cz - htd];
        let b0 = [cx - hbw, y_bot, cz + hbd];
        let b1 = [cx + hbw, y_bot, cz + hbd];
        let b2 = [cx + hbw, y_bot, cz - hbd];
        let b3 = [cx - hbw, y_bot, cz - hbd];
        self.quad_oriented(t0, t1, t2, t3, [0.0, 1.0, 0.0], m);
        self.quad_oriented(b3, b2, b1, b0, [0.0, -1.0, 0.0], m);
        self.quad_oriented(b0, b1, t1, t0, [0.0, -0.3, 1.0], m);
        self.quad_oriented(b1, b2, t2, t1, [1.0, -0.3, 0.0], m);
        self.quad_oriented(b2, b3, t3, t2, [0.0, -0.3, -1.0], m);
        self.quad_oriented(b3, b0, t0, t3, [-1.0, -0.3, 0.0], m);
    }

    fn sphere(&mut self, center: [f32; 3], radius: f32, stacks: usize, slices: usize, m: Mat) {
        self.ellipsoid_perturbed(center, [radius, radius, radius], stacks.max(8), slices.max(12), 0.0, 0, m);
    }

    fn hemi_cap_x(&mut self, center: [f32; 3], radius: f32, depth_x: f32, dir_x: f32, stacks: usize, slices: usize, m: Mat) {
        let stacks = stacks.max(6);
        let slices = slices.max(18);
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
                let n00 = normalize3([dir_x * t0.sin() * radius / depth_x.max(1.0), t0.cos() * a0.cos(), t0.cos() * a0.sin()]);
                let n10 = normalize3([dir_x * t0.sin() * radius / depth_x.max(1.0), t0.cos() * a1.cos(), t0.cos() * a1.sin()]);
                let n11 = normalize3([dir_x * t1.sin() * radius / depth_x.max(1.0), t1.cos() * a1.cos(), t1.cos() * a1.sin()]);
                let n01 = normalize3([dir_x * t1.sin() * radius / depth_x.max(1.0), t1.cos() * a0.cos(), t1.cos() * a0.sin()]);
                if st + 1 < stacks {
                    if dir_x > 0.0 {
                        self.push_smooth_quad(p0, p3, p2, p1, n00, n01, n11, n10, m);
                    } else {
                        self.push_smooth_quad(p0, p1, p2, p3, n00, n10, n11, n01, m);
                    }
                } else if dir_x > 0.0 {
                    self.push_smooth_tri(p0, p2, p1, n00, n11, n10, m);
                } else {
                    self.push_smooth_tri(p0, p1, p2, n00, n10, n11, m);
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
        let stacks = stacks.max(6);
        let slices = slices.max(10);
        let pt_at = |st: usize, sl: usize| -> ([f32; 3], [f32; 3]) {
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
                normalize3([
                    dir[0] / radii[0].max(0.1),
                    dir[1] / radii[1].max(0.1),
                    dir[2] / radii[2].max(0.1),
                ]),
            )
        };
        for st in 0..stacks {
            for sl in 0..slices {
                let (p0, n0) = pt_at(st, sl);
                let (p1, n1) = pt_at(st, sl + 1);
                let (p2, n2) = pt_at(st + 1, sl + 1);
                let (p3, n3) = pt_at(st + 1, sl);
                let shade = (0.86 + 0.16 * (0.5 * (n0[1] + n2[1]) * 0.5 + 0.5)).clamp(0.74, 1.06);
                let cm = m.with_col([
                    (m.col[0] * shade).min(1.0),
                    (m.col[1] * shade).min(1.0),
                    (m.col[2] * shade).min(1.0),
                ]);
                if st == 0 {
                    self.push_smooth_tri(p0, p3, p2, n0, n3, n2, cm);
                } else if st + 1 == stacks {
                    self.push_smooth_tri(p0, p3, p1, n0, n3, n1, cm);
                } else {
                    self.push_smooth_quad(p0, p3, p2, p1, n0, n3, n2, n1, cm);
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

    pub(crate) fn gable_roof_x(
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

    pub(crate) fn hip_roof(
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
        // Non-overlapping depth layers from wall_z outward (+side) so frame, glass & mullions never z-fight
        self.rbox([x, y, wall_z + side * 0.45], [win_w + 2.2, win_h + 2.2, 0.8], frame_m);
        self.rbox([x, y, wall_z + side * 1.05], [win_w, win_h, 0.36], glass_m);
        self.rbox([x, y, wall_z + side * 1.42], [0.9, win_h, 0.32], frame_m);
        self.rbox([x, y + win_h * 0.14, wall_z + side * 1.42], [win_w, 0.9, 0.32], frame_m);
        self.rbox([x, y - win_h * 0.5 - 1.1, wall_z + side * 1.15], [win_w + 4.0, 1.6, 2.2], sill_m);
        self.rbox([x, y + win_h * 0.5 + 1.3, wall_z + side * 0.85], [win_w + 3.2, 2.2, 1.5], lintel_m);
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
        self.rbox([wall_x + side * 0.45, y, z], [0.8, win_h + 2.2, win_d + 2.2], frame_m);
        self.rbox([wall_x + side * 1.05, y, z], [0.36, win_h, win_d], glass_m);
        self.rbox([wall_x + side * 1.42, y, z], [0.32, win_h, 0.9], frame_m);
        self.rbox([wall_x + side * 1.42, y + win_h * 0.14, z], [0.32, 0.9, win_d], frame_m);
        self.rbox([wall_x + side * 1.15, y - win_h * 0.5 - 1.1, z], [2.2, 1.6, win_d + 4.0], sill_m);
        self.rbox([wall_x + side * 0.85, y + win_h * 0.5 + 1.3, z], [1.5, 2.2, win_d + 3.2], lintel_m);
    }

    fn clock_face_z(&mut self, center: [f32; 3], side: f32, radius: f32, rim_m: Mat, dial_m: Mat, hand_m: Mat) {
        self.cyl_z([center[0], center[1], center[2] + side * 0.45], radius, radius, 0.8, 24, rim_m);
        self.cyl_z([center[0], center[1], center[2] + side * 1.05], radius * 0.84, radius * 0.84, 0.35, 24, dial_m);
        let zf = center[2] + side * 1.40;
        self.rbox([center[0] + radius * 0.26, center[1], zf], [radius * 0.52, radius * 0.13, 0.3], hand_m);
        self.rbox([center[0], center[1] + radius * 0.34, zf], [radius * 0.11, radius * 0.68, 0.3], hand_m);
    }

    fn clock_face_x(&mut self, center: [f32; 3], side: f32, radius: f32, rim_m: Mat, dial_m: Mat, hand_m: Mat) {
        self.cyl_x([center[0] + side * 0.45, center[1], center[2]], radius, radius, 0.8, 24, rim_m);
        self.cyl_x([center[0] + side * 1.05, center[1], center[2]], radius * 0.84, radius * 0.84, 0.35, 24, dial_m);
        let xf = center[0] + side * 1.40;
        self.rbox([xf, center[1], center[2] + radius * 0.26], [0.3, radius * 0.13, radius * 0.52], hand_m);
        self.rbox([xf, center[1] + radius * 0.34, center[2]], [0.3, radius * 0.68, radius * 0.11], hand_m);
    }
}

pub(crate) fn push_env_pine_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, x: f32, floor_y: f32, z: f32, h: f32) {
    let w = h * 0.34;
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
        Some(
            id @ ("loco-br243"
            | "loco-br96"
            | "loco-v60"
            | "wagon-passenger"
            | "wagon-freight"
            | "car-sedan"
            | "car-truck"
            | "car-bus"),
        ) => push_vehicle_3d(verts, idxs, id, origin, rot_rad, 0.0, [w, h, d], 0.0, [0.68, 0.14, 0.16], true, false),
        _ => build_generic_building(verts, idxs, origin, rot_rad, [w, h, d]),
    }
}

mod railway_buildings;
#[allow(unused_imports)]
use railway_buildings::*;
mod service_buildings;
#[allow(unused_imports)]
use service_buildings::*;
mod signals_and_scenery;
#[allow(unused_imports)]
use signals_and_scenery::*;
mod vehicles;
#[allow(unused_imports)]
pub(crate) use vehicles::push_vehicle_3d;
#[allow(unused_imports)]
pub(crate) use signals_and_scenery::{build_light_signal, build_turnout_accessories_3d, build_buffer_stop_with_signal};
#[cfg(test)]
mod tests;
