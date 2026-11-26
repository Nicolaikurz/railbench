use super::*;

#[test]
fn canvas_cache_populates_and_finds_pieces() {
    let cats = builtin_catalogs();
    let entry = find_entry(&cats, "24188").unwrap();
    let p0 = create_piece("p0".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    let p1 = create_piece("p1".into(), entry, Transform { x: 1000.0, y: 500.0, rotation: 0.0 }, None);
    let mut layout = make_layout_with_pieces(vec![p0, p1]);
    let mut cache = Canvas2DCache::default();

    cache.ensure_fresh(&layout, &cats, 1);
    assert_eq!(cache.pieces.len(), 2);
    assert_eq!(cache.piece_id_to_idx.get("p1").copied(), Some(1));
    assert_eq!(
        find_piece_at(&layout, &cache, Vec2 { x: 50.0, y: 5.0 }, 20.0, false).as_deref(),
        Some("p0")
    );
    assert_eq!(
        find_piece_at(&layout, &cache, Vec2 { x: 1050.0, y: 505.0 }, 20.0, false).as_deref(),
        Some("p1")
    );
    assert!(find_piece_at(&layout, &cache, Vec2 { x: 500.0, y: -300.0 }, 20.0, false).is_none());

    // Bumping layout_version invalidates and rebuilds cache
    let p2 = create_piece("p2".into(), entry, Transform { x: -400.0, y: -200.0, rotation: 0.0 }, None);
    layout.pieces.push(p2);
    cache.ensure_fresh(&layout, &cats, 2);
    assert_eq!(cache.pieces.len(), 3);
    assert_eq!(
        find_piece_at(&layout, &cache, Vec2 { x: -350.0, y: -200.0 }, 20.0, false).as_deref(),
        Some("p2")
    );
}

#[test]
fn spatial_hash_snap_matches_candidates_and_rejects_cross_type() {
    let cats = builtin_catalogs();
    let rail_entry = find_entry(&cats, "24188").unwrap();
    let road_entry = cats
        .iter()
        .find(|c| c.is_road == Some(true))
        .and_then(|c| c.entries.first())
        .unwrap();

    let anchor_rail = create_piece("r0".into(), rail_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
    // Dragged rail near connector 1 of anchor_rail (x = 188.3)
    let drag_rail = create_piece("r1".into(), rail_entry, Transform { x: 196.0, y: 6.0, rotation: 0.0 }, None);
    // Road right on top of drag_rail — must NEVER snap track to road!
    let near_road = create_piece("st0".into(), road_entry, Transform { x: 196.0, y: 6.0, rotation: 0.0 }, None);

    let layout = make_layout_with_pieces(vec![anchor_rail, drag_rail, near_road]);
    let mut sel = HashSet::new();
    sel.insert("r1".to_string());

    let cand = find_best_snap_candidate(&layout, &cats, &sel, 50.0).expect("should find rail snap");
    assert!((cand.dst_x - 188.3).abs() < 1.0);
    let d = (cand.src_x - cand.dst_x).hypot(cand.src_y - cand.dst_y);
    assert!(d < 15.0);

    // Selecting only the road when no other road exists must find None
    sel.clear();
    sel.insert("st0".to_string());
    assert!(find_best_snap_candidate(&layout, &cats, &sel, 50.0).is_none());
}

#[test]
fn model_top_view_mesh_projects_3d_viewer_triangles_sorted_by_elevation() {
    for &(bid, w, d, h) in &[
        ("station", 320.0, 140.0, 120.0),
        ("platform", 320.0, 54.0, 55.0),
        ("coaling-station", 180.0, 75.0, 125.0),
        ("water-tower", 85.0, 85.0, 170.0),
        ("diesel-fuel-station", 170.0, 65.0, 68.0),
    ] {
        let m = Model3D {
            id: format!("m-{bid}"),
            name: bid.into(),
            category: "railway".into(),
            transform: Transform {
                x: 100.0,
                y: 200.0,
                rotation: 30.0,
            },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: w,
            depth: d,
            height: h,
            model_type: "builtin".into(),
            builtin_id: Some(bid.into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let mesh = build_model_top_view_mesh_2d(&m);
        assert!(
            mesh.tris.len() > 20,
            "{bid} top-down 2D mesh should contain detailed 3D triangles, got {}",
            mesh.tris.len()
        );
    }
}

#[test]
fn track_snapping_requires_same_level_and_marks_level_mismatch_yellow() {
    let mut cats = builtin_catalogs();
    let rail_entry = find_entry(&cats, "24188").unwrap();
    let mut anchor_rail = create_piece(
        "r0".into(),
        rail_entry,
        Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    // Elevate anchor_rail to 50 mm while drag_rail stays at 0 mm
    anchor_rail.elevation = Some(railbench_core::types::Elevation {
        start: 50.0,
        end: 50.0,
    });
    let drag_rail = create_piece(
        "r1".into(),
        rail_entry,
        Transform {
            x: 188.3,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );

    let mut layout = make_layout_with_pieces(vec![anchor_rail, drag_rail]);
    let mut sel = HashSet::new();
    sel.insert("r1".to_string());

    // Strict same-level snap must reject the 50 mm height mismatch
    assert!(find_best_snap_candidate(&layout, &cats, &sel, 50.0).is_none());
    assert!(!snap_selected_pieces(&mut layout, &cats, &sel, 50.0));

    // Any-level preview candidate detects the nearby connector and marks `same_level = false` (yellow ring)
    let preview = find_best_snap_candidate_any_level(&layout, &cats, &sel, 50.0)
        .expect("should detect level-mismatched connector for yellow preview");
    assert!(!preview.same_level);

    // 2D cache marks both overlapping different-level connectors in `level_mismatch_conns` (yellow snap point)
    let mut cache = Canvas2DCache::default();
    cache.ensure_fresh(&layout, &cats, 1);
    assert!(cache.level_mismatch_conns.contains(&(0, 1)));
    assert!(cache.level_mismatch_conns.contains(&(1, 0)));

    // Once both tracks are on the same level (50 mm), snapping succeeds and yellow mismatch clears
    layout.pieces[1].elevation = Some(railbench_core::types::Elevation {
        start: 50.0,
        end: 50.0,
    });
    assert!(find_best_snap_candidate(&layout, &cats, &sel, 50.0).is_some());
    cache.ensure_fresh(&layout, &cats, 2);
    assert!(cache.level_mismatch_conns.is_empty());

    // Add synthetic straight catalog entry so overhead & door checks use known geometry
    let straight_entry = railbench_core::types::CatalogEntry {
        id: "test-c2d-straight-180".to_string(),
        name: "Test Straight 180".to_string(),
        name_en: None,
        kind: "straight".to_string(),
        category: "Track".to_string(),
        description: String::new(),
        length: Some(180.0),
        radius: None,
        angle: None,
        branch_radius: None,
        branch_angle: None,
        branch_length: None,
        gauge: Some(16.5),
        has_roadbed: None,
        is_road: None,
        road_width: None,
        local_connectors: None,
    };
    cats.push(railbench_core::types::Catalog {
        id: "test-c2d-cat".to_string(),
        name: "Test C2D".to_string(),
        scale: "H0".to_string(),
        manufacturer: None,
        system: None,
        gauge: Some(16.5),
        is_road: None,
        has_roadbed: None,
        conductors: None,
        entries: vec![straight_entry.clone()],
    });

    // (a) Overhead track at 80 mm above an open 0 mm connector marks the lower connector yellow
    let p_low_idx = layout.pieces.len();
    let p_low = crate::catalog::create_piece(
        "p_low".to_string(),
        &straight_entry,
        Transform {
            x: 2000.0,
            y: 2000.0,
            rotation: 0.0,
        },
        None,
    );
    let mut p_high = crate::catalog::create_piece(
        "p_high".to_string(),
        &straight_entry,
        Transform {
            x: 1910.0,
            y: 2004.0,
            rotation: 0.0,
        },
        None,
    );
    p_high.elevation = Some(railbench_core::types::Elevation {
        start: 80.0,
        end: 80.0,
    });
    layout.pieces.push(p_low);
    layout.pieces.push(p_high);
    cache.ensure_fresh(&layout, &cats, 3);
    assert!(
        cache.level_mismatch_conns.contains(&(p_low_idx, 0)),
        "open 0mm connector under 80mm overhead segment should be marked yellow"
    );

    // (b) Engine shed door at 0 mm beneath an 80 mm track connector yields door_state == 2 and yellow track conn
    let shed = railbench_core::types::Model3D {
        id: "shed-1".to_string(),
        name: "Engine Shed".to_string(),
        category: "Buildings".to_string(),
        transform: Transform {
            x: 3000.0,
            y: 3000.0,
            rotation: 0.0,
        },
        elevation: None,
        scale: None,
        layer_id: None,
        width: 220.0,
        depth: 110.0,
        height: 85.0,
        model_type: "builtin".to_string(),
        builtin_id: Some("engine-shed".to_string()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let mut shed_doors = Vec::new();
    crate::building_snap::collect_model_door_positions(&layout, &shed, &mut shed_doors);
    let (dx, dy, _) = shed_doors[0];
    let p_door_idx = layout.pieces.len();
    let mut p_door_high = crate::catalog::create_piece(
        "p_door_high".to_string(),
        &straight_entry,
        Transform {
            x: dx,
            y: dy,
            rotation: 0.0,
        },
        None,
    );
    p_door_high.elevation = Some(railbench_core::types::Elevation {
        start: 80.0,
        end: 80.0,
    });
    layout.models.get_or_insert_with(Vec::new).push(shed);
    layout.pieces.push(p_door_high);

    cache.ensure_fresh(&layout, &cats, 4);
    assert!(
        cache.level_mismatch_conns.contains(&(p_door_idx, 0)),
        "80mm track connector over 0mm shed door should be marked yellow"
    );
    assert!(
        cache
            .visible_doors
            .iter()
            .any(|&(vx, vy, st)| (vx - dx).hypot(vy - dy) < 1.0 && st == 2),
        "0mm shed door under 80mm track connector should have door_state == 2 (yellow)"
    );
}

#[test]
fn layer_protection_restricts_hit_testing_to_active_layer_until_disabled() {
    let cats = builtin_catalogs();
    let entry = find_entry(&cats, "24188").unwrap();
    let p_l1 = create_piece(
        "p-l1".into(),
        entry,
        Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        Some("layer-1".into()),
    );
    let p_l2 = create_piece(
        "p-l2".into(),
        entry,
        Transform {
            x: 0.0,
            y: 200.0,
            rotation: 0.0,
        },
        Some("layer-2".into()),
    );
    let mut layout = make_layout_with_pieces(vec![p_l1, p_l2]);
    layout.active_layer_id = Some("layer-2".into());
    let mut cache = Canvas2DCache::default();
    cache.ensure_fresh(&layout, &cats, 1);

    // With layer_protection = true, only layer-2 piece can be picked/moved
    assert!(
        find_piece_at(&layout, &cache, Vec2 { x: 50.0, y: 0.0 }, 20.0, true).is_none(),
        "inactive layer-1 piece must be protected when layer_protection is enabled"
    );
    assert_eq!(
        find_piece_at(&layout, &cache, Vec2 { x: 50.0, y: 200.0 }, 20.0, true).as_deref(),
        Some("p-l2")
    );

    // With layer_protection = false, both layers can be picked/moved
    assert_eq!(
        find_piece_at(&layout, &cache, Vec2 { x: 50.0, y: 0.0 }, 20.0, false).as_deref(),
        Some("p-l1")
    );
}

#[test]
fn turntable_and_transfer_table_2d_top_view_bounds_and_hit_testing() {
    let cats = builtin_catalogs();
    let tt_entry = find_entry(&cats, "74862").expect("74862 turntable");
    let tf_entry = find_entry(&cats, "72941").expect("72941 transfer table");

    let tt_piece = create_piece(
        "tt1".into(),
        tt_entry,
        Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    );
    let tf_piece = create_piece(
        "tf1".into(),
        tf_entry,
        Transform {
            x: 800.0,
            y: 100.0,
            rotation: 30.0,
        },
        None,
    );

    let tt_mesh = build_special_piece_top_view_mesh_2d(&tt_piece, tt_entry);
    let tf_mesh = build_special_piece_top_view_mesh_2d(&tf_piece, tf_entry);
    assert!(
        tt_mesh.tris.len() > 400,
        "turntable 2D top-view mesh too small: {}",
        tt_mesh.tris.len()
    );
    assert!(
        tf_mesh.tris.len() > 400,
        "transfer-table 2D top-view mesh too small: {}",
        tf_mesh.tris.len()
    );

    let (mut min_tx, mut max_tx, mut min_ty, mut max_ty) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for (pts, _) in &tt_mesh.tris {
        for pt in pts {
            min_tx = min_tx.min(pt[0] as f64);
            max_tx = max_tx.max(pt[0] as f64);
            min_ty = min_ty.min(pt[1] as f64);
            max_ty = max_ty.max(pt[1] as f64);
        }
    }
    assert!((min_tx - -189.0).abs() < 2.0 && (max_tx - 189.0).abs() < 2.0);
    assert!((min_ty - -189.0).abs() < 2.0 && (max_ty - 189.0).abs() < 2.0);

    let (mut min_fx, mut max_fx, mut min_fy, mut max_fy) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for (pts, _) in &tf_mesh.tris {
        for pt in pts {
            min_fx = min_fx.min(pt[0] as f64);
            max_fx = max_fx.max(pt[0] as f64);
            min_fy = min_fy.min(pt[1] as f64);
            max_fy = max_fy.max(pt[1] as f64);
        }
    }
    assert!(min_fx >= -2.0 && (max_fx - 360.0).abs() < 2.0);
    assert!((min_fy - -55.2).abs() < 2.0 && (max_fy - 364.8).abs() < 2.0);

    assert_eq!(piece_branches_world(&tt_piece, Some(tt_entry)).len(), 31);
    assert_eq!(piece_branches_world(&tf_piece, Some(tf_entry)).len(), 11);

    let layout = make_layout_with_pieces(vec![tt_piece, tf_piece]);
    let mut cache = Canvas2DCache::default();
    cache.ensure_fresh(&layout, &cats, 1);

    // Hit inside turntable pit off the bridge
    assert_eq!(
        find_piece_at(&layout, &cache, Vec2 { x: 0.0, y: 80.0 }, 15.0, false).as_deref(),
        Some("tt1")
    );
    // Hit inside rotated transfer-table pit off bridge & stubs: local (180, 30) rotated by 30 deg around (800, 100)
    let tf_inside_rp = rotate(Vec2 { x: 180.0, y: 30.0 }, 30.0);
    assert_eq!(
        find_piece_at(
            &layout,
            &cache,
            Vec2 {
                x: 800.0 + tf_inside_rp.x,
                y: 100.0 + tf_inside_rp.y,
            },
            15.0,
            false
        )
        .as_deref(),
        Some("tf1")
    );
    // Outside both footprints
    assert!(find_piece_at(&layout, &cache, Vec2 { x: -400.0, y: -400.0 }, 15.0, false).is_none());
}

#[test]
fn canvas_2d_cache_prunes_stale_model_protos_and_shares_arcs() {
    assert_eq!(std::mem::size_of::<([[f32; 2]; 3], [Color32; 3])>(), 36);
    let cats = crate::catalog::builtin_catalogs();
    let mk_station = |id: &str, x: f64, w: f64| Model3D {
        id: id.into(),
        name: "Station".into(),
        category: "buildings".into(),
        transform: Transform { x, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: w,
        depth: 120.0,
        height: 110.0,
        model_type: "builtin".into(),
        builtin_id: Some("station".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };

    let mut layout = make_layout_with_pieces(vec![]);
    layout.models = Some(vec![mk_station("m1", 0.0, 200.0), mk_station("m2", 400.0, 200.0)]);

    let mut cache = Canvas2DCache::default();
    cache.ensure_fresh(&layout, &cats, 1);
    assert_eq!(cache.model_proto_cache.len(), 1);
    assert_eq!(cache.model_top_meshes.len(), 2);
    assert!(
        Arc::ptr_eq(&cache.model_top_meshes[0], &cache.model_top_meshes[1]),
        "identical models must share one Arc<CachedModelTopMesh2D>"
    );

    // Simulate 25 slider resize steps: stale dimension keys must be pruned instead of leaking
    for step in 1..=25_u64 {
        if let Some(models) = &mut layout.models {
            models[0].width = 200.0 + (step as f64) * 5.0;
            models[1].width = models[0].width;
        }
        cache.ensure_fresh(&layout, &cats, 1 + step);
        assert_eq!(cache.model_proto_cache.len(), 1);
    }

    // Removing all models clears cached top-view meshes completely
    layout.models = Some(vec![]);
    cache.ensure_fresh(&layout, &cats, 100);
    assert!(cache.model_proto_cache.is_empty());
    assert!(cache.model_top_meshes.is_empty());
}

#[test]
fn feathered_mitered_ribbon_emits_transparent_aa_fringes() {
    let rect = Rect::from_min_size(Pos2::ZERO, EguiVec2::new(800.0, 600.0));
    let cam = Camera2D {
        offset: EguiVec2::new(400.0, 300.0),
        zoom: 0.75,
    };
    let pts = vec![
        Vec2 { x: -120.0, y: -40.0 },
        Vec2 { x: 0.0, y: 15.0 },
        Vec2 { x: 140.0, y: 95.0 },
    ];
    let col = Color32::from_rgb(37, 99, 235);
    let mesh = build_feathered_ribbon_mesh_2d(rect, &cam, &pts, 17.0, 0.35, col, 1.0);
    assert_eq!(mesh.vertices.len(), pts.len() * 4);
    assert_eq!(mesh.indices.len(), (pts.len() - 1) * 18);
    assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.vertices.len()));

    for station in mesh.vertices.chunks_exact(4) {
        assert_eq!(station[0].color, Color32::TRANSPARENT);
        assert_eq!(station[1].color, col);
        assert_eq!(station[2].color, col);
        assert_eq!(station[3].color, Color32::TRANSPARENT);
        let outer_span = (station[0].pos - station[3].pos).length();
        let core_span = (station[1].pos - station[2].pos).length();
        assert!(outer_span > core_span + 0.5);
    }
}

