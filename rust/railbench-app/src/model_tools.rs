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

pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(((bytes.len() + 2) / 3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(B64_TABLE[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64_TABLE[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64_TABLE[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let raw = input
        .split_once(";base64,")
        .map(|(_, rest)| rest)
        .unwrap_or(input);
    let mut lut = [255u8; 256];
    for (i, &c) in B64_TABLE.iter().enumerate() {
        lut[c as usize] = i as u8;
    }

    let mut out = Vec::with_capacity((raw.len() * 3) / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in raw.as_bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        if b == b'=' {
            break;
        }
        let val = lut[b as usize];
        if val == 255 {
            return Err(format!("Invalid base64 byte: {}", b as char));
        }
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct CustomMeshTri {
    pub positions: [[f32; 3]; 3],
    pub normals: [[f32; 3]; 3],
    pub color: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct ParsedGltfMesh {
    pub triangles: Vec<CustomMeshTri>,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

fn mat4_identity() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Column-major 4x4 matrix multiply `a * b`.
fn mat4_mul_col(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut out = [0.0_f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            out[c * 4 + r] = a[r] * b[c * 4]
                + a[4 + r] * b[c * 4 + 1]
                + a[8 + r] * b[c * 4 + 2]
                + a[12 + r] * b[c * 4 + 3];
        }
    }
    out
}

fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

fn transform_normal(m: &[f32; 16], n: [f32; 3]) -> [f32; 3] {
    let rx = m[0] * n[0] + m[4] * n[1] + m[8] * n[2];
    let ry = m[1] * n[0] + m[5] * n[1] + m[9] * n[2];
    let rz = m[2] * n[0] + m[6] * n[1] + m[10] * n[2];
    let len = (rx * rx + ry * ry + rz * rz).sqrt().max(1e-6);
    [rx / len, ry / len, rz / len]
}

fn node_local_matrix(node: &Value) -> [f32; 16] {
    if let Some(arr) = node.get("matrix").and_then(|v| v.as_array()) {
        if arr.len() == 16 {
            let mut m = [0.0_f32; 16];
            for (i, item) in arr.iter().enumerate() {
                m[i] = item.as_f64().unwrap_or(0.0) as f32;
            }
            return m;
        }
    }
    let t = node
        .get("translation")
        .and_then(|v| v.as_array())
        .map(|a| {
            [
                a.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                a.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                a.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
            ]
        })
        .unwrap_or([0.0, 0.0, 0.0]);
    let q = node
        .get("rotation")
        .and_then(|v| v.as_array())
        .map(|a| {
            [
                a.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                a.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                a.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                a.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
            ]
        })
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let s = node
        .get("scale")
        .and_then(|v| v.as_array())
        .map(|a| {
            [
                a.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                a.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                a.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
            ]
        })
        .unwrap_or([1.0, 1.0, 1.0]);

    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let x2 = x + x;
    let y2 = y + y;
    let z2 = z + z;
    let (xx, xy, xz) = (x * x2, x * y2, x * z2);
    let (yy, yz, zz) = (y * y2, y * z2, z * z2);
    let (wx, wy, wz) = (w * x2, w * y2, w * z2);

    [
        (1.0 - (yy + zz)) * s[0],
        (xy + wz) * s[0],
        (xz - wy) * s[0],
        0.0,
        (xy - wz) * s[1],
        (1.0 - (xx + zz)) * s[1],
        (yz + wx) * s[1],
        0.0,
        (xz + wy) * s[2],
        (yz - wx) * s[2],
        (1.0 - (xx + yy)) * s[2],
        0.0,
        t[0],
        t[1],
        t[2],
        1.0,
    ]
}

fn read_u16_le(buf: &[u8], off: usize) -> Option<u16> {
    buf.get(off..off + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn read_i16_le(buf: &[u8], off: usize) -> Option<i16> {
    buf.get(off..off + 2).map(|s| i16::from_le_bytes([s[0], s[1]]))
}

fn read_u32_le(buf: &[u8], off: usize) -> Option<u32> {
    buf.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn read_f32_le(buf: &[u8], off: usize) -> Option<f32> {
    buf.get(off..off + 4)
        .map(|s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn component_size(comp_type: u64) -> usize {
    match comp_type {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        5125 | 5126 => 4,
        _ => 4,
    }
}

fn type_count(type_str: &str) -> usize {
    match type_str {
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" | "MAT2" => 4,
        "MAT3" => 9,
        "MAT4" => 16,
        _ => 1,
    }
}

fn read_component_as_f32(buf: &[u8], off: usize, comp_type: u64, normalized: bool) -> Option<f32> {
    match comp_type {
        5126 => read_f32_le(buf, off),
        5121 => buf.get(off).map(|&v| {
            if normalized { v as f32 / 255.0 } else { v as f32 }
        }),
        5120 => buf.get(off).map(|&v| {
            let s = v as i8;
            if normalized { (s as f32 / 127.0).max(-1.0) } else { s as f32 }
        }),
        5123 => read_u16_le(buf, off).map(|v| {
            if normalized { v as f32 / 65535.0 } else { v as f32 }
        }),
        5122 => read_i16_le(buf, off).map(|v| {
            if normalized { (v as f32 / 32767.0).max(-1.0) } else { v as f32 }
        }),
        5125 => read_u32_le(buf, off).map(|v| v as f32),
        _ => None,
    }
}

fn read_accessor_vec3(doc: &Value, buffers: &[Vec<u8>], accessor_idx: usize) -> Option<Vec<[f32; 3]>> {
    let acc = doc.get("accessors")?.as_array()?.get(accessor_idx)?;
    let bv_idx = acc.get("bufferView")?.as_u64()? as usize;
    let bv = doc.get("bufferViews")?.as_array()?.get(bv_idx)?;
    let buf_idx = bv.get("buffer")?.as_u64()? as usize;
    let raw = buffers.get(buf_idx)?;

    let count = acc.get("count")?.as_u64()? as usize;
    let comp_type = acc.get("componentType")?.as_u64()?;
    let norm = acc.get("normalized").and_then(|v| v.as_bool()).unwrap_or(false);
    let tcount = type_count(acc.get("type")?.as_str()?);
    let acc_off = acc.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let bv_off = bv.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let comp_sz = component_size(comp_type);
    let stride = bv
        .get("byteStride")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .filter(|&s| s >= comp_sz * tcount)
        .unwrap_or(comp_sz * tcount);

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = bv_off + acc_off + i * stride;
        let x = read_component_as_f32(raw, base, comp_type, norm)?;
        let y = if tcount >= 2 {
            read_component_as_f32(raw, base + comp_sz, comp_type, norm).unwrap_or(0.0)
        } else {
            0.0
        };
        let z = if tcount >= 3 {
            read_component_as_f32(raw, base + comp_sz * 2, comp_type, norm).unwrap_or(0.0)
        } else {
            0.0
        };
        out.push([x, y, z]);
    }
    Some(out)
}

fn read_accessor_indices(doc: &Value, buffers: &[Vec<u8>], accessor_idx: usize) -> Option<Vec<usize>> {
    let acc = doc.get("accessors")?.as_array()?.get(accessor_idx)?;
    let bv_idx = acc.get("bufferView")?.as_u64()? as usize;
    let bv = doc.get("bufferViews")?.as_array()?.get(bv_idx)?;
    let buf_idx = bv.get("buffer")?.as_u64()? as usize;
    let raw = buffers.get(buf_idx)?;

    let count = acc.get("count")?.as_u64()? as usize;
    let comp_type = acc.get("componentType")?.as_u64()?;
    let acc_off = acc.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let bv_off = bv.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let comp_sz = component_size(comp_type);
    let stride = bv
        .get("byteStride")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .filter(|&s| s >= comp_sz)
        .unwrap_or(comp_sz);

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = bv_off + acc_off + i * stride;
        let idx = match comp_type {
            5121 => raw.get(base).copied().map(|v| v as usize)?,
            5123 => read_u16_le(raw, base).map(|v| v as usize)?,
            5125 => read_u32_le(raw, base).map(|v| v as usize)?,
            _ => return None,
        };
        out.push(idx);
    }
    Some(out)
}

pub fn parse_glb_bytes(bytes: &[u8]) -> Result<ParsedGltfMesh, String> {
    if bytes.len() < 12 {
        return Err("GLB header too short".into());
    }
    let magic = read_u32_le(bytes, 0).unwrap_or(0);
    if magic != 0x4654_6C67 {
        return Err("Not a valid binary .glb file".into());
    }
    let version = read_u32_le(bytes, 4).unwrap_or(0);
    if version != 2 {
        return Err(format!("Unsupported GLB version {version}"));
    }

    let mut off = 12usize;
    let mut json_str: Option<String> = None;
    let mut bin_chunk: Option<&[u8]> = None;

    while off + 8 <= bytes.len() {
        let chunk_len = read_u32_le(bytes, off).unwrap_or(0) as usize;
        let chunk_type = read_u32_le(bytes, off + 4).unwrap_or(0);
        let data_start = off + 8;
        let data_end = data_start.saturating_add(chunk_len).min(bytes.len());
        let data = &bytes[data_start..data_end];
        if chunk_type == 0x4E4F_534A {
            json_str = Some(String::from_utf8_lossy(data).to_string());
        } else if chunk_type == 0x004E_4942 {
            bin_chunk = Some(data);
        }
        let padded = (chunk_len + 3) & !3;
        off = data_start.saturating_add(padded);
    }

    let j = json_str.ok_or_else(|| "GLB contains no JSON chunk".to_string())?;
    parse_gltf_json(&j, bin_chunk, None)
}

pub fn parse_gltf_json(
    json_str: &str,
    bin_chunk: Option<&[u8]>,
    base_dir: Option<&Path>,
) -> Result<ParsedGltfMesh, String> {
    let doc: Value = serde_json::from_str(json_str).map_err(|e| format!("glTF JSON error: {e}"))?;

    let mut buffers: Vec<Vec<u8>> = Vec::new();
    if let Some(buf_defs) = doc.get("buffers").and_then(|v| v.as_array()) {
        for (bi, bdef) in buf_defs.iter().enumerate() {
            if let Some(uri) = bdef.get("uri").and_then(|v| v.as_str()) {
                if uri.starts_with("data:") {
                    buffers.push(base64_decode(uri)?);
                } else if let Some(dir) = base_dir {
                    let data = std::fs::read(dir.join(uri))
                        .map_err(|e| format!("Failed to read glTF buffer '{uri}': {e}"))?;
                    buffers.push(data);
                } else if bi == 0 {
                    buffers.push(bin_chunk.unwrap_or(&[]).to_vec());
                } else {
                    buffers.push(Vec::new());
                }
            } else if bi == 0 {
                buffers.push(bin_chunk.unwrap_or(&[]).to_vec());
            } else {
                buffers.push(Vec::new());
            }
        }
    } else if let Some(bin) = bin_chunk {
        buffers.push(bin.to_vec());
    }

    let nodes = doc.get("nodes").and_then(|v| v.as_array());
    let meshes = doc.get("meshes").and_then(|v| v.as_array());
    let materials = doc.get("materials").and_then(|v| v.as_array());

    let mut triangles = Vec::new();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    let append_mesh = |mesh_idx: usize,
                       world: &[f32; 16],
                       tris: &mut Vec<CustomMeshTri>,
                       bmin: &mut [f32; 3],
                       bmax: &mut [f32; 3]| {
        let Some(ms) = meshes else { return };
        let Some(mesh) = ms.get(mesh_idx) else { return };
        let Some(prims) = mesh.get("primitives").and_then(|v| v.as_array()) else {
            return;
        };
        for prim in prims {
            let mode = prim.get("mode").and_then(|v| v.as_u64()).unwrap_or(4);
            if mode != 4 {
                continue;
            }
            let Some(attrs) = prim.get("attributes") else { continue };
            let Some(pos_acc) = attrs.get("POSITION").and_then(|v| v.as_u64()) else {
                continue;
            };
            let Some(positions) = read_accessor_vec3(&doc, &buffers, pos_acc as usize) else {
                continue;
            };
            let normals = attrs
                .get("NORMAL")
                .and_then(|v| v.as_u64())
                .and_then(|idx| read_accessor_vec3(&doc, &buffers, idx as usize));
            let vcolors = attrs
                .get("COLOR_0")
                .and_then(|v| v.as_u64())
                .and_then(|idx| read_accessor_vec3(&doc, &buffers, idx as usize));
            let indices: Vec<usize> = prim
                .get("indices")
                .and_then(|v| v.as_u64())
                .and_then(|idx| read_accessor_indices(&doc, &buffers, idx as usize))
                .unwrap_or_else(|| (0..positions.len()).collect());

            let mat_col = prim
                .get("material")
                .and_then(|v| v.as_u64())
                .and_then(|mi| materials.and_then(|ms| ms.get(mi as usize)))
                .and_then(|mat| mat.get("pbrMetallicRoughness"))
                .and_then(|pbr| pbr.get("baseColorFactor"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    [
                        arr.first().and_then(|v| v.as_f64()).unwrap_or(0.76) as f32,
                        arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.76) as f32,
                        arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.78) as f32,
                    ]
                })
                .unwrap_or([0.76, 0.76, 0.78]);

            for tri_idx in indices.chunks_exact(3) {
                let (i0, i1, i2) = (tri_idx[0], tri_idx[1], tri_idx[2]);
                if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
                    continue;
                }
                let p0 = transform_point(world, positions[i0]);
                let p1 = transform_point(world, positions[i1]);
                let p2 = transform_point(world, positions[i2]);
                for p in [p0, p1, p2] {
                    for k in 0..3 {
                        bmin[k] = bmin[k].min(p[k]);
                        bmax[k] = bmax[k].max(p[k]);
                    }
                }
                let face_norm = {
                    let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                    let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
                    let cx = u[1] * v[2] - u[2] * v[1];
                    let cy = u[2] * v[0] - u[0] * v[2];
                    let cz = u[0] * v[1] - u[1] * v[0];
                    let len = (cx * cx + cy * cy + cz * cz).sqrt().max(1e-6);
                    [cx / len, cy / len, cz / len]
                };
                let (n0, n1, n2) = if let Some(ref ns) = normals {
                    if i0 < ns.len() && i1 < ns.len() && i2 < ns.len() {
                        (
                            transform_normal(world, ns[i0]),
                            transform_normal(world, ns[i1]),
                            transform_normal(world, ns[i2]),
                        )
                    } else {
                        (face_norm, face_norm, face_norm)
                    }
                } else {
                    (face_norm, face_norm, face_norm)
                };
                let col = if let Some(ref vc) = vcolors {
                    if i0 < vc.len() && i1 < vc.len() && i2 < vc.len() {
                        let avg = [
                            (vc[i0][0] + vc[i1][0] + vc[i2][0]) / 3.0,
                            (vc[i0][1] + vc[i1][1] + vc[i2][1]) / 3.0,
                            (vc[i0][2] + vc[i1][2] + vc[i2][2]) / 3.0,
                        ];
                        [mat_col[0] * avg[0], mat_col[1] * avg[1], mat_col[2] * avg[2]]
                    } else {
                        mat_col
                    }
                } else {
                    mat_col
                };
                tris.push(CustomMeshTri {
                    positions: [p0, p1, p2],
                    normals: [n0, n1, n2],
                    color: col,
                });
            }
        }
    };

    if let Some(node_arr) = nodes {
        let scene_idx = doc.get("scene").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let root_nodes: Vec<usize> = doc
            .get("scenes")
            .and_then(|v| v.as_array())
            .and_then(|sc| sc.get(scene_idx))
            .and_then(|s| s.get("nodes"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
            .unwrap_or_else(|| if node_arr.is_empty() { vec![] } else { vec![0] });

        let mut stack: Vec<(usize, [f32; 16])> = root_nodes
            .into_iter()
            .map(|ni| (ni, mat4_identity()))
            .collect();
        while let Some((ni, parent_m)) = stack.pop() {
            let Some(node) = node_arr.get(ni) else { continue };
            let world = mat4_mul_col(&parent_m, &node_local_matrix(node));
            if let Some(mi) = node.get("mesh").and_then(|v| v.as_u64()) {
                append_mesh(mi as usize, &world, &mut triangles, &mut min, &mut max);
            }
            if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
                for ch in children {
                    if let Some(ci) = ch.as_u64() {
                        stack.push((ci as usize, world));
                    }
                }
            }
        }
    } else if let Some(ms) = meshes {
        let ident = mat4_identity();
        for mi in 0..ms.len() {
            append_mesh(mi, &ident, &mut triangles, &mut min, &mut max);
        }
    }

    if triangles.is_empty() {
        min = [0.0, 0.0, 0.0];
        max = [1.0, 1.0, 1.0];
    }

    Ok(ParsedGltfMesh { triangles, min, max })
}

fn inline_external_gltf_buffers(json_str: &str, base_dir: &Path) -> String {
    let Ok(mut doc) = serde_json::from_str::<Value>(json_str) else {
        return json_str.to_string();
    };
    let mut modified = false;
    if let Some(bufs) = doc.get_mut("buffers").and_then(|v| v.as_array_mut()) {
        for b in bufs {
            if let Some(uri) = b.get("uri").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                if !uri.starts_with("data:") {
                    if let Ok(bytes) = std::fs::read(base_dir.join(&uri)) {
                        let embedded = format!("data:application/octet-stream;base64,{}", base64_encode(&bytes));
                        if let Some(obj) = b.as_object_mut() {
                            obj.insert("uri".into(), Value::String(embedded));
                            modified = true;
                        }
                    }
                }
            }
        }
    }
    if modified {
        serde_json::to_string(&doc).unwrap_or_else(|_| json_str.to_string())
    } else {
        json_str.to_string()
    }
}

pub fn import_model_from_path(path: &Path) -> Result<Model3D, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("model.glb")
        .to_string();
    let stem = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("Custom Model")
        .to_string();

    let (parsed, model_type, file_data) = match ext.as_str() {
        "glb" => {
            let bytes = std::fs::read(path).map_err(|e| format!("Read error: {e}"))?;
            let parsed = parse_glb_bytes(&bytes)?;
            let data = format!("data:model/gltf-binary;base64,{}", base64_encode(&bytes));
            (parsed, "glb".to_string(), data)
        }
        "gltf" => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("Read error: {e}"))?;
            let base_dir = path.parent();
            let parsed = parse_gltf_json(&text, None, base_dir)?;
            let self_contained = base_dir
                .map(|d| inline_external_gltf_buffers(&text, d))
                .unwrap_or(text);
            let data = format!("data:model/gltf+json;base64,{}", base64_encode(self_contained.as_bytes()));
            (parsed, "gltf".to_string(), data)
        }
        _ => return Err("Unsupported format. Select a .glb or .gltf 3D model.".into()),
    };

    let size_x = (parsed.max[0] - parsed.min[0]).max(0.01) as f64;
    let size_y = (parsed.max[1] - parsed.min[1]).max(0.01) as f64;
    let size_z = (parsed.max[2] - parsed.min[2]).max(0.01) as f64;
    let auto_scale = if size_x < 15.0 && size_z < 15.0 {
        1000.0 / 87.0
    } else {
        1.0
    };

    let width = (size_x * auto_scale).round().max(15.0);
    let depth = (size_z * auto_scale).round().max(15.0);
    let height = (size_y * auto_scale).round().max(10.0);

    Ok(Model3D {
        id: String::new(),
        name: stem,
        category: "custom".into(),
        transform: Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        elevation: Some(0.0),
        scale: Some(auto_scale),
        layer_id: None,
        width,
        depth,
        height,
        model_type,
        builtin_id: None,
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: Some(file_data),
        file_name: Some(file_name),
    })
}

pub fn decode_custom_model_mesh(model: &Model3D) -> Option<ParsedGltfMesh> {
    let data = model.file_data.as_deref()?;
    if model.model_type == "glb" {
        let bytes = base64_decode(data).ok()?;
        parse_glb_bytes(&bytes).ok()
    } else if data.trim_start().starts_with('{') {
        parse_gltf_json(data, None, None).ok()
    } else {
        let bytes = base64_decode(data).ok()?;
        let text = String::from_utf8(bytes).ok()?;
        parse_gltf_json(&text, None, None).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_conversion_matches_ts_ratios() {
        assert_eq!(parse_scale_code("spur n"), "H0"); // invalid "spur n" defaults to H0; check valid aliases:
        assert_eq!(parse_scale_code("spur 0"), "0");
        assert_eq!(parse_scale_code("LGB"), "G");
        assert!((scale_factor_from_h0("H0") - 1.0).abs() < 1e-6);
        assert!((scale_factor_from_h0("N") - 87.0 / 160.0).abs() < 1e-6);
        assert_eq!(BUILTIN_MODELS.len(), 16);
        let st = find_builtin_def("station").unwrap();
        let n_st = create_scaled_builtin_model(st, "N", false);
        assert!((n_st.width - 174.0).abs() < 0.2);
    }

    #[test]
    fn base64_roundtrip_and_data_uri() {
        let raw = b"RAILbench-GLB-Test-12345";
        let enc = format!("data:model/gltf-binary;base64,{}", base64_encode(raw));
        let dec = base64_decode(&enc).unwrap();
        assert_eq!(dec, raw);
    }

    #[test]
    fn glb_minimal_triangle_parse_and_bounds() {
        let verts: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 3.0, 1.0]];
        let mut bin = Vec::new();
        for v in verts {
            for c in v {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": bin.len() }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": bin.len() }],
            "accessors": [{
                "bufferView": 0,
                "componentType": 5126,
                "count": 3,
                "type": "VEC3"
            }],
            "meshes": [{
                "primitives": [{ "attributes": { "POSITION": 0 } }]
            }],
            "nodes": [{ "mesh": 0 }],
            "scenes": [{ "nodes": [0] }],
            "scene": 0
        })
        .to_string();
        let mut jbytes = json.into_bytes();
        while jbytes.len() % 4 != 0 {
            jbytes.push(b' ');
        }

        let total_len = (12 + 8 + jbytes.len() + 8 + bin.len()) as u32;
        let mut glb = Vec::with_capacity(total_len as usize);
        glb.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
        glb.extend_from_slice(&2_u32.to_le_bytes());
        glb.extend_from_slice(&total_len.to_le_bytes());
        glb.extend_from_slice(&(jbytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
        glb.extend_from_slice(&jbytes);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
        glb.extend_from_slice(&bin);

        let parsed = parse_glb_bytes(&glb).unwrap();
        assert_eq!(parsed.triangles.len(), 1);
        assert!((parsed.max[0] - 2.0).abs() < 1e-4);
        assert!((parsed.max[1] - 3.0).abs() < 1e-4);
        assert!((parsed.max[2] - 1.0).abs() < 1e-4);

        let model = Model3D {
            id: "m-glb".into(),
            name: "Tri".into(),
            category: "custom".into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 20.0,
            depth: 20.0,
            height: 30.0,
            model_type: "glb".into(),
            builtin_id: None,
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: Some(format!("data:model/gltf-binary;base64,{}", base64_encode(&glb))),
            file_name: Some("tri.glb".into()),
        };
        let dec = decode_custom_model_mesh(&model).unwrap();
        assert_eq!(dec.triangles.len(), 1);
    }
}
