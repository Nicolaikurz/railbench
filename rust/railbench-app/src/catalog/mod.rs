use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use railbench_core::geometry::{deg2rad, norm_angle_deg, rotate};
use railbench_core::types::{Catalog, CatalogEntry, Connector, TrackPiece, Transform, Vec2};

struct BuiltinCatalogIndex {
    first_entry: HashMap<String, (usize, usize)>,
    three_rail_ids: HashSet<String>,
    catalog_sig: Vec<(String, usize)>,
}

fn builtin_index() -> &'static BuiltinCatalogIndex {
    static INDEX: OnceLock<BuiltinCatalogIndex> = OnceLock::new();
    INDEX.get_or_init(|| {
        let cats = builtin_catalogs();
        let mut first_entry = HashMap::with_capacity(2048);
        let mut three_rail_ids = HashSet::with_capacity(512);
        let mut catalog_sig = Vec::with_capacity(cats.len());
        for (ci, cat) in cats.iter().enumerate() {
            catalog_sig.push((cat.id.clone(), cat.entries.len()));
            let is_three_rail = cat.conductors == Some(3);
            for (ei, entry) in cat.entries.iter().enumerate() {
                first_entry.entry(entry.id.clone()).or_insert((ci, ei));
                if is_three_rail {
                    three_rail_ids.insert(entry.id.clone());
                }
            }
        }
        BuiltinCatalogIndex {
            first_entry,
            three_rail_ids,
            catalog_sig,
        }
    })
}

fn is_builtin_slice(catalogs: &[Catalog], idx: &BuiltinCatalogIndex) -> bool {
    if crate::catalog_store::has_custom_catalogs() {
        return false;
    }
    catalogs.len() == idx.catalog_sig.len()
        && catalogs
            .iter()
            .zip(idx.catalog_sig.iter())
            .all(|(c, (id, len))| c.entries.len() == *len && c.id == *id)
}

const TRACK_CATALOG_JSONS: &[(&str, &str)] = &[
    ("maerklin-c-h0", include_str!("../../../../src/catalog/systems/maerklin-c-h0.json")),
    ("maerklin-k-h0", include_str!("../../../../src/catalog/systems/maerklin-k-h0.json")),
    ("maerklin-m-h0", include_str!("../../../../src/catalog/systems/maerklin-m-h0.json")),
    ("trix-c-h0", include_str!("../../../../src/catalog/systems/trix-c-h0.json")),
    ("roco-line-h0", include_str!("../../../../src/catalog/systems/roco-line-h0.json")),
    ("piko-a-h0", include_str!("../../../../src/catalog/systems/piko-a-h0.json")),
    ("tillig-elite-h0", include_str!("../../../../src/catalog/systems/tillig-elite-h0.json")),
    ("peco-streamline-h0-code100", include_str!("../../../../src/catalog/systems/peco-streamline-h0-code100.json")),
    ("kato-unitrack-ho", include_str!("../../../../src/catalog/systems/kato-unitrack-ho.json")),
    ("bachmann-ez-track-h0", include_str!("../../../../src/catalog/systems/bachmann-ez-track-h0.json")),
    ("fleischmann-piccolo-n", include_str!("../../../../src/catalog/systems/fleischmann-piccolo-n.json")),
    ("minitrix-n", include_str!("../../../../src/catalog/systems/minitrix-n.json")),
    ("kato-unitrack-n", include_str!("../../../../src/catalog/systems/kato-unitrack-n.json")),
    ("peco-streamline-n-code55", include_str!("../../../../src/catalog/systems/peco-streamline-n-code55.json")),
    ("bachmann-ez-track-n", include_str!("../../../../src/catalog/systems/bachmann-ez-track-n.json")),
    ("tillig-tt", include_str!("../../../../src/catalog/systems/tillig-tt.json")),
    ("tillig-bettungsgleis-tt", include_str!("../../../../src/catalog/systems/tillig-bettungsgleis-tt.json")),
    ("kuehn-gleis-tt", include_str!("../../../../src/catalog/systems/kuehn-gleis-tt.json")),
    ("maerklin-z", include_str!("../../../../src/catalog/systems/maerklin-z.json")),
    ("rokuhan-z", include_str!("../../../../src/catalog/systems/rokuhan-z.json")),
    ("lenz-0", include_str!("../../../../src/catalog/systems/lenz-0.json")),
    ("maerklin-1", include_str!("../../../../src/catalog/systems/maerklin-1.json")),
    ("lgb-g", include_str!("../../../../src/catalog/systems/lgb-g.json")),
];

