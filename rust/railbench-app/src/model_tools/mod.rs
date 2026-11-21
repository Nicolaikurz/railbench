use std::path::Path;

use railbench_core::geometry::deg2rad;
use railbench_core::types::{Model3D, Transform, Vec2};
use serde_json::Value;

/// Scale denominator definitions matching `src/core/model-scale.ts` and `src/ui/sidebar/shared.ts`.
pub const AVAILABLE_SCALES: &[(&str, f64, &str, &str)] = &[
    ("H0", 87.0, "Spur H0 (1:87)", "Scale H0 (1:87)"),
    ("TT", 120.0, "Spur TT (1:120)", "Scale TT (1:120)"),
    ("N", 160.0, "Spur N (1:160)", "Scale N (1:160)"),
    ("Z", 220.0, "Spur Z (1:220)", "Scale Z (1:220)"),
    ("0", 45.0, "Spur 0 (1:45)", "Scale 0 (1:45)"),
    ("1", 32.0, "Spur 1 (1:32)", "Scale 1 (1:32)"),
    ("G", 22.5, "Spur G (1:22.5)", "Scale G (1:22.5)"),
];

pub fn parse_scale_code(value: &str) -> &'static str {
    let s = value.trim().to_uppercase();
    match s.as_str() {
        "H0" | "HO" => "H0",
        "TT" => "TT",
        "N" => "N",
        "Z" => "Z",
        "0" | "SPUR 0" | "SPUR0" | "GAUGE 0" | "GAUGE0" | "SCALE 0" | "O" => "0",
        "1" | "SPUR 1" | "SPUR1" | "GAUGE 1" | "GAUGE1" | "SCALE 1" => "1",
        "G" | "SPUR G" | "SPURG" | "GAUGE G" | "GAUGEG" | "SCALE G" | "LGB" => "G",
        _ => "H0",
    }
}

pub fn scale_ratio_for(scale: &str) -> f64 {
    let code = parse_scale_code(scale);
    AVAILABLE_SCALES
        .iter()
        .find(|(c, _, _, _)| *c == code)
        .map(|(_, r, _, _)| *r)
        .unwrap_or(87.0)
}

/// Factor to convert H0 builtin dimensions (mm) to target scale (`H0 -> N = 87 / 160 = 0.54375`).
pub fn scale_factor_from_h0(target_scale: &str) -> f64 {
    87.0 / scale_ratio_for(target_scale)
}

#[derive(Debug, Clone, Copy)]
pub struct BuiltinModelDef {
    pub id: &'static str,
    pub name_de: &'static str,
    pub name_en: &'static str,
    pub category: &'static str,
    pub desc_de: &'static str,
    pub desc_en: &'static str,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
}

