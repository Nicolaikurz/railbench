mod physics;
#[cfg(test)]
mod tests;
mod track_graph;

use std::collections::{HashMap, HashSet};

use railbench_core::types::{Catalog, Layout};

use crate::catalog::{find_entry, is_road_id, is_uncoupler_entry};
use crate::model_tools::scale_factor_from_h0;
use crate::viewer3d::Vertex3D;

use track_graph::{
    polyline_length, recompute_vehicle_poses, sample_active_piece_branches, track_scale_and_gauge,
    walk_track_graph_location,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleDomain {
    Rail,
    Road,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleKind {
    Br243,
    Br96Mallet,
    V60Diesel,
    PassengerCoach,
    FreightCar,
    Sedan,
    Truck,
    Bus,
}

impl VehicleKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Br243 => "loco-br243",
            Self::Br96Mallet => "loco-br96",
            Self::V60Diesel => "loco-v60",
            Self::PassengerCoach => "wagon-passenger",
            Self::FreightCar => "wagon-freight",
            Self::Sedan => "car-sedan",
            Self::Truck => "car-truck",
            Self::Bus => "car-bus",
        }
    }

    pub fn domain(self) -> VehicleDomain {
        match self {
            Self::Br243
            | Self::Br96Mallet
            | Self::V60Diesel
            | Self::PassengerCoach
            | Self::FreightCar => VehicleDomain::Rail,
            Self::Sedan | Self::Truck | Self::Bus => VehicleDomain::Road,
        }
    }

    pub fn is_locomotive(self) -> bool {
        matches!(self, Self::Br243 | Self::Br96Mallet | Self::V60Diesel)
    }

    pub fn train_loco_kind(self) -> Option<crate::trains_3d::TrainLocoKind> {
        match self {
            Self::Br243 => Some(crate::trains_3d::TrainLocoKind::Br243),
            Self::Br96Mallet => Some(crate::trains_3d::TrainLocoKind::Br96Mallet),
            Self::V60Diesel => Some(crate::trains_3d::TrainLocoKind::V60Diesel),
            _ => None,
        }
    }

    pub fn def(self) -> &'static VehicleCatalogDef {
        VEHICLE_CATALOG
            .iter()
            .find(|d| d.kind == self)
            .unwrap_or(&VEHICLE_CATALOG[0])
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VehicleCatalogDef {
    pub kind: VehicleKind,
    pub name_de: &'static str,
    pub name_en: &'static str,
    pub length_h0_mm: f64,
    pub width_h0_mm: f64,
    pub height_h0_mm: f64,
    pub bogie_half_span_h0_mm: f64,
    pub max_speed_mm_s: f64,
    /// Prototype mass in metric tons (used for consist inertia, rolling resistance & ramp gravity).
    pub mass_tons: f64,
    /// Peak tractive effort in kN (0 for unpowered wagons).
    pub tractive_kn: f64,
    /// Service air/disc brake force in kN.
    pub brake_kn: f64,
    pub default_rgb: [f32; 3],
}

/// Top of the rail head (`ballast_h 5.5 + tie_h 1.6 + rail_h 2.5 = 9.6 mm`) in world coordinates.
pub const RAIL_TOP_Y_MM: f64 = 9.6;
/// Top of road asphalt surface in world coordinates.
pub const ROAD_TOP_Y_MM: f64 = 2.2;

