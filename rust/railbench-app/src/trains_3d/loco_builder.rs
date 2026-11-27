pub(crate) use crate::models3d_mesh::{Mat, PrimBuilder};
use crate::viewer3d::{normalize3, Vertex3D};

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
    yaw_sc: (f32, f32),
    pitch_sc: Option<(f32, f32)>,
    sub_pivot: Option<(f32, f32, f32)>, // (pivot_x, sin(sub_yaw), cos(sub_yaw))
}

impl<'a> LocoBuilder<'a> {
    pub(crate) fn new(verts: &'a mut Vec<Vertex3D>, idxs: &'a mut Vec<u32>, pose: TrainPose) -> Self {
        Self {
            verts,
            idxs,
            pose,
            yaw_sc: pose.yaw_rad.sin_cos(),
            pitch_sc: (pose.pitch_rad.abs() > 1e-6).then(|| pose.pitch_rad.sin_cos()),
            sub_pivot: None,
        }
    }

    pub(crate) fn with_sub_pivot<F: FnOnce(&mut LocoBuilder<'_>)>(
        &mut self,
        pivot_x: f32,
        sub_yaw_rad: f32,
        f: F,
    ) {
        let (ss, sc) = sub_yaw_rad.sin_cos();
        let mut child = LocoBuilder {
            verts: self.verts,
            idxs: self.idxs,
            pose: self.pose,
            yaw_sc: self.yaw_sc,
            pitch_sc: self.pitch_sc,
            sub_pivot: Some((pivot_x, ss, sc)),
        };
        f(&mut child);
    }
}

impl PrimBuilder for LocoBuilder<'_> {
    #[inline]
    fn xform_pos(&self, p: [f32; 3]) -> [f32; 3] {
        let [mut lx, mut ly, mut lz] = p;
        if let Some((px, ss, sc)) = self.sub_pivot {
            let dx = lx - px;
            lx = px + (dx * sc - lz * ss);
            lz = dx * ss + lz * sc;
        }
        if let Some((sp, cp)) = self.pitch_sc {
            let nx = lx * cp - ly * sp;
            let ny = lx * sp + ly * cp;
            lx = nx;
            ly = ny;
        }
        let sx = lx * self.pose.scale;
        let sy = ly * self.pose.scale;
        let sz = lz * self.pose.scale;
        let (ys, yc) = self.yaw_sc;
        [
            self.pose.pos[0] + (sx * yc - sz * ys),
            self.pose.pos[1] + sy,
            self.pose.pos[2] + (sx * ys + sz * yc),
        ]
    }

    #[inline]
    fn xform_norm(&self, n: [f32; 3]) -> [f32; 3] {
        let [mut nx, mut ny, mut nz] = n;
        if let Some((_, ss, sc)) = self.sub_pivot {
            let rnx = nx * sc - nz * ss;
            let rnz = nx * ss + nz * sc;
            nx = rnx;
            nz = rnz;
        }
        if let Some((sp, cp)) = self.pitch_sc {
            let px = nx * cp - ny * sp;
            let py = nx * sp + ny * cp;
            nx = px;
            ny = py;
        }
        let (ys, yc) = self.yaw_sc;
        normalize3([nx * yc - nz * ys, ny, nx * ys + nz * yc])
    }

    #[inline]
    fn mesh_buffers(&mut self) -> (&mut Vec<Vertex3D>, &mut Vec<u32>) {
        (self.verts, self.idxs)
    }
}

