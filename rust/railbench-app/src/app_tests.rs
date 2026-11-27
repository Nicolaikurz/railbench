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

    app.sidebar_tab = SidebarTab::Bom;
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
    app.place_catalog_entry(&feeder_entry);
    assert_eq!(app.layout.pieces.last().unwrap().power_feed, Some(false));
    app.catalog_entry_power_enabled.insert(feeder_entry.id.clone());
    app.place_catalog_entry(&feeder_entry);
    assert_eq!(app.layout.pieces.last().unwrap().power_feed, Some(true));

    let iso_entry = crate::catalog::find_entry(&app.catalogs, "24994").unwrap().clone();
    assert!(crate::catalog::is_isolating_entry(&iso_entry));
    app.place_catalog_entry(&iso_entry);
    assert_eq!(app.layout.pieces.last().unwrap().is_isolator, Some(false));
    assert_eq!(app.layout.pieces.last().unwrap().isolated_connector_index, None);
    app.catalog_entry_isolating_enabled.insert(iso_entry.id.clone());
    app.place_catalog_entry(&iso_entry);
    assert_eq!(app.layout.pieces.last().unwrap().is_isolator, Some(true));
    assert_eq!(app.layout.pieces.last().unwrap().isolated_connector_index, Some(0));
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

#[test]
fn cli_args_parse_file_path_and_electron_aliases() {
    let args: Vec<String> = [
        "railbench",
        "/tmp/my-layout.railbench",
        "--collab-server",
        "--server-port",
        "9090",
        "--server-password",
        "secret",
        "--collab-room",
        "club",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let parsed = crate::settings::parse_cli_args(&args);
    assert_eq!(parsed.file_path.as_deref(), Some("/tmp/my-layout.railbench"));
    assert!(parsed.server);
    assert_eq!(parsed.port, Some(9090));
    assert_eq!(parsed.password.as_deref(), Some("secret"));
    assert_eq!(parsed.room.as_deref(), Some("club"));
    assert!(parsed.connect.is_none());

    let args2: Vec<String> = ["railbench", "--auto-connect", "ws://127.0.0.1:4567"]
        .into_iter()
        .map(String::from)
        .collect();
    let parsed2 = crate::settings::parse_cli_args(&args2);
    assert_eq!(parsed2.connect.as_deref(), Some("ws://127.0.0.1:4567"));
    assert!(parsed2.file_path.is_none());
}

#[test]
fn settings_roundtrip_theme_mode_recent_files_window_pos_and_gpu_flags() {
    let mut cfg = crate::settings::AppSettings::default();
    cfg.theme_mode = crate::settings::ThemeMode::System;
    cfg.use_discrete_gpu = true;
    cfg.low_gpu_mode = true;
    cfg.bars_locked = true;
    cfg.window_x = Some(140.0);
    cfg.window_y = Some(85.0);
    cfg.left_sidebar_width = Some(320.0);
    cfg.right_sidebar_width = Some(280.0);
    for i in 0..10 {
        cfg.push_recent_file(&format!("/tmp/plan-{i}.railbench"));
    }
    // Re-push an older entry and ensure deduplication moves it to front while capping at 8
    cfg.push_recent_file("/tmp/plan-5.railbench");
    assert_eq!(cfg.recent_files.len(), 8);
    assert_eq!(cfg.recent_files[0], "/tmp/plan-5.railbench");
    assert_eq!(cfg.last_opened_path.as_deref(), Some("/tmp/plan-5.railbench"));

    let json = cfg.to_json_string();
    let reloaded = crate::settings::AppSettings::from_json_str(&json);
    assert_eq!(reloaded.theme_mode, crate::settings::ThemeMode::System);
    assert!(reloaded.use_discrete_gpu);
    assert!(reloaded.low_gpu_mode);
    assert!(reloaded.bars_locked);
    assert_eq!(reloaded.window_x, Some(140.0));
    assert_eq!(reloaded.window_y, Some(85.0));
    assert_eq!(reloaded.left_sidebar_width, Some(320.0));
    assert_eq!(reloaded.right_sidebar_width, Some(280.0));
    assert_eq!(reloaded.recent_files, cfg.recent_files);
}

#[test]
fn track_system_picker_formatting_and_manufacturer_order_match_electron() {
    let catalogs = crate::catalog::builtin_catalogs();
    let track_indices: Vec<usize> = catalogs
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_road != Some(true))
        .map(|(i, _)| i)
        .collect();
    let mfrs = crate::ui_panels::collect_track_manufacturers_in_order(&catalogs, &track_indices);
    assert!(!mfrs.is_empty());
    assert_eq!(mfrs[0], "Märklin");
    let unique: std::collections::HashSet<&String> = mfrs.iter().collect();
    assert_eq!(unique.len(), mfrs.len());

    let maerklin_c = catalogs
        .iter()
        .find(|c| c.id == "maerklin-c-h0")
        .expect("maerklin-c-h0 catalog");
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_title(maerklin_c, false, false, true),
        "Märklin — C-Gleis"
    );
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_subtitle(maerklin_c, false, false, 0, true),
        "16.5 mm, mit Bettung, 3-Leiter"
    );
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_subtitle(maerklin_c, false, false, 0, false),
        "16.5 mm, roadbed, 3-rail"
    );

    let tillig_elite = catalogs
        .iter()
        .find(|c| c.id == "tillig-elite-h0")
        .expect("tillig-elite-h0 catalog");
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_subtitle(tillig_elite, false, false, 0, true),
        "16.5 mm, ohne Bettung, 2-Leiter"
    );
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_subtitle(tillig_elite, false, false, 0, false),
        "16.5 mm, without roadbed, 2-rail"
    );

    assert_eq!(
        crate::ui_panels::format_catalog_trigger_title(maerklin_c, false, true, true),
        "★ Meine Favoriten"
    );
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_subtitle(maerklin_c, false, true, 4, false),
        "4 saved favorites"
    );

    let road_h0 = catalogs
        .iter()
        .find(|c| c.id == "generic-streets-h0")
        .expect("generic-streets-h0 catalog");
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_title(road_h0, true, false, true),
        "Straßensystem (H0 1:87)"
    );
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_title(road_h0, true, false, false),
        "Streets (H0 1:87)"
    );
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_subtitle(road_h0, true, false, 0, true),
        "Fahrbahnbreite 60 mm, 2-spurig"
    );
    assert_eq!(
        crate::ui_panels::format_catalog_trigger_subtitle(road_h0, true, false, 0, false),
        "Road width 60 mm, 2-lane"
    );
}

