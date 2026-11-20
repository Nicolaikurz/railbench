use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Copy, Clone)]
struct State {
    res: f64,
    dist_m: f64,
    node: usize,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.res.to_bits() == other.res.to_bits() && self.node == other.node
    }
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .res
            .total_cmp(&self.res)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Multi-source Dijkstra solver for electrical rail / catenary resistance networks.
///
/// Returns a flat `Vec<f64>` of length `4 * node_count`:
/// `[min_resistance..., min_distance..., parent_index..., edge_res_to_parent...]`.
pub fn solve_power_dijkstra(
    node_count: usize,
    adj_offsets: &[u32],
    adj_neighbors: &[u32],
    piece_len_m: &[f64],
    feeder_indices: &[u32],
    rail_ohm_per_meter: f64,
    joint_resistance_ohm: f64,
) -> Vec<f64> {
    let mut out = vec![0.0_f64; node_count * 4];
    if node_count == 0 {
        return out;
    }

    let (min_res, rest) = out.split_at_mut(node_count);
    let (min_dist, rest) = rest.split_at_mut(node_count);
    let (parent, edge_res_to_parent) = rest.split_at_mut(node_count);

    for i in 0..node_count {
        min_res[i] = f64::INFINITY;
        min_dist[i] = f64::INFINITY;
        parent[i] = -1.0;
        edge_res_to_parent[i] = 0.0;
    }

    let mut heap = BinaryHeap::with_capacity(node_count);

    for &f_idx_u32 in feeder_indices {
        let f_idx = f_idx_u32 as usize;
        if f_idx >= node_count {
            continue;
        }
        let initial_res = (piece_len_m[f_idx] * 0.5) * rail_ohm_per_meter;
        if initial_res < min_res[f_idx] {
            min_res[f_idx] = initial_res;
            min_dist[f_idx] = 0.0;
            parent[f_idx] = -1.0;
            edge_res_to_parent[f_idx] = initial_res;
            heap.push(State {
                res: initial_res,
                dist_m: 0.0,
                node: f_idx,
            });
        }
    }

    while let Some(State { res, dist_m, node }) = heap.pop() {
        if res > min_res[node] {
            continue;
        }

        let start = adj_offsets.get(node).copied().unwrap_or(0) as usize;
        let end = adj_offsets.get(node + 1).copied().unwrap_or(start as u32) as usize;
        let end = end.min(adj_neighbors.len());

        for &nbr_u32 in &adj_neighbors[start..end] {
            let nbr = nbr_u32 as usize;
            if nbr >= node_count {
                continue;
            }
            let len_m = piece_len_m[nbr];
            let edge_res = len_m * rail_ohm_per_meter + joint_resistance_ohm;
            let new_res = res + edge_res;
            let new_dist = dist_m + len_m;

            if new_res < min_res[nbr] {
                min_res[nbr] = new_res;
                min_dist[nbr] = new_dist;
                parent[nbr] = node as f64;
                edge_res_to_parent[nbr] = edge_res;
                heap.push(State {
                    res: new_res,
                    dist_m: new_dist,
                    node: nbr,
                });
            }
        }
    }

    out
}

/// Union-Find (DSU) short-circuit solver for 2-wire railway polarity checks.
pub fn solve_short_circuit_dsu(
    terminal_count: usize,
    edges: &[u32],
    rail_pairs: &[u32],
) -> Vec<u8> {
    let edge_count = edges.len() / 4;
    let pair_count = rail_pairs.len() / 2;
    let mut out = vec![0_u8; edge_count + pair_count];
    if terminal_count == 0 {
        return out;
    }

    let mut parent: Vec<usize> = (0..terminal_count).collect();
    let mut rank = vec![0_u8; terminal_count];

    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            let gp = parent[parent[x]];
            parent[x] = gp;
            x = gp;
        }
        x
    }

    fn union(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra == rb {
            return;
        }
        if rank[ra] < rank[rb] {
            parent[ra] = rb;
        } else if rank[ra] > rank[rb] {
            parent[rb] = ra;
        } else {
            parent[rb] = ra;
            rank[ra] = rank[ra].saturating_add(1);
        }
    }

    for e in 0..edge_count {
        let b = e * 4;
        let n1l = edges[b] as usize;
        let n1r = edges[b + 1] as usize;
        let n2l = edges[b + 2] as usize;
        let n2r = edges[b + 3] as usize;
        if n1l >= terminal_count
            || n1r >= terminal_count
            || n2l >= terminal_count
            || n2r >= terminal_count
        {
            continue;
        }
        let r1l = find(&mut parent, n1l);
        let r1r = find(&mut parent, n1r);
        let r2l = find(&mut parent, n2l);
        let r2r = find(&mut parent, n2r);

        let will_short = r1l == r2l || r1r == r2r || r1l == r1r || r2l == r2r;
        if will_short && r1l != r2r {
            out[e] = 1;
        }
        union(&mut parent, &mut rank, n1l, n2r);
        union(&mut parent, &mut rank, n1r, n2l);
    }

    for p in 0..pair_count {
        let a = rail_pairs[p * 2] as usize;
        let b = rail_pairs[p * 2 + 1] as usize;
        if a < terminal_count && b < terminal_count && find(&mut parent, a) == find(&mut parent, b)
        {
            out[edge_count + p] = 1;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dijkstra_linear_chain() {
        let offsets = [0u32, 1, 3, 4];
        let neighbors = [1u32, 0, 2, 1];
        let lens = [0.2_f64, 0.2, 0.2];
        let feeders = [0u32];
        let out = solve_power_dijkstra(3, &offsets, &neighbors, &lens, &feeders, 0.1, 0.01);
        assert!((out[0] - 0.01).abs() < 1e-9);
        assert!((out[1] - 0.04).abs() < 1e-9);
        assert!((out[2] - 0.07).abs() < 1e-9);
    }
}
