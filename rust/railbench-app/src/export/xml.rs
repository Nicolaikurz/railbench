use super::*;

/// Exports complete RAILbench layout XML matching `src/io/serialize.ts::toXML`.
pub fn export_layout_xml(layout: &Layout) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let feed_v = layout.feed_voltage.unwrap_or(16.0);
    let _ = write!(
        xml,
        "<layout name=\"{}\" scale=\"{}\" feedVoltage=\"{feed_v}\"",
        escape_xml(&layout.name),
        escape_xml(&layout.scale)
    );
    if let Some(cv) = layout.catenary_feed_voltage {
        let _ = write!(xml, " catenaryFeedVoltage=\"{cv}\"");
    }
    if let Some(ref cm) = layout.catenary_circuit_mode {
        let _ = write!(xml, " catenaryCircuitMode=\"{}\"", escape_xml(cm));
    }
    if let Some(lm) = layout.loco_current_ma {
        let _ = write!(xml, " locoCurrentMa=\"{lm}\"");
    }
    if let Some(lc) = layout.loco_count {
        let _ = write!(xml, " locoCount=\"{lc}\"");
    }
    if let Some(tl) = layout.turnout_lantern_current_ma {
        let _ = write!(xml, " turnoutLanternCurrentMa=\"{tl}\"");
    }
    if let Some(td) = layout.turnout_decoder_current_ma {
        let _ = write!(xml, " turnoutDecoderCurrentMa=\"{td}\"");
    }
    xml.push_str(">\n");

    let boards = get_baseboards_slice(layout);
    if !boards.is_empty() {
        xml.push_str("  <baseboards>\n");
        for bb in &boards {
            let _ = writeln!(
                xml,
                "    <baseboard enabled=\"{}\" id=\"{}\" name=\"{}\" shape=\"{}\" triangleType=\"{}\" rotation=\"{}\" width=\"{}\" depth=\"{}\" positionMode=\"{}\" style=\"{}\" cornerRadius=\"{}\" showDimensions=\"{}\" customX=\"{}\" customY=\"{}\" lWingWidth=\"{}\" lWingDepth=\"{}\" lSide=\"{}\" uLeftWingWidth=\"{}\" uRightWingWidth=\"{}\" uBackDepth=\"{}\" tBarDepth=\"{}\" tStemWidth=\"{}\"/>",
                bb.enabled,
                escape_xml(bb.id.as_deref().unwrap_or("")),
                escape_xml(bb.name.as_deref().unwrap_or("Table")),
                escape_xml(bb.shape.as_deref().unwrap_or("rect")),
                escape_xml(bb.triangle_type.as_deref().unwrap_or("corner-tl")),
                bb.rotation.unwrap_or(0.0),
                bb.width,
                bb.depth,
                escape_xml(&bb.position_mode),
                escape_xml(&bb.style),
                bb.corner_radius.unwrap_or(12.0),
                bb.show_dimensions.unwrap_or(true),
                bb.custom_x.unwrap_or(0.0),
                bb.custom_y.unwrap_or(0.0),
                bb.l_wing_width.unwrap_or(800.0),
                bb.l_wing_depth.unwrap_or(800.0),
                escape_xml(bb.l_side.as_deref().unwrap_or("left")),
                bb.u_left_wing_width.unwrap_or(700.0),
                bb.u_right_wing_width.unwrap_or(700.0),
                bb.u_back_depth.unwrap_or(600.0),
                bb.t_bar_depth.unwrap_or(600.0),
                bb.t_stem_width.unwrap_or(800.0),
            );
        }
        xml.push_str("  </baseboards>\n");
    }

    if let Some(layers) = &layout.layers {
        if !layers.is_empty() {
            let active = layout
                .active_layer_id
                .as_deref()
                .unwrap_or(layers[0].id.as_str());
            let _ = writeln!(xml, "  <layers activeLayerId=\"{}\">", escape_xml(active));
            for lay in layers {
                let _ = write!(
                    xml,
                    "    <layer id=\"{}\" name=\"{}\" color=\"{}\" visible=\"{}\" locked=\"{}\" heightMm=\"{}\"",
                    escape_xml(&lay.id),
                    escape_xml(&lay.name),
                    escape_xml(lay.color.as_deref().unwrap_or("#38bdf8")),
                    lay.visible,
                    lay.locked,
                    lay.height_mm.unwrap_or(0.0)
                );
                if let Some(ref wm) = lay.wood_plate_mode {
                    let _ = write!(xml, " woodPlateMode=\"{}\"", escape_xml(wm));
                }
                if let Some(wt) = lay.wood_plate_thickness_mm {
                    let _ = write!(xml, " woodPlateThicknessMm=\"{wt}\"");
                }
                if let Some(ref ws) = lay.wood_plate_style {
                    let _ = write!(xml, " woodPlateStyle=\"{}\"", escape_xml(ws));
                }
                if let Some(plates) = &lay.plates {
                    if !plates.is_empty() {
                        xml.push_str(">\n");
                        for pl in plates {
                            let _ = write!(
                                xml,
                                "      <plate id=\"{}\" width=\"{}\" depth=\"{}\" x=\"{}\" y=\"{}\" thicknessMm=\"{}\"",
                                escape_xml(&pl.id),
                                pl.width,
                                pl.depth,
                                pl.x.unwrap_or(0.0),
                                pl.y.unwrap_or(0.0),
                                pl.thickness_mm.unwrap_or(10.0)
                            );
                            if let Some(ref pn) = pl.name {
                                let _ = write!(xml, " name=\"{}\"", escape_xml(pn));
                            }
                            if let Some(ref bid) = pl.baseboard_id {
                                let _ = write!(xml, " baseboardId=\"{}\"", escape_xml(bid));
                            }
                            if let Some(ref sh) = pl.shape {
                                let _ = write!(xml, " shape=\"{}\"", escape_xml(sh));
                            }
                            if let Some(ref st) = pl.style {
                                let _ = write!(xml, " style=\"{}\"", escape_xml(st));
                            }
                            xml.push_str("/>\n");
                        }
                        xml.push_str("    </layer>\n");
                        continue;
                    }
                }
                xml.push_str("/>\n");
            }
            xml.push_str("  </layers>\n");
        }
    }

    xml.push_str("  <pieces>\n");
    for p in &layout.pieces {
        let _ = write!(
            xml,
            "    <piece id=\"{}\" catalogId=\"{}\" x=\"{:.2}\" y=\"{:.2}\" rotation=\"{:.2}\"",
            escape_xml(&p.id),
            escape_xml(&p.catalog_id),
            p.transform.x,
            p.transform.y,
            p.transform.rotation
        );
        if let Some(ref lid) = p.layer_id {
            let _ = write!(xml, " layerId=\"{}\"", escape_xml(lid));
        }
        if let Some(ev) = p.elevation {
            let _ = write!(xml, " elevStart=\"{:.2}\" elevEnd=\"{:.2}\"", ev.start, ev.end);
        }
        if let Some(cl) = p.custom_length {
            let _ = write!(xml, " customLength=\"{cl:.1}\"");
        }
        if let Some(ca) = p.custom_angle {
            let _ = write!(xml, " customAngle=\"{ca:.2}\"");
        }
        if p.power_feed == Some(true) { xml.push_str(" powerFeed=\"true\""); }
        if p.is_isolator == Some(true) { xml.push_str(" isIsolator=\"true\""); }
        if p.has_catenary == Some(true) { xml.push_str(" hasCatenary=\"true\""); }
        if p.catenary_power_feed == Some(true) { xml.push_str(" catenaryPowerFeed=\"true\""); }
        if p.catenary_isolator == Some(true) { xml.push_str(" catenaryIsolator=\"true\""); }
        if p.has_bridge == Some(true) { xml.push_str(" hasBridge=\"true\""); }
        if p.has_tunnel == Some(true) { xml.push_str(" hasTunnel=\"true\""); }
        if p.has_lantern == Some(true) { xml.push_str(" hasLantern=\"true\""); }
        if p.has_decoder == Some(true) { xml.push_str(" hasDecoder=\"true\""); }
        if p.has_signal == Some(true) { xml.push_str(" hasSignal=\"true\""); }
        if let Some(ref cn) = p.custom_name {
            let _ = write!(xml, " customName=\"{}\"", escape_xml(cn));
        }
        xml.push_str("/>\n");
    }
    xml.push_str("  </pieces>\n");

    if let Some(models) = &layout.models {
        if !models.is_empty() {
            xml.push_str("  <models>\n");
            for m in models {
                let _ = write!(
                    xml,
                    "    <model id=\"{}\" name=\"{}\" category=\"{}\" modelType=\"{}\" x=\"{:.2}\" y=\"{:.2}\" rotation=\"{:.2}\" width=\"{:.1}\" depth=\"{:.1}\" height=\"{:.1}\"",
                    escape_xml(&m.id),
                    escape_xml(&m.name),
                    escape_xml(&m.category),
                    escape_xml(&m.model_type),
                    m.transform.x,
                    m.transform.y,
                    m.transform.rotation,
                    m.width,
                    m.depth,
                    m.height
                );
                if let Some(ref bid) = m.builtin_id {
                    let _ = write!(xml, " builtinId=\"{}\"", escape_xml(bid));
                }
                if let Some(ref fnm) = m.file_name {
                    let _ = write!(xml, " fileName=\"{}\"", escape_xml(fnm));
                }
                if let Some(ref lid) = m.layer_id {
                    let _ = write!(xml, " layerId=\"{}\"", escape_xml(lid));
                }
                if let Some(ev) = m.elevation {
                    let _ = write!(xml, " elevation=\"{ev:.2}\"");
                }
                if let Some(sc) = m.scale {
                    let _ = write!(xml, " scale=\"{sc:.3}\"");
                }
                if let Some(ref fd) = m.file_data {
                    let _ = writeln!(xml, ">\n      <fileData>{}</fileData>\n    </model>", escape_xml(fd));
                } else {
                    xml.push_str("/>\n");
                }
            }
            xml.push_str("  </models>\n");
        }
    }

    xml.push_str("</layout>");
    xml
}