pub const VEHICLE_CATALOG: &[VehicleCatalogDef] = &[
    VehicleCatalogDef {
        kind: VehicleKind::Br243,
        name_de: "E-Lok BR 243",
        name_en: "Electric Loco BR 243",
        length_h0_mm: 191.3,
        width_h0_mm: 34.2,
        height_h0_mm: 44.1,
        bogie_half_span_h0_mm: 48.0,
        max_speed_mm_s: 280.0,
        mass_tons: 82.5,
        tractive_kn: 248.0,
        brake_kn: 175.0,
        default_rgb: [0.49, 0.11, 0.15],
    },
    VehicleCatalogDef {
        kind: VehicleKind::Br96Mallet,
        name_de: "Dampflok BR 96 Mallet",
        name_en: "Steam Loco BR 96 Mallet",
        length_h0_mm: 203.4,
        width_h0_mm: 35.6,
        height_h0_mm: 51.5,
        bogie_half_span_h0_mm: 50.0,
        max_speed_mm_s: 180.0,
        mass_tons: 127.0,
        tractive_kn: 215.0,
        brake_kn: 150.0,
        default_rgb: [0.10, 0.11, 0.13],
    },
    VehicleCatalogDef {
        kind: VehicleKind::V60Diesel,
        name_de: "Diesellok V 60",
        name_en: "Diesel Shunter V 60",
        length_h0_mm: 120.1,
        width_h0_mm: 35.6,
        height_h0_mm: 48.3,
        bogie_half_span_h0_mm: 28.0,
        max_speed_mm_s: 195.0,
        mass_tons: 53.0,
        tractive_kn: 160.0,
        brake_kn: 115.0,
        default_rgb: [0.62, 0.11, 0.14],
    },
    VehicleCatalogDef {
        kind: VehicleKind::PassengerCoach,
        name_de: "Personenwagen",
        name_en: "Passenger Coach",
        length_h0_mm: 264.0,
        width_h0_mm: 32.5,
        height_h0_mm: 46.5,
        bogie_half_span_h0_mm: 92.0,
        max_speed_mm_s: 360.0,
        mass_tons: 45.0,
        tractive_kn: 0.0,
        brake_kn: 52.0,
        default_rgb: [0.10, 0.34, 0.22],
    },
    VehicleCatalogDef {
        kind: VehicleKind::FreightCar,
        name_de: "Gueterwagen",
        name_en: "Freight Car",
        length_h0_mm: 161.0,
        width_h0_mm: 33.5,
        height_h0_mm: 46.0,
        bogie_half_span_h0_mm: 46.0,
        max_speed_mm_s: 240.0,
        mass_tons: 32.0,
        tractive_kn: 0.0,
        brake_kn: 38.0,
        default_rgb: [0.46, 0.21, 0.13],
    },
    VehicleCatalogDef {
        kind: VehicleKind::Sedan,
        name_de: "PKW Limousine",
        name_en: "Passenger Car",
        length_h0_mm: 52.0,
        width_h0_mm: 22.0,
        height_h0_mm: 17.5,
        bogie_half_span_h0_mm: 16.0,
        max_speed_mm_s: 220.0,
        mass_tons: 1.6,
        tractive_kn: 5.5,
        brake_kn: 11.0,
        default_rgb: [0.15, 0.45, 0.85],
    },
    VehicleCatalogDef {
        kind: VehicleKind::Truck,
        name_de: "LKW Lieferwagen",
        name_en: "Delivery Truck",
        length_h0_mm: 96.0,
        width_h0_mm: 27.0,
        height_h0_mm: 34.0,
        bogie_half_span_h0_mm: 30.0,
        max_speed_mm_s: 170.0,
        mass_tons: 12.0,
        tractive_kn: 26.0,
        brake_kn: 58.0,
        default_rgb: [0.85, 0.38, 0.08],
    },
    VehicleCatalogDef {
        kind: VehicleKind::Bus,
        name_de: "Linienbus",
        name_en: "City Bus",
        length_h0_mm: 118.0,
        width_h0_mm: 27.0,
        height_h0_mm: 33.0,
        bogie_half_span_h0_mm: 36.0,
        max_speed_mm_s: 185.0,
        mass_tons: 16.0,
        tractive_kn: 32.0,
        brake_kn: 72.0,
        default_rgb: [0.92, 0.72, 0.10],
    },
];

#[derive(Debug, Clone)]
pub struct VehicleRenderPose {
    pub vehicle_id: String,
    pub label: String,
    pub kind: VehicleKind,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub heading_deg: f64,
    pub pitch_rad: f64,
    pub length_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub gauge_mm: f64,
    pub livery_rgb: [f32; 3],
    pub headlights: bool,
    pub driving_backward: bool,
    pub throttle_pct: f32,
    pub wheel_angle: f32,
    pub pantograph_up: bool,
    pub steam_on: bool,
    pub steam_time: f32,
    pub speed_norm: f32,
    pub mallet_articulation_rad: f32,
}

#[derive(Debug, Clone)]
pub struct VehicleInstance {
    pub id: String,
    pub name: String,
    pub kind: VehicleKind,
    pub domain: VehicleDomain,
    pub piece_id: String,
    pub branch_idx: usize,
    pub dist_mm: f64,
    /// Orientation of the vehicle's physical front (+X) along its current track branch.
    pub forward: bool,
    /// Throttle `0.0..=1.0`.
    pub throttle: f32,
    /// Desired direction (`false` = forward, `true` = reverse; braking smoothly through zero before reversing).
    pub reverse: bool,
    /// Signed model velocity in mm/s along physical orientation (`> 0` forward, `< 0` backward).
    pub velocity_mm_s: f64,
    /// Unsigned speed magnitude in mm/s (for UI/telemetry).
    pub current_speed_mm_s: f64,
    /// Live track grade (%) in current travel direction (positive = uphill).
    pub grade_pct: f64,
    /// Live 3D pitch angle in radians (positive = nose up).
    pub pitch_rad: f64,
    /// Normalized coupler draw/buff strain (`-1.0` compressed, `+1.0` stretched).
    pub coupler_strain: f32,
    /// True when automatic collision avoidance is holding this vehicle behind another train, wagon, or car.
    pub blocked_ahead: bool,
    pub headlights: bool,
    pub wheel_angle: f32,
    pub pantograph_up: bool,
    pub steam_on: bool,
    pub steam_time: f32,
    pub mallet_articulation_rad: f32,
    pub livery_rgb: [f32; 3],
    pub coupled_cars: Vec<VehicleKind>,
    /// Remaining cooldown seconds after uncoupling so a moving train clears the uncoupler safely.
    pub uncouple_cooldown_s: f64,
    pub world_x: f64,
    pub world_y: f64,
    pub world_z: f64,
    pub heading_deg: f64,
    path_history: Vec<(String, usize)>,
    cached_poses: Vec<VehicleRenderPose>,
}

