use super::*;

fn expand_3d_group_ids(
    layout: &Layout,
    catalogs: &[Catalog],
    hit_id: &str,
    alt_held: bool,
    layer_protection: bool,
    selection_scope: crate::SelectionScope,
) -> Vec<String> {
    if alt_held {
        return vec![hit_id.to_string()];
    }
    let Some(gid) = layout
        .pieces
        .iter()
        .find(|p| p.id == hit_id)
        .and_then(|p| p.group_id.clone())
    else {
        return vec![hit_id.to_string()];
    };
    let layers = layer_info_map(layout);
    layout
        .pieces
        .iter()
        .filter(|p| {
            p.group_id.as_deref() == Some(gid.as_str())
                && selection_scope.allows_piece(catalogs, p)
                && is_obj_editable(layout, &layers, p.layer_id.as_deref(), layer_protection)
        })
        .map(|p| p.id.clone())
        .collect()
}

/// Processes 3D selection, XZ dragging with snap, Alt+Drag vertical elevation, catalog drops,
/// and keyboard shortcuts when `Edit 3D` is enabled.
/// Returns `(consumed_primary_pointer, Edit3DOutput)`.
#[allow(clippy::too_many_arguments)]
pub fn handle_3d_edit_interaction(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &Response,
    camera: &Camera3D,
    edit: &mut Edit3DState,
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &mut HashSet<String>,
    dragging_catalog: Option<&CatalogEntry>,
    dragging_model: Option<&Model3D>,
    snapping_enabled: bool,
    layer_protection: bool,
    rotation_step: f64,
    _env_mode: Viewer3DEnv,
    selection_scope: crate::SelectionScope,
) -> (bool, Edit3DOutput) {
    let mut out = Edit3DOutput::default();
    let can_build = edit.enabled;
    if !can_build {
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.dragging_rotation = false;
        edit.undo_pushed_for_drag = false;
        edit.hovered_id = None;
        edit.snap_preview_world = None;
        edit.preview_piece = None;
        return (false, out);
    }

    let hover_pos = ui.input(|i| i.pointer.hover_pos()).or_else(|| {
        if dragging_catalog.is_some() || dragging_model.is_some() {
            ui.input(|i| i.pointer.latest_pos())
        } else {
            None
        }
    });
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1").to_string();
    let active_lh = {
        let layers = layer_info_map(layout);
        layers.get(active_lid.as_str()).map(|v| v.0).unwrap_or(0.0)
    };

    // 1. Handle Catalog Entry Drag-and-Drop directly onto the 3D plane with magnetic snap preview!
    if let Some(entry) = dragging_catalog {
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.dragging_rotation = false;
        edit.undo_pushed_for_drag = false;
        edit.drag_last_xz = None;
        if let Some(hp) = hover_pos {
            if rect.contains(hp) {
                let (ray_o, ray_d) = camera.screen_to_ray(rect, hp);
                if let Some(hit) = ray_intersect_y_plane(ray_o, ray_d, active_lh as f32) {
                    let dyn_snap = (camera.distance as f64 * 0.045).clamp(45.0, 125.0);
                    let raw_xz = Vec2 {
                        x: hit[0] as f64,
                        y: hit[2] as f64,
                    };
                    let (ghost, snap_pt, prev_elev) = compute_3d_catalog_drop_preview(
                        entry,
                        raw_xz,
                        active_lh,
                        layout,
                        catalogs,
                        snapping_enabled,
                        dyn_snap,
                    );
                    edit.snap_preview_world = snap_pt.or(Some(hit));
                    edit.preview_piece = Some(ghost);
                    edit.preview_elev_mm = prev_elev;
                    if ui.input(|i| i.pointer.any_released()) {
                        out.drop_catalog_at = Some((entry.clone(), raw_xz, dyn_snap));
                        edit.snap_preview_world = None;
                        edit.preview_piece = None;
                    }
                    return (true, out);
                }
            }
        }
        edit.snap_preview_world = None;
        edit.preview_piece = None;
        return (false, out);
    }
    edit.preview_piece = None;

    // 1b. Handle 3D Model Drag-and-Drop directly onto the 3D plane
    if let Some(model) = dragging_model {
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.dragging_rotation = false;
        edit.undo_pushed_for_drag = false;
        edit.drag_last_xz = None;
        if let Some(hp) = hover_pos {
            if rect.contains(hp) {
                let (ray_o, ray_d) = camera.screen_to_ray(rect, hp);
                if let Some(hit) = ray_intersect_y_plane(ray_o, ray_d, active_lh as f32) {
                    edit.snap_preview_world = Some(hit);
                    edit.preview_elev_mm = active_lh as f32;
                    if ui.input(|i| i.pointer.any_released()) {
                        out.drop_model_at = Some((
                            model.clone(),
                            Vec2 {
                                x: hit[0] as f64,
                                y: hit[2] as f64,
                            },
                        ));
                        edit.snap_preview_world = None;
                    }
                    return (true, out);
                }
            }
        }
        edit.snap_preview_world = None;
        return (false, out);
    }
    edit.snap_preview_world = None;

    if matches!(
        selection_scope,
        crate::SelectionScope::Terrain | crate::SelectionScope::Boards
    ) {
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.dragging_rotation = false;
        edit.undo_pushed_for_drag = false;
        edit.hovered_id = None;
        return (false, out);
    }

    // 2. Update 3D hover target when not dragging
    edit.hovered_id = if !edit.dragging_selection {
        hover_pos
            .filter(|hp| rect.contains(*hp))
            .and_then(|hp| {
                let (ro, rd) = camera.screen_to_ray(rect, hp);
                pick_3d_object_scoped(ro, rd, layout, catalogs, layer_protection, selection_scope)
            })
    } else {
        None
    };

    // 3. Keyboard PageUp/PageDown elevation & Shift+Wheel rotation while Edit 3D is active
    if !ui.ctx().wants_keyboard_input() && !selected.is_empty() {
        let (rot_dir, elev_step) = ui.input(|i| {
            let mut r = 0.0_f64;
            if i.modifiers.shift && i.smooth_scroll_delta.y.abs() > 0.5 {
                r += if i.smooth_scroll_delta.y > 0.0 { -5.0 } else { 5.0 };
            }
            let mut e = 0.0_f64;
            if i.key_pressed(egui::Key::PageUp) {
                e += 5.0;
            }
            if i.key_pressed(egui::Key::PageDown) {
                e -= 5.0;
            }
            (r, e)
        });
        if rot_dir.abs() > 0.01 {
            out.rotate_delta_deg = Some(rot_dir);
        }
        if elev_step.abs() > 0.01 {
            out.push_undo = true;
            nudge_selection_elevation(layout, selected, layer_protection, elev_step);
            out.layout_changed = true;
        }
    }

    // 4. Primary Click: Check clickable '+' Open Connector Badges FIRST, then 3D Object Picking
    let shift_held = ui.input(|i| i.modifiers.shift);
    let alt_held = ui.input(|i| i.modifiers.alt);
    let rot_handle_proj = if !selected.is_empty() {
        compute_3d_rotation_handle_world(
            layout,
            catalogs,
            selected,
            layer_protection,
            camera.distance,
            edit,
            selection_scope,
        )
        .and_then(|(p3, h3, base_ang, single_rot)| {
            camera
                .world_to_screen(rect, h3)
                .map(|hs| (p3, hs, base_ang, single_rot))
        })
    } else {
        None
    };

    if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(hp) = hover_pos {
            if rot_handle_proj
                .as_ref()
                .is_some_and(|(_, hs, _, _)| hp.distance(*hs) <= 14.0)
            {
                return (true, out);
            }
            let plus_badges = collect_3d_plus_connectors(
                layout,
                catalogs,
                selected,
                edit.hovered_id.as_deref(),
                rect,
                camera,
                can_build,
                selection_scope,
            );
            if let Some((pid, ci, _)) = plus_badges.into_iter().find(|(_, _, sp)| hp.distance(*sp) <= 13.0) {
                out.quick_add_at = Some((pid, ci));
                return (true, out);
            }
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some(hit_id) = pick_3d_object_scoped(
                ro,
                rd,
                layout,
                catalogs,
                layer_protection,
                selection_scope,
            ) {
                let expand_ids = expand_3d_group_ids(
                    layout,
                    catalogs,
                    &hit_id,
                    alt_held,
                    layer_protection,
                    selection_scope,
                );
                if shift_held {
                    if selected.contains(&hit_id) {
                        for id in &expand_ids {
                            selected.remove(id);
                        }
                    } else {
                        for id in expand_ids {
                            selected.insert(id);
                        }
                    }
                } else {
                    selected.clear();
                    for id in expand_ids {
                        selected.insert(id);
                    }
                }
            } else if !shift_held {
                selected.clear();
            }
        }
        return (false, out);
    }

    // 5. Primary Drag of Selected 3D Rotation Handle OR 3D Object(s) (or empty-space camera Orbit)
    let mut consumed_drag = false;
    if response.drag_started_by(egui::PointerButton::Primary) {
        let press_sp = ui
            .input(|i| i.pointer.press_origin())
            .filter(|p| rect.contains(*p))
            .or(hover_pos);
        if let Some(hp) = press_sp {
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some((p3, _hs, base_ang, single_rot)) =
                rot_handle_proj.filter(|(_, hs, _, _)| hp.distance(*hs) <= 14.5) {
                if let Some(hit) = ray_intersect_y_plane(ro, rd, p3[1]) {
                    let pivot_xz = [p3[0] as f64, p3[2] as f64];
                    edit.dragging_rotation = true;
                    edit.dragging_selection = false;
                    edit.undo_pushed_for_drag = false;
                    edit.drag_plane_y = p3[1];
                    edit.rot_pivot_xz = pivot_xz;
                    edit.rot_start_angle_deg =
                        (hit[2] as f64 - pivot_xz[1]).atan2(hit[0] as f64 - pivot_xz[0]).to_degrees();
                    edit.rot_applied_delta_deg = 0.0;
                    edit.rot_base_angle_deg = base_ang;
                    edit.rot_initial_piece_deg = single_rot;
                }
            } else {
                let picked = pick_3d_object_scoped(
                    ro,
                    rd,
                    layout,
                    catalogs,
                    layer_protection,
                    selection_scope,
                );
                if let Some(hit_id) = picked {
                    if !selected.contains(&hit_id) {
                        if !shift_held {
                            selected.clear();
                        }
                        let expand_ids = expand_3d_group_ids(
                            layout,
                            catalogs,
                            &hit_id,
                            alt_held,
                            layer_protection,
                            selection_scope,
                        );
                        for id in expand_ids {
                            selected.insert(id);
                        }
                    }
                    let pivot = active_selection_pivot_world(layout, selected);
                    edit.dragging_selection = true;
                    edit.dragging_vertical = alt_held;
                    edit.undo_pushed_for_drag = false;
                    edit.drag_plane_y = pivot[1];
                    edit.drag_last_xz = ray_intersect_y_plane(ro, rd, pivot[1]).map(|p| [p[0] as f64, p[2] as f64]);
                }
            }
        }
    }

    if edit.dragging_rotation && response.dragged_by(egui::PointerButton::Primary) {
        consumed_drag = true;
        if let Some(hp) = hover_pos {
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some(hit) = ray_intersect_y_plane(ro, rd, edit.drag_plane_y) {
                let cur_ang = (hit[2] as f64 - edit.rot_pivot_xz[1])
                    .atan2(hit[0] as f64 - edit.rot_pivot_xz[0])
                    .to_degrees();
                let raw_delta = crate::canvas2d::unwrap_drag_angle_delta(
                    cur_ang,
                    edit.rot_start_angle_deg,
                    edit.rot_applied_delta_deg,
                );
                let desired_delta =
                    crate::canvas2d::snap_drag_rotation_delta(raw_delta, rotation_step, shift_held);
                let step_delta = desired_delta - edit.rot_applied_delta_deg;
                if step_delta.abs() > 1e-6 {
                    if !edit.undo_pushed_for_drag {
                        out.push_undo = true;
                        edit.undo_pushed_for_drag = true;
                    }
                    if crate::canvas2d::rotate_selection_around_pivot(
                        layout,
                        selected,
                        layer_protection,
                        Vec2 {
                            x: edit.rot_pivot_xz[0],
                            y: edit.rot_pivot_xz[1],
                        },
                        step_delta,
                    ) {
                        edit.rot_applied_delta_deg = desired_delta;
                        out.layout_changed = true;
                    }
                }
            }
        }
    } else if edit.dragging_rotation && !response.dragged_by(egui::PointerButton::Primary) {
        update_layout_connections(layout, 4.0, 35.0, 1.0);
        disconnect_cross_type_connections(layout, catalogs);
        edit.dragging_rotation = false;
        edit.undo_pushed_for_drag = false;
        out.layout_changed = true;
    } else if edit.dragging_selection && response.dragged_by(egui::PointerButton::Primary) {
        consumed_drag = true;
        if edit.dragging_vertical || alt_held {
            let dy_px = response.drag_delta().y;
            if dy_px.abs() > 0.01 {
                if !edit.undo_pushed_for_drag {
                    out.push_undo = true;
                    edit.undo_pushed_for_drag = true;
                }
                let delta_mm = -(dy_px as f64) * (camera.distance as f64 * 0.0014);
                nudge_selection_elevation(layout, selected, layer_protection, delta_mm);
                out.layout_changed = true;
            }
        } else if let Some(hp) = hover_pos {
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some(hit) = ray_intersect_y_plane(ro, rd, edit.drag_plane_y) {
                let cur_xz = [hit[0] as f64, hit[2] as f64];
                if let Some(prev) = edit.drag_last_xz {
                    let dx = cur_xz[0] - prev[0];
                    let dz = cur_xz[1] - prev[1];
                    if dx.hypot(dz) > 0.05 {
                        if !edit.undo_pushed_for_drag {
                            out.push_undo = true;
                            edit.undo_pushed_for_drag = true;
                        }
                        translate_selection_xz(layout, selected, layer_protection, dx, dz);
                        out.layout_changed = true;
                    }
                }
                edit.drag_last_xz = Some(cur_xz);
            }
        }
    }

    if edit.dragging_selection && !response.dragged_by(egui::PointerButton::Primary) {
        if edit.undo_pushed_for_drag && !edit.dragging_vertical && snapping_enabled {
            let mut snapped = crate::canvas2d::snap_selected_pieces(layout, catalogs, selected, 22.0);
            if !snapped {
                snapped = crate::building_snap::snap_selected_pieces_to_buildings(
                    layout,
                    catalogs,
                    selected,
                    22.0,
                );
            }
            let _ = snapped | crate::building_snap::snap_selected_models(layout, catalogs, selected);
        }
        update_layout_connections(layout, 4.0, 35.0, 1.0);
        disconnect_cross_type_connections(layout, catalogs);
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.undo_pushed_for_drag = false;
        edit.drag_last_xz = None;
        out.layout_changed = true;
    }

    (consumed_drag, out)
}