const ROAD_CATALOG_JSONS: &[(&str, &str)] = &[
    ("generic-streets-h0", include_str!("../../../../src/catalog/systems/generic-streets-h0.json")),
    ("generic-streets-n", include_str!("../../../../src/catalog/systems/generic-streets-n.json")),
    ("generic-streets-tt", include_str!("../../../../src/catalog/systems/generic-streets-tt.json")),
    ("generic-streets-z", include_str!("../../../../src/catalog/systems/generic-streets-z.json")),
    ("generic-streets-0", include_str!("../../../../src/catalog/systems/generic-streets-0.json")),
    ("generic-streets-1", include_str!("../../../../src/catalog/systems/generic-streets-1.json")),
    ("generic-streets-g", include_str!("../../../../src/catalog/systems/generic-streets-g.json")),
];

pub fn builtin_catalogs() -> Vec<Catalog> {
    let mut out = Vec::with_capacity(TRACK_CATALOG_JSONS.len() + ROAD_CATALOG_JSONS.len());
    for (name, raw) in TRACK_CATALOG_JSONS {
        match serde_json::from_str::<Catalog>(raw) {
            Ok(mut cat) => {
                let default_gauge = cat.gauge;
                let default_roadbed = cat.has_roadbed;
                for e in &mut cat.entries {
                    if e.gauge.is_none() {
                        e.gauge = default_gauge;
                    }
                    if e.has_roadbed.is_none() {
                        e.has_roadbed = default_roadbed;
                    }
                }
                out.push(cat);
            }
            Err(err) => {
                eprintln!("[catalog] failed to parse {name}: {err}");
            }
        }
    }
    for (name, raw) in ROAD_CATALOG_JSONS {
        match serde_json::from_str::<Catalog>(raw) {
            Ok(mut cat) => {
                cat.is_road = Some(true);
                for e in &mut cat.entries {
                    e.is_road = Some(true);
                }
                out.push(cat);
            }
            Err(err) => {
                eprintln!("[catalog] failed to parse road catalog {name}: {err}");
            }
        }
    }
    out
}

pub fn is_buffer_stop_entry(entry: &CatalogEntry) -> bool {
    if entry.is_road == Some(true) {
        return false;
    }
    if matches!(entry.kind.as_str(), "buffer" | "buffer-stop" | "accessory") {
        return true;
    }
    if entry.local_connectors.as_ref().map_or(false, |lc| lc.len() == 1) {
        return true;
    }
    let contains_kw = |s: &str| -> bool {
        let lower = s.to_ascii_lowercase();
        lower.contains("prellbock") || lower.contains("buffer stop") || lower.contains("buffer-stop")
    };
    contains_kw(&entry.name)
        || entry.name_en.as_deref().map_or(false, contains_kw)
        || contains_kw(&entry.description)
}

pub fn is_illuminated_buffer_entry(entry: &CatalogEntry) -> bool {
    let contains_lit = |s: &str| -> bool {
        let lower = s.to_ascii_lowercase();
        lower.contains("beleuchtet") || lower.contains("illuminat")
    };
    contains_lit(&entry.name)
        || entry.name_en.as_deref().map_or(false, contains_lit)
        || contains_lit(&entry.description)
}