impl VehicleInstance {
    /// Returns the lead vehicle's cached bogie-aligned render pose (including track-matched scale and gauge).
    pub fn lead_pose(&self) -> Option<&VehicleRenderPose> {
        self.cached_poses.first()
    }

    /// Total consist mass in metric tons (locomotive + coupled wagons).
    pub fn total_mass_tons(&self) -> f64 {
        self.kind.def().mass_tons
            + self.coupled_cars.iter().map(|w| w.def().mass_tons).sum::<f64>()
    }

    /// Prototype scale speed in km/h (H0 1:87 normalized by scale ratio).
    pub fn scale_speed_kmh(&self, scale: &str) -> f64 {
        let sf = scale_factor_from_h0(scale).max(0.1);
        (self.current_speed_mm_s / sf) * 0.087 * 3.6
    }
}

#[derive(Clone)]
struct CachedVehicleMesh3D {
    fingerprint: u64,
    version: u64,
    mesh: std::sync::Arc<crate::viewer3d::SceneMesh3D>,
}

impl std::fmt::Debug for CachedVehicleMesh3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedVehicleMesh3D")
            .field("fingerprint", &self.fingerprint)
            .field("version", &self.version)
            .finish()
    }
}

#[derive(Debug, Clone, Default)]
pub struct VehicleSimState {
    pub running: bool,
    pub vehicles: Vec<VehicleInstance>,
    pub selected_vehicle_id: Option<String>,
    pub turnout_states: HashMap<String, usize>,
    pub turnout_version: u64,
    pub turntable_angles_deg: HashMap<String, f64>,
    pub turntable_targets_deg: HashMap<String, f64>,
    pub transfer_offsets_mm: HashMap<String, f64>,
    pub transfer_targets_mm: HashMap<String, f64>,
    pub bridge_mesh_version: u64,
    /// Momentary active pulse timers (seconds remaining) for uncoupler tracks (e.g. Maerklin 24997).
    pub uncoupler_timers: HashMap<String, f64>,
    /// Uncoupler tracks latched in continuous Auto-Uncouple mode.
    pub armed_uncouplers: HashSet<String>,
    next_seq: u64,
    mesh_3d_version: u64,
    cached_mesh_3d: Option<CachedVehicleMesh3D>,
}

impl VehicleSimState {
    /// Returns true if `piece` belongs strictly to `domain` (`Rail` on rail tracks, `Road` on streets).
    pub fn piece_matches_domain(
        catalogs: &[Catalog],
        piece: &railbench_core::types::TrackPiece,
        domain: VehicleDomain,
    ) -> bool {
        let is_road = is_road_id(catalogs, &piece.catalog_id);
        if is_road != (domain == VehicleDomain::Road) {
            return false;
        }
        if domain == VehicleDomain::Road {
            if let Some(e) = find_entry(catalogs, &piece.catalog_id) {
                if matches!(e.kind.as_str(), "turntable" | "transfer-table") {
                    return false;
                }
            }
        }
        true
    }

    /// Removes any vehicle whose current piece no longer exists or belongs to the wrong domain,
    /// and prunes stale `path_history` entries.
    pub fn sanitize_for_layout(&mut self, layout: &Layout, catalogs: &[Catalog]) {
        let before = self.vehicles.len();
        self.vehicles.retain_mut(|v| {
            let Some(piece) = layout.pieces.iter().find(|p| p.id == v.piece_id) else {
                return false;
            };
            if !Self::piece_matches_domain(catalogs, piece, v.domain) {
                return false;
            }
            v.path_history.retain(|(pid, _)| {
                layout
                    .pieces
                    .iter()
                    .find(|p| p.id == *pid)
                    .is_some_and(|p| Self::piece_matches_domain(catalogs, p, v.domain))
            });
            true
        });
        if self.vehicles.len() != before {
            self.cached_mesh_3d = None;
            if let Some(ref sel) = self.selected_vehicle_id {
                if !self.vehicles.iter().any(|v| &v.id == sel) {
                    self.selected_vehicle_id = self.vehicles.first().map(|v| v.id.clone());
                }
            } else {
                self.selected_vehicle_id = self.vehicles.first().map(|v| v.id.clone());
            }
            if self.vehicles.is_empty() {
                self.running = false;
            }
        }
    }

