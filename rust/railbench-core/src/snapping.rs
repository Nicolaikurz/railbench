use std::collections::HashMap;

use crate::geometry::norm_angle_deg;
use crate::types::{ConnectorRef, Layout, TrackPiece};

#[inline(always)]
fn spatial_cell_hash(gx: i32, gy: i32, mask: usize) -> usize {
    ((gx as u32).wrapping_mul(73_856_093) ^ (gy as u32).wrapping_mul(19_349_663)) as usize & mask
}

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

    // Flat intrusive linked-list spatial hash table:
    // zero per-cell heap allocations and fast integer hashing instead of HashMap<_, Vec<_>>.
    let table_size = (count * 2).next_power_of_two().max(64);
    let mask = table_size - 1;
    let mut head = vec![-1_i32; table_size];
    let mut next = vec![-1_i32; count];

    // Insert in reverse index order so bucket traversal visits connectors in ascending index order.
    for i in (0..count).rev() {
        let base = i * 6;
        let gx = (connectors[base] * inv_cell).floor() as i32;
        let gy = (connectors[base + 1] * inv_cell).floor() as i32;
        let h = spatial_cell_hash(gx, gy, mask);
        next[i] = head[h];
        head[h] = i as i32;
    }

    let dist_tol_sq = dist_tolerance * dist_tolerance;

    let match_one = |i: usize| -> (i32, i32) {
        let b1 = i * 6;
        let x1 = connectors[b1];
        let y1 = connectors[b1 + 1];
        let a1 = connectors[b1 + 2];
        let h1 = connectors[b1 + 3];
        let p1 = connectors[b1 + 4] as i32;

        let gx = (x1 * inv_cell).floor() as i32;
        let gy = (y1 * inv_cell).floor() as i32;

        for dx in -1..=1 {
            let cx = gx + dx;
            for dy in -1..=1 {
                let mut cur = head[spatial_cell_hash(cx, gy + dy, mask)];
                while cur >= 0 {
                    let j = cur as usize;
                    cur = next[j];
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
                        return (p2, connectors[b2 + 5] as i32);
                    }
                }
            }
        }
        (-1, -1)
    };

    #[cfg(not(target_arch = "wasm32"))]
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    #[cfg(target_arch = "wasm32")]
    let threads = 1usize;

    // Only pay OS thread-spawn overhead on very large connector sets where parallel scan wins.
    if threads > 1 && count >= 32_768 {
        let items_per_chunk = (count + threads - 1) / threads;
        let chunk_elems = items_per_chunk * 2;
        std::thread::scope(|s| {
            for (chunk_idx, chunk_slice) in out.chunks_mut(chunk_elems).enumerate() {
                let start_i = chunk_idx * items_per_chunk;
                s.spawn(move || {
                    for (local_i, pair) in chunk_slice.chunks_mut(2).enumerate() {
                        let (mp, mc) = match_one(start_i + local_i);
                        pair[0] = mp;
                        pair[1] = mc;
                    }
                });
            }
        });
    } else {
        for (i, pair) in out.chunks_mut(2).enumerate() {
            let (mp, mc) = match_one(i);
            pair[0] = mp;
            pair[1] = mc;
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
    let total_conns: usize = layout.pieces.iter().map(|p| p.connectors.len()).sum();
    if total_conns == 0 {
        return;
    }

    // Fast path: avoid allocating a HashMap when all layers are at height 0.
    let has_nonzero_layer = layout
        .layers
        .as_ref()
        .is_some_and(|layers| layers.iter().any(|l| l.height_mm.unwrap_or(0.0) != 0.0));
    let layer_heights: HashMap<&str, f64> = if has_nonzero_layer {
        let layers = layout.layers.as_ref().unwrap();
        let mut map = HashMap::with_capacity(layers.len());
        for l in layers {
            map.insert(l.id.as_str(), l.height_mm.unwrap_or(0.0));
        }
        map
    } else {
        HashMap::new()
    };

    let mut flat = Vec::with_capacity(total_conns * 6);
    for (p_idx, p) in layout.pieces.iter().enumerate() {
        let lh = if has_nonzero_layer {
            p.layer_id
                .as_deref()
                .and_then(|id| layer_heights.get(id).copied())
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let p_idx_f = p_idx as f64;
        for (c_idx, c) in p.connectors.iter().enumerate() {
            flat.push(c.x);
            flat.push(c.y);
            flat.push(c.angle);
            flat.push(piece_connector_height(p, c_idx, lh));
            flat.push(p_idx_f);
            flat.push(c_idx as f64);
        }
    }

    let matched = match_connectors_spatial(&flat, dist_tolerance, angle_tolerance, height_tolerance);

    // Diff pass: only allocate/clone String IDs for connectors whose link actually changed.
    let piece_count = layout.pieces.len();
    let mut updates: Vec<(usize, usize, Option<ConnectorRef>)> = Vec::new();
    let mut cursor = 0usize;
    for (p_idx, p) in layout.pieces.iter().enumerate() {
        for (c_idx, c) in p.connectors.iter().enumerate() {
            let mp = matched[cursor * 2];
            let mc = matched[cursor * 2 + 1];
            cursor += 1;
            if mp >= 0 && (mp as usize) < piece_count && mc >= 0 {
                let target_id = layout.pieces[mp as usize].id.as_str();
                let target_ci = mc as usize;
                let unchanged = c
                    .connected_to
                    .as_ref()
                    .is_some_and(|r| r.connector_index == target_ci && r.piece_id == target_id);
                if !unchanged {
                    updates.push((
                        p_idx,
                        c_idx,
                        Some(ConnectorRef {
                            piece_id: target_id.to_owned(),
                            connector_index: target_ci,
                        }),
                    ));
                }
            } else if c.connected_to.is_some() {
                updates.push((p_idx, c_idx, None));
            }
        }
    }

    for (p_idx, c_idx, new_ref) in updates {
        layout.pieces[p_idx].connectors[c_idx].connected_to = new_ref;
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

    #[test]
    fn update_layout_connections_connect_idempotent_and_disconnect() {
        let mut layout = crate::io::load_layout_json(
            r#"{
                "name": "SnapTest",
                "scale": "H0",
                "pieces": [
                    {
                        "id": "a",
                        "catalogId": "24188",
                        "transform": { "x": 0.0, "y": 0.0, "rotation": 0.0 },
                        "connectors": [
                            { "x": 0.0, "y": 0.0, "angle": 180.0 },
                            { "x": 188.3, "y": 0.0, "angle": 0.0 }
                        ]
                    },
                    {
                        "id": "b",
                        "catalogId": "24188",
                        "transform": { "x": 188.3, "y": 0.0, "rotation": 0.0 },
                        "connectors": [
                            { "x": 188.3, "y": 0.0, "angle": 180.0 },
                            { "x": 376.6, "y": 0.0, "angle": 0.0 }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
        assert_eq!(
            layout.pieces[0].connectors[1].connected_to,
            Some(ConnectorRef { piece_id: "b".into(), connector_index: 0 })
        );
        assert_eq!(
            layout.pieces[1].connectors[0].connected_to,
            Some(ConnectorRef { piece_id: "a".into(), connector_index: 1 })
        );

        // Second call should be idempotent (no-op diff)
        update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
        assert_eq!(
            layout.pieces[0].connectors[1].connected_to,
            Some(ConnectorRef { piece_id: "b".into(), connector_index: 0 })
        );

        // Move piece b away -> connections should clear
        layout.pieces[1].connectors[0].x += 50.0;
        update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
        assert_eq!(layout.pieces[0].connectors[1].connected_to, None);
        assert_eq!(layout.pieces[1].connectors[0].connected_to, None);
    }
}