fn translate_selection_xz(
    layout: &mut Layout,
    selected: &HashSet<String>,
    layer_protection: bool,
    dx: f64,
    dz: f64,
) {
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1").to_string();
    let layers = layer_info_map(layout);
    let can_edit = |lid: Option<&str>| -> bool {
        let l = lid.unwrap_or("layer-1");
        let (_, vis, locked) = layers.get(l).copied().unwrap_or((0.0, true, false));
        vis && !locked && (!layer_protection || l == active_lid.as_str())
    };
    let editable_ids: Vec<String> = selected
        .iter()
        .filter(|id| {
            layout.pieces.iter().any(|p| &p.id == *id && can_edit(p.layer_id.as_deref()))
                || layout
                    .models
                    .as_ref()
                    .map_or(false, |ms| ms.iter().any(|m| &m.id == *id && can_edit(m.layer_id.as_deref())))
        })
        .cloned()
        .collect();
    drop(layers);

    for p in &mut layout.pieces {
        if editable_ids.contains(&p.id) {
            p.transform.x += dx;
            p.transform.y += dz;
            for c in &mut p.connectors {
                c.x += dx;
                c.y += dz;
                c.connected_to = None;
            }
        }
    }
    if let Some(models) = &mut layout.models {
        for m in models {
            if editable_ids.contains(&m.id) {
                m.transform.x += dx;
                m.transform.y += dz;
            }
        }
    }
}