/// Complete catalog of 16 builtin 3D buildings, railway structures, and foliage matching `src/core/models-3d.ts`.
pub const BUILTIN_MODELS: &[BuiltinModelDef] = &[
    BuiltinModelDef {
        id: "station",
        name_de: "Bahnhof Neustadt",
        name_en: "Passenger Station",
        category: "buildings",
        desc_de: "Klassischer Landbahnhof mit Satteldach, Uhrturm und Bahnsteigdach.",
        desc_en: "Classic country passenger station with gabled roof, clock tower, and canopy.",
        width: 320.0,
        depth: 140.0,
        height: 120.0,
    },
    BuiltinModelDef {
        id: "signal-box",
        name_de: "Stellwerk W1",
        name_en: "Signal Box W1",
        category: "railway",
        desc_de: "Zweistoeckiges Backstein-Stellwerk mit Panoramafenstern.",
        desc_en: "Two-storey brick signal tower with panoramic upper control room windows.",
        width: 120.0,
        depth: 75.0,
        height: 110.0,
    },
    BuiltinModelDef {
        id: "engine-shed",
        name_de: "Lokschuppen",
        name_en: "Engine Shed",
        category: "railway",
        desc_de: "Einstand-Backstein-Lokschuppen mit Tordurchfahrt und Rauchabzug.",
        desc_en: "Single-stall red brick locomotive shed with doorway and exhaust vent.",
        width: 240.0,
        depth: 72.0,
        height: 105.0,
    },
    BuiltinModelDef {
        id: "town-house",
        name_de: "Fachwerkhaus",
        name_en: "Half-Timbered House",
        category: "buildings",
        desc_de: "Traditionelles Wohnhaus mit steilem Satteldach und Fenstern.",
        desc_en: "Traditional half-timbered residential house with steep pitched roof.",
        width: 110.0,
        depth: 90.0,
        height: 125.0,
    },
    BuiltinModelDef {
        id: "water-tower",
        name_de: "Wasserturm",
        name_en: "Water Tower",
        category: "railway",
        desc_de: "Gemauerter Wasserturm mit Wasserbehaelter fuer Dampfloks.",
        desc_en: "Stone railway tower supporting an elevated water reservoir.",
        width: 85.0,
        depth: 85.0,
        height: 170.0,
    },
    BuiltinModelDef {
        id: "coaling-station",
        name_de: "Bekohlungsanlage (Dampflok)",
        name_en: "Coaling Station (Steam)",
        category: "railway",
        desc_de: "Kohlenbansen, Hochbunker, Drehkran mit Greifer und Wasserkran.",
        desc_en: "Concrete coal bunker, elevated hopper, slewing crane, and water crane.",
        width: 180.0,
        depth: 75.0,
        height: 125.0,
    },
    BuiltinModelDef {
        id: "diesel-fuel-station",
        name_de: "Dieseltankstelle (Diesellok)",
        name_en: "Diesel Fueling Station",
        category: "railway",
        desc_de: "Haupt-Dieseltank, Pumpenhaus, Auffangwanne und 2 Zapfsaeulen.",
        desc_en: "Main diesel tank, pump house, drip pan, and 2 covered fuel dispensers.",
        width: 170.0,
        depth: 65.0,
        height: 68.0,
    },
    BuiltinModelDef {
        id: "light-signal",
        name_de: "Lichtsignal (Blocksignal)",
        name_en: "Color-Light Block Signal",
        category: "railway",
        desc_de: "Standard-Lichtblocksignal mit Stahlmast und Signalschirm.",
        desc_en: "Standard railway color-light block signal with steel mast.",
        width: 25.0,
        depth: 25.0,
        height: 85.0,
    },
    BuiltinModelDef {
        id: "buffer-stop",
        name_de: "Prellbock",
        name_en: "Buffer Stop",
        category: "railway",
        desc_de: "Gleisabschluss-Prellbock aus Stahlprofilen mit Pufferbohle.",
        desc_en: "End of track buffer stop with steel A-frame and wooden beam.",
        width: 25.0,
        depth: 35.0,
        height: 25.0,
    },
    BuiltinModelDef {
        id: "pine-tree",
        name_de: "Fichte / Tanne",
        name_en: "Pine / Fir Tree",
        category: "nature",
        desc_de: "Immergruener Nadelbaum mit gestufter Krone.",
        desc_en: "Layered evergreen coniferous pine tree.",
        width: 55.0,
        depth: 55.0,
        height: 135.0,
    },
    BuiltinModelDef {
        id: "deciduous-tree",
        name_de: "Laubbaum / Eiche",
        name_en: "Deciduous Oak Tree",
        category: "nature",
        desc_de: "Laubbaum mit mehrteiliger Baumkrone.",
        desc_en: "Broadleaf deciduous oak tree with branching canopy.",
        width: 80.0,
        depth: 80.0,
        height: 120.0,
    },
    BuiltinModelDef {
        id: "platform-lamp",
        name_de: "Bahnsteigleuchte",
        name_en: "Platform Lamppost",
        category: "railway",
        desc_de: "Geschwungene Bahnsteig-Peitschenleuchte.",
        desc_en: "Curved swan-neck platform lamppost with lantern head.",
        width: 20.0,
        depth: 20.0,
        height: 70.0,
    },
    BuiltinModelDef {
        id: "platform",
        name_de: "Bahnsteig",
        name_en: "Passenger Platform",
        category: "railway",
        desc_de: "Ueberdachter Bahnsteig mit Kanten, Bank, Dach und Uhr.",
        desc_en: "Covered passenger platform with concrete curbs, canopy, and benches.",
        width: 320.0,
        depth: 54.0,
        height: 55.0,
    },
    BuiltinModelDef {
        id: "turntable-trainhall",
        name_de: "Ringlokschuppen (Drehscheibe)",
        name_en: "Roundhouse (Turntable)",
        category: "railway",
        desc_de: "3-staendiger Ringlokschuppen fuer Drehscheiben.",
        desc_en: "3-stall curved roundhouse for turntables with locomotive bays.",
        width: 320.0,
        depth: 310.0,
        height: 115.0,
    },
    BuiltinModelDef {
        id: "skyscraper",
        name_de: "Wolkenkratzer / Hochhaus",
        name_en: "Skyscraper / High-Rise",
        category: "buildings",
        desc_de: "Modernes Hochhaus mit Glasfassade und Antennenmast.",
        desc_en: "Modern high-rise tower with glass facade, setback, and antenna.",
        width: 130.0,
        depth: 110.0,
        height: 280.0,
    },
    BuiltinModelDef {
        id: "industry-building",
        name_de: "Industriehalle / Fabrik",
        name_en: "Industrial Hall / Factory",
        category: "buildings",
        desc_de: "Fertigungshalle mit Sheddach, Laderampe und Stahlsilo.",
        desc_en: "Industrial manufacturing hall with sawtooth roof, loading dock, and silo.",
        width: 260.0,
        depth: 160.0,
        height: 110.0,
    },
];

#[cfg(test)]
pub fn find_builtin_def(id: &str) -> Option<&'static BuiltinModelDef> {
    let norm = if id == "bahnsteig" { "platform" } else { id };
    BUILTIN_MODELS.iter().find(|m| m.id == norm)
}

pub fn create_scaled_builtin_model(
    def: &BuiltinModelDef,
    target_scale: &str,
    is_de: bool,
) -> Model3D {
    let f = scale_factor_from_h0(target_scale);
    let round1 = |v: f64| (v * f * 10.0).round() / 10.0;
    Model3D {
        id: String::new(),
        name: if is_de { def.name_de } else { def.name_en }.into(),
        category: def.category.into(),
        transform: Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: round1(def.width),
        depth: round1(def.depth),
        height: round1(def.height),
        model_type: "builtin".into(),
        builtin_id: Some(def.id.into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    }
}

pub fn get_model_rotation_handle_world(model: &Model3D) -> Vec2 {
    let rad = deg2rad(model.transform.rotation);
    let handle_distance = (model.depth * model.scale.unwrap_or(1.0)) * 0.5 + 18.0;
    Vec2 {
        x: model.transform.x + handle_distance * rad.sin(),
        y: model.transform.y - handle_distance * rad.cos(),
    }
}

// ============================================================================
//  Pure-Rust Base64 Codec + .GLB / .GLTF Mesh Importer & Decoder
// ============================================================================

const B64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

mod gltf;
#[allow(unused_imports)]
use gltf::*;
pub use gltf::{import_model_from_path, decode_custom_model_mesh};
#[cfg(test)]
pub use gltf::base64_encode;
#[cfg(test)]
mod tests;
