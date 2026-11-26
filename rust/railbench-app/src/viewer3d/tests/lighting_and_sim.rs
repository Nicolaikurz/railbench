use super::*;

#[test]
fn unified_heightfield_generates_3d_mesh_with_skirts_water_and_track_carving() {
    let catalogs = builtin_catalogs();
    let e24188 = find_entry(&catalogs, "24188").expect("24188 in catalog");
    let piece = create_piece(
        "hf-t1".into(),
        e24188,
        Transform {
            x: -94.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let bb = BaseboardConfig {
        id: Some("hf-board".into()),
        name: None,
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1000.0,
        depth: 600.0,
        rotation: None,
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: Some(0.0),
        custom_y: Some(0.0),
        style: "wood".into(),
        corner_radius: None,
        show_dimensions: None,
    };
    let mut layout = Layout {
        name: "UnifiedHF3D".into(),
        scale: "H0".into(),
        pieces: vec![piece],
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
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb]),
        terrain_field: None,
    };

    // Stamp corner mountain and paint a pond
    assert!(crate::terrain_tools::stamp_preset_into_heightfield(&mut layout, "corner", 190.0));
    assert!(crate::terrain_tools::apply_heightfield_brush(
        &mut layout,
        crate::terrain_tools::SculptOp::PaintBiome(railbench_core::terrain::BIOME_WATER),
        Vec2 { x: 240.0, y: 120.0 },
        140.0,
        0.9,
        240.0,
        railbench_core::terrain::BIOME_WATER,
    ));

    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: true,
        ..Default::default()
    };
    let mut cache = Scene3DCache::default();
    let b1 = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    let terr1 = &b1.terrain.as_ref().expect("heightfield emits terrain submesh").1;
    assert!(!terr1.vertices.is_empty());
    assert!(terr1.vertices.iter().all(|v| {
        v.pos[0].is_finite()
            && v.pos[1].is_finite()
            && v.pos[2].is_finite()
            && v.pos[0].abs() <= 500.5
            && v.pos[2].abs() <= 300.5
    }));
    // Water surface MAT_WATER should be present
    assert!(
        terr1
            .vertices
            .iter()
            .any(|v| (v.mat_f32()[0] - MAT_WATER[0]).abs() < 0.04 && v.color_f32()[2] > 0.5),
        "expected glossy water surface vertices from BIOME_WATER"
    );

    // Mutate only heightfield revision via sculpt and confirm Scene3DCache reuses env/boards/tracks!
    assert!(crate::terrain_tools::apply_heightfield_brush(
        &mut layout,
        crate::terrain_tools::SculptOp::Raise,
        Vec2 { x: -180.0, y: 140.0 },
        130.0,
        0.8,
        220.0,
        0,
    ));
    let b2 = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    assert!(std::sync::Arc::ptr_eq(&b1.env.as_ref().unwrap().1, &b2.env.as_ref().unwrap().1));
    assert!(std::sync::Arc::ptr_eq(&b1.boards.as_ref().unwrap().1, &b2.boards.as_ref().unwrap().1));
    assert!(std::sync::Arc::ptr_eq(&b1.tracks.as_ref().unwrap().1, &b2.tracks.as_ref().unwrap().1));
    assert!(!std::sync::Arc::ptr_eq(&b1.terrain.as_ref().unwrap().1, &b2.terrain.as_ref().unwrap().1));
}

