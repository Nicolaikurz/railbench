use super::*;

fn make_bare_app() -> RailbenchApp {
    let mut app = RailbenchApp::default();
    app.layout = make_empty_layout();
    app.selected.clear();
    app.undo_stack.clear();
    app.redo_stack.clear();
    app.clipboard = None;
    app.paste_offset_count = 1;
    app
}

#[test]
fn clipboard_copy_paste_and_duplicate_create_fresh_ids_and_offsets() {
    let mut app = make_bare_app();
    let entry = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
    app.place_catalog_entry(&entry);
    assert_eq!(app.layout.pieces.len(), 1);
    let orig_id = app.layout.pieces[0].id.clone();

    assert!(app.copy_selected());
    assert!(app.paste_clipboard(None));
    assert_eq!(app.layout.pieces.len(), 2);
    let pasted = &app.layout.pieces[1];
    assert_ne!(pasted.id, orig_id);
    assert!((pasted.transform.x - 40.0).abs() < 1e-6);
    assert!((pasted.transform.y - 40.0).abs() < 1e-6);
    assert!(app.selected.contains(&pasted.id));

    assert!(app.duplicate_selected());
    assert_eq!(app.layout.pieces.len(), 3);
    let dup = &app.layout.pieces[2];
    assert!((dup.transform.x - 80.0).abs() < 1e-6);
    assert!((dup.transform.y - 80.0).abs() < 1e-6);
}

#[test]
fn move_selected_to_layer_and_reassign_orphans() {
    let mut app = make_bare_app();
    app.add_layer();
    let layer2_id = app.layout.active_layer_id.clone().unwrap();
    let entry = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
    app.place_catalog_entry(&entry);
    app.place_builtin_model("station", "Bahnhof", 320.0, 140.0, 130.0);

    app.select_all();
    assert_eq!(app.selected.len(), 2);
    app.move_selected_to_layer("layer-1");
    assert_eq!(app.layout.pieces[0].layer_id.as_deref(), Some("layer-1"));
    assert_eq!(app.layout.models.as_ref().unwrap()[0].layer_id.as_deref(), Some("layer-1"));

    // Orphan items on a deleted layer and verify detection + reassignment
    app.layout.pieces[0].layer_id = Some("deleted-layer".into());
    assert_eq!(app.count_orphaned_items(), 1);
    app.layout.active_layer_id = Some(layer2_id.clone());
    let reassigned = app.reassign_orphaned_items();
    assert_eq!(reassigned, 1);
    assert_eq!(app.count_orphaned_items(), 0);
    assert_eq!(app.layout.pieces[0].layer_id.as_deref(), Some(layer2_id.as_str()));
}

#[test]
fn append_track_to_connector_snaps_and_inherits_props() {
    let mut app = make_bare_app();
    let entry = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
    app.place_catalog_entry(&entry);
    app.layout.pieces[0].elevation = Some(Elevation { start: 0.0, end: 50.0 });
    app.layout.pieces[0].has_catenary = Some(true);
    let first_id = app.layout.pieces[0].id.clone();

    assert!(app.append_track_to_connector(&first_id, 1, None));
    assert_eq!(app.layout.pieces.len(), 2);
    let added = &app.layout.pieces[1];
    assert_eq!(added.has_catenary, Some(true));
    assert_eq!(added.elevation.unwrap().start, 50.0);
    // Should automatically connect to piece 0 connector 1
    assert!(added.connectors[0].connected_to.is_some());
}

