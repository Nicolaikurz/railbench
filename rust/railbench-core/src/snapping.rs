use std::collections::HashMap;

use crate::geometry::norm_angle_deg;
use crate::types::{ConnectorRef, Layout, TrackPiece};

/// Spatial-hashed connector matcher.
///
/// `connectors` has stride 6 per connector: `[x, y, angle_deg, height, piece_idx, conn_idx]`.
/// Returns `Vec<i32>` of length `2 * count`: `[matched_piece_idx, matched_conn_idx, ...]`
/// (`-1, -1` when unmatched).
pub fn match_connectors_spatial(
    connectors: &[f64],
    dist_tolerance: f64,
    angle_tolerance: f64,
    height_tolerance: f64,
) -> Vec<i32> {
    let count = connectors.len() / 6;
    let mut out = vec![-1_i32; count * 2];
    if count < 2 {
        return out;
    }

    let cell_size = (dist_tolerance * 4.0).max(16.0);
    let inv_cell = 1.0 / cell_size;
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::with_capacity(count);

    for i in 0..count {
        let base = i * 6;
        let gx = (connectors[base] * inv_cell).floor() as i32;
        let gy = (connectors[base + 1] * inv_cell).floor() as i32;
        grid.entry((gx, gy)).or_default().push(i);
    }

    let dist_tol_sq = dist_tolerance * dist_tolerance;

    for i in 0..count {
        let b1 = i * 6;
        let x1 = connectors[b1];
        let y1 = connectors[b1 + 1];
        let a1 = connectors[b1 + 2];
        let h1 = connectors[b1 + 3];
        let p1 = connectors[b1 + 4] as i32;

        let gx = (x1 * inv_cell).floor() as i32;
        let gy = (y1 * inv_cell).floor() as i32;

        'search: for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(bucket) = grid.get(&(gx + dx, gy + dy)) {
                    for &j in bucket {
                        let b2 = j * 6;
                        let p2 = connectors[b2 + 4] as i32;
                        if p2 == p1 {
                            continue;
                        }
                        if (h1 - connectors[b2 + 3]).abs() > height_tolerance {
                            continue;
                        }
                        let ddx = x1 - connectors[b2];
                        let ddy = y1 - connectors[b2 + 1];
                        if ddx * ddx + ddy * ddy > dist_tol_sq {
                            continue;
                        }
                        let diff_a = (norm_angle_deg(a1 - connectors[b2 + 2]) - 180.0).abs();
                        if diff_a <= angle_tolerance {
                            out[i * 2] = p2;
                            out[i * 2 + 1] = connectors[b2 + 5] as i32;
                            break 'search;
                        }
                    }
                }
            }
        }
    }

    out
}

/// Computes the world elevation (mm) of a track piece connector using its layer height + piece elevation.
pub fn piece_connector_height(piece: &TrackPiece, connector_index: usize, layer_height_mm: f64) -> f64 {
    let local = match piece.elevation {
        Some(el) => {
            if connector_index == 0 {
                el.start
            } else {
                el.end
            }
        }
        None => 0.0,
    };
    layer_height_mm + local
}

/// High-level helper for native Rust apps: rebuilds all `connected_to` links in a `Layout`.
pub fn update_layout_connections(
    layout: &mut Layout,
    dist_tolerance: f64,
    angle_tolerance: f64,
    height_tolerance: f64,
) {
    let mut layer_heights: HashMap<&str, f64> = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_heights.insert(l.id.as_str(), l.height_mm.unwrap_or(0.0));
        }
    }

    let mut flat = Vec::new();
    for (p_idx, p) in layout.pieces.iter().enumerate() {
        let lh = p
            .layer_id
            .as_deref()
            .and_then(|id| layer_heights.get(id).copied())
            .unwrap_or(0.0);
        for (c_idx, c) in p.connectors.iter().enumerate() {
            flat.push(c.x);
            flat.push(c.y);
            flat.push(c.angle);
            flat.push(piece_connector_height(p, c_idx, lh));
            flat.push(p_idx as f64);
            flat.push(c_idx as f64);
        }
    }

    let matched = match_connectors_spatial(&flat, dist_tolerance, angle_tolerance, height_tolerance);
    let piece_ids: Vec<String> = layout.pieces.iter().map(|p| p.id.clone()).collect();
    let mut cursor = 0usize;
    for p in &mut layout.pieces {
        for c in &mut p.connectors {
            let mp = matched[cursor * 2];
            let mc = matched[cursor * 2 + 1];
            if mp >= 0 && (mp as usize) < piece_ids.len() && mc >= 0 {
                c.connected_to = Some(ConnectorRef {
                    piece_id: piece_ids[mp as usize].clone(),
                    connector_index: mc as usize,
                });
            } else {
                c.connected_to = None;
            }
            cursor += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_connector_matching() {
        let conns = [
            100.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            101.0, 0.0, 180.0, 0.0, 1.0, 0.0,
        ];
        let matched = match_connectors_spatial(&conns, 4.0, 35.0, 1.0);
        assert_eq!(matched, vec![1, 0, 0, 0]);
    }
}