impl<'a> LocoBuilder<'a> {
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
            let (st0, ct0) = t0.sin_cos();
            let (st1, ct1) = t1.sin_cos();
            let (r0, y0) = (ct0 * radius, st0 * height_y);
            let (r1, y1) = (ct1 * radius, st1 * height_y);
            for sl in 0..slices {
                let a0 = (sl as f32 / slices as f32) * std::f32::consts::TAU;
                let a1 = ((sl + 1) as f32 / slices as f32) * std::f32::consts::TAU;
                let (sa0, ca0) = a0.sin_cos();
                let (sa1, ca1) = a1.sin_cos();
                let n0 = normalize3([ca0 * ct0, st0, sa0 * ct0]);
                let n1 = normalize3([ca1 * ct0, st0, sa1 * ct0]);
                let n2 = normalize3([ca1 * ct1, st1, sa1 * ct1]);
                let n3 = normalize3([ca0 * ct1, st1, sa0 * ct1]);
                let p0 = [center[0] + ca0 * r0, center[1] + y0, center[2] + sa0 * r0];
                let p1 = [center[0] + ca1 * r0, center[1] + y0, center[2] + sa1 * r0];
                let p2 = [center[0] + ca1 * r1, center[1] + y1, center[2] + sa1 * r1];
                let p3 = [center[0] + ca0 * r1, center[1] + y1, center[2] + sa0 * r1];
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
            let (st0, ct0) = t0.sin_cos();
            let (st1, ct1) = t1.sin_cos();
            let (r0, x0) = (ct0 * radius, st0 * depth_x * dir_x);
            let (r1, x1) = (ct1 * radius, st1 * depth_x * dir_x);
            for sl in 0..slices {
                let a0 = (sl as f32 / slices as f32) * std::f32::consts::TAU;
                let a1 = ((sl + 1) as f32 / slices as f32) * std::f32::consts::TAU;
                let (sa0, ca0) = a0.sin_cos();
                let (sa1, ca1) = a1.sin_cos();
                let n0 = normalize3([dir_x * st0, ca0 * ct0, sa0 * ct0]);
                let n1 = normalize3([dir_x * st0, ca1 * ct0, sa1 * ct0]);
                let n2 = normalize3([dir_x * st1, ca1 * ct1, sa1 * ct1]);
                let n3 = normalize3([dir_x * st1, ca0 * ct1, sa0 * ct1]);
                let p0 = [center[0] + x0, center[1] + ca0 * r0, center[2] + sa0 * r0];
                let p1 = [center[0] + x0, center[1] + ca1 * r0, center[2] + sa1 * r0];
                let p2 = [center[0] + x1, center[1] + ca1 * r1, center[2] + sa1 * r1];
                let p3 = [center[0] + x1, center[1] + ca0 * r1, center[2] + sa0 * r1];
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
            let (sp0, cp0) = p0_ang.sin_cos();
            let (sp1, cp1) = p1_ang.sin_cos();
            for sl in 0..slices {
                let t0 = (sl as f32 / slices as f32) * std::f32::consts::TAU;
                let t1 = ((sl + 1) as f32 / slices as f32) * std::f32::consts::TAU;
                let (st0, ct0) = t0.sin_cos();
                let (st1, ct1) = t1.sin_cos();
                let mk = |sp: f32, cp: f32, st: f32, ct: f32| -> ([f32; 3], [f32; 3]) {
                    let dir = [sp * ct, cp, sp * st];
                    (
                        [center[0] + dir[0] * radius, center[1] + dir[1] * radius, center[2] + dir[2] * radius],
                        dir,
                    )
                };
                let (v0, n0) = mk(sp0, cp0, st0, ct0);
                let (v1, n1) = mk(sp0, cp0, st1, ct1);
                let (v2, n2) = mk(sp1, cp1, st1, ct1);
                let (v3, n3) = mk(sp1, cp1, st0, ct0);
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
            self.cyl_z([x, cy, wz], r, r, 1.75, 20, tire_m);
            self.cyl_z([x, cy, flange_z], r + 0.85, r + 0.85, 0.55, 20, tire_m);
            self.cyl_z([x, cy, wz], r - 0.55, r - 0.55, 1.85, 18, spoke_m);
            self.cyl_z([x, cy, wz], r * 0.72, r * 0.72, 1.65, 18, axle_m);
            self.cyl_z([x, cy, wz], 1.75, 1.75, 2.05, 12, spoke_m);

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
            let puff_m = Mat::matte(0xeb_f0_f5_u32, 0.96);
            self.sphere([px, py, pz], r, 6, 8, puff_m);
        }
    }
}