#[test]
fn accessory_toggles_and_catenary_power_sim() {
    let mut app = make_bare_app();
    let sw_entry = crate::catalog::find_entry(&app.catalogs, "24611").unwrap().clone();
    app.place_catalog_entry(&sw_entry);
    let sw_id = app.layout.pieces[0].id.clone();

    app.toggle_selected_lantern();
    app.toggle_selected_decoder();
    app.toggle_selected_signal();
    app.cycle_selected_signal_side();
    app.cycle_selected_signal_direction();
    app.toggle_selected_catenary_feeder();

    let p = &app.layout.pieces[0];
    assert_eq!(p.has_lantern, Some(true));
    assert_eq!(p.has_decoder, Some(true));
    assert_eq!(p.has_signal, Some(true));
    assert_eq!(p.signal_side.as_deref(), Some("left"));
    assert_eq!(p.signal_direction.as_deref(), Some("backward"));
    assert_eq!(p.has_catenary, Some(true));
    assert_eq!(p.catenary_power_feed, Some(true));

    let sim = app.get_or_update_power_sim();
    let res = sim.results.get(&sw_id).unwrap();
    assert!(res.has_catenary);
    assert!(res.catenary_voltage.is_some());

    // Catenary isolator cycles through all 3 turnout connectors then disables
    let n_conns = app.layout.pieces[0].connectors.len();
    app.toggle_selected_catenary_isolator();
    assert_eq!(app.layout.pieces[0].catenary_isolator, Some(true));
    assert_eq!(app.layout.pieces[0].catenary_isolated_connector_index, Some(0));
    for expected in 1..n_conns {
        app.toggle_selected_catenary_isolator();
        assert_eq!(app.layout.pieces[0].catenary_isolated_connector_index, Some(expected));
    }
    app.toggle_selected_catenary_isolator();
    assert_eq!(app.layout.pieces[0].catenary_isolator, Some(false));
}

#[test]
fn bulk_equip_turnouts_sets_and_clears_lanterns_and_decoders() {
    let mut app = make_bare_app();
    let straight = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
    let sw_entry = crate::catalog::find_entry(&app.catalogs, "24611").unwrap().clone();
    app.place_catalog_entry(&straight);
    app.place_catalog_entry(&sw_entry);

    let equipped = app.bulk_equip_turnouts(true);
    assert_eq!(equipped, 1);
    assert_ne!(app.layout.pieces[0].has_lantern, Some(true));
    assert_eq!(app.layout.pieces[1].has_lantern, Some(true));
    assert_eq!(app.layout.pieces[1].has_decoder, Some(true));

    let cleared = app.bulk_equip_turnouts(false);
    assert_eq!(cleared, 1);
    assert_eq!(app.layout.pieces[1].has_lantern, Some(false));
    assert_eq!(app.layout.pieces[1].has_decoder, Some(false));
}

#[test]
fn custom_model_file_data_arc_shared_and_undo_budget_bounded() {
    let mut app = make_bare_app();
    let payload = "data:model/gltf-binary;base64,AAAA".to_string();
    let m1 = Model3D {
        id: "glb-1".into(),
        name: "Custom Loco".into(),
        category: "custom".into(),
        transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: None,
        width: 80.0,
        depth: 30.0,
        height: 45.0,
        model_type: "glb".into(),
        builtin_id: None,
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: Some(Arc::from(payload.as_str())),
        file_name: Some("loco.glb".into()),
    };
    let mut m2 = m1.clone();
    m2.id = "glb-2".into();
    app.layout.models = Some(vec![m1, m2]);

    let json = save_layout_json(&app.layout).unwrap();
    let mut reloaded = load_layout_json(&json).unwrap();
    let ms = reloaded.models.as_mut().unwrap();
    assert_eq!(ms[0].file_data.as_deref(), Some(payload.as_str()));
    dedup_model_file_data(ms, &[]);
    assert!(
        Arc::ptr_eq(
            ms[0].file_data.as_ref().unwrap(),
            ms[1].file_data.as_ref().unwrap()
        ),
        "identical GLB file_data must share a single Arc allocation"
    );

    let cloned = reloaded.clone();
    assert!(Arc::ptr_eq(
        reloaded.models.as_ref().unwrap()[0].file_data.as_ref().unwrap(),
        cloned.models.as_ref().unwrap()[0].file_data.as_ref().unwrap(),
    ));

    // Populate a large layout (>6 MB estimated per snapshot) and verify undo_stack trims below 60 steps
    app.generate_stress_tracks(9000);
    assert!(estimate_layout_ram_bytes(&app.layout) > 5_000_000);
    for _ in 0..20 {
        app.push_undo();
    }
    assert!(
        app.undo_stack.len() >= MIN_UNDO_STEPS && app.undo_stack.len() < 16,
        "undo_stack len {} should honor MIN_UNDO_STEPS..budget cap",
        app.undo_stack.len()
    );
}

