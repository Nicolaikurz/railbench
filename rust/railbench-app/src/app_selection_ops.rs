use crate::*;

impl RailbenchApp {
    pub fn append_track_to_connector(
        &mut self,
        anchor_piece_id: &str,
        connector_idx: usize,
        specific_catalog_id: Option<&str>,
    ) -> bool {
        let Some(anchor) = self.layout.pieces.iter().find(|p| p.id == anchor_piece_id).cloned() else {
            return false;
        };
        let Some(anchor_conn) = anchor.connectors.get(connector_idx).cloned() else {
            return false;
        };
        if anchor_conn.connected_to.is_some() {
            return false;
        }

        let anchor_is_road = crate::catalog::is_road_id(&self.catalogs, &anchor.catalog_id);
        let anchor_entry = crate::catalog::find_entry(&self.catalogs, &anchor.catalog_id);
        let anchor_gauge = anchor_entry.and_then(|e| e.gauge);

        let chosen_entry = specific_catalog_id
            .and_then(|cid| crate::catalog::find_entry(&self.catalogs, cid))
            .or_else(|| {
                self.last_placed_catalog_id
                    .as_deref()
                    .and_then(|cid| crate::catalog::find_entry(&self.catalogs, cid))
                    .filter(|e| {
                        let e_is_road = e.is_road == Some(true)
                            || crate::catalog::is_road_id(&self.catalogs, &e.id);
                        let gauge_ok = match (anchor_gauge, e.gauge) {
                            (Some(ag), Some(eg)) => (ag - eg).abs() < 2.0,
                            _ => true,
                        };
                        e_is_road == anchor_is_road && gauge_ok && e.kind != "turntable"
                    })
            })
            .or(anchor_entry);

        let Some(entry) = chosen_entry.cloned() else {
            return false;
        };

        let locals = crate::catalog::local_connectors(&entry);
        if locals.is_empty() {
            return false;
        }
        let mut visited = HashSet::new();
        let use_end_conn = entry.kind == "curve"
            && locals.len() >= 2
            && self.detect_track_turn(&anchor, connector_idx, &mut visited) == "right";
        let moving_local = if use_end_conn { &locals[1] } else { &locals[0] };

        let rot = norm_angle_deg(anchor_conn.angle + 180.0 - moving_local.angle);
        let ro = rotate(Vec2 { x: moving_local.x, y: moving_local.y }, rot);
        let tr = Transform {
            x: anchor_conn.x - ro.x,
            y: anchor_conn.y - ro.y,
            rotation: rot,
        };

        self.push_undo();
        let new_id = format!("piece-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let mut piece = create_piece(new_id.clone(), &entry, tr, anchor.layer_id.clone());
        if let Some(el) = anchor.elevation {
            let elev_v = if connector_idx == 0 { el.start } else { el.end };
            if elev_v.abs() > 0.01 {
                piece.elevation = Some(Elevation { start: elev_v, end: elev_v });
            }
        }
        if anchor.has_catenary == Some(true) {
            piece.has_catenary = Some(true);
        }

        self.layout.pieces.push(piece);
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.selected.clear();
        self.selected.insert(new_id);
        self.last_placed_catalog_id = Some(entry.id.clone());
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("Gleis #{} angesetzt", entry.id)
        } else {
            format!("Attached track #{}", entry.id)
        });
        true
    }

    pub fn move_selected_to_layer(&mut self, target_layer_id: &str) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        let mut moved = 0usize;
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.layer_id = Some(target_layer_id.to_string());
                moved += 1;
            }
        }
        if let Some(models) = &mut self.layout.models {
            for m in models {
                if self.selected.contains(&m.id) {
                    m.layer_id = Some(target_layer_id.to_string());
                    moved += 1;
                }
            }
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.vehicle_sim.sanitize_for_layout(&self.layout, &self.catalogs);
        self.mark_layout_dirty();
        let layer_name = self
            .layout
            .layers
            .as_ref()
            .and_then(|ls| ls.iter().find(|l| l.id == target_layer_id))
            .map(|l| l.name.clone())
            .unwrap_or_else(|| target_layer_id.to_string());
        self.status_message = Some(if self.lang == Lang::De {
            format!("{moved} Element(e) auf \"{layer_name}\" verschoben")
        } else {
            format!("Moved {moved} item(s) to \"{layer_name}\"")
        });
    }

    pub fn count_orphaned_items(&self) -> usize {
        let Some(layers) = &self.layout.layers else {
            return 0;
        };
        if layers.is_empty() {
            return 0;
        }
        let valid: HashSet<&str> = layers.iter().map(|l| l.id.as_str()).collect();
        let orphaned_pieces = self
            .layout
            .pieces
            .iter()
            .filter(|p| !valid.contains(p.layer_id.as_deref().unwrap_or("layer-1")))
            .count();
        let orphaned_models = self
            .layout
            .models
            .as_ref()
            .map(|ms| {
                ms.iter()
                    .filter(|m| !valid.contains(m.layer_id.as_deref().unwrap_or("layer-1")))
                    .count()
            })
            .unwrap_or(0);
        orphaned_pieces + orphaned_models
    }

    pub fn reassign_orphaned_items(&mut self) -> usize {
        let valid: HashSet<String> = self
            .layout
            .layers
            .as_ref()
            .map(|ls| ls.iter().map(|l| l.id.clone()).collect())
            .unwrap_or_default();
        if valid.is_empty() {
            return 0;
        }
        let target = self
            .layout
            .active_layer_id
            .clone()
            .filter(|id| valid.contains(id))
            .or_else(|| {
                self.layout
                    .layers
                    .as_ref()
                    .and_then(|ls| ls.first().map(|l| l.id.clone()))
            })
            .unwrap_or_else(|| "layer-1".into());
        if self.count_orphaned_items() == 0 {
            return 0;
        }
        self.push_undo();
        let mut count = 0usize;
        for p in &mut self.layout.pieces {
            let lid = p.layer_id.as_deref().unwrap_or("layer-1");
            if !valid.contains(lid) {
                p.layer_id = Some(target.clone());
                count += 1;
            }
        }
        if let Some(models) = &mut self.layout.models {
            for m in models {
                let lid = m.layer_id.as_deref().unwrap_or("layer-1");
                if !valid.contains(lid) {
                    m.layer_id = Some(target.clone());
                    count += 1;
                }
            }
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.vehicle_sim.sanitize_for_layout(&self.layout, &self.catalogs);
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{count} verwaiste(s) Element(e) neu zugewiesen")
        } else {
            format!("Reassigned {count} orphaned item(s)")
        });
        count
    }

    pub fn detect_track_turn(
        &self,
        anchor: &TrackPiece,
        chosen_idx: usize,
        visited: &mut HashSet<String>,
    ) -> &'static str {
        if !visited.insert(anchor.id.clone()) {
            return "straight";
        }
        let Some(chosen_conn) = anchor.connectors.get(chosen_idx) else {
            return "straight";
        };
        if let Some((_in_idx, in_conn)) = anchor
            .connectors
            .iter()
            .enumerate()
            .find(|(idx, c)| *idx != chosen_idx && c.connected_to.is_some())
        {
            let in_dir = norm_angle_deg(in_conn.angle + 180.0);
            let out_dir = chosen_conn.angle;
            let mut turn_delta = norm_angle_deg(out_dir - in_dir);
            if turn_delta > 180.0 {
                turn_delta -= 360.0;
            }
            if turn_delta > 1.0 {
                return "left";
            }
            if turn_delta < -1.0 {
                return "right";
            }
            if let Some(ref prev_ref) = in_conn.connected_to {
                if let Some(prev_piece) = self.layout.pieces.iter().find(|p| p.id == prev_ref.piece_id) {
                    if let Some(prev_idx) = prev_piece.connectors.iter().position(|c| {
                        c.connected_to
                            .as_ref()
                            .map_or(false, |r| r.piece_id == anchor.id)
                    }) {
                        return self.detect_track_turn(prev_piece, prev_idx, visited);
                    }
                }
            }
            return "straight";
        }

        if let Some(entry) = crate::catalog::find_entry(&self.catalogs, &anchor.catalog_id) {
            if entry.kind == "curve" {
                return if chosen_idx == 1 { "left" } else { "right" };
            }
            if matches!(entry.kind.as_str(), "turnout-right" | "curved-turnout-right") && chosen_idx == 2 {
                return "right";
            }
            if matches!(entry.kind.as_str(), "turnout-left" | "curved-turnout-left") && chosen_idx == 2 {
                return "left";
            }
        }
        "straight"
    }

    /// Mirrors left/right turnouts to their catalog counterpart, flips flextracks end-for-end,
    /// or rotates other selected pieces by 180°.
    pub fn flip_selected(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let target_ids: Vec<String> = self
            .layout
            .pieces
            .iter()
            .filter(|p| self.selected.contains(&p.id) && self.is_layer_editable(p.layer_id.as_deref()))
            .map(|p| p.id.clone())
            .collect();
        if target_ids.is_empty() {
            return false;
        }
        self.push_undo();
        let cats = self.catalogs.clone();

        for id in &target_ids {
            let Some(p) = self.layout.pieces.iter_mut().find(|pc| &pc.id == id) else {
                continue;
            };
            let Some(entry) = crate::catalog::find_entry(&cats, &p.catalog_id).cloned() else {
                continue;
            };
            if crate::catalog::is_flex_id(&cats, &p.catalog_id) {
                crate::catalog::flip_flex_track_piece(p, &cats);
            } else {
                let owner_entries = cats
                    .iter()
                    .find(|c| c.entries.iter().any(|e| e.id == entry.id))
                    .map(|c| c.entries.as_slice())
                    .unwrap_or(&[]);
                let counterpart = match entry.kind.as_str() {
                    "turnout-left" => owner_entries.iter().find(|e| {
                        e.kind == "turnout-right"
                            && (e.length.unwrap_or(0.0) - entry.length.unwrap_or(0.0)).abs() < 5.0
                    }),
                    "turnout-right" => owner_entries.iter().find(|e| {
                        e.kind == "turnout-left"
                            && (e.length.unwrap_or(0.0) - entry.length.unwrap_or(0.0)).abs() < 5.0
                    }),
                    "curved-turnout-left" => owner_entries.iter().find(|e| {
                        e.kind == "curved-turnout-right"
                            && (e.radius.unwrap_or(0.0) - entry.radius.unwrap_or(0.0)).abs() < 5.0
                    }),
                    "curved-turnout-right" => owner_entries.iter().find(|e| {
                        e.kind == "curved-turnout-left"
                            && (e.radius.unwrap_or(0.0) - entry.radius.unwrap_or(0.0)).abs() < 5.0
                    }),
                    _ => None,
                };
                if let Some(cp) = counterpart {
                    p.catalog_id = cp.id.clone();
                    p.connectors = crate::catalog::compute_world_connectors(cp, p.transform);
                } else {
                    p.transform.rotation = norm_angle_deg(p.transform.rotation + 180.0);
                    p.connectors = crate::catalog::compute_piece_world_connectors(p, &entry);
                }
            }
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.vehicle_sim.sanitize_for_layout(&self.layout, &self.catalogs);
        self.mark_layout_dirty();
        let msg = if self.lang == Lang::De {
            "Gleise gespiegelt / gewendet".to_string()
        } else {
            "Tracks mirrored / flipped".to_string()
        };
        self.status_message = Some(msg);
        true
    }

    pub fn toggle_group_selected(&mut self) -> bool {
        let sel_ids: Vec<String> = self
            .layout
            .pieces
            .iter()
            .filter(|p| self.selected.contains(&p.id) && self.is_layer_editable(p.layer_id.as_deref()))
            .map(|p| p.id.clone())
            .collect();
        if sel_ids.len() < 2 {
            self.status_message = Some(if self.lang == Lang::De {
                "Mindestens 2 Gleise zum Gruppieren markieren".into()
            } else {
                "Select at least 2 tracks to group".into()
            });
            return false;
        }
        let first_gid = self
            .layout
            .pieces
            .iter()
            .find(|p| sel_ids.contains(&p.id))
            .and_then(|p| p.group_id.clone());
        let all_same_group = first_gid.is_some()
            && self
                .layout
                .pieces
                .iter()
                .filter(|p| sel_ids.contains(&p.id))
                .all(|p| p.group_id == first_gid);

        self.push_undo();
        if all_same_group {
            for p in &mut self.layout.pieces {
                if sel_ids.contains(&p.id) {
                    p.group_id = None;
                }
            }
            self.status_message = Some(if self.lang == Lang::De {
                "Gruppierung aufgehoben".into()
            } else {
                "Tracks ungrouped".into()
            });
        } else {
            let gid = format!("group-{}", self.next_piece_seq);
            self.next_piece_seq += 1;
            for p in &mut self.layout.pieces {
                if sel_ids.contains(&p.id) {
                    p.group_id = Some(gid.clone());
                }
            }
            let cnt = sel_ids.len();
            self.status_message = Some(if self.lang == Lang::De {
                format!("{cnt} Gleise gruppiert")
            } else {
                format!("Grouped {cnt} tracks")
            });
        }
        self.mark_layout_dirty();
        true
    }

    pub fn split_track(&mut self, piece_id: &str) -> bool {
        let Some(idx) = self.layout.pieces.iter().position(|p| p.id == piece_id) else {
            return false;
        };
        let piece = self.layout.pieces[idx].clone();
        let Some(entry) = crate::catalog::find_entry(&self.catalogs, &piece.catalog_id).cloned() else {
            return false;
        };

        if crate::catalog::is_flex_id(&self.catalogs, &piece.catalog_id) {
            let second_id = format!("piece-{}", self.next_piece_seq);
            let Some((first, second)) = crate::catalog::cut_flex_track(&piece, &self.catalogs, second_id.clone()) else {
                return false;
            };
            self.push_undo();
            self.next_piece_seq += 1;
            self.layout.pieces[idx] = first;
            self.layout.pieces.push(second);
            update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
            crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
            self.selected.clear();
            self.selected.insert(piece_id.to_string());
            self.selected.insert(second_id);
            self.mark_layout_dirty();
            self.status_message = Some(if self.lang == Lang::De {
                "Flexgleis in zwei Haelften geteilt".into()
            } else {
                "Split flextrack into two halves".into()
            });
            return true;
        }

        let cur_len = entry.length.unwrap_or(180.0);
        let half_target = cur_len * 0.5;
        let owner_entries = self
            .catalogs
            .iter()
            .find(|c| c.entries.iter().any(|e| e.id == entry.id))
            .map(|c| c.entries.as_slice())
            .unwrap_or(&[]);
        let half_entry = owner_entries
            .iter()
            .filter(|e| e.kind == "straight")
            .filter_map(|e| {
                let d = (e.length.unwrap_or(0.0) - half_target).abs();
                if d <= 8.0 { Some((d, e)) } else { None }
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, e)| e.clone());

        let Some(half_entry) = half_entry else {
            self.status_message = Some(if self.lang == Lang::De {
                "Kein passendes Teilgleis im Katalog gefunden".into()
            } else {
                "No matching half-track found in catalog".into()
            });
            return false;
        };

        self.push_undo();
        let half_len = half_entry.length.unwrap_or(half_target);
        let (e1, e2) = match piece.elevation {
            Some(el) => {
                let mid = ((el.start + el.end) * 0.5 * 10.0).round() / 10.0;
                (Some(Elevation { start: el.start, end: mid }), Some(Elevation { start: mid, end: el.end }))
            }
            None => (None, None),
        };
        let mut first = piece.clone();
        first.catalog_id = half_entry.id.clone();
        first.connectors = crate::catalog::compute_world_connectors(&half_entry, first.transform);
        first.elevation = e1;

        let rad = railbench_core::geometry::deg2rad(first.transform.rotation);
        let tr2 = Transform {
            x: first.transform.x + half_len * rad.cos(),
            y: first.transform.y + half_len * rad.sin(),
            rotation: first.transform.rotation,
        };
        let second_id = format!("piece-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let mut second = create_piece(second_id.clone(), &half_entry, tr2, first.layer_id.clone());
        second.elevation = e2;
        second.has_catenary = first.has_catenary;
        second.has_bridge = first.has_bridge;
        second.has_tunnel = first.has_tunnel;

        self.layout.pieces[idx] = first;
        self.layout.pieces.push(second);
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.selected.clear();
        self.selected.insert(piece_id.to_string());
        self.selected.insert(second_id);
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("Gleis geteilt in 2x #{}", half_entry.id)
        } else {
            format!("Split track into 2x #{}", half_entry.id)
        });
        true
    }

    pub fn replace_track(&mut self, piece_id: &str, new_catalog_id: &str) -> bool {
        let Some(new_entry) = crate::catalog::find_entry(&self.catalogs, new_catalog_id).cloned() else {
            return false;
        };
        let Some(idx) = self.layout.pieces.iter().position(|p| p.id == piece_id) else {
            return false;
        };
        self.push_undo();
        let old = self.layout.pieces[idx].clone();
        let mut replacement = create_piece(old.id.clone(), &new_entry, old.transform, old.layer_id.clone());
        replacement.elevation = old.elevation;
        replacement.has_catenary = old.has_catenary;
        replacement.has_bridge = old.has_bridge;
        replacement.has_tunnel = old.has_tunnel;
        replacement.has_signal = old.has_signal;
        replacement.signal_side = old.signal_side;
        replacement.signal_direction = old.signal_direction;
        replacement.group_id = old.group_id;
        if old.power_feed == Some(true) {
            replacement.power_feed = Some(true);
        }
        if old.is_isolator == Some(true) {
            replacement.is_isolator = Some(true);
            replacement.isolated_connector_index = old.isolated_connector_index;
        }
        self.layout.pieces[idx] = replacement;
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.vehicle_sim.sanitize_for_layout(&self.layout, &self.catalogs);
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("Gleis durch #{new_catalog_id} ersetzt")
        } else {
            format!("Replaced track with #{new_catalog_id}")
        });
        true
    }

}
