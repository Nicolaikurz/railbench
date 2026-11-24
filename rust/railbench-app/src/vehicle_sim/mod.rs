mod physics;
#[cfg(test)]
mod tests;
mod track_graph;

use std::collections::HashMap;

use railbench_core::types::{Catalog, Layout};

use crate::catalog::{find_entry, is_road_id};
use crate::model_tools::scale_factor_from_h0;
use crate::viewer3d::Vertex3D;

use track_graph::{polyline_length, recompute_vehicle_poses, sample_active_piece_branches};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleDomain {
    Rail,
    Road,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleKind {
    SteamLoco,
    ElectricLoco,
    DieselLoco,
    PassengerCoach,
    FreightCar,
    Sedan,
    Truck,
    Bus,
}

impl VehicleKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::SteamLoco => "loco-steam",
            Self::ElectricLoco => "loco-electric",
            Self::DieselLoco => "loco-diesel",
            Self::PassengerCoach => "wagon-passenger",
            Self::FreightCar => "wagon-freight",
            Self::Sedan => "car-sedan",
            Self::Truck => "car-truck",
            Self::Bus => "car-bus",
        }
    }

    pub fn domain(self) -> VehicleDomain {
        match self {
            Self::SteamLoco
            | Self::ElectricLoco
            | Self::DieselLoco
            | Self::PassengerCoach
            | Self::FreightCar => VehicleDomain::Rail,
            Self::Sedan | Self::Truck | Self::Bus => VehicleDomain::Road,
        }
    }

    pub fn is_locomotive(self) -> bool {
        matches!(self, Self::SteamLoco | Self::ElectricLoco | Self::DieselLoco)
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
        kind: VehicleKind::SteamLoco,
        name_de: "Dampflok BR 01",
        name_en: "Steam Loco BR 01",
        length_h0_mm: 258.0,
        width_h0_mm: 35.0,
        height_h0_mm: 50.0,
        bogie_half_span_h0_mm: 82.0,
        max_speed_mm_s: 260.0,
        mass_tons: 110.0,
        tractive_kn: 155.0,
        brake_kn: 135.0,
        default_rgb: [0.12, 0.14, 0.18],
    },
    VehicleCatalogDef {
        kind: VehicleKind::ElectricLoco,
        name_de: "E-Lok BR 103",
        name_en: "Electric Loco BR 103",
        length_h0_mm: 224.0,
        width_h0_mm: 35.0,
        height_h0_mm: 50.0,
        bogie_half_span_h0_mm: 63.0,
        max_speed_mm_s: 360.0,
        mass_tons: 114.0,
        tractive_kn: 312.0,
        brake_kn: 210.0,
        default_rgb: [0.58, 0.10, 0.13],
    },
    VehicleCatalogDef {
        kind: VehicleKind::DieselLoco,
        name_de: "Diesellok BR 218",
        name_en: "Diesel Loco BR 218",
        length_h0_mm: 189.0,
        width_h0_mm: 34.5,
        height_h0_mm: 48.0,
        bogie_half_span_h0_mm: 50.0,
        max_speed_mm_s: 300.0,
        mass_tons: 80.0,
        tractive_kn: 240.0,
        brake_kn: 165.0,
        default_rgb: [0.76, 0.13, 0.16],
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
    pub livery_rgb: [f32; 3],
    pub coupled_cars: Vec<VehicleKind>,
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
    next_seq: u64,
}

impl VehicleSimState {
    pub fn spawn_vehicle(
        &mut self,
        layout: &Layout,
        catalogs: &[Catalog],
        kind: VehicleKind,
        preferred_piece_id: Option<&str>,
    ) -> Result<String, String> {
        let domain = kind.domain();
        let want_road = domain == VehicleDomain::Road;
        let target_piece = preferred_piece_id
            .and_then(|pid| layout.pieces.iter().find(|p| p.id == pid))
            .filter(|p| is_road_id(catalogs, &p.catalog_id) == want_road && p.connectors.len() >= 2)
            .or_else(|| {
                layout
                    .pieces
                    .iter()
                    .find(|p| is_road_id(catalogs, &p.catalog_id) == want_road && p.connectors.len() >= 2)
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
            livery_rgb: def.default_rgb,
            coupled_cars: Vec::new(),
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

    /// Returns cached track-following bogie poses for all active vehicles & coupled consist wagons.
    pub fn all_render_poses(&self, _scale: &str) -> Vec<VehicleRenderPose> {
        let mut out = Vec::with_capacity(self.vehicles.len() * 2);
        for v in &self.vehicles {
            out.extend(v.cached_poses.iter().cloned());
        }
        out
    }

    /// Generates combined 3D vertex/index mesh for all active vehicles & coupled wagons,
    /// including authentic ramp/helix 3D pitch, exact railhead height, and gauge-matched wheelsets.
    pub fn build_vehicles_mesh_3d(&self, scale: &str) -> (Vec<Vertex3D>, Vec<u32>) {
        let poses = self.all_render_poses(scale);
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        for p in &poses {
            let y_off = if p.kind.domain() == VehicleDomain::Rail {
                RAIL_TOP_Y_MM
            } else {
                ROAD_TOP_Y_MM
            };
            crate::models3d_mesh::push_vehicle_3d(
                &mut verts,
                &mut idxs,
                p.kind.id(),
                [p.x as f32, (p.z + y_off) as f32, p.y as f32],
                (p.heading_deg as f32).to_radians(),
                p.pitch_rad as f32,
                [p.length_mm as f32, p.height_mm as f32, p.width_mm as f32],
                p.gauge_mm as f32,
                p.livery_rgb,
                p.headlights,
                p.driving_backward,
            );
        }
        (verts, idxs)
    }
}