#[test]
fn place_feeder_and_isolating_track_buttons_add_and_snap() {
    let mut app = make_bare_app();
    app.place_feeder_track();
    assert_eq!(app.layout.pieces.len(), 1);
    assert_eq!(app.layout.pieces[0].power_feed, Some(true));

    // Second click adds an isolating track snapped directly onto the feeder track
    app.place_isolating_track();
    assert_eq!(app.layout.pieces.len(), 2);
    assert_eq!(app.layout.pieces[1].is_isolator, Some(true));
    assert_eq!(app.layout.pieces[1].isolated_connector_index, Some(0));
    assert!(app.layout.pieces[1].connectors[0].connected_to.is_some());

    // Electron-parity per-entry catalog card toggles for feeder & isolating tracks
    let feeder_entry = crate::catalog::find_entry(&app.catalogs, "24088").unwrap().clone();
    assert!(crate::catalog::is_feeder_entry(&feeder_entry));
    app.catalog_entry_power_enabled.insert(feeder_entry.id.clone());
    app.place_catalog_entry(&feeder_entry);
    assert_eq!(app.layout.pieces.last().unwrap().power_feed, Some(true));

    let iso_entry = crate::catalog::find_entry(&app.catalogs, "24994").unwrap().clone();
    assert!(crate::catalog::is_isolating_entry(&iso_entry));
    app.catalog_entry_isolating_enabled.insert(iso_entry.id.clone());
    app.place_catalog_entry(&iso_entry);
    assert_eq!(app.layout.pieces.last().unwrap().is_isolator, Some(true));
}

#[test]
fn flip_group_split_replace_and_safe_layer_delete() {
    let mut app = make_bare_app();
    let e24188 = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
    let flex_id = app
        .catalogs
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.kind == "flex")
        .expect("builtin flex entry")
        .id
        .clone();
    app.place_catalog_entry(&e24188);
    app.place_catalog_entry(&e24188);
    assert_eq!(app.layout.pieces.len(), 2);

    // Group & Ungroup
    let ids: Vec<String> = app.layout.pieces.iter().map(|p| p.id.clone()).collect();
    for id in &ids {
        app.selected.insert(id.clone());
    }
    app.toggle_group_selected();
    assert!(app.layout.pieces.iter().all(|p| p.group_id.is_some()));
    app.toggle_group_selected();
    assert!(app.layout.pieces.iter().all(|p| p.group_id.is_none()));

    // Flip single piece
    app.selected.clear();
    app.selected.insert(ids[0].clone());
    let orig_rot = app.layout.pieces[0].transform.rotation;
    app.flip_selected();
    assert!((app.layout.pieces[0].transform.rotation - orig_rot).abs() > 1.0);

    // Replace with flextrack & Split track
    app.replace_track(&ids[0], &flex_id);
    assert_eq!(app.layout.pieces[0].catalog_id, flex_id);
    assert!(app.split_track(&ids[0]));
    assert_eq!(app.layout.pieces.len(), 3);

    // Safe delete layer moves items cleanly to fallback layer-1
    app.add_layer_immediate();
    let second_lid = app.layout.layers.as_ref().unwrap().last().unwrap().id.clone();
    app.layout.pieces[0].layer_id = Some(second_lid.clone());
    assert_eq!(app.count_items_on_layer(&second_lid), 1);
    assert!(app.safe_delete_layer(&second_lid));
    assert_eq!(app.count_items_on_layer(&second_lid), 0);
}