#[test]
fn part_list_tab_labels_are_localized() {
    assert_eq!(crate::i18n::EN.tab_bom, "Part List");
    assert_eq!(crate::i18n::DE.tab_bom, "Teileliste");
}

#[test]
fn light_palette_uses_warm_crema_tones() {
    let p = crate::theme::palette(false);
    assert_ne!(p.bg_card, eframe::egui::Color32::WHITE);
    assert!(p.bg_sidebar.r() > p.bg_sidebar.b());
    assert!(p.bg_card.r() > p.bg_card.b());
    assert!(p.bg_input.r() > p.bg_input.b());
    assert!(p.canvas_bg.r() > p.canvas_bg.b());
}

#[test]
fn stable_text_width_is_language_invariant() {
    let ctx = eframe::egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            for (de, en, sz, strong) in [
                (crate::i18n::DE.menu_edit, crate::i18n::EN.menu_edit, None, false),
                (crate::i18n::DE.tab_bom, crate::i18n::EN.tab_bom, Some(11.5), true),
                ("CAD-Tools", "CAD", None, false),
            ] {
                let w_de_en = crate::ui_panels::stable_text_width(ui, de, en, sz, strong);
                let w_en_de = crate::ui_panels::stable_text_width(ui, en, de, sz, strong);
                let w_de = crate::ui_panels::stable_text_width(ui, de, de, sz, strong);
                let w_en = crate::ui_panels::stable_text_width(ui, en, en, sz, strong);
                assert_eq!(w_de_en, w_en_de);
                assert!(w_de_en >= w_de && w_de_en >= w_en);
            }
        });
    });
}

#[test]
fn favorite_accent_has_strong_contrast_in_light_mode() {
    let dark_col = eframe::egui::Color32::from_rgb(250, 204, 21);
    assert_eq!(crate::ui_panels::favorite_accent(true, dark_col), dark_col);
    let light = crate::ui_panels::favorite_accent(false, dark_col);
    let bg = crate::theme::palette(false).bg_card;
    let lum = |c: eframe::egui::Color32| {
        let f = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    };
    let ratio = (lum(bg) + 0.05) / (lum(light) + 0.05);
    assert!(ratio >= 4.5, "favorite contrast ratio {ratio:.2} must be >= 4.5 on light cards");
}

#[test]
fn undo_redo_toolbar_tooltips_and_toolbar_fade_color() {
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::De, false, 0),
        "Rückgängig (Strg+Z) — nichts rückgängig zu machen"
    );
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::De, true, 0),
        "Wiederholen (Strg+Y) — nichts zu wiederholen"
    );
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::En, false, 0),
        "Undo (Ctrl+Z) — nothing to undo"
    );
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::En, true, 0),
        "Redo (Ctrl+Y) — nothing to redo"
    );
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::De, false, 1),
        "Rückgängig (Strg+Z) · 1 Schritt"
    );
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::De, false, 3),
        "Rückgängig (Strg+Z) · 3 Schritte"
    );
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::En, true, 1),
        "Redo (Ctrl+Y) · 1 step"
    );
    assert_eq!(
        crate::ui_panels::format_undo_redo_tooltip(crate::i18n::Lang::En, true, 2),
        "Redo (Ctrl+Y) · 2 steps"
    );

    let ctx = eframe::egui::Context::default();
    for dark in [false, true] {
        let p = crate::theme::palette(dark);
        let _ = ctx.run(Default::default(), |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                crate::theme::style_toolbar_ui(ui, p);
                assert_eq!(ui.visuals().fade_out_to_color(), p.bg_toolbar);
            });
        });
    }
}

