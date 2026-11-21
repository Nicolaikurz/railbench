use crate::*;

impl RailbenchApp {
    pub fn place_catalog_entry(&mut self, entry: &CatalogEntry) {
        self.push_undo();
        let id = format!("piece-{}", self.next_piece_seq);
        self.next_piece_seq += 1;

        // If a single piece of the SAME type (track-to-track or road-to-road) and SAME vertical level is selected with an open connector, auto-snap!
        let new_is_road = entry.is_road == Some(true) || crate::catalog::is_road_id(&self.catalogs, &entry.id);
        let active_lid = self.layout.active_layer_id.as_deref().unwrap_or("layer-1");
        let active_lh = self
            .layout
            .layers
            .as_ref()
            .and_then(|ls| ls.iter().find(|l| l.id == active_lid))
            .and_then(|l| l.height_mm)
            .unwrap_or(0.0);
        let mut tr = Transform { x: 0.0, y: 0.0, rotation: 0.0 };
        if self.selected.len() == 1 {
            if let Some(sel_id) = self.selected.iter().next() {
                if let Some(sel_p) = self.layout.pieces.iter().find(|p| &p.id == sel_id) {
                    let sel_is_road = crate::catalog::is_road_id(&self.catalogs, &sel_p.catalog_id);
                    if sel_is_road == new_is_road {
                        let sel_lid = sel_p.layer_id.as_deref().unwrap_or("layer-1");
                        let sel_lh = self
                            .layout
                            .layers
                            .as_ref()
                            .and_then(|ls| ls.iter().find(|l| l.id == sel_lid))
                            .and_then(|l| l.height_mm)
                            .unwrap_or(0.0);
                        if let Some((c_idx, open_c)) = sel_p
                            .connectors
                            .iter()
                            .enumerate()
                            .rev()
                            .find(|&(ci, c)| {
                                c.connected_to.is_none()
                                    && (railbench_core::snapping::piece_connector_height(sel_p, ci, sel_lh)
                                        - active_lh)
                                        .abs()
                                        <= 1.0
                            })
                        {
                            let _ = c_idx;
                            let locals = crate::catalog::local_connectors(entry);
                            let loc0 = &locals[0];
                            let rot = norm_angle_deg(open_c.angle + 180.0 - loc0.angle);
                            let ro = rotate(Vec2 { x: loc0.x, y: loc0.y }, rot);
                            tr = Transform {
                                x: open_c.x - ro.x,
                                y: open_c.y - ro.y,
                                rotation: rot,
                            };
                        }
                    }
                }
            }
        }

        let piece = create_piece(id.clone(), entry, tr, self.layout.active_layer_id.clone());
        self.layout.pieces.push(piece);
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.selected.clear();
        self.selected.insert(id);
        self.last_placed_catalog_id = Some(entry.id.clone());
        self.mark_layout_dirty();
    }

