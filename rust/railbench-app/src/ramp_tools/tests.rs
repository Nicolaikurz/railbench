use super::*;
use crate::catalog::builtin_catalogs;

    fn make_layout_with_chain(n: usize) -> (Layout, Vec<Catalog>, HashSet<String>) {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24188").unwrap();
        let mut pieces = Vec::new();
        let mut sel = HashSet::new();
        for i in 0..n {
            let id = format!("s{i}");
            let mut p = create_piece(
                id.clone(),
                entry,
                Transform { x: i as f64 * 188.3, y: 0.0, rotation: 0.0 },
                Some("layer-1".into()),
            );
            p.elevation = None;
            sel.insert(id);
            pieces.push(p);
        }
        let mut layout = Layout {
            name: "RampTest".into(),
            scale: "H0".into(),
            pieces,
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
            active_layer_id: Some("layer-1".into()),
            baseboard: None,
            baseboards: None,
            terrain_field: None,
        };
        update_layout_connections(&mut layout, 4.0, 35.0, 2.0);
        (layout, catalogs, sel)
    }

    #[test]
    fn auto_ramp_linear_chain() {
        let (mut layout, catalogs, sel) = make_layout_with_chain(4);
        let sum = apply_auto_ramp(&mut layout, &catalogs, &sel, 0.0, 80.0, RampProfile::Linear).unwrap();
        assert_eq!(sum.piece_count, 4);
        for i in 0..3 {
            let e_cur = layout.pieces[i].elevation.unwrap().end;
            let s_nxt = layout.pieces[i + 1].elevation.unwrap().start;
            assert!((e_cur - s_nxt).abs() < 0.02);
        }
        assert!((layout.pieces[3].elevation.unwrap().end - 80.0).abs() < 0.02);
    }

    #[test]
    fn generate_straight_ramp_and_helix() {
        let (mut layout, catalogs, _) = make_layout_with_chain(1);
        let mut seq = 1u64;
        let (ids, sum) = generate_ramp_or_helix(
            &mut layout,
            &catalogs,
            0,
            &mut seq,
            "s0",
            RampShape::Straight,
            false,
            80.0,
            3.0,
            1.0,
            None,
        )
        .unwrap();
        assert!(!ids.is_empty());
        assert!((sum.total_rise_mm - 80.0).abs() < 0.1);
        let last = layout.pieces.last().unwrap().elevation.unwrap();
        assert!((last.end - 80.0).abs() < 0.1);

        let (h_ids, h_sum) = generate_ramp_or_helix(
            &mut layout,
            &catalogs,
            0,
            &mut seq,
            ids.last().unwrap(),
            RampShape::Helix,
            true,
            160.0,
            3.0,
            1.0,
            None,
        )
        .unwrap();
        assert!(h_ids.len() >= 6);
        assert!((h_sum.total_rise_mm - 80.0).abs() < 0.1);
    }

    #[test]
    fn ramp_start_end_path_and_reverse_direction() {
        let (mut layout, catalogs, _) = make_layout_with_chain(4);
        let empty_sel = HashSet::new();

        // Forward path s0 -> s3 (without needing s1, s2 selected!)
        let fwd = resolve_ramp_chain(&layout, &empty_sel, Some("s0"), Some("s3"));
        assert_eq!(
            fwd,
            vec![
                ("s0".into(), false),
                ("s1".into(), false),
                ("s2".into(), false),
                ("s3".into(), false),
            ]
        );
        let segs_fwd = ramp_chain_direction_segments(&layout, &fwd);
        assert_eq!(segs_fwd.len(), 4);
        assert!(segs_fwd[0].1.x > segs_fwd[0].0.x);

        // Reversed path s3 -> s0: rises from 0 mm at s3's outer end to 80 mm at s0's outer end
        let rev = resolve_ramp_chain(&layout, &empty_sel, Some("s3"), Some("s0"));
        assert_eq!(
            rev,
            vec![
                ("s3".into(), true),
                ("s2".into(), true),
                ("s1".into(), true),
                ("s0".into(), true),
            ]
        );
        let segs_rev = ramp_chain_direction_segments(&layout, &rev);
        assert!(segs_rev[0].1.x < segs_rev[0].0.x);

        let sum = apply_auto_ramp_for_chain(&mut layout, &catalogs, &rev, 0.0, 80.0, RampProfile::Linear)
            .unwrap();
        assert_eq!(sum.piece_count, 4);
        // s3 entered at c[1] (0 mm) -> exited at c[0] (20 mm); s0 exited at c[0] (80 mm)
        assert!((layout.pieces[3].elevation.unwrap().end - 0.0).abs() < 0.02);
        assert!((layout.pieces[0].elevation.unwrap().start - 80.0).abs() < 0.02);
    }

    #[test]
    fn auto_ramp_breakdown_and_max_grade_end_height() {
        use railbench_core::types::ConnectorRef;
        let cats = builtin_catalogs();
        let e24188 = crate::catalog::find_entry(&cats, "24188").expect("24188");
        let e24130 = crate::catalog::find_entry(&cats, "24130").expect("24130");
        let e24130_r = crate::catalog::find_entry(&cats, "24130").expect("24130");
        let mut p_start = create_piece("a".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        p_start.elevation = Some(Elevation { start: 0.0, end: 0.0 });
        let mut c1 = create_piece("c1".into(), e24130, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
        // Curve opposite turn after c1 to create an S-curve pair
        let mut c2 = create_piece("c2".into(), e24130_r, Transform { x: 376.6, y: 0.0, rotation: 0.0 }, None);
        c1.connectors[0].connected_to = Some(ConnectorRef { piece_id: "a".into(), connector_index: 1 });
        p_start.connectors[1].connected_to = Some(ConnectorRef { piece_id: "c1".into(), connector_index: 0 });
        c1.connectors[1].connected_to = Some(ConnectorRef { piece_id: "c2".into(), connector_index: 0 });
        c2.connectors[0].connected_to = Some(ConnectorRef { piece_id: "c1".into(), connector_index: 1 });

        let mut p_end = create_piece("b".into(), e24188, Transform { x: 564.9, y: 0.0, rotation: 0.0 }, None);
        p_end.elevation = Some(Elevation { start: 40.0, end: 40.0 });
        c2.connectors[1].connected_to = Some(ConnectorRef { piece_id: "b".into(), connector_index: 0 });
        p_end.connectors[0].connected_to = Some(ConnectorRef { piece_id: "c2".into(), connector_index: 1 });

        let layout = Layout {
            name: "r1".into(),
            scale: "H0".into(),
            pieces: vec![p_start, c1, c2, p_end],
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
        let mut sel = HashSet::new();
        sel.insert("c1".into());
        sel.insert("c2".into());
        let auto = calculate_auto_ramp(&layout, &sel, &cats).expect("auto ramp inferred");
        assert_eq!(auto.0.len(), 2);
        assert!((auto.1 - 0.0).abs() < 1e-6);
        assert!((auto.2 - 40.0).abs() < 1e-6);

        let rows = build_ramp_breakdown(&layout, &cats, &auto.0, 0.0, 40.0, RampProfile::Linear);
        assert_eq!(rows.len(), 2);
        let max_end = calculate_end_height_from_max_gradient(&layout, &cats, &auto.0, 0.0, 3.0, true, RampProfile::Eased);
        assert!(max_end > 0.0 && max_end < 40.0);
    }

    #[test]
    fn collect_open_connector_options_prioritizes_selected_and_reports_world_z() {
        let (mut layout, _cats, _sel) = make_layout_with_chain(3);
        layout.layers = Some(vec![railbench_core::types::Layer {
            id: "layer-1".into(),
            name: "Ebene 0".into(),
            color: Some("#38bdf8".into()),
            visible: true,
            locked: false,
            height_mm: Some(20.0),
            wood_plate_mode: None,
            wood_plate_thickness_mm: None,
            wood_plate_style: None,
            plates: None,
            terrain_field: None,
        }]);
        layout.pieces[2].elevation = Some(Elevation {
            start: 0.0,
            end: 15.0,
        });

        let mut selected = HashSet::new();
        selected.insert("s2".into());

        let opts = collect_open_connector_options(&layout, &selected, None);
        // Only outer endpoints (s0 c0 and s2 c1) are open
        assert_eq!(opts.len(), 2);
        // Selected s2 must sort ahead of unselected s0
        assert_eq!(opts[0].piece_id, "s2");
        assert_eq!(opts[0].connector_index, 1);
        assert!((opts[0].world_z - 35.0).abs() < 0.05);
        assert_eq!(opts[1].piece_id, "s0");
        assert_eq!(opts[1].connector_index, 0);
        assert!((opts[1].world_z - 20.0).abs() < 0.05);

        // Explicit preferred_piece overrides selection ordering
        let opts_pref = collect_open_connector_options(&layout, &selected, Some("s0"));
        assert_eq!(opts_pref[0].piece_id, "s0");
    }

    #[test]
    fn find_nearest_open_connector_in_chain_jumps_from_middle_piece_to_nearest_end() {
        let (layout, _cats, _sel) = make_layout_with_chain(4);
        // s1 and s2 are middle pieces with both connectors occupied.
        // Clicking near s2's right side (closer to s3's open connector 1 at x ~ 753.2) jumps to (s3, 1).
        let right_pick = find_nearest_open_connector_in_chain(
            &layout,
            "s2",
            Vec2 { x: 520.0, y: 0.0 },
        )
        .expect("should resolve open end of chain");
        assert_eq!(right_pick, ("s3".into(), 1));

        // Clicking near s1's left side (closer to s0's open connector 0 at x = 0) jumps to (s0, 0).
        let left_pick = find_nearest_open_connector_in_chain(
            &layout,
            "s1",
            Vec2 { x: 200.0, y: 0.0 },
        )
        .expect("should resolve open start of chain");
        assert_eq!(left_pick, ("s0".into(), 0));
    }

    #[test]
    fn preview_generate_ramp_or_helix_is_read_only_and_supports_360_curve_and_auto_helix() {
        let (layout, cats, _sel) = make_layout_with_chain(1);
        let original_len = layout.pieces.len();

        // 360-degree full-circle curved ramp preview must not mutate layout
        let (curve_pieces, curve_sum, curve_turns) = preview_generate_ramp_or_helix(
            &layout,
            &cats,
            0,
            "s0",
            Some(1),
            None,
            RampShape::Curved,
            false,
            360.0,
            80.0,
            3.0,
            0.0,
            None,
        )
        .expect("360 curve ramp preview succeeds");
        assert_eq!(layout.pieces.len(), original_len);
        assert!(curve_pieces.len() >= 12);
        assert!((curve_sum.total_rise_mm - 80.0).abs() < 0.1);
        assert_eq!(curve_turns, 0.0);

        // Auto-turns helix (turns = 0.0) sizes itself to match rise & grade
        let (helix_pieces, helix_sum, helix_turns) = preview_generate_ramp_or_helix(
            &layout,
            &cats,
            0,
            "s0",
            Some(1),
            None,
            RampShape::Helix,
            true,
            180.0,
            80.0,
            2.8,
            0.0,
            None,
        )
        .expect("auto helix preview succeeds");
        assert_eq!(layout.pieces.len(), original_len);
        assert!(helix_pieces.len() >= 6);
        assert!(helix_turns > 0.5);
        assert!((helix_sum.total_rise_mm - 80.0).abs() < 0.1);
        assert!(helix_sum.is_continuous);
    }

    #[test]
    fn ramp_summary_detects_continuous_vs_split_and_auto_fit_clamps_end_height() {
        let (mut layout, cats, _) = make_layout_with_chain(2);
        let cont_chain = vec![("s0".to_string(), false), ("s1".to_string(), false)];
        let cont_sum =
            preview_auto_ramp_for_chain(&layout, &cats, &cont_chain, 0.0, 10.0, RampProfile::Linear)
                .unwrap();
        assert!(cont_sum.is_continuous);

        // Move s1 far away and disconnect it to simulate a split selection
        layout.pieces[1].transform.x = 1200.0;
        for c in &mut layout.pieces[1].connectors {
            c.x += 1000.0;
            c.connected_to = None;
        }
        for c in &mut layout.pieces[0].connectors {
            c.connected_to = None;
        }
        let split_sum =
            preview_auto_ramp_for_chain(&layout, &cats, &cont_chain, 0.0, 10.0, RampProfile::Linear)
                .unwrap();
        assert!(!split_sum.is_continuous);

        // Over 1000 mm at max 2.5%, +80 mm rise must clamp to +25 mm (0.5 mm precision)
        let clamped = compute_auto_fit_end_height(0.0, 80.0, 1000.0, 2.5);
        assert!((clamped - 25.0).abs() < 0.01);
        let unchanged = compute_auto_fit_end_height(0.0, 20.0, 1000.0, 2.5);
        assert!((unchanged - 20.0).abs() < 0.01);
    }