#[test]
fn per_layer_heightfield_3d_renders_at_layer_elevation_and_clips_to_plate() {
    let catalogs = builtin_catalogs();
    let bb = BaseboardConfig {
        id: Some("hf-board".into()),
        name: None,
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1000.0,
        depth: 600.0,
        rotation: None,
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: Some(0.0),
        custom_y: Some(0.0),
        style: "wood".into(),
        corner_radius: None,
        show_dimensions: None,
    };
    let mut layout = Layout {
        name: "LayerHF3D".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
        models: None,
        feed_voltage: None,
        catenary_feed_voltage: None,
        catenary_circuit_mode: None,
        loco_current_ma: None,
        loco_count: None,
        train_current_amps: None,
        turnout_lantern_current_ma: None,
        turnout_decoder_current_ma: None,
        layers: Some(vec![
            railbench_core::types::Layer {
                id: "layer-1".into(),
                name: "Base".into(),
                color: None,
                visible: true,
                locked: false,
                height_mm: Some(0.0),
                wood_plate_mode: None,
                wood_plate_thickness_mm: None,
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            },
            railbench_core::types::Layer {
                id: "layer-2".into(),
                name: "Upper".into(),
                color: None,
                visible: true,
                locked: false,
                height_mm: Some(80.0),
                wood_plate_mode: Some("custom".into()),
                wood_plate_thickness_mm: Some(10.0),
                wood_plate_style: Some("wood".into()),
                plates: Some(vec![railbench_core::types::LayerPlate {
                    id: "plate-l2".into(),
                    name: None,
                    baseboard_id: Some("hf-board".into()),
                    shape: Some("rect".into()),
                    custom_polygon: None,
                    triangle_type: None,
                    l_wing_width: None,
                    l_wing_depth: None,
                    l_side: None,
                    u_left_wing_width: None,
                    u_right_wing_width: None,
                    u_back_depth: None,
                    t_bar_depth: None,
                    t_stem_width: None,
                    rotation: Some(0.0),
                    width: 400.0,
                    depth: 300.0,
                    x: Some(300.0),
                    y: Some(150.0),
                    thickness_mm: Some(10.0),
                    position_mode: Some("custom".into()),
                    style: Some("wood".into()),
                }]),
                terrain_field: None,
            },
        ]),
        active_layer_id: Some("layer-2".into()),
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb]),
        terrain_field: None,
    };

    assert!(crate::terrain_tools::apply_heightfield_brush(
        &mut layout,
        crate::terrain_tools::SculptOp::Raise,
        Vec2 { x: 0.0, y: 0.0 },
        180.0,
        1.0,
        240.0,
        0,
    ));

    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        show_wood_plates: true,
        ..Default::default()
    };
    let mut cache = Scene3DCache::default();
    let batch = update_scene_3d_cache(&layout, &catalogs, &opts, &mut cache);
    let terr = &batch.terrain.as_ref().expect("layer-2 heightfield emits 3D submesh").1;
    assert!(!terr.vertices.is_empty());
    assert!(
        terr.vertices.iter().all(|v| v.pos[0].abs() <= 200.5 && v.pos[2].abs() <= 150.5),
        "layer-2 3D terrain should clip to its 400x300 plate"
    );
    assert!(
        terr.vertices.iter().any(|v| v.pos[1] > 85.0),
        "layer-2 3D terrain vertices should rise above its 80mm layer height"
    );
}

#[test]
fn shadow_ortho_and_sun_view_proj_cover_layout_within_ndc_bounds() {
    let ortho = mat4_ortho(-100.0, 100.0, -100.0, 100.0, 100.0, 1100.0);
    // In right-handed view space, -Z points forward: -near maps to z=0.0, -far maps to z=1.0
    let z_near = ortho[2][2] * (-100.0) + ortho[3][2];
    let z_far = ortho[2][2] * (-1100.0) + ortho[3][2];
    assert!(z_near.abs() < 1e-5, "expected near depth 0.0, got {z_near}");
    assert!((z_far - 1.0).abs() < 1e-5, "expected far depth 1.0, got {z_far}");

    let bb = BaseboardConfig {
        id: Some("shadow-board".into()),
        name: None,
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 1200.0,
        depth: 800.0,
        rotation: None,
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: Some(0.0),
        custom_y: Some(0.0),
        style: "wood".into(),
        corner_radius: None,
        show_dimensions: None,
    };
    let layout = Layout {
        name: "ShadowTest".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
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
        terrain_field: None,
    };
    let boards = vec![bb];
    let (center, half_extent) = compute_shadow_bounds(&layout, &boards);
    assert!(half_extent >= 650.0);

    for sun_dir in [
        [0.6_f32, 0.8, 0.3],
        [-0.8, 0.35, -0.4],
        [0.0, 1.0, 0.0],
    ] {
        let m = compute_sun_view_proj(sun_dir, center, half_extent);
        for corner in [
            [-600.0_f32, 0.0, -400.0],
            [600.0, 0.0, -400.0],
            [600.0, 0.0, 400.0],
            [-600.0, 0.0, 400.0],
        ] {
            let cx = m[0][0] * corner[0] + m[1][0] * corner[1] + m[2][0] * corner[2] + m[3][0];
            let cy = m[0][1] * corner[0] + m[1][1] * corner[1] + m[2][1] * corner[2] + m[3][1];
            let cz = m[0][2] * corner[0] + m[1][2] * corner[1] + m[2][2] * corner[2] + m[3][2];
            let cw = m[0][3] * corner[0] + m[1][3] * corner[1] + m[2][3] * corner[2] + m[3][3];
            assert!((cw - 1.0).abs() < 1e-5);
            assert!((-1.05..=1.05).contains(&cx), "ndc x out of range: {cx}");
            assert!((-1.05..=1.05).contains(&cy), "ndc y out of range: {cy}");
            assert!((0.0..=1.0).contains(&cz), "ndc z out of range: {cz}");
        }
    }
    assert!((Scene3DBundle::default().shadow_half_extent - 1600.0).abs() < 1e-4);
}