pub(super) fn unescape_xml(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub(super) fn xml_attr(tag: &str, attr_name: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let needle = format!("{attr_name}=");
    let nbytes = needle.as_bytes();
    let mut i = 0;
    while i + nbytes.len() < bytes.len() {
        if &bytes[i..i + nbytes.len()] == nbytes {
            let prev_ok = i == 0 || bytes[i - 1].is_ascii_whitespace();
            if prev_ok {
                let qpos = i + nbytes.len();
                let quote = bytes[qpos];
                if quote == b'"' || quote == b'\'' {
                    let start = qpos + 1;
                    let mut end = start;
                    while end < bytes.len() && bytes[end] != quote {
                        end += 1;
                    }
                    if end < bytes.len() {
                        return Some(unescape_xml(&tag[start..end]));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

pub(super) fn xml_attr_f64(tag: &str, attr_name: &str) -> Option<f64> {
    xml_attr(tag, attr_name).and_then(|v| v.parse::<f64>().ok())
}

pub(super) fn xml_attr_bool(tag: &str, attr_name: &str) -> Option<bool> {
    xml_attr(tag, attr_name).map(|v| v == "true" || v == "1")
}

/// Parses a RAILbench XML layout file (matching `export_layout_xml` & `src/io/serialize.ts::fromXML`)
/// and rebuilds all track piece connectors & snap links.
pub fn import_layout_xml(raw: &str, catalogs: &[Catalog]) -> Result<Layout, String> {
    let Some(layout_pos) = raw.find("<layout") else {
        return Err("Missing <layout> root element in XML".into());
    };
    let rest_layout = &raw[layout_pos..];
    let Some(layout_end) = rest_layout.find('>') else {
        return Err("Malformed <layout> opening tag".into());
    };
    let layout_tag = &rest_layout[..=layout_end];

    let name = xml_attr(layout_tag, "name").unwrap_or_else(|| "Unbenannter Plan".into());
    let scale = xml_attr(layout_tag, "scale").unwrap_or_else(|| "H0".into());
    let feed_voltage = xml_attr_f64(layout_tag, "feedVoltage");
    let catenary_feed_voltage = xml_attr_f64(layout_tag, "catenaryFeedVoltage");
    let catenary_circuit_mode = xml_attr(layout_tag, "catenaryCircuitMode");
    let loco_current_ma = xml_attr_f64(layout_tag, "locoCurrentMa");
    let loco_count = xml_attr(layout_tag, "locoCount").and_then(|v| v.parse::<u32>().ok());
    let turnout_lantern_current_ma = xml_attr_f64(layout_tag, "turnoutLanternCurrentMa");
    let turnout_decoder_current_ma = xml_attr_f64(layout_tag, "turnoutDecoderCurrentMa");

    // 1. Baseboards
    let mut baseboards: Vec<BaseboardConfig> = Vec::new();
    let mut search_pos = 0;
    while let Some(rel) = raw[search_pos..].find("<baseboard") {
        let start = search_pos + rel;
        let next_c = raw.as_bytes().get(start + 10).copied().unwrap_or(b' ');
        if !next_c.is_ascii_whitespace() && next_c != b'/' && next_c != b'>' {
            search_pos = start + 10;
            continue;
        }
        let Some(end_rel) = raw[start..].find('>') else {
            break;
        };
        let tag = &raw[start..=start + end_rel];
        search_pos = start + end_rel + 1;

        baseboards.push(BaseboardConfig {
            id: xml_attr(tag, "id").filter(|s| !s.is_empty()),
            name: xml_attr(tag, "name"),
            enabled: xml_attr_bool(tag, "enabled").unwrap_or(true),
            shape: xml_attr(tag, "shape"),
            custom_polygon: None,
            triangle_type: xml_attr(tag, "triangleType"),
            width: xml_attr_f64(tag, "width").unwrap_or(2400.0),
            depth: xml_attr_f64(tag, "depth").unwrap_or(1200.0),
            rotation: xml_attr_f64(tag, "rotation"),
            l_wing_width: xml_attr_f64(tag, "lWingWidth"),
            l_wing_depth: xml_attr_f64(tag, "lWingDepth"),
            l_side: xml_attr(tag, "lSide"),
            u_left_wing_width: xml_attr_f64(tag, "uLeftWingWidth"),
            u_right_wing_width: xml_attr_f64(tag, "uRightWingWidth"),
            u_back_depth: xml_attr_f64(tag, "uBackDepth"),
            t_bar_depth: xml_attr_f64(tag, "tBarDepth"),
            t_stem_width: xml_attr_f64(tag, "tStemWidth"),
            position_mode: xml_attr(tag, "positionMode").unwrap_or_else(|| "center".into()),
            custom_x: xml_attr_f64(tag, "customX"),
            custom_y: xml_attr_f64(tag, "customY"),
            style: xml_attr(tag, "style").unwrap_or_else(|| "wood".into()),
            corner_radius: xml_attr_f64(tag, "cornerRadius"),
            show_dimensions: xml_attr_bool(tag, "showDimensions"),
        });
    }

    // 2. Layers & Custom Wood Plates
    let mut active_layer_id: Option<String> = None;
    if let Some(lpos) = raw.find("<layers") {
        if let Some(lend) = raw[lpos..].find('>') {
            let ltag = &raw[lpos..=lpos + lend];
            active_layer_id = xml_attr(ltag, "activeLayerId");
        }
    }
    let mut layers: Vec<Layer> = Vec::new();
    search_pos = 0;
    while let Some(rel) = raw[search_pos..].find("<layer") {
        let start = search_pos + rel;
        let next_c = raw.as_bytes().get(start + 6).copied().unwrap_or(b' ');
        if !next_c.is_ascii_whitespace() && next_c != b'/' && next_c != b'>' {
            search_pos = start + 6;
            continue;
        }
        let Some(end_rel) = raw[start..].find('>') else {
            break;
        };
        let tag = &raw[start..=start + end_rel];
        let is_self_closing = tag.ends_with("/>");
        let mut body_end = start + end_rel + 1;
        let mut plates: Vec<LayerPlate> = Vec::new();

        if !is_self_closing {
            if let Some(close_rel) = raw[body_end..].find("</layer>") {
                let inner = &raw[body_end..body_end + close_rel];
                body_end += close_rel + "</layer>".len();
                let mut p_pos = 0;
                while let Some(prel) = inner[p_pos..].find("<plate") {
                    let pstart = p_pos + prel;
                    let Some(pend) = inner[pstart..].find('>') else {
                        break;
                    };
                    let ptag = &inner[pstart..=pstart + pend];
                    p_pos = pstart + pend + 1;
                    plates.push(LayerPlate {
                        id: xml_attr(ptag, "id").unwrap_or_else(|| format!("plate-{}", plates.len() + 1)),
                        name: xml_attr(ptag, "name"),
                        baseboard_id: xml_attr(ptag, "baseboardId"),
                        shape: xml_attr(ptag, "shape"),
                        custom_polygon: None,
                        triangle_type: xml_attr(ptag, "triangleType"),
                        l_wing_width: xml_attr_f64(ptag, "lWingWidth"),
                        l_wing_depth: xml_attr_f64(ptag, "lWingDepth"),
                        l_side: xml_attr(ptag, "lSide"),
                        u_left_wing_width: xml_attr_f64(ptag, "uLeftWingWidth"),
                        u_right_wing_width: xml_attr_f64(ptag, "uRightWingWidth"),
                        u_back_depth: xml_attr_f64(ptag, "uBackDepth"),
                        t_bar_depth: xml_attr_f64(ptag, "tBarDepth"),
                        t_stem_width: xml_attr_f64(ptag, "tStemWidth"),
                        rotation: xml_attr_f64(ptag, "rotation"),
                        width: xml_attr_f64(ptag, "width").unwrap_or(1000.0),
                        depth: xml_attr_f64(ptag, "depth").unwrap_or(600.0),
                        x: xml_attr_f64(ptag, "x"),
                        y: xml_attr_f64(ptag, "y"),
                        thickness_mm: xml_attr_f64(ptag, "thicknessMm"),
                        position_mode: xml_attr(ptag, "positionMode"),
                        style: xml_attr(ptag, "style"),
                    });
                }
            }
        }
        search_pos = body_end;

        layers.push(Layer {
            id: xml_attr(tag, "id").unwrap_or_else(|| format!("layer-{}", layers.len() + 1)),
            name: xml_attr(tag, "name").unwrap_or_else(|| format!("Ebene {}", layers.len())),
            color: xml_attr(tag, "color"),
            visible: xml_attr_bool(tag, "visible").unwrap_or(true),
            locked: xml_attr_bool(tag, "locked").unwrap_or(false),
            height_mm: xml_attr_f64(tag, "heightMm"),
            wood_plate_mode: xml_attr(tag, "woodPlateMode"),
            wood_plate_thickness_mm: xml_attr_f64(tag, "woodPlateThicknessMm"),
            wood_plate_style: xml_attr(tag, "woodPlateStyle"),
            plates: if plates.is_empty() { None } else { Some(plates) },
        });
    }

    // 3. Track & Road Pieces
    let mut pieces: Vec<TrackPiece> = Vec::new();
    search_pos = 0;
    while let Some(rel) = raw[search_pos..].find("<piece") {
        let start = search_pos + rel;
        let next_c = raw.as_bytes().get(start + 6).copied().unwrap_or(b' ');
        if !next_c.is_ascii_whitespace() && next_c != b'/' && next_c != b'>' {
            search_pos = start + 6;
            continue;
        }
        let Some(end_rel) = raw[start..].find('>') else {
            break;
        };
        let tag = &raw[start..=start + end_rel];
        search_pos = start + end_rel + 1;

        let id = xml_attr(tag, "id").unwrap_or_else(|| format!("piece-{}", pieces.len() + 1));
        let catalog_id = xml_attr(tag, "catalogId").unwrap_or_default();
        let x = xml_attr_f64(tag, "x").unwrap_or(0.0);
        let y = xml_attr_f64(tag, "y").unwrap_or(0.0);
        let rotation = xml_attr_f64(tag, "rotation").unwrap_or(0.0);
        let layer_id = xml_attr(tag, "layerId");
        let custom_length = xml_attr_f64(tag, "customLength");
        let custom_angle = xml_attr_f64(tag, "customAngle");

        let mut piece = if let Some(entry) = find_entry(catalogs, &catalog_id) {
            let mut entry_adj = entry.clone();
            if custom_length.is_some() {
                entry_adj.length = custom_length;
            }
            if custom_angle.is_some() {
                entry_adj.angle = custom_angle;
            }
            create_piece(id, &entry_adj, Transform { x, y, rotation }, layer_id)
        } else {
            TrackPiece {
                id,
                catalog_id,
                transform: Transform { x, y, rotation },
                connectors: Vec::new(),
                power_feed: None,
                is_isolator: None,
                isolated_connector_index: None,
                has_catenary: None,
                catenary_power_feed: None,
                catenary_isolator: None,
                catenary_isolated_connector_index: None,
                has_bridge: None,
                has_tunnel: None,
                has_lantern: None,
                has_decoder: None,
                turnout_current_ma: None,
                has_signal: None,
                signal_side: None,
                signal_direction: None,
                elevation: None,
                custom_name: None,
                group_id: None,
                layer_id,
                custom_length: None,
                custom_angle: None,
                custom_radius: None,
                custom_offsets: None,
                custom_chord_dist: None,
            }
        };
        piece.custom_length = custom_length;
        piece.custom_angle = custom_angle;
        if let (Some(es), Some(ee)) = (xml_attr_f64(tag, "elevStart"), xml_attr_f64(tag, "elevEnd")) {
            piece.elevation = Some(Elevation { start: es, end: ee });
        }
        piece.power_feed = xml_attr_bool(tag, "powerFeed");
        piece.is_isolator = xml_attr_bool(tag, "isIsolator");
        piece.has_catenary = xml_attr_bool(tag, "hasCatenary");
        piece.catenary_power_feed = xml_attr_bool(tag, "catenaryPowerFeed");
        piece.catenary_isolator = xml_attr_bool(tag, "catenaryIsolator");
        piece.has_bridge = xml_attr_bool(tag, "hasBridge");
        piece.has_tunnel = xml_attr_bool(tag, "hasTunnel");
        piece.has_lantern = xml_attr_bool(tag, "hasLantern");
        piece.has_decoder = xml_attr_bool(tag, "hasDecoder");
        piece.has_signal = xml_attr_bool(tag, "hasSignal");
        piece.custom_name = xml_attr(tag, "customName");
        pieces.push(piece);
    }

    // 4. 3D Models & Buildings
    let mut models: Vec<Model3D> = Vec::new();
    search_pos = 0;
    while let Some(rel) = raw[search_pos..].find("<model") {
        let start = search_pos + rel;
        let next_c = raw.as_bytes().get(start + 6).copied().unwrap_or(b' ');
        if !next_c.is_ascii_whitespace() && next_c != b'/' && next_c != b'>' {
            search_pos = start + 6;
            continue;
        }
        let Some(end_rel) = raw[start..].find('>') else {
            break;
        };
        let tag = &raw[start..=start + end_rel];
        let is_self_closing = tag.ends_with("/>");
        let mut body_end = start + end_rel + 1;
        let mut file_data: Option<std::sync::Arc<str>> = None;

        if !is_self_closing {
            if let Some(close_rel) = raw[body_end..].find("</model>") {
                let inner = &raw[body_end..body_end + close_rel];
                body_end += close_rel + "</model>".len();
                if let (Some(fs), Some(fe)) = (inner.find("<fileData>"), inner.find("</fileData>")) {
                    let fd_start = fs + "<fileData>".len();
                    if fe >= fd_start {
                        file_data = Some(std::sync::Arc::from(unescape_xml(inner[fd_start..fe].trim())));
                    }
                }
            }
        }
        search_pos = body_end;

        models.push(Model3D {
            id: xml_attr(tag, "id").unwrap_or_else(|| format!("model-{}", models.len() + 1)),
            name: xml_attr(tag, "name").unwrap_or_else(|| "Model".into()),
            category: xml_attr(tag, "category").unwrap_or_else(|| "buildings".into()),
            model_type: xml_attr(tag, "modelType").unwrap_or_else(|| "builtin".into()),
            builtin_id: xml_attr(tag, "builtinId"),
            file_name: xml_attr(tag, "fileName"),
            file_data,
            transform: Transform {
                x: xml_attr_f64(tag, "x").unwrap_or(0.0),
                y: xml_attr_f64(tag, "y").unwrap_or(0.0),
                rotation: xml_attr_f64(tag, "rotation").unwrap_or(0.0),
            },
            width: xml_attr_f64(tag, "width").unwrap_or(100.0),
            depth: xml_attr_f64(tag, "depth").unwrap_or(80.0),
            height: xml_attr_f64(tag, "height").unwrap_or(90.0),
            mountain_style: xml_attr(tag, "mountainStyle"),
            mountain_path: None,
            ground_terrain_type: xml_attr(tag, "groundTerrainType"),
            scale: xml_attr_f64(tag, "scale"),
            elevation: xml_attr_f64(tag, "elevation"),
            layer_id: xml_attr(tag, "layerId"),
        });
    }

    let first_bb = baseboards.first().cloned();
    let mut layout = Layout {
        name,
        scale,
        pieces,
        models: if models.is_empty() { None } else { Some(models) },
        feed_voltage,
        catenary_feed_voltage,
        catenary_circuit_mode,
        loco_current_ma,
        loco_count,
        train_current_amps: None,
        turnout_lantern_current_ma,
        turnout_decoder_current_ma,
        layers: if layers.is_empty() { None } else { Some(layers) },
        active_layer_id,
        baseboard: first_bb,
        baseboards: if baseboards.is_empty() { None } else { Some(baseboards) },
    };
    update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
    Ok(layout)
}