pub(crate) fn nudge_selection_elevation(
    layout: &mut Layout,
    selected: &HashSet<String>,
    layer_protection: bool,
    delta_mm: f64,
) {
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1").to_string();
    let layers = layer_info_map(layout);
    let can_edit = |lid: Option<&str>| -> bool {
        let l = lid.unwrap_or("layer-1");
        let (_, vis, locked) = layers.get(l).copied().unwrap_or((0.0, true, false));
        vis && !locked && (!layer_protection || l == active_lid.as_str())
    };
    let editable_ids: Vec<String> = selected
        .iter()
        .filter(|id| {
            layout.pieces.iter().any(|p| &p.id == *id && can_edit(p.layer_id.as_deref()))
                || layout
                    .models
                    .as_ref()
                    .map_or(false, |ms| ms.iter().any(|m| &m.id == *id && can_edit(m.layer_id.as_deref())))
        })
        .cloned()
        .collect();
    drop(layers);

    for p in &mut layout.pieces {
        if editable_ids.contains(&p.id) {
            let e = p.elevation.get_or_insert(Elevation {
                start: 0.0,
                end: 0.0,
            });
            e.start = (e.start + delta_mm).clamp(-300.0, 1200.0);
            e.end = (e.end + delta_mm).clamp(-300.0, 1200.0);
        }
    }
    if let Some(models) = &mut layout.models {
        for m in models {
            if editable_ids.contains(&m.id) {
                let h = m.elevation.get_or_insert(0.0);
                *h = (*h + delta_mm).clamp(-300.0, 1200.0);
            }
        }
    }
}