    pub fn spawn_vehicle(
        &mut self,
        layout: &Layout,
        catalogs: &[Catalog],
        kind: VehicleKind,
        preferred_piece_id: Option<&str>,
    ) -> Result<String, String> {
        let domain = kind.domain();
        let want_road = domain == VehicleDomain::Road;
        let is_piece_visible = |piece: &railbench_core::types::TrackPiece| -> bool {
            let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
            layout
                .layers
                .as_ref()
                .and_then(|ls| ls.iter().find(|l| l.id == lid))
                .is_none_or(|l| l.visible)
        };
        let target_piece = preferred_piece_id
            .and_then(|pid| layout.pieces.iter().find(|p| p.id == pid))
            .filter(|p| Self::piece_matches_domain(catalogs, p, domain) && p.connectors.len() >= 2)
            .or_else(|| {
                layout
                    .pieces
                    .iter()
                    .find(|p| {
                        is_piece_visible(p)
                            && Self::piece_matches_domain(catalogs, p, domain)
                            && p.connectors.len() >= 2
                    })
                    .or_else(|| {
                        layout
                            .pieces
                            .iter()
                            .find(|p| {
                                Self::piece_matches_domain(catalogs, p, domain)
                                    && p.connectors.len() >= 2
                            })
                    })
            });

        let Some(piece) = target_piece else {
            return Err(if want_road {
                "Place at least one street piece first.".into()
            } else {
                "Place at least one track piece first.".into()
            });
        };

        self.next_seq += 1;
        let id = format!("veh-{}", self.next_seq);
        let def = kind.def();
        let entry = find_entry(catalogs, &piece.catalog_id);
        let branches = sample_active_piece_branches(
            piece,
            entry,
            &self.turntable_angles_deg,
            &self.transfer_offsets_mm,
        );
        let br = branches.first().map(|v| v.as_slice()).unwrap_or(&[]);
        let mid = polyline_length(br) * 0.5;

        let mut veh = VehicleInstance {
            id: id.clone(),
            name: format!("{} #{}", def.name_en, self.next_seq),
            kind,
            domain,
            piece_id: piece.id.clone(),
            branch_idx: 0,
            dist_mm: mid,
            forward: true,
            throttle: 0.45,
            reverse: false,
            velocity_mm_s: 0.0,
            current_speed_mm_s: 0.0,
            grade_pct: 0.0,
            pitch_rad: 0.0,
            coupler_strain: 0.0,
            blocked_ahead: false,
            headlights: true,
            wheel_angle: 0.0,
            pantograph_up: true,
            steam_on: true,
            steam_time: 0.0,
            mallet_articulation_rad: 0.0,
            livery_rgb: def.default_rgb,
            coupled_cars: Vec::new(),
            uncouple_cooldown_s: 0.0,
            world_x: 0.0,
            world_y: 0.0,
            world_z: 0.0,
            heading_deg: 0.0,
            path_history: vec![(piece.id.clone(), 0)],
            cached_poses: Vec::new(),
        };
        recompute_vehicle_poses(
            &mut veh,
            layout,
            catalogs,
            &self.turnout_states,
            &self.turntable_angles_deg,
            &self.transfer_offsets_mm,
        );
        self.vehicles.push(veh);
        self.selected_vehicle_id = Some(id.clone());
        Ok(id)
    }

    pub fn remove_vehicle(&mut self, id: &str) {
        self.vehicles.retain(|v| v.id != id);
        if self.selected_vehicle_id.as_deref() == Some(id) {
            self.selected_vehicle_id = self.vehicles.first().map(|v| v.id.clone());
        }
    }

    pub fn clear_all(&mut self) {
        self.vehicles.clear();
        self.selected_vehicle_id = None;
        self.running = false;
    }

    pub fn emergency_stop_all(&mut self) {
        for v in &mut self.vehicles {
            v.throttle = 0.0;
            v.velocity_mm_s = 0.0;
            v.current_speed_mm_s = 0.0;
            v.coupler_strain = -1.0;
        }
    }

    pub fn toggle_turnout(&mut self, piece_id: &str, branch_count: usize) {
        let count = branch_count.max(2);
        let cur = self.turnout_states.entry(piece_id.to_string()).or_insert(0);
        *cur = (*cur + 1) % count;
        self.turnout_version = self.turnout_version.wrapping_add(1);
    }

    pub fn add_wagon(&mut self, vehicle_id: &str, wagon: VehicleKind, layout: &Layout, catalogs: &[Catalog]) {
        if let Some(v) = self.vehicles.iter_mut().find(|v| v.id == vehicle_id) {
            if v.kind.is_locomotive() && v.coupled_cars.len() < 8 {
                v.coupled_cars.push(wagon);
                recompute_vehicle_poses(
                    v,
                    layout,
                    catalogs,
                    &self.turnout_states,
                    &self.turntable_angles_deg,
                    &self.transfer_offsets_mm,
                );
            }
        }
    }

    pub fn remove_last_wagon(&mut self, vehicle_id: &str, layout: &Layout, catalogs: &[Catalog]) {
        if let Some(v) = self.vehicles.iter_mut().find(|v| v.id == vehicle_id) {
            if v.coupled_cars.pop().is_some() {
                recompute_vehicle_poses(
                    v,
                    layout,
                    catalogs,
                    &self.turnout_states,
                    &self.turntable_angles_deg,
                    &self.transfer_offsets_mm,
                );
            }
        }
    }

    pub fn is_uncoupler_raised(&self, piece_id: &str) -> bool {
        self.armed_uncouplers.contains(piece_id)
            || self.uncoupler_timers.get(piece_id).copied().unwrap_or(0.0) > 0.0
    }

