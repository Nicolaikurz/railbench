use railbench_core::geometry::{deg2rad, norm_angle_deg, rotate};
use railbench_core::types::{Catalog, CatalogEntry, Connector, TrackPiece, Transform, Vec2};

const TRACK_CATALOG_JSONS: &[(&str, &str)] = &[
    ("maerklin-c-h0", include_str!("../../../src/catalog/systems/maerklin-c-h0.json")),
    ("maerklin-k-h0", include_str!("../../../src/catalog/systems/maerklin-k-h0.json")),
    ("maerklin-m-h0", include_str!("../../../src/catalog/systems/maerklin-m-h0.json")),
    ("trix-c-h0", include_str!("../../../src/catalog/systems/trix-c-h0.json")),
    ("roco-line-h0", include_str!("../../../src/catalog/systems/roco-line-h0.json")),
    ("piko-a-h0", include_str!("../../../src/catalog/systems/piko-a-h0.json")),
    ("tillig-elite-h0", include_str!("../../../src/catalog/systems/tillig-elite-h0.json")),
    ("peco-streamline-h0-code100", include_str!("../../../src/catalog/systems/peco-streamline-h0-code100.json")),
    ("kato-unitrack-ho", include_str!("../../../src/catalog/systems/kato-unitrack-ho.json")),
    ("bachmann-ez-track-h0", include_str!("../../../src/catalog/systems/bachmann-ez-track-h0.json")),
    ("fleischmann-piccolo-n", include_str!("../../../src/catalog/systems/fleischmann-piccolo-n.json")),
    ("minitrix-n", include_str!("../../../src/catalog/systems/minitrix-n.json")),
    ("kato-unitrack-n", include_str!("../../../src/catalog/systems/kato-unitrack-n.json")),
    ("peco-streamline-n-code55", include_str!("../../../src/catalog/systems/peco-streamline-n-code55.json")),
    ("bachmann-ez-track-n", include_str!("../../../src/catalog/systems/bachmann-ez-track-n.json")),
    ("tillig-tt", include_str!("../../../src/catalog/systems/tillig-tt.json")),
    ("tillig-bettungsgleis-tt", include_str!("../../../src/catalog/systems/tillig-bettungsgleis-tt.json")),
    ("kuehn-gleis-tt", include_str!("../../../src/catalog/systems/kuehn-gleis-tt.json")),
    ("maerklin-z", include_str!("../../../src/catalog/systems/maerklin-z.json")),
    ("rokuhan-z", include_str!("../../../src/catalog/systems/rokuhan-z.json")),
    ("lenz-0", include_str!("../../../src/catalog/systems/lenz-0.json")),
    ("maerklin-1", include_str!("../../../src/catalog/systems/maerklin-1.json")),
    ("lgb-g", include_str!("../../../src/catalog/systems/lgb-g.json")),
];

const ROAD_CATALOG_JSONS: &[(&str, &str)] = &[
    ("generic-streets-h0", include_str!("../../../src/catalog/systems/generic-streets-h0.json")),
    ("generic-streets-n", include_str!("../../../src/catalog/systems/generic-streets-n.json")),
    ("generic-streets-tt", include_str!("../../../src/catalog/systems/generic-streets-tt.json")),
    ("generic-streets-z", include_str!("../../../src/catalog/systems/generic-streets-z.json")),
    ("generic-streets-0", include_str!("../../../src/catalog/systems/generic-streets-0.json")),
    ("generic-streets-1", include_str!("../../../src/catalog/systems/generic-streets-1.json")),
    ("generic-streets-g", include_str!("../../../src/catalog/systems/generic-streets-g.json")),
];