pub fn local_connectors(entry: &CatalogEntry) -> Vec<Connector> {
    if let Some(lc) = &entry.local_connectors {
        if !lc.is_empty() {
            return lc
                .iter()
                .map(|c| Connector {
                    x: c.x,
                    y: c.y,
                    angle: c.angle,
                    connected_to: None,
                })
                .collect();
        }
    }

    if is_buffer_stop_entry(entry) {
        return vec![Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None }];
    }

    match entry.kind.as_str() {
        "transfer-table" => {
            let l = entry.length.unwrap_or(360.0);
            let spacing = 77.4;
            let count = 5;
            let mut conns = Vec::with_capacity(count * 2);
            for i in 0..count {
                let y = i as f64 * spacing;
                conns.push(Connector { x: 0.0, y, angle: 180.0, connected_to: None });
                conns.push(Connector { x: l, y, angle: 0.0, connected_to: None });
            }
            conns
        }
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
        "feeder" | "flex" if entry.radius.is_some() && entry.angle.is_some_and(|a| a.abs() > 0.1) => {
            let r = entry.radius.unwrap();
            let a = entry.angle.unwrap();
            let dir = if a < 0.0 { -1.0 } else { 1.0 };
            let rad = deg2rad(a.abs());
            let ex = r * rad.sin();
            let ey = dir * r * (1.0 - rad.cos());
            vec![
                Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: ex, y: ey, angle: norm_angle_deg(dir * a.abs()), connected_to: None },
            ]
        }
        "curved-turnout-left" | "curved-turnout-right" => {
            let dir = if entry.kind == "curved-turnout-left" { -1.0 } else { 1.0 };
            let r1 = entry.radius.unwrap_or(360.0);
            let a1 = entry.angle.unwrap_or(30.0);
            let lead = entry.branch_length.unwrap_or_else(|| {
                entry.branch_radius.filter(|&br| br > r1).map_or(0.0, |br| br - r1)
            });
            let r2 = if entry.branch_length.is_some() {
                entry.branch_radius.unwrap_or(r1)
            } else if entry.branch_radius.is_some_and(|br| br > r1) {
                r1
            } else {
                entry.branch_radius.unwrap_or(r1)
            };
            let a2 = entry.branch_angle.unwrap_or(30.0);
            let rad1 = deg2rad(a1);
            let rad2 = deg2rad(a2);
            vec![
                Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: r1 * rad1.sin(), y: dir * r1 * (1.0 - rad1.cos()), angle: norm_angle_deg(dir * a1), connected_to: None },
                Connector { x: lead + r2 * rad2.sin(), y: dir * r2 * (1.0 - rad2.cos()), angle: norm_angle_deg(dir * a2), connected_to: None },
            ]
        }
        "turnout-left" | "turnout-right" => {
            let dir = if entry.kind == "turnout-left" { -1.0 } else { 1.0 };
            if entry.radius.is_some() && entry.length.is_none() {
                let r1 = entry.radius.unwrap_or(360.0);
                let a1 = entry.angle.unwrap_or(30.0);
                let lead = entry.branch_length.unwrap_or_else(|| {
                    entry.branch_radius.filter(|&br| br > r1).map_or(0.0, |br| br - r1)
                });
                let r2 = if entry.branch_length.is_some() {
                    entry.branch_radius.unwrap_or(r1)
                } else if entry.branch_radius.is_some_and(|br| br > r1) {
                    r1
                } else {
                    entry.branch_radius.unwrap_or(r1)
                };
                let a2 = entry.branch_angle.unwrap_or(30.0);
                let rad1 = deg2rad(a1);
                let rad2 = deg2rad(a2);
                vec![
                    Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                    Connector { x: r1 * rad1.sin(), y: dir * r1 * (1.0 - rad1.cos()), angle: norm_angle_deg(dir * a1), connected_to: None },
                    Connector { x: lead + r2 * rad2.sin(), y: dir * r2 * (1.0 - rad2.cos()), angle: norm_angle_deg(dir * a2), connected_to: None },
                ]
            } else {
                let len = entry.length.unwrap_or(188.3);
                let br = entry.branch_radius.unwrap_or(437.5);
                let ba = entry.branch_angle.unwrap_or(24.3);
                let rad = deg2rad(ba);
                let bx = br * rad.sin();
                let by = dir * br * (1.0 - rad.cos());
                vec![
                    Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                    Connector { x: len, y: 0.0, angle: 0.0, connected_to: None },
                    Connector { x: bx, y: by, angle: norm_angle_deg(dir * ba), connected_to: None },
                ]
            }
        }
        "three-way" => {
            let len = entry.length.unwrap_or(188.3);
            let br = entry.branch_radius.unwrap_or(437.5);
            let ba = entry.branch_angle.unwrap_or(24.3);
            let rad = deg2rad(ba);
            let bx = br * rad.sin();
            let by = br * (1.0 - rad.cos());
            vec![
                Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: len, y: 0.0, angle: 0.0, connected_to: None },
                Connector { x: bx, y: -by, angle: norm_angle_deg(-ba), connected_to: None },
                Connector { x: bx, y: by, angle: norm_angle_deg(ba), connected_to: None },
            ]
        }
        "crossing" => {
            let len = entry.length.unwrap_or(188.3);
            let a = entry.angle.unwrap_or(24.3);
            let rad = deg2rad(a);
            let h = len * 0.5;
            vec![
                Connector { x: -h, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: h, y: 0.0, angle: 0.0, connected_to: None },
                Connector { x: -h * rad.cos(), y: -h * rad.sin(), angle: norm_angle_deg(180.0 + a), connected_to: None },
                Connector { x: h * rad.cos(), y: h * rad.sin(), angle: norm_angle_deg(a), connected_to: None },
            ]
        }
        "double-slip" => {
            let len = entry.length.unwrap_or(188.3);
            let a = entry.angle.or(entry.branch_angle).unwrap_or(24.3);
            let rad = deg2rad(a);
            let h = len * 0.5;
            let cx = h;
            vec![
                Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None },
                Connector { x: len, y: 0.0, angle: 0.0, connected_to: None },
                Connector { x: cx - h * rad.cos(), y: -h * rad.sin(), angle: norm_angle_deg(180.0 + a), connected_to: None },
                Connector { x: cx + h * rad.cos(), y: h * rad.sin(), angle: norm_angle_deg(a), connected_to: None },
            ]
        }
        "buffer" => vec![Connector { x: 0.0, y: 0.0, angle: 180.0, connected_to: None }],
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