    pub fn has_active_uncoupler_timers(&self) -> bool {
        self.uncoupler_timers.values().any(|&t| t > 0.0)
    }

    pub fn active_uncoupler_ids(&self) -> HashSet<String> {
        let mut ids = self.armed_uncouplers.clone();
        for (pid, &t) in &self.uncoupler_timers {
            if t > 0.0 {
                ids.insert(pid.clone());
            }
        }
        ids
    }

    /// Disconnects all wagons starting at `split_idx` (`0..coupled_cars.len()`) into an
    /// independent parked cut on the track behind the locomotive.
    pub fn uncouple_at(
        &mut self,
        vehicle_id: &str,
        split_idx: usize,
        layout: &Layout,
        catalogs: &[Catalog],
    ) -> Option<String> {
        let idx = self
            .vehicles
            .iter()
            .position(|v| v.id == vehicle_id && v.kind.is_locomotive() && split_idx < v.coupled_cars.len())?;

        let (lead_kind, tail_cars, det_pid, det_br, det_dist, det_fwd, det_hist) = {
            let v = &self.vehicles[idx];
            let (sf, _) = track_scale_and_gauge(layout, catalogs, &v.piece_id, v.domain);
            let l = v.kind.def().length_h0_mm * sf;
            let coupler_gap = (3.5 + 1.4 * v.coupler_strain as f64).clamp(1.8, 6.5) * sf;
            let mut trail_offset = l * 0.5 + coupler_gap + 6.5 * sf;
            for &wk in &v.coupled_cars[..split_idx] {
                trail_offset += wk.def().length_h0_mm * sf + coupler_gap;
            }
            let first_detached = v.coupled_cars[split_idx];
            let center_trail = trail_offset + (first_detached.def().length_h0_mm * sf) * 0.5;
            let orient_sign = if v.forward { 1.0 } else { -1.0 };
            let (det_pid, det_br, det_dist, dir_pos) = walk_track_graph_location(
                layout,
                catalogs,
                &self.turnout_states,
                &self.turntable_angles_deg,
                &self.transfer_offsets_mm,
                &v.path_history,
                &v.piece_id,
                v.branch_idx,
                v.dist_mm,
                -orient_sign * center_trail,
                v.domain,
            );
            (
                first_detached,
                v.coupled_cars[split_idx + 1..].to_vec(),
                det_pid,
                det_br,
                det_dist,
                !dir_pos,
                v.path_history.clone(),
            )
        };

        {
            let v = &mut self.vehicles[idx];
            v.coupled_cars.truncate(split_idx);
            v.uncouple_cooldown_s = 1.6;
            recompute_vehicle_poses(
                v,
                layout,
                catalogs,
                &self.turnout_states,
                &self.turntable_angles_deg,
                &self.transfer_offsets_mm,
            );
        }

        self.next_seq += 1;
        let det_id = format!("veh-{}", self.next_seq);
        let ldef = lead_kind.def();
        let det_name = if tail_cars.is_empty() {
            format!("{} #{}", ldef.name_en, self.next_seq)
        } else {
            format!("{} #{} (+{})", ldef.name_en, self.next_seq, tail_cars.len())
        };
        let mut parked = VehicleInstance {
            id: det_id.clone(),
            name: det_name,
            kind: lead_kind,
            domain: VehicleDomain::Rail,
            piece_id: det_pid,
            branch_idx: det_br,
            dist_mm: det_dist,
            forward: det_fwd,
            throttle: 0.0,
            reverse: false,
            velocity_mm_s: 0.0,
            current_speed_mm_s: 0.0,
            grade_pct: 0.0,
            pitch_rad: 0.0,
            coupler_strain: 0.0,
            blocked_ahead: false,
            headlights: false,
            wheel_angle: 0.0,
            pantograph_up: false,
            steam_on: false,
            steam_time: 0.0,
            mallet_articulation_rad: 0.0,
            livery_rgb: ldef.default_rgb,
            coupled_cars: tail_cars,
            uncouple_cooldown_s: 1.6,
            world_x: 0.0,
            world_y: 0.0,
            world_z: 0.0,
            heading_deg: 0.0,
            path_history: det_hist,
            cached_poses: Vec::new(),
        };
        recompute_vehicle_poses(
            &mut parked,
            layout,
            catalogs,
            &self.turnout_states,
            &self.turntable_angles_deg,
            &self.transfer_offsets_mm,
        );
        self.vehicles.push(parked);
        Some(det_id)
    }

    pub fn uncouple_last_wagon(
        &mut self,
        vehicle_id: &str,
        layout: &Layout,
        catalogs: &[Catalog],
    ) -> Option<String> {
        let len = self
            .vehicles
            .iter()
            .find(|v| v.id == vehicle_id && v.kind.is_locomotive())
            .map(|v| v.coupled_cars.len())?;
        if len == 0 {
            return None;
        }
        self.uncouple_at(vehicle_id, len - 1, layout, catalogs)
    }

