use super::*;
use crate::catalog::{builtin_catalogs, create_piece};
use railbench_core::types::{BaseboardConfig, Layer, Model3D, Transform};

fn sample_export_layout() -> (Layout, Vec<Catalog>) {
    let catalogs = builtin_catalogs();
    let straight = find_entry(&catalogs, "24188").unwrap();
    let curve = find_entry(&catalogs, "24130").unwrap();
    let p1 = create_piece("p1".into(), straight, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, Some("layer-1".into()));
    let p2 = create_piece("p2".into(), curve, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, Some("layer-2".into()));
    let m1 = Model3D {
        id: "m1".into(),
        name: "Bahnhof".into(),
        category: "buildings".into(),
        transform: Transform { x: 100.0, y: -120.0, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id: Some("layer-1".into()),
        width: 320.0,
        depth: 140.0,
        height: 120.0,
        model_type: "builtin".into(),
        builtin_id: Some("station".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };
    let bb = BaseboardConfig {
        id: Some("board-1".into()),
        name: Some("Tisch".into()),
        enabled: true,
        shape: Some("l-shape".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 2400.0,
        depth: 1200.0,
        rotation: Some(0.0),
        l_wing_width: Some(800.0),
        l_wing_depth: Some(600.0),
        l_side: Some("left".into()),
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: None,
        custom_y: None,
        style: "wood".into(),
        corner_radius: Some(12.0),
        show_dimensions: Some(true),
    };
    let layout = Layout {
        name: "ExportTest".into(),
        scale: "H0".into(),
        pieces: vec![p1, p2],
        models: Some(vec![m1]),
        feed_voltage: Some(16.0),
        catenary_feed_voltage: Some(15.0),
        catenary_circuit_mode: Some("separate".into()),
        loco_current_ma: Some(450.0),
        loco_count: Some(2),
        train_current_amps: None,
        turnout_lantern_current_ma: Some(20.0),
        turnout_decoder_current_ma: Some(15.0),
        layers: Some(vec![
            Layer {
                id: "layer-1".into(),
                name: "Ebene 0".into(),
                color: Some("#38bdf8".into()),
                visible: true,
                locked: false,
                height_mm: Some(0.0),
                wood_plate_mode: Some("table".into()),
                wood_plate_thickness_mm: Some(20.0),
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            },
            Layer {
                id: "layer-2".into(),
                name: "Ebene 1".into(),
                color: Some("#f59e0b".into()),
                visible: true,
                locked: false,
                height_mm: Some(85.0),
                wood_plate_mode: None,
                wood_plate_thickness_mm: None,
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            },
        ]),
        active_layer_id: Some("layer-1".into()),
        baseboard: Some(bb.clone()),
        baseboards: Some(vec![bb]),
        terrain_field: None,
    };
    (layout, catalogs)
}

#[test]
fn bom_summary_and_csv_separate_tracks_roads_and_models() {
    let (layout, catalogs) = sample_export_layout();
    let sum = get_piece_list_summary(&layout, &catalogs, true);
    assert_eq!(sum.tracks.total_pieces, 2);
    assert_eq!(sum.total_buildings, 1);
    assert_eq!(sum.total_objects, 3);
    let csv = export_bom_csv(&layout, &catalogs, false);
    assert!(csv.contains("24188"));
    assert!(csv.contains("24130"));
    assert!(csv.contains("Bahnhof"));
    assert!(csv.contains("--- 1. Railway Tracks ---"));
}

#[test]
fn xml_export_includes_multi_table_layers_pieces_and_models() {
    let (layout, catalogs) = sample_export_layout();
    let xml = export_layout_xml(&layout);
    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("<baseboards>"));
    assert!(xml.contains("shape=\"l-shape\""));
    assert!(xml.contains("<layers activeLayerId=\"layer-1\">"));
    assert!(xml.contains("catalogId=\"24188\""));
    assert!(xml.contains("<model id=\"m1\""));

    let imported = import_layout_xml(&xml, &catalogs).expect("import_layout_xml failed");
    assert_eq!(imported.name, layout.name);
    assert_eq!(imported.scale, layout.scale);
    assert_eq!(imported.pieces.len(), layout.pieces.len());
    assert_eq!(imported.pieces[0].catalog_id, "24188");
    assert_eq!(imported.models.as_ref().map(|m| m.len()), Some(1));
    assert_eq!(imported.layers.as_ref().map(|l| l.len()), Some(2));
    assert_eq!(imported.baseboards.as_ref().map(|b| b.len()), Some(1));
    assert_eq!(
        imported.baseboards.as_ref().unwrap()[0].shape.as_deref(),
        Some("l-shape")
    );
}

#[test]
fn svg_and_multipage_pdf_export_valid_structure() {
    let (layout, catalogs) = sample_export_layout();
    let opts = BlueprintExportOptions {
        include_overview: true,
        include_layer_pages: true,
        include_bom_page: true,
        ..Default::default()
    };
    let svg = export_layout_svg_with_options(&layout, &catalogs, &opts);
    assert!(svg.contains("<polygon points="));
    assert!(svg.contains("<polyline points="));

    let pdf_bytes = export_layout_pdf_with_options(&layout, &catalogs, &opts, false);
    let pdf_text = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_text.starts_with("%PDF-1.4"));
    assert!(pdf_text.contains("/Count 4"));
    assert!(pdf_text.contains("%%EOF"));

    let mut tt_layout = layout.clone();
    let tt_entry = find_entry(&catalogs, "74862").expect("74862 turntable");
    let tf_entry = find_entry(&catalogs, "72941").expect("72941 transfer table");
    tt_layout.pieces.push(create_piece(
        "tt1".into(),
        tt_entry,
        Transform {
            x: 600.0,
            y: 400.0,
            rotation: 15.0,
        },
        Some("layer-1".into()),
    ));
    tt_layout.pieces.push(create_piece(
        "tf1".into(),
        tf_entry,
        Transform {
            x: 1100.0,
            y: 300.0,
            rotation: 30.0,
        },
        Some("layer-1".into()),
    ));
    let tt_svg = export_layout_svg_with_options(&tt_layout, &catalogs, &opts);
    assert!(tt_svg.contains("<circle cx=\"600.0\" cy=\"400.0\" r=\"189.0\""));
    assert!(tt_svg.contains("<polygon points="));
    assert!(tt_svg.contains("74862"));
    assert!(tt_svg.contains("72941"));

    let tt_pdf = export_layout_pdf_with_options(&tt_layout, &catalogs, &opts, false);
    let tt_pdf_text = String::from_utf8_lossy(&tt_pdf);
    assert!(tt_pdf_text.starts_with("%PDF-1.4"));
    assert!(tt_pdf_text.contains("74862"));
    assert!(tt_pdf_text.contains("72941"));
    assert!(tt_pdf_text.contains("%%EOF"));
}

#[test]
fn svg_and_pdf_grid_and_electrical_annotations_plus_xml_roundtrip() {
    let catalogs = builtin_catalogs();
    let e24188 = find_entry(&catalogs, "24188").expect("24188");
    let mut p0 = create_piece(
        "elec-1".into(),
        e24188,
        Transform { x: 100.0, y: 100.0, rotation: 0.0 },
        Some("layer-1".into()),
    );
    p0.power_feed = Some(true);
    p0.is_isolator = Some(true);
    p0.isolated_connector_index = Some(1);
    p0.has_signal = Some(true);
    p0.signal_side = Some("left".into());
    p0.signal_direction = Some("backward".into());
    p0.group_id = Some("grp-1".into());
    p0.elevation = Some(railbench_core::types::Elevation { start: 35.0, end: 45.0 });

    let layout = Layout {
        name: "Grid & Elec".into(),
        scale: "H0".into(),
        pieces: vec![p0],
        models: None,
        feed_voltage: Some(16.0),
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
    let opts = BlueprintExportOptions {
        show_grid: true,
        show_electrical: true,
        ..Default::default()
    };
    let svg = export_layout_svg_with_options(&layout, &catalogs, &opts);
    assert!(svg.contains("<line x1="));
    assert!(svg.contains("+40mm"));
    let pdf = export_layout_pdf_with_options(&layout, &catalogs, &opts, false);
    let pdf_txt = String::from_utf8_lossy(&pdf);
    assert!(pdf_txt.contains("+PWR"));
    assert!(pdf_txt.contains("SIG"));
    assert!(pdf_txt.contains("+40mm"));

    let xml = super::export_layout_xml(&layout);
    let imported = super::import_layout_xml(&xml, &catalogs).expect("xml re-imports");
    let rp = &imported.pieces[0];
    assert_eq!(rp.isolated_connector_index, Some(1));
    assert_eq!(rp.signal_side.as_deref(), Some("left"));
    assert_eq!(rp.signal_direction.as_deref(), Some("backward"));
    assert_eq!(rp.group_id.as_deref(), Some("grp-1"));
}