    pub fn place_catalog_entry_at(&mut self, entry: &CatalogEntry, world_pos: Vec2, snap_radius: f64) {
        self.push_undo();
        let id = format!("piece-{}", self.next_piece_seq);
        self.next_piece_seq += 1;

        // Center the piece on the drop position
        let locals = crate::catalog::local_connectors(entry);
        let (cx, cy) = if locals.is_empty() {
            (0.0, 0.0)
        } else {
            let sx: f64 = locals.iter().map(|c| c.x).sum();
            let sy: f64 = locals.iter().map(|c| c.y).sum();
            (sx / locals.len() as f64, sy / locals.len() as f64)
        };
        let tr = Transform {
            x: world_pos.x - cx,
            y: world_pos.y - cy,
            rotation: 0.0,
        };
        let piece = create_piece(id.clone(), entry, tr, self.layout.active_layer_id.clone());
        self.layout.pieces.push(piece);
        self.selected.clear();
        self.selected.insert(id);
        self.last_placed_catalog_id = Some(entry.id.clone());

        if self.snapping_enabled
            && !crate::canvas2d::snap_selected_pieces(&mut self.layout, &self.catalogs, &self.selected, snap_radius)
        {
            crate::building_snap::snap_selected_pieces_to_buildings(
                &mut self.layout,
                &self.catalogs,
                &self.selected,
                snap_radius,
            );
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
    }

    pub fn rotate_selected(&mut self, delta_deg: f64) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        let active_lid = self.layout.active_layer_id.clone();
        let active_lid_str = active_lid.as_deref().unwrap_or("layer-1");
        let layer_prot = self.layer_protection;
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id)
                && (!layer_prot || p.layer_id.as_deref().unwrap_or("layer-1") == active_lid_str)
            {
                let px = p.transform.x;
                let py = p.transform.y;
                p.transform.rotation = norm_angle_deg(p.transform.rotation + delta_deg);
                for c in &mut p.connectors {
                    let rel = rotate(Vec2 { x: c.x - px, y: c.y - py }, delta_deg);
                    c.x = px + rel.x;
                    c.y = py + rel.y;
                    c.angle = norm_angle_deg(c.angle + delta_deg);
                    c.connected_to = None;
                }
            }
        }
        if let Some(models) = &mut self.layout.models {
            for m in models {
                if self.selected.contains(&m.id)
                    && (!layer_prot || m.layer_id.as_deref().unwrap_or("layer-1") == active_lid_str)
                {
                    m.transform.rotation = norm_angle_deg(m.transform.rotation + delta_deg);
                }
            }
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
    }

    pub fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        self.layout.pieces.retain(|p| !self.selected.contains(&p.id));
        if let Some(models) = &mut self.layout.models {
            models.retain(|m| !self.selected.contains(&m.id));
        }
        self.selected.clear();
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        self.mark_layout_dirty();
    }

    pub fn place_model_instance(&mut self, mut model: Model3D) {
        self.push_undo();
        let id = format!("model-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        model.id = id.clone();
        model.layer_id = self.layout.active_layer_id.clone();

        if let Some(ref bid) = model.builtin_id {
            let model_h = crate::building_snap::model_world_height(&self.layout, &model);
            if (bid == "engine-shed" || bid == "turntable-trainhall") && self.selected.len() == 1 {
                if let Some(sid) = self.selected.iter().next() {
                    if let Some(sp) = self.layout.pieces.iter().find(|p| &p.id == sid) {
                        if bid == "turntable-trainhall"
                            && sp.connectors.len() >= 6
                            && (crate::building_snap::piece_connector_world_height(&self.layout, sp, 0)
                                - model_h)
                                .abs()
                                <= crate::building_snap::SNAP_HEIGHT_TOL_MM
                        {
                            let tt_r = crate::catalog::find_entry(&self.catalogs, &sp.catalog_id)
                                .and_then(|e| e.radius.or_else(|| e.length.map(|l| l * 0.5)))
                                .unwrap_or(189.0);
                            let target_dist = tt_r + 6.0 + model.depth * 0.5;
                            model.transform = Transform {
                                x: sp.transform.x + target_dist,
                                y: sp.transform.y,
                                rotation: 270.0,
                            };
                        } else if let Some((_ci, c)) = sp.connectors.iter().enumerate().rev().find(|&(ci, c)| {
                            c.connected_to.is_none()
                                && (crate::building_snap::piece_connector_world_height(
                                    &self.layout,
                                    sp,
                                    ci,
                                ) - model_h)
                                    .abs()
                                    <= crate::building_snap::SNAP_HEIGHT_TOL_MM
                        }) {
                            let rad = railbench_core::geometry::deg2rad(c.angle);
                            model.transform = Transform {
                                x: c.x + model.width * 0.5 * rad.cos(),
                                y: c.y + model.width * 0.5 * rad.sin(),
                                rotation: c.angle,
                            };
                        }
                    }
                }
            }
        }

        let name = model.name.clone();
        self.layout.models.get_or_insert_with(Vec::new).push(model);
        self.selected.clear();
        self.selected.insert(id);
        if self.snapping_enabled {
            crate::building_snap::snap_selected_models(&mut self.layout, &self.catalogs, &self.selected);
        }
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed 3D model: {name}"));
    }

    pub fn place_builtin_model(
        &mut self,
        builtin_id: &str,
        name: &str,
        width: f64,
        depth: f64,
        height: f64,
    ) {
        let category = match builtin_id {
            "mountain" => "terrain",
            "engine-shed" | "turntable-trainhall" => "railway",
            _ => "buildings",
        };
        self.place_model_instance(Model3D {
            id: String::new(),
            name: name.to_string(),
            category: category.into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width,
            depth,
            height,
            model_type: "builtin".into(),
            builtin_id: Some(builtin_id.to_string()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        });
    }

    pub fn import_custom_model_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("3D Models (GLB / GLTF)", &["glb", "gltf"])
            .pick_files()
        {
            for path in paths {
                self.import_custom_model_path(&path);
            }
        }
    }

    pub fn import_custom_model_path(&mut self, path: &std::path::Path) {
        match model_tools::import_model_from_path(path) {
            Ok(mut model) => {
                if let Some(existing) = self
                    .custom_models_library
                    .iter()
                    .find(|c| c.file_data.is_some() && c.file_data == model.file_data)
                {
                    model.file_data = existing.file_data.clone();
                }
                if !self.custom_models_library.iter().any(|c| c.name == model.name) {
                    self.custom_models_library.push(model.clone());
                }
                self.place_model_instance(model);
            }
            Err(err) => {
                self.status_message = Some(format!("3D import error: {err}"));
            }
        }
    }

    pub fn copy_selected(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let pieces: Vec<TrackPiece> = self
            .layout
            .pieces
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .cloned()
            .collect();
        let models: Vec<Model3D> = self
            .layout
            .models
            .as_ref()
            .map(|ms| ms.iter().filter(|m| self.selected.contains(&m.id)).cloned().collect())
            .unwrap_or_default();
        let total = pieces.len() + models.len();
        if total == 0 {
            return false;
        }
        self.clipboard = Some(ClipboardData { pieces, models });
        self.paste_offset_count = 1;
        self.status_message = Some(if self.lang == Lang::De {
            format!("{total} Element(e) kopiert (Strg+C)")
        } else {
            format!("{total} item(s) copied (Ctrl+C)")
        });
        true
    }

    pub fn paste_clipboard(&mut self, custom_offset: Option<Vec2>) -> bool {
        let Some(cb) = self.clipboard.clone() else {
            self.status_message = Some(if self.lang == Lang::De {
                "Zwischenablage ist leer".into()
            } else {
                "Clipboard is empty".into()
            });
            return false;
        };
        if cb.pieces.is_empty() && cb.models.is_empty() {
            return false;
        }

        let offset = match custom_offset {
            Some(off) => off,
            None => {
                let k = self.paste_offset_count as f64;
                self.paste_offset_count += 1;
                Vec2 {
                    x: 40.0 * k,
                    y: 40.0 * k,
                }
            }
        };

        self.push_undo();
        let mut new_ids = Vec::new();
        let mut last_cat_id = None;

        for mut piece in cb.pieces {
            let new_id = format!("piece-{}", self.next_piece_seq);
            self.next_piece_seq += 1;
            piece.id = new_id.clone();
            piece.transform.x += offset.x;
            piece.transform.y += offset.y;
            for c in &mut piece.connectors {
                c.x += offset.x;
                c.y += offset.y;
                c.connected_to = None;
            }
            last_cat_id = Some(piece.catalog_id.clone());
            self.layout.pieces.push(piece);
            new_ids.push(new_id);
        }

        if !cb.models.is_empty() {
            let models = self.layout.models.get_or_insert_with(Vec::new);
            for mut model in cb.models {
                let new_id = format!("model-{}", self.next_piece_seq);
                self.next_piece_seq += 1;
                model.id = new_id.clone();
                model.transform.x += offset.x;
                model.transform.y += offset.y;
                models.push(model);
                new_ids.push(new_id);
            }
        }

        if let Some(cid) = last_cat_id {
            self.last_placed_catalog_id = Some(cid);
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.selected.clear();
        let total = new_ids.len();
        for id in new_ids {
            self.selected.insert(id);
        }
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{total} Element(e) eingefügt (Strg+V)")
        } else {
            format!("{total} item(s) pasted (Ctrl+V)")
        });
        true
    }

    pub fn duplicate_selected(&mut self) -> bool {
        if !self.copy_selected() {
            return false;
        }
        self.paste_clipboard(Some(Vec2 { x: 40.0, y: 40.0 }))
    }

    pub fn select_all(&mut self) {
        self.selected.clear();
        for p in &self.layout.pieces {
            if self.is_layer_editable(p.layer_id.as_deref()) {
                self.selected.insert(p.id.clone());
            }
        }
        if let Some(models) = &self.layout.models {
            for m in models {
                if self.is_layer_editable(m.layer_id.as_deref()) {
                    self.selected.insert(m.id.clone());
                }
            }
        }
        let n = self.selected.len();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{n} Element(e) ausgewählt")
        } else {
            format!("Selected {n} item(s)")
        });
    }

    pub fn select_connected_tracks(&mut self) {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<String> = self.selected.iter().cloned().collect();
        while let Some(cur_id) = queue.pop_front() {
            if !visited.insert(cur_id.clone()) {
                continue;
            }
            if let Some(piece) = self.layout.pieces.iter().find(|p| p.id == cur_id) {
                for conn in &piece.connectors {
                    if let Some(ref cto) = conn.connected_to {
                        if !visited.contains(&cto.piece_id) {
                            queue.push_back(cto.piece_id.clone());
                        }
                    }
                }
            }
        }
        self.selected = visited;
        let n = self.selected.len();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{n} verbundene Gleise ausgewählt")
        } else {
            format!("Selected {n} connected track(s)")
        });
    }

    pub fn select_layer_tracks(&mut self) {
        let target_lid = self
            .layout
            .pieces
            .iter()
            .find(|p| self.selected.contains(&p.id))
            .and_then(|p| p.layer_id.clone())
            .or_else(|| self.layout.active_layer_id.clone())
            .unwrap_or_else(|| "layer-1".into());
        self.selected.clear();
        for p in &self.layout.pieces {
            if p.layer_id.as_deref().unwrap_or("layer-1") == target_lid
                && self.is_layer_editable(p.layer_id.as_deref())
            {
                self.selected.insert(p.id.clone());
            }
        }
        let n = self.selected.len();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{n} Gleise auf Ebene ausgewählt")
        } else {
            format!("Selected {n} track(s) on layer")
        });
    }

    fn is_layer_editable(&self, layer_id: Option<&str>) -> bool {
        let lid = layer_id.unwrap_or("layer-1");
        if self.layer_protection {
            let active_lid = self.layout.active_layer_id.as_deref().unwrap_or("layer-1");
            if lid != active_lid {
                return false;
            }
        }
        self.layout
            .layers
            .as_ref()
            .and_then(|ls| ls.iter().find(|l| l.id == lid))
            .map(|l| l.visible && !l.locked)
            .unwrap_or(true)
    }

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
        let use_end_conn = entry.kind == "curve" && locals.len() >= 2 && connector_idx == 0;
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
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{count} verwaiste(s) Element(e) neu zugewiesen")
        } else {
            format!("Reassigned {count} orphaned item(s)")
        });
        count
    }

}
