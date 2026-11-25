pub mod br243;
pub mod br96;
pub mod hud;
pub mod loco_builder;
pub mod track_graph;
pub mod v60;

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui;
use railbench_core::types::{Catalog, Layout};

use crate::i18n::Lang;
use crate::viewer3d::{Camera3D, SceneMesh3D, Vertex3D};

use self::br243::push_br243_3d;
use self::br96::push_br96_3d;
use self::loco_builder::{LocoAnimState, TrainPose};
use self::track_graph::build_rail_network;
use self::v60::push_v60_3d;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainLocoKind {
    Br243,
    Br96Mallet,
    V60Diesel,
}

impl TrainLocoKind {
    pub const ALL: &'static [TrainLocoKind] = &[
        TrainLocoKind::Br243,
        TrainLocoKind::Br96Mallet,
        TrainLocoKind::V60Diesel,
    ];

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Br243 => "BR 243 (E-Lok)",
            Self::Br96Mallet => "BR 96 (Mallet)",
            Self::V60Diesel => "V 60 (Diesel)",
        }
    }

    pub fn max_scale_kmh(self) -> f32 {
        match self {
            Self::Br243 => 120.0,
            Self::Br96Mallet => 50.0,
            Self::V60Diesel => 60.0,
        }
    }

    /// Wheel tread radius in H0 mm (`r = 7.2 mm` BR 243/V 60, `7.0 mm` BR 96).
    pub fn wheel_radius_h0(self) -> f32 {
        match self {
            Self::Br96Mallet => 7.0,
            Self::Br243 | Self::V60Diesel => 7.2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainCameraMode {
    Free,
    Follow,
    Cab,
}

impl TrainCameraMode {
    pub fn next(self) -> Self {
        match self {
            Self::Free => Self::Follow,
            Self::Follow => Self::Cab,
            Self::Cab => Self::Free,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveTrain3D {
    pub kind: TrainLocoKind,
    pub seg_idx: usize,
    pub dist_s: f32,
    pub forward_on_seg: bool,
    pub throttle: f32,
    pub wheel_angle: f32,
    pub headlights_on: bool,
    pub pantograph_up: bool,
    pub steam_on: bool,
    pub steam_time: f32,
}

impl Default for ActiveTrain3D {
    fn default() -> Self {
        Self {
            kind: TrainLocoKind::Br243,
            seg_idx: 0,
            dist_s: 90.0,
            forward_on_seg: true,
            throttle: 0.45,
            wheel_angle: 0.0,
            headlights_on: true,
            pantograph_up: true,
            steam_on: true,
            steam_time: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Train3DState {
    pub active: bool,
    pub trains: Vec<ActiveTrain3D>,
    pub selected_idx: usize,
    pub camera_mode: TrainCameraMode,
    pub turnout_branches: HashMap<String, usize>,
    pub turnout_branches_version: u64,
    pub mesh_version: u64,
}

impl Default for Train3DState {
    fn default() -> Self {
        Self {
            active: false,
            trains: vec![ActiveTrain3D::default()],
            selected_idx: 0,
            camera_mode: TrainCameraMode::Free,
            turnout_branches: HashMap::new(),
            turnout_branches_version: 0,
            mesh_version: 1,
        }
    }
}

pub fn scale_factor_for_layout(scale: &str) -> f32 {
    match scale.trim().to_ascii_uppercase().as_str() {
        "N" => 9.0 / 16.5,
        "TT" => 12.0 / 16.5,
        "Z" => 6.5 / 16.5,
        "0" => 32.0 / 16.5,
        "1" | "G" => 45.0 / 16.5,
        _ => 1.0,
    }
}

impl Train3DState {
    pub fn enter_mode(&mut self, layout: &Layout, catalogs: &[Catalog], camera: &mut Camera3D) {
        self.active = true;
        let net = build_rail_network(layout, catalogs, &self.turnout_branches);
        if self.trains.is_empty() {
            self.trains.push(ActiveTrain3D::default());
        }
        if !net.segments.is_empty() {
            for (idx, tr) in self.trains.iter_mut().enumerate() {
                tr.seg_idx = tr.seg_idx.min(net.segments.len() - 1);
                if tr.dist_s > net.segments[tr.seg_idx].total_len || idx == 0 && tr.dist_s <= 1.0 {
                    tr.dist_s = net.segments[tr.seg_idx].total_len * 0.5;
                }
            }
            if let Some(tr) = self.trains.get(self.selected_idx) {
                let (pose, _) = net.compute_train_pose(
                    tr.seg_idx,
                    tr.dist_s,
                    tr.forward_on_seg,
                    scale_factor_for_layout(&layout.scale),
                );
                camera.target = [pose.pos[0], pose.pos[1] + 22.0 * pose.scale, pose.pos[2]];
                camera.distance = camera.distance.clamp(160.0, 340.0);
                camera.pitch = camera.pitch.clamp(0.18, 0.58);
            }
        }
        self.mesh_version = self.mesh_version.wrapping_add(1);
    }

    pub fn add_train(&mut self, layout: &Layout, catalogs: &[Catalog]) {
        let net = build_rail_network(layout, catalogs, &self.turnout_branches);
        let next_kind = match self.trains.len() % 3 {
            0 => TrainLocoKind::Br243,
            1 => TrainLocoKind::Br96Mallet,
            _ => TrainLocoKind::V60Diesel,
        };
        let seg_idx = if net.segments.is_empty() {
            0
        } else {
            (self.trains.len() * 2) % net.segments.len()
        };
        let dist_s = net
            .segments
            .get(seg_idx)
            .map_or(0.0, |s| s.total_len * 0.5);
        self.trains.push(ActiveTrain3D {
            kind: next_kind,
            seg_idx,
            dist_s,
            throttle: 0.35,
            ..Default::default()
        });
        self.selected_idx = self.trains.len() - 1;
        self.mesh_version = self.mesh_version.wrapping_add(1);
    }

    pub fn toggle_nearest_turnout(&mut self, layout: &Layout, catalogs: &[Catalog]) {
        let net = build_rail_network(layout, catalogs, &self.turnout_branches);
        if net.segments.is_empty() || net.turnouts.is_empty() {
            return;
        }
        let Some(tr) = self.trains.get(self.selected_idx) else {
            return;
        };
        let s_idx = tr.seg_idx.min(net.segments.len() - 1);
        let (pose, _) = net.compute_train_pose(
            s_idx,
            tr.dist_s,
            tr.forward_on_seg,
            scale_factor_for_layout(&layout.scale),
        );
        if let Some(tid) = net.nearest_turnout_id(pose.pos) {
            if let Some(tinfo) = net.turnouts.iter().find(|t| t.piece_id == tid) {
                let entry = self.turnout_branches.entry(tid).or_insert(0);
                *entry = (*entry + 1) % tinfo.num_branches.max(1);
                self.turnout_branches_version = self.turnout_branches_version.wrapping_add(1);
            }
        }
    }

    pub fn needs_repaint(&self) -> bool {
        self.active
            && self.trains.iter().any(|t| {
                t.throttle.abs() > 0.01
                    || (t.kind == TrainLocoKind::Br96Mallet && t.steam_on)
            })
    }

    pub fn update(&mut self, layout: &Layout, catalogs: &[Catalog], dt_secs: f32, camera: &mut Camera3D) {
        if !self.active {
            return;
        }
        let net = build_rail_network(layout, catalogs, &self.turnout_branches);
        if net.segments.is_empty() {
            return;
        }
        let lscale = scale_factor_for_layout(&layout.scale);
        let dt = dt_secs.clamp(0.0, 0.08);

        for tr in &mut self.trains {
            if tr.seg_idx >= net.segments.len() {
                tr.seg_idx = 0;
                tr.dist_s = net.segments[0].total_len * 0.5;
            }
            if tr.kind == TrainLocoKind::Br96Mallet && tr.steam_on {
                tr.steam_time = (tr.steam_time + dt).rem_euclid(1000.0);
            }
            if tr.throttle.abs() > 0.01 {
                let top_mm_per_s = 260.0 * (tr.kind.max_scale_kmh() / 120.0) * lscale;
                let ds = tr.throttle * top_mm_per_s * dt;
                let (ns, nd, nf, bounced) = net.advance_along(tr.seg_idx, tr.dist_s, tr.forward_on_seg, ds);
                tr.seg_idx = ns;
                tr.dist_s = nd;
                tr.forward_on_seg = nf;
                if bounced {
                    tr.throttle = -tr.throttle;
                }
                let r_wheel = tr.kind.wheel_radius_h0() * lscale.max(0.25);
                tr.wheel_angle = (tr.wheel_angle - ds / r_wheel).rem_euclid(std::f32::consts::TAU);
            }
        }

        if let Some(tr) = self.trains.get(self.selected_idx) {
            let s_idx = tr.seg_idx.min(net.segments.len() - 1);
            let (pose, _) = net.compute_train_pose(s_idx, tr.dist_s, tr.forward_on_seg, lscale);
            net.apply_camera_tracking(camera, pose, self.camera_mode);
        }
        self.mesh_version = self.mesh_version.wrapping_add(1);
    }

    pub fn build_mesh(&self, layout: &Layout, catalogs: &[Catalog]) -> Option<(u64, Arc<SceneMesh3D>)> {
        if !self.active || self.trains.is_empty() {
            return None;
        }
        let net = build_rail_network(layout, catalogs, &self.turnout_branches);
        if net.segments.is_empty() {
            return None;
        }
        let lscale = scale_factor_for_layout(&layout.scale);
        let mut verts = Vec::with_capacity(self.trains.len() * 4500);
        let mut idxs = Vec::with_capacity(self.trains.len() * 9000);

        for tr in &self.trains {
            let s_idx = tr.seg_idx.min(net.segments.len() - 1);
            let (pose, mallet_art) = net.compute_train_pose(s_idx, tr.dist_s, tr.forward_on_seg, lscale);
            let anim = LocoAnimState {
                wheel_angle: tr.wheel_angle,
                forward_lights: tr.throttle >= -0.01,
                headlights_on: tr.headlights_on,
                pantograph_up: tr.pantograph_up,
                steam_on: tr.steam_on,
                steam_time: tr.steam_time,
                speed_norm: tr.throttle.abs(),
                mallet_articulation_rad: mallet_art,
            };
            push_locomotive_3d(&mut verts, &mut idxs, tr.kind, pose, anim);
        }

        Some((self.mesh_version, Arc::new(SceneMesh3D::from_parts(verts, idxs))))
    }

    pub fn draw_hud(
        &mut self,
        ctx: &egui::Context,
        viewport_rect: egui::Rect,
        layout: &Layout,
        catalogs: &[Catalog],
        camera: &mut Camera3D,
        dark_mode: bool,
        lang: Lang,
    ) {
        hud::draw_train_hud(self, ctx, viewport_rect, layout, catalogs, camera, dark_mode, lang);
    }
}

pub fn push_locomotive_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    kind: TrainLocoKind,
    pose: TrainPose,
    anim: LocoAnimState,
) {
    match kind {
        TrainLocoKind::Br243 => push_br243_3d(verts, idxs, pose, anim),
        TrainLocoKind::Br96Mallet => push_br96_3d(verts, idxs, pose, anim),
        TrainLocoKind::V60Diesel => push_v60_3d(verts, idxs, pose, anim),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};
    use railbench_core::types::Transform;

    fn origin_pose() -> TrainPose {
        TrainPose {
            pos: [0.0, 0.0, 0.0],
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            scale: 1.0,
        }
    }

    fn default_anim() -> LocoAnimState {
        LocoAnimState {
            wheel_angle: 0.0,
            forward_lights: true,
            headlights_on: true,
            pantograph_up: true,
            steam_on: true,
            steam_time: 0.25,
            speed_norm: 0.5,
            mallet_articulation_rad: 0.0,
        }
    }

    #[test]
    fn all_3_locomotives_generate_detailed_valid_meshes() {
        for kind in TrainLocoKind::ALL {
            let mut verts = Vec::new();
            let mut idxs = Vec::new();
            push_locomotive_3d(&mut verts, &mut idxs, *kind, origin_pose(), default_anim());

            assert!(verts.len() >= 1000, "{kind:?} should be detailed, got {} verts", verts.len());
            assert!(idxs.len() >= 1500 && idxs.len() % 3 == 0, "{kind:?} invalid indices {}", idxs.len());
            let max_i = *idxs.iter().max().unwrap() as usize;
            assert!(max_i < verts.len(), "{kind:?} out-of-bounds index {max_i}");
            assert!(
                verts.iter().all(|v| v.pos.iter().all(|c| c.is_finite())),
                "{kind:?} contains non-finite vertex position"
            );
            // Must contain emissive headlights & night-glow cab windows
            assert!(verts.iter().any(|v| v.mat[2] > 20), "{kind:?} missing emissive lights");
            assert!(verts.iter().any(|v| v.mat[2] < -10), "{kind:?} missing warm cab windows");

            // Check realistic H0 bounding dimensions (length X, width Z, height Y)
            let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
            let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
            let (mut min_z, mut max_z) = (f32::INFINITY, f32::NEG_INFINITY);
            for v in &verts {
                min_x = min_x.min(v.pos[0]);
                max_x = max_x.max(v.pos[0]);
                min_y = min_y.min(v.pos[1]);
                max_y = max_y.max(v.pos[1]);
                min_z = min_z.min(v.pos[2]);
                max_z = max_z.max(v.pos[2]);
            }
            let len_x = max_x - min_x;
            let h_y = max_y - min_y;
            let w_z = max_z - min_z;
            match kind {
                TrainLocoKind::Br243 => {
                    assert!((185.0..=205.0).contains(&len_x), "BR243 len {len_x}");
                    assert!((60.0..=70.0).contains(&h_y), "BR243 height {h_y}");
                    assert!((32.0..=38.0).contains(&w_z), "BR243 width {w_z}");
                }
                TrainLocoKind::Br96Mallet => {
                    assert!((195.0..=215.0).contains(&len_x), "BR96 len {len_x}");
                    assert!((48.0..=82.0).contains(&h_y), "BR96 height {h_y}");
                    assert!((32.0..=38.0).contains(&w_z), "BR96 width {w_z}");
                }
                TrainLocoKind::V60Diesel => {
                    assert!((115.0..=130.0).contains(&len_x), "V60 len {len_x}");
                    assert!((46.0..=56.0).contains(&h_y), "V60 height {h_y}");
                    assert!((32.0..=38.0).contains(&w_z), "V60 width {w_z}");
                }
            }
        }
    }

    #[test]
    fn br243_pantograph_and_br96_mallet_articulation_respond_to_anim_state() {
        let mut v_up = Vec::new();
        let mut i_up = Vec::new();
        push_locomotive_3d(&mut v_up, &mut i_up, TrainLocoKind::Br243, origin_pose(), default_anim());
        let max_y_up = v_up.iter().map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);

        let mut anim_down = default_anim();
        anim_down.pantograph_up = false;
        let mut v_dn = Vec::new();
        let mut i_dn = Vec::new();
        push_locomotive_3d(&mut v_dn, &mut i_dn, TrainLocoKind::Br243, origin_pose(), anim_down);
        let max_y_dn = v_dn.iter().map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);
        assert!(max_y_up > max_y_dn + 10.0, "raised pantograph {max_y_up} vs lowered {max_y_dn}");

        // BR 96 Mallet articulation bends forward chassis away from Z = 0 at X > 60 mm
        let mut anim_straight = default_anim();
        anim_straight.steam_on = false;
        let mut vs = Vec::new();
        let mut is = Vec::new();
        push_locomotive_3d(&mut vs, &mut is, TrainLocoKind::Br96Mallet, origin_pose(), anim_straight);
        let mean_z_straight = vs.iter().filter(|v| v.pos[0] > 70.0).map(|v| v.pos[2]).sum::<f32>()
            / vs.iter().filter(|v| v.pos[0] > 70.0).count().max(1) as f32;

        let mut anim_bent = anim_straight;
        anim_bent.mallet_articulation_rad = 0.25;
        let mut vb = Vec::new();
        let mut ib = Vec::new();
        push_locomotive_3d(&mut vb, &mut ib, TrainLocoKind::Br96Mallet, origin_pose(), anim_bent);
        let mean_z_bent = vb.iter().filter(|v| v.pos[0] > 70.0).map(|v| v.pos[2]).sum::<f32>()
            / vb.iter().filter(|v| v.pos[0] > 70.0).count().max(1) as f32;
        assert!(
            (mean_z_bent - mean_z_straight).abs() > 4.0,
            "Mallet front unit should pivot laterally on curves: straight={mean_z_straight}, bent={mean_z_bent}"
        );
    }

    #[test]
    fn train_traverses_connected_tracks_and_bounces_at_dead_end() {
        let catalogs = builtin_catalogs();
        let e_str = find_entry(&catalogs, "24188").unwrap();
        let p0 = create_piece("p0".into(), e_str, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        let p1 = create_piece("p1".into(), e_str, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
        let layout = Layout {
            name: "TrainRun".into(),
            scale: "H0".into(),
            pieces: vec![p0, p1],
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
        };

        let mut st = Train3DState::default();
        st.active = true;
        st.trains[0].seg_idx = 0;
        st.trains[0].dist_s = 100.0;
        st.trains[0].throttle = 1.0;

        let mut cam = Camera3D::default();
        for _ in 0..20 {
            st.update(&layout, &catalogs, 0.05, &mut cam);
        }
        assert_eq!(st.trains[0].seg_idx, 1, "train should cross into connected second track");
        assert!(st.trains[0].wheel_angle.abs() > 0.1, "wheel_angle must advance when moving");

        // Drive past end -> reverses cleanly
        for _ in 0..40 {
            st.update(&layout, &catalogs, 0.05, &mut cam);
        }
        assert!(st.trains[0].throttle < 0.0, "train should auto-reverse at dead end");
    }
}