/// Computes world connectors for an existing `TrackPiece`, respecting flex overrides
/// (`custom_length`, `custom_radius`, `custom_angle`, `custom_offsets`, `custom_chord_dist`).
pub fn compute_piece_world_connectors(piece: &TrackPiece, entry: &CatalogEntry) -> Vec<Connector> {
    let mut eff = entry.clone();
    if let (Some(offs), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offs.len() >= 3 && chord > 1.0 {
            let m0 = 2.0 * (4.0 * offs[0] - offs[1]);
            let m4 = -2.0 * (4.0 * offs[2] - offs[1]);
            let a0 = norm_angle_deg(m0.atan2(chord).to_degrees() + 180.0);
            let a1 = norm_angle_deg(m4.atan2(chord).to_degrees());
            eff.local_connectors = Some(vec![
                railbench_core::types::LocalConnector { x: 0.0, y: 0.0, angle: a0 },
                railbench_core::types::LocalConnector { x: chord, y: 0.0, angle: a1 },
            ]);
            return compute_world_connectors(&eff, piece.transform);
        }
    }
    if piece.custom_length.is_some() || piece.custom_radius.is_some() || piece.custom_angle.is_some() {
        eff.local_connectors = None;
        eff.length = piece.custom_length.or(entry.length);
        eff.radius = piece.custom_radius.or(entry.radius);
        eff.angle = piece.custom_angle.or(entry.angle);
        if piece.custom_radius.is_some() && piece.custom_angle.is_some_and(|a| a.abs() > 0.1) {
            eff.kind = "curve".into();
        } else if eff.kind == "flex" {
            eff.kind = "straight".into();
        }
    }
    compute_world_connectors(&eff, piece.transform)
}