pub fn builtin_catalogs() -> Vec<Catalog> {
    let mut out = Vec::with_capacity(TRACK_CATALOG_JSONS.len() + ROAD_CATALOG_JSONS.len());
    for (name, raw) in TRACK_CATALOG_JSONS {
        match serde_json::from_str::<Catalog>(raw) {
            Ok(mut cat) => {
                let default_gauge = cat.gauge;
                for e in &mut cat.entries {
                    if e.gauge.is_none() {
                        e.gauge = default_gauge;
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

pub fn create_piece(id: String, entry: &CatalogEntry, tr: Transform, layer_id: Option<String>) -> TrackPiece {
    let connectors = compute_world_connectors(entry, tr);
    let is_feeder = entry.kind == "feeder" || entry.id == "24088";
    let is_isolator = entry.kind == "isolator";
    TrackPiece {
        id,
        catalog_id: entry.id.clone(),
        transform: tr,
        connectors,
        elevation: None,
        power_feed: if is_feeder { Some(true) } else { None },
        is_isolator: if is_isolator { Some(true) } else { None },
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
    catalog_id.to_lowercase().contains("flex")
}

/// Recomputes a flex piece's connector 1 (`c[1]`) from `custom_length` (mm) and `custom_angle` (deg)
/// while keeping connector 0 (`c[0]`) anchored at its current world position & angle.
pub fn apply_flex_geometry(piece: &mut TrackPiece, catalogs: &[Catalog], length_mm: f64, angle_deg: f64) {
    let default_len = find_entry(catalogs, &piece.catalog_id)
        .and_then(|e| e.length)
        .unwrap_or(900.0);
    let len = length_mm.clamp(25.0, default_len.max(1200.0));
    let ang = angle_deg.clamp(-180.0, 180.0);

    piece.custom_length = Some(len);
    piece.custom_angle = Some(ang);
    piece.custom_offsets = None;
    piece.custom_chord_dist = None;

    if piece.connectors.len() < 2 {
        return;
    }
    let x0 = piece.connectors[0].x;
    let y0 = piece.connectors[0].y;
    let entry_heading = norm_angle_deg(piece.connectors[0].angle + 180.0);

    let (local_x, local_y) = if ang.abs() < 0.25 {
        piece.custom_radius = None;
        (len, 0.0)
    } else {
        let rad = deg2rad(ang.abs());
        let r = len / rad;
        piece.custom_radius = Some(r);
        let dir = if ang < 0.0 { -1.0 } else { 1.0 };
        (r * rad.sin(), dir * r * (1.0 - rad.cos()))
    };

    let rp = rotate(Vec2 { x: local_x, y: local_y }, entry_heading);
    piece.connectors[1].x = x0 + rp.x;
    piece.connectors[1].y = y0 + rp.y;
    piece.connectors[1].angle = norm_angle_deg(entry_heading + ang);
    piece.connectors[1].connected_to = None;
}

/// Returns the 4 interactive flex handles in world coordinates:
/// `[bend_25%, bend_50%, bend_75%, endpoint_c1]`.
pub fn flex_bend_handles_world(piece: &TrackPiece) -> Option<[Vec2; 4]> {
    if piece.connectors.len() < 2 {
        return None;
    }
    let c0 = &piece.connectors[0];
    let c1 = &piece.connectors[1];
    let (chord, rot, origin_x, origin_y, offsets) =
        if let (Some(offs), Some(ch)) = (&piece.custom_offsets, piece.custom_chord_dist) {
            if offs.len() >= 3 {
                (ch, piece.transform.rotation, piece.transform.x, piece.transform.y, [offs[0], offs[1], offs[2]])
            } else {
                let dx = c1.x - c0.x;
                let dy = c1.y - c0.y;
                (dx.hypot(dy).max(1.0), dy.atan2(dx).to_degrees(), c0.x, c0.y, [0.0, 0.0, 0.0])
            }
        } else {
            let dx = c1.x - c0.x;
            let dy = c1.y - c0.y;
            let ch = dx.hypot(dy).max(1.0);
            let ang = dy.atan2(dx).to_degrees();
            // If currently a circular arc, sample its 25/50/75% points as initial handle positions
            let pts = railbench_core::geometry::sample_arc_between_connectors(
                c0.x, c0.y, c0.angle, c1.x, c1.y, c1.angle, 12,
            );
            if pts.len() >= 13 {
                return Some([pts[3], pts[6], pts[9], Vec2 { x: c1.x, y: c1.y }]);
            }
            (ch, ang, c0.x, c0.y, [0.0, 0.0, 0.0])
        };

    let mut out = [Vec2 { x: 0.0, y: 0.0 }; 4];
    for (i, &t) in [0.25_f64, 0.50, 0.75].iter().enumerate() {
        let rp = rotate(Vec2 { x: chord * t, y: offsets[i] }, rot);
        out[i] = Vec2 {
            x: origin_x + rp.x,
            y: origin_y + rp.y,
        };
    }
    out[3] = Vec2 { x: c1.x, y: c1.y };
    Some(out)
}

/// Dragging one of the 3 interior flex bend points (`handle_idx` in `0..3`) updates
/// `custom_offsets` (`[h1, h2, h3]`) and `custom_chord_dist` (1:1 with TS flextrack splines).
pub fn apply_flex_spline_offset(piece: &mut TrackPiece, handle_idx: usize, world_pos: Vec2) {
    if piece.connectors.len() < 2 || handle_idx >= 3 {
        return;
    }
    let c0x = piece.connectors[0].x;
    let c0y = piece.connectors[0].y;
    let c1x = piece.connectors[1].x;
    let c1y = piece.connectors[1].y;

    // Ensure piece is initialized in spline coordinates anchored at c[0] -> c[1]
    if piece.custom_offsets.as_ref().map_or(true, |o| o.len() < 3) || piece.custom_chord_dist.is_none() {
        let dx = c1x - c0x;
        let dy = c1y - c0y;
        let chord = dx.hypot(dy).max(25.0);
        let chord_rot = norm_angle_deg(dy.atan2(dx).to_degrees());

        // Seed initial 3 offsets from current arc shape so there's no jump on first drag
        let pts = railbench_core::geometry::sample_arc_between_connectors(
            c0x, c0y, piece.connectors[0].angle, c1x, c1y, piece.connectors[1].angle, 12,
        );
        let mut init_offs = vec![0.0, 0.0, 0.0];
        if pts.len() >= 13 {
            for (i, &pi) in [3usize, 6, 9].iter().enumerate() {
                let loc = rotate(Vec2 { x: pts[pi].x - c0x, y: pts[pi].y - c0y }, -chord_rot);
                init_offs[i] = loc.y;
            }
        }
        piece.transform.x = c0x;
        piece.transform.y = c0y;
        piece.transform.rotation = chord_rot;
        piece.custom_chord_dist = Some(chord);
        piece.custom_offsets = Some(init_offs);
    }

    let chord = piece.custom_chord_dist.unwrap_or(200.0).max(25.0);
    let rot = piece.transform.rotation;
    let local = rotate(
        Vec2 {
            x: world_pos.x - piece.transform.x,
            y: world_pos.y - piece.transform.y,
        },
        -rot,
    );
    let max_off = chord * 0.65;
    if let Some(offs) = &mut piece.custom_offsets {
        offs[handle_idx] = local.y.clamp(-max_off, max_off);
        let (h1, h2, h3) = (offs[0], offs[1], offs[2]);
        // Update connector 0 & 1 angles from the Hermite spline endpoint slopes m0 & m4
        let m0 = 2.0 * (4.0 * h1 - h2);
        let m4 = -2.0 * (4.0 * h3 - h2);
        let tan0_deg = m0.atan2(chord).to_degrees();
        let tan1_deg = m4.atan2(chord).to_degrees();
        piece.connectors[0].angle = norm_angle_deg(rot + tan0_deg + 180.0);
        piece.connectors[1].angle = norm_angle_deg(rot + tan1_deg);
        let end_rp = rotate(Vec2 { x: chord, y: 0.0 }, rot);
        piece.connectors[1].x = piece.transform.x + end_rp.x;
        piece.connectors[1].y = piece.transform.y + end_rp.y;
    }
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
}
