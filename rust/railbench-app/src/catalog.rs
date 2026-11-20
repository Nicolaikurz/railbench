use railbench_core::geometry::{deg2rad, norm_angle_deg, rotate};
use railbench_core::types::{Catalog, CatalogEntry, Connector, TrackPiece, Transform, Vec2};

pub fn builtin_catalogs() -> Vec<Catalog> {
    vec![
        Catalog {
            id: "maerklin-c".into(),
            name: "Märklin C-Gleis (H0)".into(),
            scale: "H0".into(),
            entries: vec![
                straight("24188", "Gerade 188,3 mm", 188.3, 16.5),
                straight("24172", "Gerade 171,7 mm", 171.7, 16.5),
                straight("24094", "Gerade 94,2 mm", 94.2, 16.5),
                straight("24077", "Gerade 77,5 mm", 77.5, 16.5),
                straight("24064", "Gerade 64,3 mm", 64.3, 16.5),
                straight("24360", "Gerade 360,0 mm", 360.0, 16.5),
                straight("24088", "Anschlussgleis 188,3 mm", 188.3, 16.5),
                curve("24130", "Bogen R1 360mm 30°", 360.0, 30.0, 16.5),
                curve("24230", "Bogen R2 437,5mm 30°", 437.5, 30.0, 16.5),
                curve("24224", "Bogen R2 437,5mm 24,3°", 437.5, 24.3, 16.5),
                curve("24330", "Bogen R3 515mm 30°", 515.0, 30.0, 16.5),
                curve("24430", "Bogen R4 579,3mm 30°", 579.3, 30.0, 16.5),
                turnout("24611", "Weiche links 188,3mm 24,3°", "turnout-left", 188.3, 437.5, 24.3, 16.5),
                turnout("24612", "Weiche rechts 188,3mm 24,3°", "turnout-right", 188.3, 437.5, 24.3, 16.5),
                flex("24900-flex", "Flexgleis 900 mm", 900.0, 16.5),
            ],
        },
        Catalog {
            id: "fleischmann-n".into(),
            name: "Fleischmann Piccolo (N)".into(),
            scale: "N".into(),
            entries: vec![
                straight("9100", "Gerade 222 mm", 222.0, 9.0),
                straight("9101", "Gerade 111 mm", 111.0, 9.0),
                straight("9102", "Gerade 57,5 mm", 57.5, 9.0),
                curve("9120", "Bogen R1 192mm 45°", 192.0, 45.0, 9.0),
                curve("9125", "Bogen R2 225,6mm 45°", 225.6, 45.0, 9.0),
                curve("9130", "Bogen R3 396,4mm 30°", 396.4, 30.0, 9.0),
                turnout("9170", "Weiche links 111mm 15°", "turnout-left", 111.0, 430.0, 15.0, 9.0),
                turnout("9171", "Weiche rechts 111mm 15°", "turnout-right", 111.0, 430.0, 15.0, 9.0),
            ],
        },
        Catalog {
            id: "tillig-tt".into(),
            name: "Tillig Modellgleis (TT)".into(),
            scale: "TT".into(),
            entries: vec![
                straight("83101", "Gerade G1 166 mm", 166.0, 12.0),
                straight("83102", "Gerade G2 83 mm", 83.0, 12.0),
                curve("83109", "Bogen R11 310mm 30°", 310.0, 30.0, 12.0),
                curve("83116", "Bogen R21 353mm 30°", 353.0, 30.0, 12.0),
                turnout("83322", "Weiche EW1 links 129,5mm 15°", "turnout-left", 129.5, 353.0, 15.0, 12.0),
                turnout("83321", "Weiche EW1 rechts 129,5mm 15°", "turnout-right", 129.5, 353.0, 15.0, 12.0),
            ],
        },
    ]
}

fn straight(id: &str, name: &str, length: f64, gauge: f64) -> CatalogEntry {
    CatalogEntry {
        id: id.into(),
        name: name.into(),
        kind: "straight".into(),
        category: "Geraden".into(),
        description: String::new(),
        length: Some(length),
        radius: None,
        angle: None,
        branch_radius: None,
        branch_angle: None,
        gauge: Some(gauge),
        has_roadbed: Some(true),
        is_road: Some(false),
        road_width: None,
    }
}

fn curve(id: &str, name: &str, radius: f64, angle: f64, gauge: f64) -> CatalogEntry {
    CatalogEntry {
        id: id.into(),
        name: name.into(),
        kind: "curve".into(),
        category: "Bögen".into(),
        description: String::new(),
        length: None,
        radius: Some(radius),
        angle: Some(angle),
        branch_radius: None,
        branch_angle: None,
        gauge: Some(gauge),
        has_roadbed: Some(true),
        is_road: Some(false),
        road_width: None,
    }
}