pub fn create_piece(id: String, entry: &CatalogEntry, tr: Transform, layer_id: Option<String>) -> TrackPiece {
    let connectors = compute_world_connectors(entry, tr);
    let is_feeder = is_feeder_entry(entry) || entry.id == "24088";
    let is_isolator = is_isolating_entry(entry);
    TrackPiece {
        id,
        catalog_id: entry.id.clone(),
        transform: tr,
        connectors,
        elevation: None,
        power_feed: if is_feeder { Some(true) } else { None },
        is_isolator: if is_isolator { Some(true) } else { None },
        isolated_connector_index: if is_isolator { Some(0) } else { None },
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
    let idx = builtin_index();
    if is_builtin_slice(catalogs, idx) {
        if let Some(&(ci, ei)) = idx.first_entry.get(catalog_id) {
            if let Some(entry) = catalogs.get(ci).and_then(|c| c.entries.get(ei)) {
                if entry.id == catalog_id {
                    return Some(entry);
                }
            }
        }
        return None;
    }

    for cat in catalogs {
        if let Some(e) = cat.entries.iter().find(|e| e.id == catalog_id) {
            return Some(e);
        }
    }
    None
}

pub fn has_three_rail_catalog(catalogs: &[Catalog], catalog_id: &str) -> bool {
    let idx = builtin_index();
    if is_builtin_slice(catalogs, idx) {
        return idx.three_rail_ids.contains(catalog_id);
    }
    for cat in catalogs {
        if cat.entries.iter().any(|e| e.id == catalog_id) && cat.conductors == Some(3) {
            return true;
        }
    }
    false
}

pub fn is_road_id(catalogs: &[Catalog], catalog_id: &str) -> bool {
    if let Some(e) = find_entry(catalogs, catalog_id) {
        if e.is_road == Some(true) {
            return true;
        }
    }
    catalog_id.starts_with("ST-") || catalog_id.starts_with("street-") || catalog_id.starts_with("road-")
}

pub fn is_flex_id(catalogs: &[Catalog], catalog_id: &str) -> bool {
    if let Some(e) = find_entry(catalogs, catalog_id) {
        if e.kind == "flex" {
            return true;
        }
    }
    catalog_id
        .as_bytes()
        .windows(4)
        .any(|w| w.eq_ignore_ascii_case(b"flex"))
}

fn entry_matches_keywords(e: &CatalogEntry, keywords: &[&str]) -> bool {
    let id_l = e.id.to_ascii_lowercase();
    let name_l = e.name.to_ascii_lowercase();
    let en_l = e.name_en.as_deref().unwrap_or("").to_ascii_lowercase();
    let desc_l = e.description.to_ascii_lowercase();
    keywords.iter().any(|kw| {
        id_l.contains(kw) || name_l.contains(kw) || en_l.contains(kw) || desc_l.contains(kw)
    })
}

/// Checks whether a catalog entry is a Feeder Track (1:1 with Electron `isFeederEntry`).
pub fn is_feeder_entry(entry: &CatalogEntry) -> bool {
    if entry.kind == "feeder" {
        return true;
    }
    entry_matches_keywords(
        entry,
        &[
            "anschlussgleis",
            "einspeisung",
            "einspeise",
            "anschluss-gleis",
            "power feeder",
            "feeder track",
            "stromanschluss",
        ],
    )
}

/// Checks whether a catalog entry is an Isolating Track (1:1 with Electron `isIsolatingEntry`).
pub fn is_isolating_entry(entry: &CatalogEntry) -> bool {
    if matches!(entry.kind.as_str(), "isolating" | "isolator") {
        return true;
    }
    entry_matches_keywords(
        entry,
        &[
            "trennstrecke",
            "trenngleis",
            "isoliergleis",
            "isolating track",
            "isolated track",
            "unterbrechergleis",
            "kontaktgleis",
            "isolier",
        ],
    )
}

/// Checks whether a catalog entry is an Uncoupler Track (e.g. Maerklin C-Gleis 24997 Entkupplungsgleis).
pub fn is_uncoupler_entry(entry: &CatalogEntry) -> bool {
    if entry.is_road == Some(true) {
        return false;
    }
    if entry_matches_keywords(entry, &["entkuppl", "uncoupl"]) {
        return true;
    }
    entry.kind == "uncoupler"
        && !entry_matches_keywords(entry, &["schaltgleis", "kontaktgleis"])
}

/// Finds the best Feeder Track entry in `catalog` (prefers straight feeder tracks,
/// falling back to any feeder or standard straight track in the catalog).
pub fn find_feeder_entry(catalog: &Catalog) -> Option<&CatalogEntry> {
    catalog
        .entries
        .iter()
        .find(|e| e.kind == "feeder" && e.radius.is_none())
        .or_else(|| catalog.entries.iter().find(|e| e.kind == "feeder"))
        .or_else(|| {
            catalog
                .entries
                .iter()
                .find(|e| e.radius.is_none() && entry_matches_keywords(e, &["anschlussgleis", "feeder"]))
        })
        .or_else(|| catalog.entries.iter().find(|e| e.kind == "straight"))
        .or_else(|| catalog.entries.first())
}

/// Finds the best Isolating Track entry in `catalog` (prefers straight isolating/Trenngleis tracks,
/// falling back to a short or standard straight track in the catalog).
pub fn find_isolating_entry(catalog: &Catalog) -> Option<&CatalogEntry> {
    let iso_kws = ["trenngleis", "isoliergleis", "isolating", "unterbrechergleis"];
    catalog
        .entries
        .iter()
        .find(|e| (e.kind == "isolator" || entry_matches_keywords(e, &iso_kws)) && e.radius.is_none())
        .or_else(|| {
            catalog
                .entries
                .iter()
                .find(|e| e.kind == "isolator" || entry_matches_keywords(e, &iso_kws))
        })
        .or_else(|| {
            catalog
                .entries
                .iter()
                .filter(|e| e.kind == "straight" && e.length.unwrap_or(999.0) <= 120.0)
                .min_by(|a, b| {
                    a.length
                        .unwrap_or(999.0)
                        .partial_cmp(&b.length.unwrap_or(999.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .or_else(|| catalog.entries.iter().find(|e| e.kind == "straight"))
        .or_else(|| catalog.entries.first())
}

/// Natural alphanumeric comparison for catalog article IDs (matches Electron `compareCatalogEntriesByNumber`).
pub fn compare_catalog_entries_by_number(a: &CatalogEntry, b: &CatalogEntry) -> std::cmp::Ordering {
    fn split_chunks(s: &str) -> Vec<(bool, String)> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut cur_num: Option<bool> = None;
        for ch in s.chars() {
            let is_d = ch.is_ascii_digit();
            if cur_num == Some(is_d) {
                cur.push(ch);
            } else {
                if let Some(was_num) = cur_num {
                    out.push((was_num, std::mem::take(&mut cur)));
                }
                cur.push(ch);
                cur_num = Some(is_d);
            }
        }
        if let Some(was_num) = cur_num {
            out.push((was_num, cur));
        }
        out
    }
    let ca = split_chunks(&a.id);
    let cb = split_chunks(&b.id);
    for (pa, pb) in ca.iter().zip(cb.iter()) {
        let ord = match (pa.0, pb.0) {
            (true, true) => {
                let na = pa.1.parse::<u64>().unwrap_or(0);
                let nb = pb.1.parse::<u64>().unwrap_or(0);
                na.cmp(&nb).then_with(|| pa.1.len().cmp(&pb.1.len()))
            }
            _ => pa.1.to_ascii_lowercase().cmp(&pb.1.to_ascii_lowercase()),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    ca.len().cmp(&cb.len())
}


/// Disconnects any `connected_to` links between tracks and roads so they never snap/join together.
pub fn disconnect_cross_type_connections(layout: &mut railbench_core::types::Layout, catalogs: &[Catalog]) {
    let road_flags: std::collections::HashMap<String, bool> = layout
        .pieces
        .iter()
        .map(|p| (p.id.clone(), is_road_id(catalogs, &p.catalog_id)))
        .collect();
    for p in &mut layout.pieces {
        let my_road = road_flags.get(&p.id).copied().unwrap_or(false);
        for c in &mut p.connectors {
            if let Some(ref link) = c.connected_to {
                let other_road = road_flags.get(&link.piece_id).copied().unwrap_or(false);
                if my_road != other_road {
                    c.connected_to = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_embedded_catalogs_parse() {
        let cats = builtin_catalogs();
        assert_eq!(cats.len(), TRACK_CATALOG_JSONS.len() + ROAD_CATALOG_JSONS.len());
        assert!(cats.iter().all(|c| !c.entries.is_empty()));
    }

    #[test]
    fn indexed_catalog_lookup_matches_linear_scan() {
        let cats = builtin_catalogs();
        for cat in &cats {
            for entry in &cat.entries {
                let linear = cats.iter().find_map(|c| c.entries.iter().find(|e| e.id == entry.id));
                let indexed = find_entry(&cats, &entry.id);
                assert_eq!(
                    indexed.map(|e| (&e.id, &e.kind)),
                    linear.map(|e| (&e.id, &e.kind)),
                    "mismatch for catalog_id {}",
                    entry.id
                );
            }
        }
        assert!(find_entry(&cats, "__missing_id__").is_none());
        // Subset slice fallback
        let single = &cats[0..1];
        let first_id = &cats[0].entries[0].id;
        assert_eq!(find_entry(single, first_id).map(|e| e.id.as_str()), Some(first_id.as_str()));
        // Three-rail detection: Maerklin C H0 (24188) vs Piko A H0 (55200)
        assert!(has_three_rail_catalog(&cats, "24188"));
        assert!(!has_three_rail_catalog(&cats, "55200"));
        assert!(!has_three_rail_catalog(&cats, "__missing__"));
        // Case-insensitive flex id fallback without allocation
        assert!(is_flex_id(&cats, "CUSTOM-FLEX-01"));
        assert!(!is_flex_id(&cats, "abc"));
    }

    #[test]
    fn flex_snap_endpoint_aligns_tangent_bends_and_shortens() {
        let cats = builtin_catalogs();
        let entry = find_entry(&cats, "24900")
            .or_else(|| cats.iter().flat_map(|c| c.entries.iter()).find(|e| e.kind == "flex"))
            .expect("flex entry");
        let mut piece = create_piece(
            "flex-1".into(),
            entry,
            Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            Some("layer-1".into()),
        );
        let orig_len = entry.length.unwrap_or(900.0);

        // Snap connector 1 to a track 450 mm ahead, 80 mm lateral offset, facing 150° (so flex exits at 330° / -30°)
        let ok = snap_flex_endpoint(&mut piece, &cats, 1, Vec2 { x: 450.0, y: 80.0 }, 150.0);
        assert!(ok, "flex endpoint should snap within range");
        assert!((piece.connectors[0].x - 0.0).abs() < 1e-6);
        assert!((piece.connectors[0].y - 0.0).abs() < 1e-6);
        assert!((piece.connectors[1].x - 450.0).abs() < 1e-6);
        assert!((piece.connectors[1].y - 80.0).abs() < 1e-6);
        // Exact 180° opposite of target connector angle (150° + 180° = 330°)
        assert!((piece.connectors[1].angle - 330.0).abs() < 1e-6);
        // Auto-shortened from ~900 mm down to curved arc length (~460-490 mm)
        let new_len = piece.custom_length.expect("custom_length set");
        assert!(new_len < orig_len && new_len > 450.0 && new_len < 520.0, "unexpected len {new_len}");
        assert!(piece.custom_offsets.is_none());

        // Now snap connector 0 while keeping connector 1 locked
        let ok0 = snap_flex_endpoint(&mut piece, &cats, 0, Vec2 { x: 50.0, y: -20.0 }, 10.0);
        assert!(ok0, "connector 0 should also snap while connector 1 stays anchored");
        assert!((piece.connectors[1].x - 450.0).abs() < 1e-6);
        assert!((piece.connectors[1].y - 80.0).abs() < 1e-6);
        assert!((piece.connectors[0].x - 50.0).abs() < 1e-6);
        assert!((piece.connectors[0].y - (-20.0)).abs() < 1e-6);
        assert!((piece.connectors[0].angle - 190.0).abs() < 1e-6);

        // Should expose all 5 handles (3 interior + c[1] + c[0])
        let handles = flex_bend_handles_world(&piece).expect("5 flex handles");
        assert!((handles[3].x - piece.connectors[1].x).abs() < 1e-6);
        assert!((handles[4].x - piece.connectors[0].x).abs() < 1e-6);

        // Over-length target beyond max flex length should be rejected
        assert!(!snap_flex_endpoint(&mut piece, &cats, 1, Vec2 { x: 5000.0, y: 0.0 }, 180.0));
    }

    #[test]
    fn flex_endpoint_drag_both_ends_and_spline_preserves_locked_angles() {
        let cats = builtin_catalogs();
        let entry = cats
            .iter()
            .flat_map(|c| c.entries.iter())
            .find(|e| e.kind == "flex")
            .expect("flex entry");
        let mut piece = create_piece(
            "flex-2".into(),
            entry,
            Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            Some("layer-1".into()),
        );

        // Free-drag c[1] to shorten to ~300 mm and bend
        apply_flex_endpoint_drag(&mut piece, &cats, 1, Vec2 { x: 280.0, y: 60.0 });
        let len1 = piece.custom_length.unwrap();
        assert!(len1 > 270.0 && len1 < 330.0, "expected shortened len, got {len1}");
        assert!((piece.connectors[0].x - 0.0).abs() < 1e-6);

        // Mark c[0] as connected and drag an interior bend handle -> c[0].angle must stay locked!
        let locked_a0 = piece.connectors[0].angle;
        piece.connectors[0].connected_to = Some(railbench_core::types::ConnectorRef {
            piece_id: "track-a".into(),
            connector_index: 1,
        });
        apply_flex_spline_offset(&mut piece, &cats, 1, Vec2 { x: 140.0, y: -40.0 });
        assert!((piece.connectors[0].angle - locked_a0).abs() < 1e-6);
        assert!(piece.custom_length.unwrap() > 250.0);
    }

    #[test]
    fn flex_bend_cut_flip_reset_and_ramp_info() {
        let cats = builtin_catalogs();
        let entry = find_entry(&cats, "24094").expect("24094 flex");
        let mut flex = create_piece(
            "flx-a".into(),
            entry,
            Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            None,
        );
        flex.elevation = Some(railbench_core::types::Elevation { start: 0.0, end: 18.0 });
        bend_flex_track(&mut flex, &cats, 600.0, Some(30.0));
        assert_eq!(flex.custom_radius, Some(600.0));
        let ri = get_flex_track_ramp_info(&flex, &cats);
        assert!(ri.is_ramp);
        assert!((ri.rise_mm - 18.0).abs() < 1e-6);

        let (first, second) = cut_flex_track(&flex, &cats, "flx-b".into()).expect("cut halves");
        assert_eq!(first.id, "flx-a");
        assert_eq!(second.id, "flx-b");
        assert!((first.elevation.unwrap().end - 9.0).abs() < 0.1);
        assert!((second.elevation.unwrap().start - 9.0).abs() < 0.1);

        let orig_end = (flex.connectors[1].x, flex.connectors[1].y);
        flip_flex_track_piece(&mut flex, &cats);
        assert!((flex.transform.x - orig_end.0).abs() < 0.2);
        assert!((flex.elevation.unwrap().start - 18.0).abs() < 0.1);

        reset_flex_track_to_original(&mut flex, &cats);
        assert!(flex.custom_radius.is_none());
        assert!(flex.custom_angle.is_none());
    }
}

mod flex;
pub use flex::*;
