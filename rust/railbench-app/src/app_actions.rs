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
        let count = terrain_tools::apply_auto_tunnel_all_mountains(&mut self.layout)
            + terrain_tools::apply_auto_tunnel_for_all_layer_heightfields(&mut self.layout);
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
                if p.is_isolator == Some(true) {
                    let cur = p.isolated_connector_index.unwrap_or(0);
                    let count = p.connectors.len().max(2);
                    if cur + 1 < count {
                        p.isolated_connector_index = Some(cur + 1);
                    } else {
                        p.is_isolator = Some(false);
                        p.isolated_connector_index = None;
                    }
                } else {
                    p.is_isolator = Some(true);
                    p.isolated_connector_index = Some(0);
                }
            }
        } else {
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
                    } else if !any_off {
                        p.isolated_connector_index = None;
                    }
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
                if any_off {
                    p.has_tunnel = Some(false);
                }
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
                if any_off {
                    p.has_bridge = Some(false);
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn add_layer(&mut self) {
        self.add_layer_immediate();
    }

    pub fn open_layer_dialog_new(&mut self) {
        let layers = self.layout.layers.as_deref().unwrap_or(&[]);
        let max_h = layers
            .iter()
            .filter_map(|l| l.height_mm)
            .fold(f64::NEG_INFINITY, f64::max);
        let next_idx = layers.len() + 1;
        let palette_hex = ["#38bdf8", "#34d399", "#fbbf24", "#f472b6", "#a78bfa", "#f97316", "#2dd4bf", "#ef4444"];
        self.layer_dialog_edit_id = None;
        self.layer_dialog_name = if self.lang == Lang::De {
            format!("Ebene {}", next_idx.saturating_sub(1))
        } else {
            format!("Layer {next_idx}")
        };
        self.layer_dialog_height_mm = if layers.is_empty() || !max_h.is_finite() {
            0.0
        } else {
            max_h + 80.0
        };
        self.layer_dialog_color = palette_hex[(next_idx.saturating_sub(1)) % palette_hex.len()].into();
        self.layer_dialog_wood_mode = "none".into();
        self.layer_dialog_wood_thickness = 10.0;
        self.show_layer_dialog = true;
    }

    pub fn open_layer_dialog_edit(&mut self, layer_id: &str) {
        let Some(layer) = self
            .layout
            .layers
            .as_ref()
            .and_then(|ls| ls.iter().find(|l| l.id == layer_id))
            .cloned()
        else {
            return;
        };
        self.layer_dialog_edit_id = Some(layer.id);
        self.layer_dialog_name = layer.name;
        self.layer_dialog_height_mm = layer.height_mm.unwrap_or(0.0);
        self.layer_dialog_color = layer.color.unwrap_or_else(|| "#38bdf8".into());
        self.layer_dialog_wood_mode = layer.wood_plate_mode.unwrap_or_else(|| "none".into());
        self.layer_dialog_wood_thickness = layer.wood_plate_thickness_mm.unwrap_or(10.0);
        self.show_layer_dialog = true;
    }

    pub fn apply_layer_dialog(&mut self) {
        self.push_undo();
        let name = self.layer_dialog_name.trim().to_string();
        let final_name = if name.is_empty() {
            "Layer".to_string()
        } else {
            name
        };
        let height_mm = (self.layer_dialog_height_mm * 10.0).round() / 10.0;
        let color = self.layer_dialog_color.clone();
        let wood_mode = Some(self.layer_dialog_wood_mode.clone());
        let wood_thickness = Some(self.layer_dialog_wood_thickness.clamp(2.0, 50.0));

        if let Some(ref edit_id) = self.layer_dialog_edit_id.clone() {
            if let Some(layers) = &mut self.layout.layers {
                if let Some(layer) = layers.iter_mut().find(|l| &l.id == edit_id) {
                    layer.name = final_name;
                    layer.height_mm = Some(height_mm);
                    layer.color = Some(color);
                    layer.wood_plate_mode = wood_mode;
                    layer.wood_plate_thickness_mm = wood_thickness;
                }
            }
        } else {
            let new_id = format!("layer-{}", self.next_piece_seq + 100);
            self.next_piece_seq += 1;
            let layers = self.layout.layers.get_or_insert_with(Vec::new);
            layers.push(Layer {
                id: new_id.clone(),
                name: final_name,
                color: Some(color),
                visible: true,
                locked: false,
                height_mm: Some(height_mm),
                wood_plate_mode: wood_mode,
                wood_plate_thickness_mm: wood_thickness,
                wood_plate_style: None,
                plates: None,
                terrain_field: None,
            });
            self.layout.active_layer_id = Some(new_id);
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
        self.show_layer_dialog = false;
    }

    pub fn add_layer_immediate(&mut self) -> String {
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
            terrain_field: None,
        });
        self.layout.active_layer_id = Some(id.clone());
        self.mark_layout_dirty();
        id
    }

    pub fn count_items_on_layer(&self, layer_id: &str) -> usize {
        let p_cnt = self
            .layout
            .pieces
            .iter()
            .filter(|p| p.layer_id.as_deref().unwrap_or("layer-1") == layer_id)
            .count();
        let m_cnt = self
            .layout
            .models
            .as_ref()
            .map_or(0, |ms| ms.iter().filter(|m| m.layer_id.as_deref().unwrap_or("layer-1") == layer_id).count());
        p_cnt + m_cnt
    }

    pub fn safe_delete_layer(&mut self, del_id: &str) -> bool {
        let Some(layers) = &self.layout.layers else {
            return false;
        };
        if layers.len() <= 1 || !layers.iter().any(|l| l.id == del_id) {
            return false;
        }
        let fallback_id = layers
            .iter()
            .find(|l| l.id != del_id && l.id == "layer-1")
            .or_else(|| layers.iter().find(|l| l.id != del_id))
            .map(|l| l.id.clone());
        let Some(fallback_id) = fallback_id else {
            return false;
        };

        self.push_undo();
        for p in &mut self.layout.pieces {
            if p.layer_id.as_deref().unwrap_or("layer-1") == del_id {
                p.layer_id = Some(fallback_id.clone());
            }
        }
        if let Some(models) = &mut self.layout.models {
            for m in models {
                if m.layer_id.as_deref().unwrap_or("layer-1") == del_id {
                    m.layer_id = Some(fallback_id.clone());
                }
            }
        }
        if let Some(layers_mut) = &mut self.layout.layers {
            layers_mut.retain(|l| l.id != del_id);
        }
        if self.layout.active_layer_id.as_deref() == Some(del_id) {
            self.layout.active_layer_id = Some(fallback_id);
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
        true
    }
}
