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
        file_data: Some(std::sync::Arc::from(format!(
            "data:model/gltf-binary;base64,{}",
            base64_encode(&glb)
        ))),
        file_name: Some("tri.glb".into()),
    };
    let dec = decode_custom_model_mesh(&model).unwrap();
    assert_eq!(dec.triangles.len(), 1);
}

#[test]
fn glb_textured_triangle_decodes_uvs_and_embedded_png_texture() {
    let img = image::RgbaImage::from_fn(2, 2, |x, y| {
        image::Rgba([(x * 200) as u8, (y * 200) as u8, 64, 255])
    });
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .unwrap();

    let verts: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 3.0, 1.0]];
    let uvs: [[f32; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let mut bin = Vec::new();
    for v in verts {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let uv_off = bin.len();
    for uv in uvs {
        for c in uv {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let img_off = bin.len();
    bin.extend_from_slice(&png_bytes);
    while bin.len() % 4 != 0 {
        bin.push(0);
    }

    let json = serde_json::json!({
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": bin.len() }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": uv_off, "byteLength": 24 },
            { "buffer": 0, "byteOffset": img_off, "byteLength": png_bytes.len() }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }
        ],
        "images": [{ "bufferView": 2, "mimeType": "image/png" }],
        "textures": [{ "source": 0 }],
        "materials": [{
            "pbrMetallicRoughness": {
                "baseColorTexture": { "index": 0 },
                "roughnessFactor": 0.42,
                "metallicFactor": 0.25
            }
        }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0, "TEXCOORD_0": 1 },
                "material": 0
            }]
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
    assert_eq!(parsed.textures.len(), 1);
    assert_eq!(parsed.textures[0].width, 2);
    assert_eq!(parsed.textures[0].height, 2);
    assert_eq!(parsed.textures[0].rgba.len(), 16);
    assert_eq!(parsed.triangles.len(), 1);
    let tri = &parsed.triangles[0];
    assert_eq!(tri.texture_idx, Some(0));
    assert_eq!(tri.uvs, uvs);
    assert_eq!(tri.color, [1.0, 1.0, 1.0]);
    assert!((tri.roughness - 0.42).abs() < 1e-5);
    assert!((tri.metallic - 0.25).abs() < 1e-5);
}
