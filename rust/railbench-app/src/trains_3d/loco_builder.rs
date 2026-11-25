use crate::viewer3d::{normalize3, rot_xz, Vertex3D};

/// Per-vertex PBR-like material descriptor matching `viewer3d::SCENE_SHADER_WGSL`.
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
            metal: 0.03,
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

    /// Daytime dark reflective glass that glows warmly when dusk/night falls.
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

    #[inline]
    fn pack(self) -> [f32; 3] {
        [self.rough, self.metal, self.emissive]
    }
}

pub(crate) const fn hex(rgb: u32) -> [f32; 3] {
    let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
    let b = (rgb & 0xFF) as f32 / 255.0;
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

#[derive(Debug, Clone, Copy)]
pub struct TrainPose {
    pub pos: [f32; 3],
    pub yaw_rad: f32,
    pub pitch_rad: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct LocoAnimState {
    pub wheel_angle: f32,
    pub forward_lights: bool,
    pub headlights_on: bool,
    pub pantograph_up: bool,
    pub steam_on: bool,
    pub steam_time: f32,
    pub speed_norm: f32,
    pub mallet_articulation_rad: f32,
}

pub(crate) struct LocoBuilder<'a> {
    verts: &'a mut Vec<Vertex3D>,
    idxs: &'a mut Vec<u32>,
    pose: TrainPose,
    sub_pivot: Option<(f32, f32)>,
}

impl<'a> LocoBuilder<'a> {
    pub(crate) fn new(verts: &'a mut Vec<Vertex3D>, idxs: &'a mut Vec<u32>, pose: TrainPose) -> Self {
        Self {
            verts,
            idxs,
            pose,
            sub_pivot: None,
        }
    }

    pub(crate) fn with_sub_pivot<F: FnOnce(&mut LocoBuilder<'_>)>(
        &mut self,
        pivot_x: f32,
        sub_yaw_rad: f32,
        f: F,
    ) {
        let mut child = LocoBuilder {
            verts: self.verts,
            idxs: self.idxs,
            pose: self.pose,
            sub_pivot: Some((pivot_x, sub_yaw_rad)),
        };
        f(&mut child);
    }

    #[inline]
    fn xform_pos(&self, p: [f32; 3]) -> [f32; 3] {
        let [mut lx, mut ly, mut lz] = p;
        if let Some((px, sub_yaw)) = self.sub_pivot {
            let (rx, rz) = rot_xz(lx - px, lz, sub_yaw);
            lx = px + rx;
            lz = rz;
        }
        // Apply longitudinal track grade pitch around lateral Z
        if self.pose.pitch_rad.abs() > 1e-6 {
            let (sp, cp) = self.pose.pitch_rad.sin_cos();
            let nx = lx * cp - ly * sp;
            let ny = lx * sp + ly * cp;
            lx = nx;
            ly = ny;
        }
        let sx = lx * self.pose.scale;
        let sy = ly * self.pose.scale;
        let sz = lz * self.pose.scale;
        let (rx, rz) = rot_xz(sx, sz, self.pose.yaw_rad);
        [
            self.pose.pos[0] + rx,
            self.pose.pos[1] + sy,
            self.pose.pos[2] + rz,
        ]
    }

    #[inline]
    fn xform_norm(&self, n: [f32; 3]) -> [f32; 3] {
        let [mut nx, mut ny, mut nz] = n;
        if let Some((_, sub_yaw)) = self.sub_pivot {
            let (rx, rz) = rot_xz(nx, nz, sub_yaw);
            nx = rx;
            nz = rz;
        }
        if self.pose.pitch_rad.abs() > 1e-6 {
            let (sp, cp) = self.pose.pitch_rad.sin_cos();
            let px = nx * cp - ny * sp;
            let py = nx * sp + ny * cp;
            nx = px;
            ny = py;
        }
        let (rx, rz) = rot_xz(nx, nz, self.pose.yaw_rad);
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

        self.push_raw_quad(pt(-hx, hy, -hz), pt(hx, hy, -hz), pt(hx, hy, hz), pt(-hx, hy, hz), nm(0.0, 1.0, 0.0), m);
        self.push_raw_quad(pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, -hy, -hz), pt(-hx, -hy, -hz), nm(0.0, -1.0, 0.0), m);
        self.push_raw_quad(pt(-hx, -hy, hz), pt(hx, -hy, hz), pt(hx, hy, hz), pt(-hx, hy, hz), nm(0.0, 0.0, 1.0), m);
        self.push_raw_quad(pt(hx, -hy, -hz), pt(-hx, -hy, -hz), pt(-hx, hy, -hz), pt(hx, hy, -hz), nm(0.0, 0.0, -1.0), m);
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
        self.cyl_between(
            [center[0], center[1] - height * 0.5, center[2]],
            [center[0], center[1] + height * 0.5, center[2]],
            r_bot,
            r_top,
            segs,
            m,
        );
    }

    pub(crate) fn cyl_x(&mut self, center: [f32; 3], r_neg: f32, r_pos: f32, length: f32, segs: usize, m: Mat) {
        self.cyl_between(
            [center[0] - length * 0.5, center[1], center[2]],
            [center[0] + length * 0.5, center[1], center[2]],
            r_neg,
            r_pos,
            segs,
            m,
        );
    }

    pub(crate) fn cyl_z(&mut self, center: [f32; 3], r_neg: f32, r_pos: f32, length: f32, segs: usize, m: Mat) {
        self.cyl_between(
            [center[0], center[1], center[2] - length * 0.5],
            [center[0], center[1], center[2] + length * 0.5],
            r_neg,
            r_pos,
            segs,
            m,
        );
    }

    pub(crate) fn dome_y(&mut self, base_center: [f32; 3], radius: f32, cyl_h: f32, cap_h: f32, segs: usize, m: Mat) {
        self.cyl_y(
            [base_center[0], base_center[1] + cyl_h * 0.5, base_center[2]],
            radius,
            radius,
            cyl_h,
            segs,
            m,
        );
        self.hemi_cap_y([base_center[0], base_center[1] + cyl_h, base_center[2]], radius, cap_h, 4, segs, m);
    }

    pub(crate) fn hemi_cap_y(&mut self, center: [f32; 3], radius: f32, height_y: f32, stacks: usize, slices: usize, m: Mat) {
        let stacks = stacks.max(3);
        let slices = slices.max(8);
        for st in 0..stacks {
            let t0 = (st as f32 / stacks as f32) * std::f32::consts::FRAC_PI_2;
            let t1 = ((st + 1) as f32 / stacks as f32) * std::f32::consts::FRAC_PI_2;
            let (r0, y0) = (t0.cos() * radius, t0.sin() * height_y);
            let (r1, y1) = (t1.cos() * radius, t1.sin() * height_y);
            for sl in 0..slices {
                let a0 = (sl as f32 / slices as f32) * std::f32::consts::TAU;
                let a1 = ((sl + 1) as f32 / slices as f32) * std::f32::consts::TAU;
                let n0 = normalize3([a0.cos() * t0.cos(), t0.sin(), a0.sin() * t0.cos()]);
                let n1 = normalize3([a1.cos() * t0.cos(), t0.sin(), a1.sin() * t0.cos()]);
                let n2 = normalize3([a1.cos() * t1.cos(), t1.sin(), a1.sin() * t1.cos()]);
                let n3 = normalize3([a0.cos() * t1.cos(), t1.sin(), a0.sin() * t1.cos()]);
                let p0 = [center[0] + a0.cos() * r0, center[1] + y0, center[2] + a0.sin() * r0];
                let p1 = [center[0] + a1.cos() * r0, center[1] + y0, center[2] + a1.sin() * r0];
                let p2 = [center[0] + a1.cos() * r1, center[1] + y1, center[2] + a1.sin() * r1];
                let p3 = [center[0] + a0.cos() * r1, center[1] + y1, center[2] + a0.sin() * r1];
                if st + 1 < stacks {
                    self.push_smooth_quad(p0, p1, p2, p3, n0, n1, n2, n3, m);
                } else {
                    self.push_smooth_tri(p0, p1, [center[0], center[1] + height_y, center[2]], n0, n1, [0.0, 1.0, 0.0], m);
                }
            }
        }
    }

    pub(crate) fn hemi_cap_x(&mut self, center: [f32; 3], radius: f32, depth_x: f32, dir_x: f32, stacks: usize, slices: usize, m: Mat) {
        let stacks = stacks.max(4);
        let slices = slices.max(10);
        for st in 0..stacks {
            let t0 = (st as f32 / stacks as f32) * std::f32::consts::FRAC_PI_2;
            let t1 = ((st + 1) as f32 / stacks as f32) * std::f32::consts::FRAC_PI_2;
            let (r0, x0) = (t0.cos() * radius, t0.sin() * depth_x * dir_x);
            let (r1, x1) = (t1.cos() * radius, t1.sin() * depth_x * dir_x);
            for sl in 0..slices {
                let a0 = (sl as f32 / slices as f32) * std::f32::consts::TAU;
                let a1 = ((sl + 1) as f32 / slices as f32) * std::f32::consts::TAU;
                let n0 = normalize3([dir_x * t0.sin(), a0.cos() * t0.cos(), a0.sin() * t0.cos()]);
                let n1 = normalize3([dir_x * t0.sin(), a1.cos() * t0.cos(), a1.sin() * t0.cos()]);
                let n2 = normalize3([dir_x * t1.sin(), a1.cos() * t1.cos(), a1.sin() * t1.cos()]);
                let n3 = normalize3([dir_x * t1.sin(), a0.cos() * t1.cos(), a0.sin() * t1.cos()]);
                let p0 = [center[0] + x0, center[1] + a0.cos() * r0, center[2] + a0.sin() * r0];
                let p1 = [center[0] + x0, center[1] + a1.cos() * r0, center[2] + a1.sin() * r0];
                let p2 = [center[0] + x1, center[1] + a1.cos() * r1, center[2] + a1.sin() * r1];
                let p3 = [center[0] + x1, center[1] + a0.cos() * r1, center[2] + a0.sin() * r1];
                if st + 1 < stacks {
                    self.push_smooth_quad(p0, p1, p2, p3, n0, n1, n2, n3, m);
                } else {
                    self.push_smooth_tri(p0, p1, [center[0] + dir_x * depth_x, center[1], center[2]], n0, n1, [dir_x, 0.0, 0.0], m);
                }
            }
        }
    }

    pub(crate) fn sphere(&mut self, center: [f32; 3], radius: f32, stacks: usize, slices: usize, m: Mat) {
        let stacks = stacks.max(4);
        let slices = slices.max(8);
        for st in 0..stacks {
            let p0_ang = (st as f32 / stacks as f32) * std::f32::consts::PI;
            let p1_ang = ((st + 1) as f32 / stacks as f32) * std::f32::consts::PI;
            for sl in 0..slices {
                let t0 = (sl as f32 / slices as f32) * std::f32::consts::TAU;
                let t1 = ((sl + 1) as f32 / slices as f32) * std::f32::consts::TAU;
                let pt = |phi: f32, th: f32| -> ([f32; 3], [f32; 3]) {
                    let dir = [phi.sin() * th.cos(), phi.cos(), phi.sin() * th.sin()];
                    (
                        [center[0] + dir[0] * radius, center[1] + dir[1] * radius, center[2] + dir[2] * radius],
                        dir,
                    )
                };
                let (v0, n0) = pt(p0_ang, t0);
                let (v1, n1) = pt(p0_ang, t1);
                let (v2, n2) = pt(p1_ang, t1);
                let (v3, n3) = pt(p1_ang, t0);
                if st == 0 {
                    self.push_smooth_tri(v0, v2, v3, n0, n2, n3, m);
                } else if st + 1 == stacks {
                    self.push_smooth_tri(v0, v1, v2, n0, n1, n2, m);
                } else {
                    self.push_smooth_quad(v0, v1, v2, v3, n0, n1, n2, n3, m);
                }
            }
        }
    }

    pub(crate) fn curved_pipe(&mut self, pts: &[[f32; 3]], radius: f32, segs: usize, m: Mat) {
        for w in pts.windows(2) {
            self.cyl_between(w[0], w[1], radius, radius, segs, m);
        }
    }

    /// Detailed H0 spoked driving/coupled wheelset at longitudinal `x`, tread radius `r`, gauge `±8.25 mm`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_spoked_wheelset(
        &mut self,
        x: f32,
        r: f32,
        wheel_angle: f32,
        num_spokes: usize,
        has_counterweight: bool,
        tire_m: Mat,
        spoke_m: Mat,
        axle_m: Mat,
    ) {
        let cy = r;
        let gauge_half = 8.25_f32;
        self.cyl_z([x, cy, 0.0], 1.15, 1.15, 16.0, 10, axle_m);

        for side in [-1.0_f32, 1.0] {
            let wz = side * gauge_half;
            let flange_z = side * (gauge_half - 1.05);
            // Outer steel running tire + inner flange sitting inside H0 railhead
            self.cyl_z([x, cy, wz], r, r, 1.75, 20, tire_m);
            self.cyl_z([x, cy, flange_z], r + 0.85, r + 0.85, 0.55, 20, tire_m);
            // Painted inner wheel rim & central hub boss
            self.cyl_z([x, cy, wz], r - 0.55, r - 0.55, 1.85, 18, spoke_m);
            // Dark recessed spoke cavity ring
            self.cyl_z([x, cy, wz], r * 0.72, r * 0.72, 1.65, 18, axle_m);
            self.cyl_z([x, cy, wz], 1.75, 1.75, 2.05, 12, spoke_m);

            // Rotating spokes + counterweight crescent + crankpin boss
            for k in 0..num_spokes {
                let a = wheel_angle + (k as f32 / num_spokes as f32) * std::f32::consts::TAU;
                let (sa, ca) = a.sin_cos();
                self.strut_between(
                    [x + ca * 1.4, cy + sa * 1.4, wz],
                    [x + ca * (r * 0.76), cy + sa * (r * 0.76), wz],
                    0.42,
                    1.82,
                    spoke_m,
                );
            }
            if has_counterweight {
                for off in [-0.36_f32, 0.0, 0.36] {
                    let a = wheel_angle + std::f32::consts::PI + off;
                    let (sa, ca) = a.sin_cos();
                    self.strut_between(
                        [x + ca * (r * 0.34), cy + sa * (r * 0.34), wz],
                        [x + ca * (r * 0.75), cy + sa * (r * 0.75), wz],
                        r * 0.28,
                        1.88,
                        spoke_m,
                    );
                }
            }
            // Crankpin boss protruding outward
            let (sa, ca) = wheel_angle.sin_cos();
            let cr = r * 0.54;
            self.cyl_z(
                [x + ca * cr, cy + sa * cr, wz + side * 0.75],
                0.85,
                0.85,
                1.2,
                10,
                tire_m,
            );
        }
    }

    /// Solid disc railway wheelset (used on electric locomotives like BR 243).
    pub(crate) fn push_solid_wheelset(
        &mut self,
        x: f32,
        r: f32,
        wheel_angle: f32,
        tire_m: Mat,
        disc_m: Mat,
        axle_m: Mat,
    ) {
        let cy = r;
        let gauge_half = 8.25_f32;
        self.cyl_z([x, cy, 0.0], 1.15, 1.15, 16.0, 10, axle_m);
        for side in [-1.0_f32, 1.0] {
            let wz = side * gauge_half;
            let flange_z = side * (gauge_half - 1.05);
            self.cyl_z([x, cy, wz], r, r, 1.75, 20, tire_m);
            self.cyl_z([x, cy, flange_z], r + 0.85, r + 0.85, 0.55, 20, tire_m);
            self.cyl_z([x, cy, wz], r - 0.65, r - 0.65, 1.85, 18, disc_m);
            self.cyl_z([x, cy, wz], 1.75, 1.75, 2.05, 12, tire_m);
            // Rotating inspection/balance bosses on wheel face so wheel rotation is clearly visible
            for k in 0..3 {
                let a = wheel_angle + (k as f32 / 3.0) * std::f32::consts::TAU;
                let (sa, ca) = a.sin_cos();
                self.cyl_z(
                    [x + ca * (r * 0.52), cy + sa * (r * 0.52), wz + side * 0.96],
                    0.55,
                    0.55,
                    0.35,
                    8,
                    tire_m,
                );
            }
        }
    }

    /// Twin sleeve buffers + central screw coupler + twin curved brake hoses on buffer beam at `beam_x`.
    pub(crate) fn push_buffer_beam_details(
        &mut self,
        beam_x: f32,
        buffer_y: f32,
        dir_x: f32,
        steel_m: Mat,
        hose_m: Mat,
    ) {
        for side in [-1.0_f32, 1.0] {
            let bz = side * 10.0;
            self.rbox([beam_x + dir_x * 0.5, buffer_y, bz], [0.8, 3.6, 3.6], steel_m);
            self.cyl_x([beam_x + dir_x * 2.1, buffer_y, bz], 1.15, 0.95, 2.6, 12, steel_m);
            self.cyl_x([beam_x + dir_x * 3.8, buffer_y, bz], 2.05, 2.05, 0.75, 14, steel_m);
            // Hanging rubber brake hoses inside the buffers
            let hz = side * 4.5;
            self.curved_pipe(
                &[
                    [beam_x + dir_x * 0.6, buffer_y - 1.2, hz],
                    [beam_x + dir_x * 2.1, buffer_y - 4.0, hz],
                    [beam_x + dir_x * 1.4, buffer_y - 6.8, hz * 0.85],
                ],
                0.42,
                8,
                hose_m,
            );
        }
        // Central draw hook + screw coupling link
        self.rbox([beam_x + dir_x * 1.4, buffer_y, 0.0], [2.4, 1.8, 1.4], steel_m);
        self.curved_pipe(
            &[
                [beam_x + dir_x * 2.4, buffer_y, 0.0],
                [beam_x + dir_x * 3.0, buffer_y - 2.6, 0.0],
                [beam_x + dir_x * 1.9, buffer_y - 4.4, 0.0],
            ],
            0.45,
            8,
            steel_m,
        );
    }

    pub(crate) fn push_lantern(&mut self, center: [f32; 3], dir_x: f32, r: f32, body_m: Mat, lens_m: Mat) {
        self.cyl_x(center, r, r, 2.0, 12, body_m);
        self.cyl_x([center[0] + dir_x * 1.0, center[1], center[2]], r * 0.76, r * 0.76, 0.4, 12, lens_m);
    }

    pub(crate) fn push_steam_puffs(&mut self, chimney_top: [f32; 3], steam_time: f32, speed_norm: f32) {
        let count = 6usize;
        for i in 0..count {
            let phase = (steam_time * (0.45 + 0.55 * speed_norm) + i as f32 / count as f32).fract();
            let px = chimney_top[0] - (8.0 + 34.0 * speed_norm) * phase * phase;
            let py = chimney_top[1] + 2.0 + 26.0 * phase * (1.0 - 0.18 * phase);
            let pz = (phase * 6.0 + i as f32).sin() * (1.2 + 2.5 * phase);
            let r = (1.6 + 5.2 * phase) * (1.0 - 0.35 * phase);
            let shade = (0xeb_f0_f5_u32).min(0xff_ff_ff);
            let puff_m = Mat::matte(shade, 0.96);
            self.sphere([px, py, pz], r, 6, 8, puff_m);
        }
    }
}
