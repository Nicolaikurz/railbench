use crate::*;

impl RailbenchApp {
    fn active_board_bounds(&self) -> board_tools::BoardBounds {
        let boards = board_tools::get_baseboards_slice(&self.layout);
        let target = self
            .active_board_id
            .as_deref()
            .and_then(|aid| boards.iter().find(|b| b.id.as_deref() == Some(aid)))
            .or_else(|| boards.first());
        target
            .map(board_tools::get_baseboard_bounds)
            .unwrap_or(board_tools::BoardBounds {
                x: -1200.0,
                y: -600.0,
                width: 2400.0,
                depth: 1200.0,
            })
    }

    pub fn place_mountain_preset(&mut self, preset: &str) {
        self.push_undo();
        let bounds = self.active_board_bounds();
        let existing_count = self
            .layout
            .models
            .as_ref()
            .map_or(0, |ms| ms.iter().filter(|m| m.builtin_id.as_deref() == Some("mountain")).count());
        let id = format!("mtn-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let style = self.terrain_brush.style.clone();
        let mtn = terrain_tools::make_mountain_preset_model(
            preset,
            id.clone(),
            existing_count,
            &style,
            bounds,
            self.layout.active_layer_id.clone(),
            self.lang == Lang::De,
        );
        self.layout.models.get_or_insert_with(Vec::new).push(mtn.clone());
        if self.terrain_brush.auto_tunnel {
            terrain_tools::apply_auto_tunnel_for_mountain(&mtn, &mut self.layout);
        }
        self.selected.clear();
        self.selected.insert(id.clone());
        self.active_terrain_id = Some(id);
        self.terrain_brush.set_active(false, None, None);
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed mountain: {}", mtn.name));
    }

    pub fn place_ground_terrain(&mut self, terrain_type: &str, name: &str, width: f64, depth: f64, height: f64) {
        self.push_undo();
        let bounds = self.active_board_bounds();
        let id = format!("terrain-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let models = self.layout.models.get_or_insert_with(Vec::new);
        models.push(Model3D {
            id: id.clone(),
            name: name.to_string(),
            category: "terrain".into(),
            transform: Transform {
                x: (bounds.x + bounds.width * 0.5).round(),
                y: (bounds.y + bounds.depth * 0.5).round(),
                rotation: 0.0,
            },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: self.layout.active_layer_id.clone(),
            width,
            depth,
            height,
            model_type: "builtin".into(),
            builtin_id: Some("ground-terrain".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: Some(terrain_type.to_string()),
            file_data: None,
            file_name: None,
        });
        self.selected.clear();
        self.selected.insert(id.clone());
        self.active_terrain_id = Some(id);
        self.terrain_brush.set_active(false, None, None);
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed terrain: {name}"));
    }

    pub fn auto_tunnel_all_mountains(&mut self) -> usize {
        self.push_undo();
        let count = terrain_tools::apply_auto_tunnel_all_mountains(&mut self.layout);
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("Auto-Tunnel: {count} Gleis-/Strassenstueck(e) markiert")
        } else {
            format!("Auto-tunnel marked {count} intersecting track/road piece(s)")
        });
        count
    }

    pub fn clear_all_tunnels(&mut self) -> usize {
        self.push_undo();
        let mut cleared = 0usize;
        for p in &mut self.layout.pieces {
            if p.has_tunnel == Some(true) {
                p.has_tunnel = Some(false);
                cleared += 1;
            }
        }
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{cleared} Tunnel-Markierung(en) entfernt")
        } else {
            format!("Cleared {cleared} tunnel flag(s)")
        });
        cleared
    }

    pub fn center_terrain_item_on_board(&mut self, id: &str) {
        let bounds = self.active_board_bounds();
        let cx = (bounds.x + bounds.width * 0.5).round();
        let cy = (bounds.y + bounds.depth * 0.5).round();
        self.push_undo();
        let mut updated_mtn: Option<Model3D> = None;
        if let Some(models) = &mut self.layout.models {
            if let Some(m) = models.iter_mut().find(|m| m.id == id) {
                m.transform.x = cx;
                m.transform.y = cy;
                if m.builtin_id.as_deref() == Some("mountain") {
                    updated_mtn = Some(m.clone());
                }
            }
        }
        if let Some(m) = updated_mtn {
            if self.terrain_brush.auto_tunnel {
                terrain_tools::apply_auto_tunnel_for_mountain(&m, &mut self.layout);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_feeder(&mut self) {
        let has_target = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        });
        if !has_target { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.power_feed != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.power_feed = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_isolator(&mut self) {
        let has_target = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        });
        if !has_target { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.is_isolator != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.is_isolator = Some(any_off);
                if any_off && p.isolated_connector_index.is_none() {
                    p.isolated_connector_index = Some(0);
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn cycle_selected_isolator_connector(&mut self) {
        let has_target = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.is_isolator == Some(true)
        });
        if !has_target { return; }
        self.push_undo();
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.is_isolator == Some(true)
            {
                let count = p.connectors.len().max(2);
                let cur = p.isolated_connector_index.unwrap_or(0);
                p.isolated_connector_index = Some((cur + 1) % count);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_catenary(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.has_catenary != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.has_catenary = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_catenary_feeder(&mut self) {
        let has_target = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        });
        if !has_target { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.catenary_power_feed != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.catenary_power_feed = Some(any_off);
                if any_off {
                    p.has_catenary = Some(true);
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_catenary_isolator(&mut self) {
        let matching_ids: Vec<String> = self
            .layout
            .pieces
            .iter()
            .filter(|p| {
                self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
            })
            .map(|p| p.id.clone())
            .collect();
        if matching_ids.is_empty() { return; }
        self.push_undo();
        if matching_ids.len() == 1 {
            if let Some(p) = self.layout.pieces.iter_mut().find(|p| p.id == matching_ids[0]) {
                if p.catenary_isolator == Some(true) {
                    let cur = p.catenary_isolated_connector_index.unwrap_or(0);
                    if cur + 1 < p.connectors.len() {
                        p.catenary_isolated_connector_index = Some(cur + 1);
                    } else {
                        p.catenary_isolator = Some(false);
                        p.catenary_isolated_connector_index = None;
                    }
                } else {
                    p.catenary_isolator = Some(true);
                    p.catenary_isolated_connector_index = Some(0);
                    p.has_catenary = Some(true);
                }
            }
        } else {
            let any_off = self.layout.pieces.iter().any(|p| {
                self.selected.contains(&p.id)
                    && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                    && p.catenary_isolator != Some(true)
            });
            for p in &mut self.layout.pieces {
                if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                    p.catenary_isolator = Some(any_off);
                    if any_off {
                        p.catenary_isolated_connector_index = Some(0);
                        p.has_catenary = Some(true);
                    } else {
                        p.catenary_isolated_connector_index = None;
                    }
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_lantern(&mut self) {
        let is_sw = |p: &TrackPiece, cats: &[Catalog]| -> bool {
            crate::catalog::find_entry(cats, &p.catalog_id).map_or(false, |e| is_turnout_kind(&e.kind))
        };
        if !self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && is_sw(p, &self.catalogs)) {
            return;
        }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && is_sw(p, &self.catalogs) && p.has_lantern != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && is_sw(p, &self.catalogs) {
                p.has_lantern = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_decoder(&mut self) {
        let is_sw = |p: &TrackPiece, cats: &[Catalog]| -> bool {
            crate::catalog::find_entry(cats, &p.catalog_id).map_or(false, |e| is_turnout_kind(&e.kind))
        };
        if !self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && is_sw(p, &self.catalogs)) {
            return;
        }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && is_sw(p, &self.catalogs) && p.has_decoder != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && is_sw(p, &self.catalogs) {
                p.has_decoder = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_signal(&mut self) {
        if !self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        }) {
            return;
        }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.has_signal != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.has_signal = Some(any_off);
                if any_off {
                    if p.signal_side.is_none() {
                        p.signal_side = Some("right".into());
                    }
                    if p.signal_direction.is_none() {
                        p.signal_direction = Some("forward".into());
                    }
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn cycle_selected_signal_side(&mut self) {
        let cur = self
            .layout
            .pieces
            .iter()
            .find(|p| self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id))
            .and_then(|p| p.signal_side.as_deref())
            .unwrap_or("right");
        let next = match cur {
            "right" => "left",
            "left" => "both",
            _ => "right",
        };
        if !self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        }) {
            return;
        }
        self.push_undo();
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.has_signal = Some(true);
                p.signal_side = Some(next.into());
            }
        }
        self.mark_layout_dirty();
    }

    pub fn cycle_selected_signal_direction(&mut self) {
        let cur = self
            .layout
            .pieces
            .iter()
            .find(|p| self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id))
            .and_then(|p| p.signal_direction.as_deref())
            .unwrap_or("forward");
        let next = match cur {
            "forward" => "backward",
            "backward" => "both",
            _ => "forward",
        };
        if !self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        }) {
            return;
        }
        self.push_undo();
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.has_signal = Some(true);
                p.signal_direction = Some(next.into());
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_bridge(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.has_bridge != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.has_bridge = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_tunnel(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.has_tunnel != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.has_tunnel = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn add_layer(&mut self) {
        self.push_undo();
        let layers = self.layout.layers.get_or_insert_with(Vec::new);
        let idx = layers.len();
        let id = format!("layer-{}", idx + 1);
        let palette_hex = ["#38bdf8", "#f59e0b", "#10b981", "#a855f7", "#ef4444", "#ec4899"];
        layers.push(Layer {
            id: id.clone(),
            name: format!("Ebene {idx}"),
            color: Some(palette_hex[idx % palette_hex.len()].into()),
            visible: true,
            locked: false,
            height_mm: Some((idx as f64) * 80.0),
            wood_plate_mode: None,
            wood_plate_thickness_mm: None,
            wood_plate_style: None,
            plates: None,
        });
        self.layout.active_layer_id = Some(id);
    }
}
