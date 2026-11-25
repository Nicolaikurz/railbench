pub mod br243;
pub mod br96;
pub mod loco_builder;
pub mod v60;

use crate::viewer3d::Vertex3D;

use self::br243::push_br243_3d;
use self::br96::push_br96_3d;
use self::v60::push_v60_3d;

pub use self::loco_builder::{LocoAnimState, TrainPose};

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

    /// Wheel tread radius in H0 mm (`r = 7.2 mm` BR 243/V 60, `7.0 mm` BR 96).
    pub fn wheel_radius_h0(self) -> f32 {
        match self {
            Self::Br96Mallet => 7.0,
            Self::Br243 | Self::V60Diesel => 7.2,
        }
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
}
