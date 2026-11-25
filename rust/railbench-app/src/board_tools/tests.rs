use super::*;
use crate::catalog::{builtin_catalogs, create_piece};
use railbench_core::types::{Elevation, Transform};

#[test]
fn baseboard_shapes_generate_expected_polygons() {
    let mut l = create_default_baseboard("b1".into(), "L".into());
    l.shape = Some("l-shape".into());
    l.position_mode = "top-left".into();
    l.width = 2600.0;
    l.depth = 2000.0;
    let l_poly = get_baseboard_polygon(&l);
    assert_eq!(l_poly.len(), 6);
    assert!(is_point_inside_baseboard(Vec2 { x: 200.0, y: 200.0 }, &l));
    assert!(!is_point_inside_baseboard(Vec2 { x: 2000.0, y: 1500.0 }, &l));

    let mut u = l.clone();
    u.shape = Some("u-shape".into());
    let u_poly = get_baseboard_polygon(&u);
    assert_eq!(u_poly.len(), 8);

    let mut t = l.clone();
    t.shape = Some("t-shape".into());
    let t_poly = get_baseboard_polygon(&t);
    assert_eq!(t_poly.len(), 8);

    let mut tri = l.clone();
    tri.shape = Some("triangle".into());
    let tri_poly = get_baseboard_polygon(&tri);
    assert_eq!(tri_poly.len(), 3);
}

#[test]
fn triangulate_concave_l_shape_and_polygon_with_hole() {
    let outer = vec![
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 1000.0, y: 0.0 },
        Vec2 { x: 1000.0, y: 400.0 },
        Vec2 { x: 400.0, y: 400.0 },
        Vec2 { x: 400.0, y: 1000.0 },
        Vec2 { x: 0.0, y: 1000.0 },
    ];
    let hole = vec![
        Vec2 { x: 100.0, y: 100.0 },
        Vec2 { x: 300.0, y: 100.0 },
        Vec2 { x: 300.0, y: 300.0 },
        Vec2 { x: 100.0, y: 300.0 },
    ];
    let expected_area = signed_polygon_area(&outer).abs() - signed_polygon_area(&hole).abs();
    let tris = triangulate_polygon_with_holes(&outer, &[hole]);
    assert!(!tris.is_empty());
    let sum_area: f64 = tris
        .iter()
        .map(|tri| cross2(tri[0], tri[1], tri[2]).abs() * 0.5)
        .sum();
    assert!((sum_area - expected_area).abs() < 1.0);
}

#[test]
fn snap_baseboard_aligns_edge_to_edge_and_emits_guide() {
    let mut b1 = create_default_baseboard("b1".into(), "Main".into());
    b1.position_mode = "top-left".into();
    b1.width = 2000.0;
    b1.depth = 1000.0;
    let mut b2 = create_default_baseboard("b2".into(), "Wing".into());
    b2.position_mode = "custom".into();
    b2.width = 1000.0;
    b2.depth = 800.0;

    let res = snap_baseboard(&b2, 2018.0, 12.0, &[b1], 35.0);
    assert!(res.snapped_x);
    assert!(res.snapped_y);
    assert_eq!(res.x, 2000.0);
    assert_eq!(res.y, 0.0);
    assert!(!res.guides.is_empty());
}