fn turnout(
    id: &str,
    name: &str,
    kind: &str,
    length: f64,
    branch_radius: f64,
    branch_angle: f64,
    gauge: f64,
) -> CatalogEntry {
    CatalogEntry {
        id: id.into(),
        name: name.into(),
        kind: kind.into(),
        category: "Weichen".into(),
        description: String::new(),
        length: Some(length),
        radius: None,
        angle: None,
        branch_radius: Some(branch_radius),
        branch_angle: Some(branch_angle),
        gauge: Some(gauge),
        has_roadbed: Some(true),
        is_road: Some(false),
        road_width: None,
    }
}

fn flex(id: &str, name: &str, length: f64, gauge: f64) -> CatalogEntry {
    CatalogEntry {
        id: id.into(),
        name: name.into(),
        kind: "flex".into(),
        category: "Flexgleise".into(),
        description: String::new(),
        length: Some(length),
        radius: None,
        angle: None,
        branch_radius: None,
        branch_angle: None,
        gauge: Some(gauge),
        has_roadbed: Some(true),
        is_road: Some(false),
        road_width: None,
    }
}

pub fn local_connectors(entry: &CatalogEntry) -> Vec<Connector> {
    match entry.kind.as_str() {
        "curve" => {
            let r = entry.radius.unwrap_or(437.5);
            let a = entry.angle.unwrap_or(30.0);
            let dir = if a < 0.0 { -1.0 } else { 1.0 };
            let rad = deg2rad(a.abs());
            let ex = r * rad.sin();
            let ey = dir * r * (1.0 - rad.cos());
            vec![
                Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: ex, y: ey, angle: norm_angle_deg(dir * a.abs()), connected_to: None },
            ]
        }
        "turnout-left" | "turnout-right" => {
            let len = entry.length.unwrap_or(188.3);
            let br = entry.branch_radius.unwrap_or(437.5);
            let ba = entry.branch_angle.unwrap_or(24.3);
            let dir = if entry.kind == "turnout-left" { -1.0 } else { 1.0 };
            let rad = deg2rad(ba);
            let bx = br * rad.sin();
            let by = dir * br * (1.0 - rad.cos());
            vec![
                Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: len, y: 0.0, angle: 0.0, connected_to: None },
                Connector { x: bx, y: by, angle: norm_angle_deg(dir * ba), connected_to: None },
            ]
        }
        _ => {
            let len = entry.length.unwrap_or(188.3);
            vec![
                Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: len, y: 0.0, angle: 0.0, connected_to: None },
            ]
        }
    }
}

pub fn compute_world_connectors(entry: &CatalogEntry, tr: Transform) -> Vec<Connector> {
    local_connectors(entry)
        .into_iter()
        .map(|c| {
            let rp = rotate(Vec2 { x: c.x, y: c.y }, tr.rotation);
            Connector {
                x: tr.x + rp.x,
                y: tr.y + rp.y,
                angle: norm_angle_deg(c.angle + tr.rotation),
                connected_to: None,
            }
        })
        .collect()
}

pub fn create_piece(id: String, entry: &CatalogEntry, tr: Transform, layer_id: Option<String>) -> TrackPiece {
    let connectors = compute_world_connectors(entry, tr);
    let is_feeder = entry.id == "24088";
    TrackPiece {
        id,
        catalog_id: entry.id.clone(),
        transform: tr,
        connectors,
        elevation: None,
        power_feed: if is_feeder { Some(true) } else { None },
        is_isolator: None,
        isolated_connector_index: None,
        has_catenary: None,
        catenary_power_feed: None,
        catenary_isolator: None,
        catenary_isolated_connector_index: None,
        has_lantern: None,
        has_decoder: None,
        turnout_current_ma: None,
        has_signal: None,
        signal_side: None,
        signal_direction: None,
        has_bridge: None,
        has_tunnel: None,
        custom_name: None,
        custom_length: None,
        custom_radius: None,
        custom_angle: None,
        custom_offsets: None,
        custom_chord_dist: None,
        group_id: None,
        layer_id,
    }
}

pub fn find_entry<'a>(catalogs: &'a [Catalog], catalog_id: &str) -> Option<&'a CatalogEntry> {
    for cat in catalogs {
        if let Some(e) = cat.entries.iter().find(|e| e.id == catalog_id) {
            return Some(e);
        }
    }
    None
}