#[test]
fn select_all_respects_active_sidebar_tab() {
    let mut app = make_bare_app();
    let e24188 = crate::catalog::find_entry(&app.catalogs, "24188").expect("24188");
    let road_entry = app
        .catalogs
        .iter()
        .flat_map(|c| c.entries.iter())
        .find(|e| e.is_road == Some(true))
        .expect("builtin road entry");
    app.layout.pieces.push(crate::catalog::create_piece(
        "track-1".into(),
        e24188,
        railbench_core::types::Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        },
        None,
    ));
    app.layout.pieces.push(crate::catalog::create_piece(
        "road-1".into(),
        road_entry,
        railbench_core::types::Transform {
            x: 0.0,
            y: 200.0,
            rotation: 0.0,
        },
        None,
    ));
    app.layout.models = Some(vec![
        railbench_core::types::Model3D {
            id: "bld-1".into(),
            name: "House".into(),
            category: "Buildings".into(),
            transform: railbench_core::types::Transform {
                x: 200.0,
                y: 0.0,
                rotation: 0.0,
            },
            elevation: None,
            scale: None,
            layer_id: None,
            width: 80.0,
            depth: 60.0,
            height: 50.0,
            model_type: "builtin".into(),
            builtin_id: Some("town-house".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        },
        railbench_core::types::Model3D {
            id: "ter-1".into(),
            name: "Hill".into(),
            category: "Terrain".into(),
            transform: railbench_core::types::Transform {
                x: 500.0,
                y: 0.0,
                rotation: 0.0,
            },
            elevation: None,
            scale: None,
            layer_id: None,
            width: 180.0,
            depth: 180.0,
            height: 90.0,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: Some("meadow".into()),
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        },
    ]);

    app.sidebar_tab = SidebarTab::Tracks;
    app.select_all();
    assert_eq!(app.selected.len(), 1);
    assert!(app.selected.contains("track-1"));

    app.sidebar_tab = SidebarTab::Roads;
    app.select_all();
    assert_eq!(app.selected.len(), 1);
    assert!(app.selected.contains("road-1"));

    app.sidebar_tab = SidebarTab::Models;
    app.select_all();
    assert_eq!(app.selected.len(), 1);
    assert!(app.selected.contains("bld-1"));

    app.sidebar_tab = SidebarTab::Terrain;
    app.select_all();
    assert_eq!(app.selected.len(), 1);
    assert!(app.selected.contains("ter-1"));

    app.sidebar_tab = SidebarTab::Board;
    app.select_all();
    assert!(app.selected.is_empty());

    app.sidebar_tab = SidebarTab::Bom;
    app.select_all();
    assert_eq!(app.selected.len(), 4);
}

#[test]
fn paste_and_duplicate_do_not_bind_to_original_group_id() {
    let mut app = make_bare_app();
    let e24188 = crate::catalog::find_entry(&app.catalogs, "24188").expect("24188").clone();
    app.place_catalog_entry(&e24188);
    app.place_catalog_entry(&e24188);
    let orig_ids: Vec<String> = app.layout.pieces.iter().map(|p| p.id.clone()).collect();
    for id in &orig_ids {
        app.selected.insert(id.clone());
    }
    assert!(app.toggle_group_selected());
    let orig_gid = app.layout.pieces[0].group_id.clone().expect("group_id set");
    assert_eq!(app.layout.pieces[1].group_id.as_deref(), Some(orig_gid.as_str()));

    // Duplicating a single grouped piece must NOT inherit the original group_id
    app.selected.clear();
    app.selected.insert(orig_ids[0].clone());
    assert!(app.duplicate_selected());
    assert_eq!(app.layout.pieces.len(), 3);
    assert!(
        app.layout.pieces[2].group_id.is_none(),
        "single pasted/duplicated piece must not bind to original group"
    );

    // Duplicating a multi-piece group assigns a fresh group_id separate from original
    app.selected.clear();
    app.selected.insert(orig_ids[0].clone());
    app.selected.insert(orig_ids[1].clone());
    assert!(app.duplicate_selected());
    assert_eq!(app.layout.pieces.len(), 5);
    let new_gid = app.layout.pieces[3]
        .group_id
        .clone()
        .expect("multi-piece copy preserves internal group");
    assert_ne!(new_gid, orig_gid);
    assert_eq!(app.layout.pieces[4].group_id.as_deref(), Some(new_gid.as_str()));
}