    pub fn uncouple_all_wagons(
        &mut self,
        vehicle_id: &str,
        layout: &Layout,
        catalogs: &[Catalog],
    ) -> Option<String> {
        let has_cars = self
            .vehicles
            .iter()
            .find(|v| v.id == vehicle_id && v.kind.is_locomotive())
            .is_some_and(|v| !v.coupled_cars.is_empty());
        if !has_cars {
            return None;
        }
        self.uncouple_at(vehicle_id, 0, layout, catalogs)
    }

    pub fn find_nearby_coupleable_cut(&self, vehicle_id: &str, layout: &Layout) -> Option<usize> {
        let v = self
            .vehicles
            .iter()
            .find(|v| v.id == vehicle_id && v.kind.is_locomotive() && v.coupled_cars.len() < 8)?;
        let last_pose = v.cached_poses.last()?;
        let sf = scale_factor_from_h0(&layout.scale).max(0.1);
        let lr = last_pose.heading_deg.to_radians();
        let rear_x = last_pose.x - lr.cos() * (last_pose.length_mm * 0.5);
        let rear_y = last_pose.y - lr.sin() * (last_pose.length_mm * 0.5);
        let rear_z = last_pose.z;
        let max_dist = 48.0 * sf;
        let max_dz = 24.0 * sf;

        let mut best: Option<(f64, usize)> = None;
        for (oi, other) in self.vehicles.iter().enumerate() {
            if other.id == vehicle_id || other.kind.is_locomotive() || other.domain != VehicleDomain::Rail {
                continue;
            }
            for pose in [other.cached_poses.first(), other.cached_poses.last()]
                .into_iter()
                .flatten()
            {
                if (pose.z - rear_z).abs() > max_dz {
                    continue;
                }
                let r = pose.heading_deg.to_radians();
                for end_sign in [1.0_f64, -1.0] {
                    let ex = pose.x + end_sign * r.cos() * (pose.length_mm * 0.5);
                    let ey = pose.y + end_sign * r.sin() * (pose.length_mm * 0.5);
                    let d = (ex - rear_x).hypot(ey - rear_y);
                    if d <= max_dist && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, oi));
                    }
                }
            }
        }
        best.map(|(_, oi)| oi)
    }

    /// Couples the nearest standing wagon cut behind `vehicle_id` onto the locomotive consist.
    pub fn couple_nearby_wagons(
        &mut self,
        vehicle_id: &str,
        layout: &Layout,
        catalogs: &[Catalog],
    ) -> usize {
        let Some(other_idx) = self.find_nearby_coupleable_cut(vehicle_id, layout) else {
            return 0;
        };
        let other_id = self.vehicles[other_idx].id.clone();
        let loco_idx = match self.vehicles.iter().position(|v| v.id == vehicle_id) {
            Some(i) => i,
            None => return 0,
        };
        let space = 8usize.saturating_sub(self.vehicles[loco_idx].coupled_cars.len());
        if space == 0 {
            return 0;
        }
        let mut cut = self.vehicles.remove(other_idx);
        let mut all_new = Vec::with_capacity(1 + cut.coupled_cars.len());
        all_new.push(cut.kind);
        all_new.extend(cut.coupled_cars.drain(..));

        let take = all_new.len().min(space);
        let loco = self.vehicles.iter_mut().find(|v| v.id == vehicle_id);
        if let Some(v) = loco {
            v.coupled_cars.extend(all_new.into_iter().take(take));
            recompute_vehicle_poses(
                v,
                layout,
                catalogs,
                &self.turnout_states,
                &self.turntable_angles_deg,
                &self.transfer_offsets_mm,
            );
        }
        if self.selected_vehicle_id.as_deref() == Some(other_id.as_str()) {
            self.selected_vehicle_id = Some(vehicle_id.to_string());
        }
        take
    }

    /// Finds an active coupler (`(vehicle_id, split_idx)`) positioned above uncoupler track `piece_id`.
    pub fn find_coupler_over_piece(
        &self,
        piece_id: &str,
        layout: &Layout,
        catalogs: &[Catalog],
    ) -> Option<(String, usize)> {
        let piece = layout.pieces.iter().find(|p| p.id == piece_id)?;
        let entry = find_entry(catalogs, &piece.catalog_id).filter(|e| is_uncoupler_entry(e))?;
        let seg_len = entry.length.unwrap_or(94.2).max(20.0);
        let sf = scale_factor_from_h0(&layout.scale).max(0.1);
        let lh = piece
            .layer_id
            .as_deref()
            .and_then(|lid| {
                layout
                    .layers
                    .as_ref()
                    .and_then(|ls| ls.iter().find(|l| l.id == lid))
                    .and_then(|l| l.height_mm)
            })
            .unwrap_or(0.0);
        let piece_z = lh + piece.elevation.map_or(0.0, |e| (e.start + e.end) * 0.5);
        let rot = (-piece.transform.rotation).to_radians();
        let (sr, cr) = rot.sin_cos();

        let mut best: Option<(f64, String, usize)> = None;
        for v in &self.vehicles {
            if !v.kind.is_locomotive() || v.coupled_cars.is_empty() || v.cached_poses.len() < 2 {
                continue;
            }
            for k in 0..v.coupled_cars.len() {
                let p0 = &v.cached_poses[k];
                let p1 = &v.cached_poses[k + 1];
                let r0 = p0.heading_deg.to_radians();
                let r1 = p1.heading_deg.to_radians();
                let cx = ((p0.x - r0.cos() * p0.length_mm * 0.5)
                    + (p1.x + r1.cos() * p1.length_mm * 0.5))
                    * 0.5;
                let cy = ((p0.y - r0.sin() * p0.length_mm * 0.5)
                    + (p1.y + r1.sin() * p1.length_mm * 0.5))
                    * 0.5;
                let cz = (p0.z + p1.z) * 0.5;
                if (cz - piece_z).abs() > 24.0 * sf {
                    continue;
                }
                let dx = cx - piece.transform.x;
                let dy = cy - piece.transform.y;
                let lx = dx * cr - dy * sr;
                let ly = dx * sr + dy * cr;
                if lx >= -14.0 * sf && lx <= seg_len + 14.0 * sf && ly.abs() <= 22.0 * sf {
                    let mid_err = (lx - seg_len * 0.5).abs();
                    if best.as_ref().is_none_or(|(be, _, _)| mid_err < *be) {
                        best = Some((mid_err, v.id.clone(), k));
                    }
                }
            }
        }
        best.map(|(_, vid, k)| (vid, k))
    }

    /// Pulses electric uncoupler track `piece_id` (`2.0 s` ramp lift) and immediately
    /// disconnects any coupler positioned above it.
    pub fn trigger_uncoupler(
        &mut self,
        piece_id: &str,
        layout: &Layout,
        catalogs: &[Catalog],
    ) -> Option<String> {
        self.uncoupler_timers.insert(piece_id.to_string(), 2.0);
        self.bridge_mesh_version = self.bridge_mesh_version.wrapping_add(1);
        if let Some((vid, split_idx)) = self.find_coupler_over_piece(piece_id, layout, catalogs) {
            return self.uncouple_at(&vid, split_idx, layout, catalogs);
        }
        None
    }

    pub fn toggle_uncoupler_armed(&mut self, piece_id: &str) -> bool {
        let now_armed = if self.armed_uncouplers.remove(piece_id) {
            false
        } else {
            self.armed_uncouplers.insert(piece_id.to_string());
            true
        };
        self.bridge_mesh_version = self.bridge_mesh_version.wrapping_add(1);
        now_armed
    }

    /// Updates uncoupler pulse timers, vehicle cooldowns, and auto-uncoupling triggers.
    pub fn step_uncouplers(&mut self, dt_secs: f64, layout: &Layout, catalogs: &[Catalog]) {
        let dt = dt_secs.clamp(0.0, 0.1);
        for v in &mut self.vehicles {
            if v.uncouple_cooldown_s > 0.0 {
                v.uncouple_cooldown_s = (v.uncouple_cooldown_s - dt).max(0.0);
            }
        }

        let mut expired = Vec::new();
        for (pid, rem) in &mut self.uncoupler_timers {
            *rem -= dt;
            if *rem <= 0.0 {
                expired.push(pid.clone());
            }
        }
        if !expired.is_empty() {
            for pid in expired {
                self.uncoupler_timers.remove(&pid);
            }
            self.bridge_mesh_version = self.bridge_mesh_version.wrapping_add(1);
        }

        let active_ids = self.active_uncoupler_ids();
        for pid in active_ids {
            if let Some((vid, split_idx)) = self.find_coupler_over_piece(&pid, layout, catalogs) {
                let ready = self
                    .vehicles
                    .iter()
                    .find(|v| v.id == vid)
                    .is_some_and(|v| v.uncouple_cooldown_s <= 0.0);
                if ready {
                    self.uncouple_at(&vid, split_idx, layout, catalogs);
                }
            }
        }
    }

    /// Returns cached track-following bogie poses for all active vehicles & coupled consist wagons.
    pub fn all_render_poses(&self, _scale: &str) -> Vec<VehicleRenderPose> {
        let mut out = Vec::with_capacity(self.vehicles.len() * 2);
        for v in &self.vehicles {
            out.extend(v.cached_poses.iter().cloned());
        }
        out
    }

    fn visual_mesh_fingerprint(&self, scale: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h = DefaultHasher::new();
        scale.hash(&mut h);
        self.vehicles.len().hash(&mut h);
        for v in &self.vehicles {
            v.id.hash(&mut h);
            (v.kind as u8).hash(&mut h);
            v.headlights.hash(&mut h);
            v.reverse.hash(&mut h);
            (v.velocity_mm_s < -1.0).hash(&mut h);
            v.pantograph_up.hash(&mut h);
            v.steam_on.hash(&mut h);
            v.wheel_angle.to_bits().hash(&mut h);
            v.steam_time.to_bits().hash(&mut h);
            v.mallet_articulation_rad.to_bits().hash(&mut h);
            for c in v.livery_rgb {
                c.to_bits().hash(&mut h);
            }
            v.cached_poses.len().hash(&mut h);
            for p in &v.cached_poses {
                (p.kind as u8).hash(&mut h);
                p.x.to_bits().hash(&mut h);
                p.y.to_bits().hash(&mut h);
                p.z.to_bits().hash(&mut h);
                p.heading_deg.to_bits().hash(&mut h);
                p.pitch_rad.to_bits().hash(&mut h);
                p.length_mm.to_bits().hash(&mut h);
                p.width_mm.to_bits().hash(&mut h);
                p.height_mm.to_bits().hash(&mut h);
                p.gauge_mm.to_bits().hash(&mut h);
                p.headlights.hash(&mut h);
                p.driving_backward.hash(&mut h);
                p.wheel_angle.to_bits().hash(&mut h);
                p.pantograph_up.hash(&mut h);
                p.steam_on.hash(&mut h);
                p.steam_time.to_bits().hash(&mut h);
                p.speed_norm.to_bits().hash(&mut h);
                p.mallet_articulation_rad.to_bits().hash(&mut h);
                for c in p.livery_rgb {
                    c.to_bits().hash(&mut h);
                }
            }
        }
        h.finish()
    }

    /// Returns a cached `(version, Arc<SceneMesh3D>)` for all active vehicles, rebuilding only
    /// when a vehicle pose, wheel rotation, light state, steam phase, or scale has changed.
    pub fn get_or_build_vehicles_mesh_3d(
        &mut self,
        scale: &str,
    ) -> Option<(u64, std::sync::Arc<crate::viewer3d::SceneMesh3D>)> {
        if self.vehicles.is_empty() {
            self.cached_mesh_3d = None;
            return None;
        }
        let fp = self.visual_mesh_fingerprint(scale);
        if let Some(ref cached) = self.cached_mesh_3d {
            if cached.fingerprint == fp {
                return Some((cached.version, std::sync::Arc::clone(&cached.mesh)));
            }
        }
        let (verts, idxs) = self.build_vehicles_mesh_3d(scale);
        if idxs.is_empty() {
            self.cached_mesh_3d = None;
            return None;
        }
        self.mesh_3d_version = self.mesh_3d_version.wrapping_add(1).max(1);
        let ver = self.mesh_3d_version;
        let mesh = std::sync::Arc::new(crate::viewer3d::SceneMesh3D::from_raw(verts, idxs));
        self.cached_mesh_3d = Some(CachedVehicleMesh3D {
            fingerprint: fp,
            version: ver,
            mesh: std::sync::Arc::clone(&mesh),
        });
        Some((ver, mesh))
    }

    /// Generates combined 3D vertex/index mesh for all active vehicles & coupled wagons,
    /// including authentic ramp/helix 3D pitch, exact railhead height, and gauge-matched wheelsets.
    pub fn build_vehicles_mesh_3d(&self, _scale: &str) -> (Vec<Vertex3D>, Vec<u32>) {
        let pose_count: usize = self.vehicles.iter().map(|v| v.cached_poses.len()).sum();
        let mut verts = Vec::with_capacity(pose_count * 2048);
        let mut idxs = Vec::with_capacity(pose_count * 3600);
        for p in self.vehicles.iter().flat_map(|v| v.cached_poses.iter()) {
            let y_off = if p.kind.domain() == VehicleDomain::Rail {
                RAIL_TOP_Y_MM
            } else {
                ROAD_TOP_Y_MM
            };
            let origin = [p.x as f32, (p.z + y_off) as f32, p.y as f32];
            if let Some(loco_kind) = p.kind.train_loco_kind() {
                crate::trains_3d::push_locomotive_3d(
                    &mut verts,
                    &mut idxs,
                    loco_kind,
                    crate::trains_3d::TrainPose {
                        pos: origin,
                        yaw_rad: (p.heading_deg as f32).to_radians(),
                        pitch_rad: p.pitch_rad as f32,
                        scale: (p.gauge_mm / 16.5).max(0.25) as f32,
                    },
                    crate::trains_3d::LocoAnimState {
                        wheel_angle: p.wheel_angle,
                        forward_lights: !p.driving_backward,
                        headlights_on: p.headlights,
                        pantograph_up: p.pantograph_up,
                        steam_on: p.steam_on,
                        steam_time: p.steam_time,
                        speed_norm: p.speed_norm,
                        mallet_articulation_rad: p.mallet_articulation_rad,
                    },
                );
            } else {
                crate::models3d_mesh::push_vehicle_3d(
                    &mut verts,
                    &mut idxs,
                    p.kind.id(),
                    origin,
                    (p.heading_deg as f32).to_radians(),
                    p.pitch_rad as f32,
                    [p.length_mm as f32, p.height_mm as f32, p.width_mm as f32],
                    p.gauge_mm as f32,
                    p.livery_rgb,
                    p.headlights,
                    p.driving_backward,
                );
            }
        }
        (verts, idxs)
    }
}