#[test]
fn textured_custom_glb_model_emits_textured_submesh_in_3d_scene() {
    let img = image::RgbaImage::from_fn(2, 2, |x, y| {
        image::Rgba([(x * 255) as u8, (y * 255) as u8, 128, 255])
    });
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .unwrap();

    let verts: [[f32; 3]; 3] = [[-10.0, 0.0, -10.0], [10.0, 0.0, -10.0], [0.0, 20.0, 10.0]];
    let uvs: [[f32; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
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
            "pbrMetallicRoughness": { "baseColorTexture": { "index": 0 } }
        }],
        "meshes": [{
            "primitives": [{ "attributes": { "POSITION": 0, "TEXCOORD_0": 1 }, "material": 0 }]
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

    let data_uri = format!(
        "data:model/gltf-binary;base64,{}",
        crate::model_tools::base64_encode(&glb)
    );
    let model = Model3D {
        id: "m-tex".into(),
        name: "Textured GLB".into(),
        category: "custom".into(),
        transform: railbench_core::types::Transform {
            x: 100.0,
            y: 50.0,
            rotation: 0.0,
        },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 20.0,
        depth: 20.0,
        height: 20.0,
        model_type: "glb".into(),
        builtin_id: None,
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: Some(std::sync::Arc::from(data_uri)),
        file_name: Some("tex.glb".into()),
    };

    let layout = Layout {
        name: "TexGlbTest".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
        models: Some(vec![model]),
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
        terrain_field: None,
    };
    let opts = Scene3DOptions {
        env_mode: Viewer3DEnv::Minimal,
        ..Default::default()
    };
    let mut cache = Scene3DCache::default();
    let bundle = update_scene_3d_cache(&layout, &[], &opts, &mut cache);
    let tracks_mesh = &bundle.tracks.as_ref().expect("tracks/models submesh built").1;
    assert_eq!(tracks_mesh.textured_meshes.len(), 1);
    let tm = &tracks_mesh.textured_meshes[0];
    assert_eq!(tm.tex_width, 2);
    assert_eq!(tm.tex_height, 2);
    assert_eq!(tm.tex_rgba.len(), 16);
    assert_eq!(tm.vertices.len(), 3);
    assert_eq!(tm.indices.len(), 3);
    assert!(bundle.triangle_count() >= 1);
    assert!(bundle.gpu_buffer_bytes() >= 3 * std::mem::size_of::<TexturedVertex3D>());
}

#[test]
fn minimal_cad_env_emits_no_grid_geometry() {
    let layout = Layout {
        name: "EmptyMinimal".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
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
        baseboards: Some(Vec::new()),
        terrain_field: None,
    };

    for dark_mode in [true, false] {
        let mesh = build_scene_mesh_3d(
            &layout,
            &[],
            &Scene3DOptions {
                env_mode: Viewer3DEnv::Minimal,
                dark_mode,
                ..Default::default()
            },
        );
        assert!(
            mesh.vertices.is_empty(),
            "Minimal CAD env should not emit grid vertices (dark_mode={dark_mode})"
        );
        assert!(
            mesh.indices.is_empty(),
            "Minimal CAD env should not emit grid indices (dark_mode={dark_mode})"
        );
    }
}

#[test]
fn speedometer_needle_angle_clamps_and_sweeps_monotonically() {
    let a0 = speedometer_needle_angle(0.0, 120.0);
    let amid = speedometer_needle_angle(60.0, 120.0);
    let amax = speedometer_needle_angle(120.0, 120.0);
    let aover = speedometer_needle_angle(220.0, 120.0);

    assert!((a0 - (-220.0_f32).to_radians()).abs() < 1e-4);
    assert!((amid - (-90.0_f32).to_radians()).abs() < 1e-4);
    assert!((amax - 40.0_f32.to_radians()).abs() < 1e-4);
    assert!(a0 < amid && amid < amax);
    assert!((aover - amax).abs() < 1e-4);
}

#[test]
fn uncoupler_24997_generates_3d_ramp_that_lifts_when_active() {
    let cats = builtin_catalogs();
    let e24997 = find_entry(&cats, "24997").expect("Maerklin C-Gleis 24997 in catalog");
    assert!(crate::catalog::is_uncoupler_entry(e24997));

    let piece = create_piece("u1".into(), e24997, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let layout = Layout {
        name: "Uncoupler3D".into(),
        scale: "H0".into(),
        pieces: vec![piece],
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
        terrain_field: None,
    };

    let idle_mesh = build_scene_mesh_3d(&layout, &cats, &Scene3DOptions::default());
    let mut active_opts = Scene3DOptions::default();
    active_opts.active_uncouplers.insert("u1".to_string());
    let active_mesh = build_scene_mesh_3d(&layout, &cats, &active_opts);

    let max_y_idle = idle_mesh
        .vertices
        .iter()
        .filter(|v| v.pos[0] >= 10.0 && v.pos[0] <= 80.0 && v.pos[2].abs() <= 6.0 && v.pos[1] < 40.0)
        .map(|v| v.pos[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y_active = active_mesh
        .vertices
        .iter()
        .filter(|v| v.pos[0] >= 10.0 && v.pos[0] <= 80.0 && v.pos[2].abs() <= 6.0 && v.pos[1] < 40.0)
        .map(|v| v.pos[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max_y_active > max_y_idle + 1.0,
        "raised uncoupler ramp {max_y_active} must sit higher than idle {max_y_idle}"
    );
}
