use super::*;

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
pub struct DecodedTexture {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: std::sync::Arc<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct CustomMeshTri {
    pub positions: [[f32; 3]; 3],
    pub normals: [[f32; 3]; 3],
    pub uvs: [[f32; 2]; 3],
    pub color: [f32; 3],
    pub texture_idx: Option<usize>,
    pub roughness: f32,
    pub metallic: f32,
}

#[derive(Debug, Clone)]
pub struct ParsedGltfMesh {
    pub triangles: Vec<CustomMeshTri>,
    pub textures: Vec<DecodedTexture>,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

pub(super) fn mat4_identity() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Column-major 4x4 matrix multiply `a * b`.
pub(super) fn mat4_mul_col(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
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

pub(super) fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

pub(super) fn transform_normal(m: &[f32; 16], n: [f32; 3]) -> [f32; 3] {
    let rx = m[0] * n[0] + m[4] * n[1] + m[8] * n[2];
    let ry = m[1] * n[0] + m[5] * n[1] + m[9] * n[2];
    let rz = m[2] * n[0] + m[6] * n[1] + m[10] * n[2];
    let len = (rx * rx + ry * ry + rz * rz).sqrt().max(1e-6);
    [rx / len, ry / len, rz / len]
}

pub(super) fn node_local_matrix(node: &Value) -> [f32; 16] {
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

pub(super) fn read_u16_le(buf: &[u8], off: usize) -> Option<u16> {
    buf.get(off..off + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

pub(super) fn read_i16_le(buf: &[u8], off: usize) -> Option<i16> {
    buf.get(off..off + 2).map(|s| i16::from_le_bytes([s[0], s[1]]))
}

pub(super) fn read_u32_le(buf: &[u8], off: usize) -> Option<u32> {
    buf.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub(super) fn read_f32_le(buf: &[u8], off: usize) -> Option<f32> {
    buf.get(off..off + 4)
        .map(|s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub(super) fn component_size(comp_type: u64) -> usize {
    match comp_type {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        5125 | 5126 => 4,
        _ => 4,
    }
}

pub(super) fn type_count(type_str: &str) -> usize {
    match type_str {
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" | "MAT2" => 4,
        "MAT3" => 9,
        "MAT4" => 16,
        _ => 1,
    }
}

pub(super) fn read_component_as_f32(buf: &[u8], off: usize, comp_type: u64, normalized: bool) -> Option<f32> {
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

fn accessor_slice<'a>(
    doc: &'a Value,
    buffers: &'a [Vec<u8>],
    accessor_idx: usize,
) -> Option<(&'a [u8], usize, u64, bool, usize, usize, usize, usize)> {
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
    Some((raw, count, comp_type, norm, tcount, bv_off + acc_off, comp_sz, stride))
}

pub(super) fn read_accessor_vec3(doc: &Value, buffers: &[Vec<u8>], accessor_idx: usize) -> Option<Vec<[f32; 3]>> {
    let (raw, count, comp_type, norm, tcount, base_off, comp_sz, stride) =
        accessor_slice(doc, buffers, accessor_idx)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = base_off + i * stride;
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

pub(super) fn read_accessor_vec2(doc: &Value, buffers: &[Vec<u8>], accessor_idx: usize) -> Option<Vec<[f32; 2]>> {
    let (raw, count, comp_type, norm, tcount, base_off, comp_sz, stride) =
        accessor_slice(doc, buffers, accessor_idx)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = base_off + i * stride;
        let u = read_component_as_f32(raw, base, comp_type, norm)?;
        let v = if tcount >= 2 {
            read_component_as_f32(raw, base + comp_sz, comp_type, norm).unwrap_or(0.0)
        } else {
            0.0
        };
        out.push([u, v]);
    }
    Some(out)
}

fn hash_texture_rgba(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &b in w
        .to_le_bytes()
        .iter()
        .chain(h.to_le_bytes().iter())
        .chain(rgba.iter())
    {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

fn decode_gltf_images(
    doc: &Value,
    buffers: &[Vec<u8>],
    base_dir: Option<&Path>,
) -> Vec<Option<DecodedTexture>> {
    let Some(images) = doc.get("images").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let bvs = doc.get("bufferViews").and_then(|v| v.as_array());
    images
        .iter()
        .map(|img| {
            let owned: Vec<u8>;
            let raw_bytes: &[u8] = if let Some(bv_idx) = img.get("bufferView").and_then(|v| v.as_u64()) {
                let bv = bvs?.get(bv_idx as usize)?;
                let buf_idx = bv.get("buffer")?.as_u64()? as usize;
                let off = bv.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let len = bv.get("byteLength")?.as_u64()? as usize;
                buffers.get(buf_idx)?.get(off..off.checked_add(len)?)?
            } else if let Some(uri) = img.get("uri").and_then(|v| v.as_str()) {
                if uri.starts_with("data:") {
                    owned = base64_decode(uri).ok()?;
                    &owned
                } else {
                    let dir = base_dir?;
                    owned = std::fs::read(dir.join(uri)).ok()?;
                    &owned
                }
            } else {
                return None;
            };

            let mut rgba_img = image::load_from_memory(raw_bytes).ok()?.to_rgba8();
            let (w, h) = rgba_img.dimensions();
            if w > 2048 || h > 2048 {
                let scale = 2048.0_f64 / (w.max(h) as f64);
                let nw = ((w as f64) * scale).round().max(1.0) as u32;
                let nh = ((h as f64) * scale).round().max(1.0) as u32;
                rgba_img = image::imageops::resize(
                    &rgba_img,
                    nw,
                    nh,
                    image::imageops::FilterType::Triangle,
                );
            }
            let (width, height) = rgba_img.dimensions();
            if width == 0 || height == 0 {
                return None;
            }
            let rgba = rgba_img.into_raw();
            let id = hash_texture_rgba(width, height, &rgba);
            Some(DecodedTexture {
                id,
                width,
                height,
                rgba: std::sync::Arc::new(rgba),
            })
        })
        .collect()
}

pub(super) fn read_accessor_indices(doc: &Value, buffers: &[Vec<u8>], accessor_idx: usize) -> Option<Vec<usize>> {
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
    let gltf_textures = doc.get("textures").and_then(|v| v.as_array());

    let mut textures: Vec<DecodedTexture> = Vec::new();
    let mut image_to_tex_idx: Vec<Option<usize>> = Vec::new();
    for opt_tex in decode_gltf_images(&doc, &buffers, base_dir) {
        if let Some(tex) = opt_tex {
            let idx = textures.len();
            textures.push(tex);
            image_to_tex_idx.push(Some(idx));
        } else {
            image_to_tex_idx.push(None);
        }
    }

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
            let texcoords = attrs
                .get("TEXCOORD_0")
                .and_then(|v| v.as_u64())
                .and_then(|idx| read_accessor_vec2(&doc, &buffers, idx as usize));
            let vcolors = attrs
                .get("COLOR_0")
                .and_then(|v| v.as_u64())
                .and_then(|idx| read_accessor_vec3(&doc, &buffers, idx as usize));
            let indices: Vec<usize> = prim
                .get("indices")
                .and_then(|v| v.as_u64())
                .and_then(|idx| read_accessor_indices(&doc, &buffers, idx as usize))
                .unwrap_or_else(|| (0..positions.len()).collect());

            let mat_obj = prim
                .get("material")
                .and_then(|v| v.as_u64())
                .and_then(|mi| materials.and_then(|ms| ms.get(mi as usize)));
            let pbr = mat_obj.and_then(|mat| mat.get("pbrMetallicRoughness"));
            let texture_idx = if texcoords.is_some() {
                pbr.and_then(|p| p.get("baseColorTexture"))
                    .and_then(|bt| bt.get("index"))
                    .and_then(|v| v.as_u64())
                    .and_then(|ti| gltf_textures.and_then(|ts| ts.get(ti as usize)))
                    .and_then(|t| t.get("source"))
                    .and_then(|v| v.as_u64())
                    .and_then(|si| image_to_tex_idx.get(si as usize).copied().flatten())
            } else {
                None
            };
            let default_col = if texture_idx.is_some() {
                [1.0, 1.0, 1.0]
            } else {
                [0.76, 0.76, 0.78]
            };
            let mat_col = pbr
                .and_then(|p| p.get("baseColorFactor"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    [
                        arr.first().and_then(|v| v.as_f64()).unwrap_or(default_col[0] as f64) as f32,
                        arr.get(1).and_then(|v| v.as_f64()).unwrap_or(default_col[1] as f64) as f32,
                        arr.get(2).and_then(|v| v.as_f64()).unwrap_or(default_col[2] as f64) as f32,
                    ]
                })
                .unwrap_or(default_col);
            let roughness = pbr
                .and_then(|p| p.get("roughnessFactor"))
                .and_then(|v| v.as_f64())
                .map(|v| (v as f32).clamp(0.04, 1.0))
                .unwrap_or(0.72);
            let metallic = pbr
                .and_then(|p| p.get("metallicFactor"))
                .and_then(|v| v.as_f64())
                .map(|v| (v as f32).clamp(0.0, 1.0))
                .unwrap_or(0.08);

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
                let uvs = if let Some(ref tc) = texcoords {
                    if i0 < tc.len() && i1 < tc.len() && i2 < tc.len() {
                        [tc[i0], tc[i1], tc[i2]]
                    } else {
                        [[0.0, 0.0]; 3]
                    }
                } else {
                    [[0.0, 0.0]; 3]
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
                    uvs,
                    color: col,
                    texture_idx,
                    roughness,
                    metallic,
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

    Ok(ParsedGltfMesh {
        triangles,
        textures,
        min,
        max,
    })
}

pub(super) fn inline_external_gltf_buffers(json_str: &str, base_dir: &Path) -> String {
    let Ok(mut doc) = serde_json::from_str::<Value>(json_str) else {
        return json_str.to_string();
    };
    let mut modified = false;
    for (section, is_image) in [("buffers", false), ("images", true)] {
        if let Some(entries) = doc.get_mut(section).and_then(|v| v.as_array_mut()) {
            for item in entries {
                let Some(uri) = item.get("uri").and_then(|v| v.as_str()).map(|s| s.to_string()) else {
                    continue;
                };
                if uri.starts_with("data:") {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(base_dir.join(&uri)) {
                    let mime = if !is_image {
                        "application/octet-stream"
                    } else if uri.to_ascii_lowercase().ends_with(".jpg")
                        || uri.to_ascii_lowercase().ends_with(".jpeg")
                        || bytes.starts_with(&[0xFF, 0xD8])
                    {
                        "image/jpeg"
                    } else {
                        "image/png"
                    };
                    let embedded = format!("data:{mime};base64,{}", base64_encode(&bytes));
                    if let Some(obj) = item.as_object_mut() {
                        obj.insert("uri".into(), Value::String(embedded));
                        modified = true;
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
        file_data: Some(std::sync::Arc::from(file_data)),
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