#[test]
fn compute_table_track_cutouts_cuts_sloped_ramp_but_ignores_flat_shadow_station() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let mut ramp = create_piece(
        "r1".into(),
        entry,
        Transform { x: -180.0, y: 0.0, rotation: 0.0 },
        None,
    );
    ramp.elevation = Some(Elevation { start: 5.0, end: -45.0 });

    let mut shadow = create_piece(
        "s1".into(),
        entry,
        Transform { x: -180.0, y: 200.0, rotation: 0.0 },
        None,
    );
    shadow.elevation = Some(Elevation { start: -100.0, end: -100.0 });

    let board = create_default_baseboard("b1".into(), "Main".into());
    let layout = Layout {
        name: "CutoutTest".into(),
        scale: "H0".into(),
        pieces: vec![ramp, shadow],
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
        baseboard: Some(board.clone()),
        baseboards: Some(vec![board.clone()]),
        terrain_field: None,
    };

    let cutouts = compute_table_track_cutouts(&layout, &catalogs, &board, 0.0, 15.0);
    assert_eq!(cutouts.len(), 1);
    assert!(is_point_inside_cutouts(Vec2 { x: 0.0, y: 0.0 }, &cutouts, 5.0));
    assert!(!is_point_inside_cutouts(Vec2 { x: 0.0, y: 200.0 }, &cutouts, 5.0));
}

#[test]
fn table_cutout_widens_and_deepens_for_catenary_ramp() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let mut ramp_plain = create_piece(
        "rp".into(),
        entry,
        Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        None,
    );
    ramp_plain.elevation = Some(Elevation { start: 5.0, end: -130.0 });

    let mut ramp_cat = ramp_plain.clone();
    ramp_cat.has_catenary = Some(true);

    let board = create_default_baseboard("b1".into(), "Main".into());
    let mk = |p| Layout {
        name: "CatCutout".into(),
        scale: "H0".into(),
        pieces: vec![p],
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
        baseboard: Some(board.clone()),
        baseboards: Some(vec![board.clone()]),
        terrain_field: None,
    };

    let cut_plain = compute_table_track_cutouts(&mk(ramp_plain), &catalogs, &board, 0.0, 20.0);
    let cut_cat = compute_table_track_cutouts(&mk(ramp_cat), &catalogs, &board, 0.0, 20.0);
    assert_eq!(cut_plain.len(), 1);
    assert_eq!(cut_cat.len(), 1);

    let width_of = |poly: &[Vec2]| -> f64 {
        let min_y = poly.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = poly.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        max_y - min_y
    };
    let w_plain = width_of(&cut_plain[0].points);
    let w_cat = width_of(&cut_cat[0].points);
    assert!(w_cat >= 68.0 && w_cat > w_plain + 10.0, "expected catenary cutout >= 68mm (was {w_cat}, plain {w_plain})");

    // Where ramp y = -95mm (u = 100/135 -> x = 266.7mm), catenary mast top (-3mm) hits a 20mm tabletop unless cut.
    let deep_pt = Vec2 { x: 266.7, y: 0.0 };
    assert!(is_point_inside_cutouts(deep_pt, &cut_cat, 0.0));
    assert!(!is_point_inside_cutouts(deep_pt, &cut_plain, 0.0));
}

#[test]
fn layer_plate_crud_and_fit_to_tracks() {
    let catalogs = builtin_catalogs();
    let entry = find_entry(&catalogs, "24360").unwrap();
    let p = create_piece("p1".into(), entry, Transform { x: 100.0, y: 50.0, rotation: 0.0 }, None);
    let bb = create_default_baseboard("b1".into(), "Main".into());
    let mut seq = 2u64;
    let mut layout = Layout {
        name: "T".into(),
        scale: "H0".into(),
        pieces: vec![p],
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

    let b2_id = add_baseboard(&mut layout, &mut seq);
    assert_eq!(layout.baseboards.as_ref().unwrap().len(), 2);
    let dup_id = duplicate_baseboard(&mut layout, &b2_id, &mut seq).unwrap();
    assert_eq!(layout.baseboards.as_ref().unwrap().len(), 3);
    assert!(remove_baseboard(&mut layout, &dup_id));
    assert_eq!(layout.baseboards.as_ref().unwrap().len(), 2);

    let mut fitted = layout.baseboards.as_ref().unwrap()[0].clone();
    assert!(fit_board_to_tracks(&layout, &mut fitted));
    assert!(is_point_inside_baseboard(Vec2 { x: 250.0, y: 50.0 }, &fitted));
}