/// Renders 3D projected selection wireframes, hover feedback, catalog/model drop ghost,
/// open-connector `+` add badges, and interactive 3D rotation handle.
#[allow(clippy::too_many_arguments)]
pub fn draw_3d_edit_overlays(
    ui: &mut egui::Ui,
    rect: Rect,
    camera: &Camera3D,
    edit: &Edit3DState,
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    dragging_catalog: Option<&CatalogEntry>,
    dragging_model: Option<&Model3D>,
    _env_mode: Viewer3DEnv,
    selection_scope: crate::SelectionScope,
) {
    let can_build = edit.enabled;
    if !can_build {
        return;
    }

    let painter = ui.painter_at(rect);
    let layers = layer_info_map(layout);

    // 1. Projected 3D Selection & Hover Wireframes
    for p in &layout.pieces {
        let is_sel = selected.contains(&p.id);
        let is_hov = edit.hovered_id.as_deref() == Some(p.id.as_str());
        if !is_sel && !is_hov {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0) as f32;
        let e0 = lh + p.elevation.map(|e| e.start).unwrap_or(0.0) as f32 + 9.0;
        let e1 = lh + p.elevation.map(|e| e.end).unwrap_or((e0 - lh - 9.0) as f64) as f32 + 9.0;
        let entry = find_entry(catalogs, &p.catalog_id);
        let col = if is_sel {
            Color32::from_rgb(56, 189, 248)
        } else {
            Color32::from_rgba_unmultiplied(250, 204, 21, 185)
        };
        let stroke = Stroke::new(if is_sel { 2.6_f32 } else { 1.6_f32 }, col);
        for br in sample_track_branches_world(p, entry) {
            let n = (br.len().saturating_sub(1)).max(1) as f32;
            let mut spts = Vec::with_capacity(br.len());
            for (k, pt) in br.iter().enumerate() {
                let y = e0 + (e1 - e0) * (k as f32 / n);
                if let Some(sp) = camera.world_to_screen(rect, [pt.x as f32, y, pt.y as f32]) {
                    spts.push(sp);
                }
            }
            if spts.len() >= 2 {
                painter.add(egui::Shape::line(spts, stroke));
            }
        }
        if is_sel {
            for (ci, c) in p.connectors.iter().enumerate() {
                let ch = piece_connector_height(p, ci, lh as f64) as f32 + 9.0;
                if let Some(sp) = camera.world_to_screen(rect, [c.x as f32, ch, c.y as f32]) {
                    let cc = if c.connected_to.is_some() {
                        Color32::from_rgb(34, 197, 94)
                    } else {
                        Color32::from_rgb(250, 204, 21)
                    };
                    painter.circle_filled(sp, 4.5, cc);
                    painter.circle_stroke(sp, 5.2, Stroke::new(1.2_f32, Color32::WHITE));
                }
            }
        }
    }

    if let Some(models) = &layout.models {
        for m in models {
            let is_sel = selected.contains(&m.id);
            let is_hov = edit.hovered_id.as_deref() == Some(m.id.as_str());
            if !is_sel && !is_hov {
                continue;
            }
            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
            let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0) as f32;
            let y0 = lh + m.elevation.unwrap_or(0.0) as f32;
            let s = m.scale.unwrap_or(1.0);
            let hw = m.width * 0.5 * s;
            let hd = m.depth * 0.5 * s;
            let h = (m.height * s) as f32;
            let col = if is_sel {
                Color32::from_rgb(56, 189, 248)
            } else {
                Color32::from_rgb(250, 204, 21)
            };
            let corners_l = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
            let mut bot = Vec::with_capacity(4);
            let mut top = Vec::with_capacity(4);
            for &(lx, ly) in &corners_l {
                let r = rotate(Vec2 { x: lx, y: ly }, m.transform.rotation);
                let wx = (m.transform.x + r.x) as f32;
                let wz = (m.transform.y + r.y) as f32;
                if let (Some(sb), Some(st)) = (
                    camera.world_to_screen(rect, [wx, y0, wz]),
                    camera.world_to_screen(rect, [wx, y0 + h, wz]),
                ) {
                    bot.push(sb);
                    top.push(st);
                }
            }
            if bot.len() == 4 && top.len() == 4 {
                let st = Stroke::new(if is_sel { 2.0_f32 } else { 1.3_f32 }, col);
                for k in 0..4 {
                    let nk = (k + 1) % 4;
                    painter.line_segment([bot[k], bot[nk]], st);
                    painter.line_segment([top[k], top[nk]], st);
                    painter.line_segment([bot[k], top[k]], st);
                }
            }
        }
    }

    // 2. Catalog & Model Drag Ghost Preview in 3D + Open Connector `+` Badges
    if let (Some(entry), Some(ghost)) = (dragging_catalog, edit.preview_piece.as_ref()) {
        let y_preview = edit.preview_elev_mm + 9.0;
        let ghost_stroke = Stroke::new(3.2_f32, Color32::from_rgb(56, 189, 248));
        for br in sample_track_branches_world(ghost, Some(entry)) {
            let mut spts = Vec::with_capacity(br.len());
            for pt in br {
                if let Some(sp) = camera.world_to_screen(rect, [pt.x as f32, y_preview, pt.y as f32]) {
                    spts.push(sp);
                }
            }
            if spts.len() >= 2 {
                painter.add(egui::Shape::line(spts, ghost_stroke));
            }
        }
        for c in &ghost.connectors {
            if let Some(sp) = camera.world_to_screen(rect, [c.x as f32, y_preview, c.y as f32]) {
                painter.circle_filled(sp, 4.5, Color32::from_rgb(250, 204, 21));
                painter.circle_stroke(sp, 5.2, Stroke::new(1.1_f32, Color32::WHITE));
            }
        }
        if let Some(snap_pt) = edit.snap_preview_world {
            if let Some(sp) = camera.world_to_screen(rect, [snap_pt[0], snap_pt[1] + 9.0, snap_pt[2]]) {
                painter.circle_stroke(sp, 10.0, Stroke::new(2.0_f32, Color32::from_rgb(34, 197, 94)));
            }
        }
    } else if let (Some(model), Some(gp)) = (dragging_model, edit.snap_preview_world) {
        let s = model.scale.unwrap_or(1.0);
        let hw = (model.width * 0.5 * s) as f32;
        let hd = (model.depth * 0.5 * s) as f32;
        let h = (model.height * s) as f32;
        let corners_l = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
        let mut bot = Vec::with_capacity(4);
        let mut top = Vec::with_capacity(4);
        for (lx, lz) in corners_l {
            let wx = gp[0] + lx;
            let wz = gp[2] + lz;
            if let (Some(sb), Some(st)) = (
                camera.world_to_screen(rect, [wx, gp[1], wz]),
                camera.world_to_screen(rect, [wx, gp[1] + h, wz]),
            ) {
                bot.push(sb);
                top.push(st);
            }
        }
        if bot.len() == 4 && top.len() == 4 {
            let st = Stroke::new(2.0_f32, Color32::from_rgb(56, 189, 248));
            for k in 0..4 {
                let nk = (k + 1) % 4;
                painter.line_segment([bot[k], bot[nk]], st);
                painter.line_segment([top[k], top[nk]], st);
                painter.line_segment([bot[k], top[k]], st);
            }
        }
    }

    if dragging_catalog.is_none() && dragging_model.is_none() {
        let hover_pos = ui.ctx().pointer_hover_pos();
        let badges = collect_3d_plus_connectors(
            layout,
            catalogs,
            selected,
            edit.hovered_id.as_deref(),
            rect,
            camera,
            can_build,
            selection_scope,
        );
        for (_pid, _ci, sp) in badges {
            let is_hov = hover_pos.is_some_and(|hp| hp.distance(sp) <= 12.0);
            let fill = if is_hov {
                Color32::from_rgb(34, 197, 94)
            } else {
                Color32::from_rgb(14, 165, 233)
            };
            painter.circle_filled(sp, 9.5, fill);
            painter.circle_stroke(sp, 9.8, Stroke::new(1.3_f32, Color32::WHITE));
            painter.line_segment(
                [sp - egui::vec2(4.2, 0.0), sp + egui::vec2(4.2, 0.0)],
                Stroke::new(2.0_f32, Color32::WHITE),
            );
            painter.line_segment(
                [sp - egui::vec2(0.0, 4.2), sp + egui::vec2(0.0, 4.2)],
                Stroke::new(2.0_f32, Color32::WHITE),
            );
        }
    }

    // 3. XYZ RGB Axis Triad at Active Selection Pivot
    if !selected.is_empty() {
        let pivot = active_selection_pivot_world(layout, selected);
        let arm = (camera.distance * 0.065).clamp(30.0, 95.0);
        if let Some(sp0) = camera.world_to_screen(rect, pivot) {
            for (axis, col, lbl) in [
                ([arm, 0.0, 0.0], Color32::from_rgb(239, 68, 68), "X"),
                ([0.0, arm, 0.0], Color32::from_rgb(34, 197, 94), "Y"),
                ([0.0, 0.0, arm], Color32::from_rgb(59, 130, 246), "Z"),
            ] {
                if let Some(sp1) = camera.world_to_screen(
                    rect,
                    [pivot[0] + axis[0], pivot[1] + axis[1], pivot[2] + axis[2]],
                ) {
                    painter.line_segment([sp0, sp1], Stroke::new(2.2_f32, col));
                    painter.circle_filled(sp1, 3.5, col);
                    painter.text(
                        sp1 + egui::vec2(6.0, -6.0),
                        egui::Align2::LEFT_BOTTOM,
                        lbl,
                        FontId::proportional(10.0),
                        col,
                    );
                }
            }
            painter.circle_filled(sp0, 4.0, Color32::WHITE);
        }

        // 3B. Interactive 3D Track/Selection Rotation Handle Point
        if let Some((rot_piv3, rot_hdl3, _, _)) = compute_3d_rotation_handle_world(
            layout,
            catalogs,
            selected,
            false,
            camera.distance,
            edit,
            selection_scope,
        ) {
            if let (Some(ps), Some(hs)) = (
                camera.world_to_screen(rect, rot_piv3),
                camera.world_to_screen(rect, rot_hdl3),
            ) {
                let is_hov = ui
                    .ctx()
                    .pointer_hover_pos()
                    .is_some_and(|hp| hp.distance(hs) <= 14.0);
                let active_or_hov = edit.dragging_rotation || is_hov;
                painter.line_segment(
                    [ps, hs],
                    Stroke::new(1.8_f32, Color32::from_rgb(56, 189, 248)),
                );
                let knob_r = if active_or_hov { 9.0_f32 } else { 8.0_f32 };
                painter.circle_filled(hs, knob_r, Color32::from_rgb(245, 158, 11));
                painter.circle_stroke(hs, knob_r + 1.0, Stroke::new(1.6_f32, Color32::WHITE));
                if edit.dragging_rotation {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    let shift_held = ui.input(|i| i.modifiers.shift);
                    let readout = if let Some(init) = edit.rot_initial_piece_deg {
                        let abs_deg = norm_angle_deg(init + edit.rot_applied_delta_deg);
                        crate::canvas2d::format_rotation_readout(abs_deg, false, shift_held)
                    } else {
                        crate::canvas2d::format_rotation_readout(
                            edit.rot_applied_delta_deg,
                            true,
                            shift_held,
                        )
                    };
                    let pill = Rect::from_min_size(
                        hs + egui::vec2(14.0, -24.0),
                        egui::vec2(68.0, 22.0),
                    );
                    painter.rect_filled(
                        pill,
                        6.0,
                        Color32::from_rgba_unmultiplied(15, 23, 42, 230),
                    );
                    painter.rect_stroke(
                        pill,
                        6.0,
                        Stroke::new(1.2_f32, Color32::from_rgb(245, 158, 11)),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        pill.center(),
                        egui::Align2::CENTER_CENTER,
                        readout,
                        FontId::proportional(11.5),
                        Color32::WHITE,
                    );
                } else if is_hov {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
        }
    }
}
