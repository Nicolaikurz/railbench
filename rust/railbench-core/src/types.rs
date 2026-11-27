use std::sync::Arc;

use serde::{Deserialize, Serialize};

mod opt_arc_str_serde {
    use std::sync::Arc;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(val: &Option<Arc<str>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match val {
            Some(s) => serializer.serialize_some(s.as_ref()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Arc<str>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(opt.map(Arc::from))
    }
}

mod arc_vec_f32_serde {
    use std::sync::Arc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(val: &Arc<Vec<f32>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        val.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<Vec<f32>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Arc::new(Vec::<f32>::deserialize(deserializer)?))
    }
}

mod arc_vec_u8_serde {
    use std::sync::Arc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(val: &Arc<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        val.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Arc::new(Vec::<u8>::deserialize(deserializer)?))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainHeightfield {
    pub cols: usize,
    pub rows: usize,
    pub min_x: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_z: f64,
    #[serde(with = "arc_vec_f32_serde")]
    pub heights: Arc<Vec<f32>>,
    #[serde(with = "arc_vec_u8_serde")]
    pub materials: Arc<Vec<u8>>,
    #[serde(default)]
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorRef {
    pub piece_id: String,
    pub connector_index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connector {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_to: Option<ConnectorRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Elevation {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPiece {
    pub id: String,
    pub catalog_id: String,
    pub transform: Transform,
    pub connectors: Vec<Connector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation: Option<Elevation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_feed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_isolator: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolated_connector_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_catenary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catenary_power_feed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catenary_isolator: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catenary_isolated_connector_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_lantern: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_decoder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turnout_current_ma: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_signal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_bridge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_tunnel: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_angle: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_offsets: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_chord_dist: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerPlate {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseboard_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_polygon: Option<Vec<Vec2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triangle_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l_wing_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l_wing_depth: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u_left_wing_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u_right_wing_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u_back_depth: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t_bar_depth: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t_stem_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    pub width: f64,
    pub depth: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thickness_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub visible: bool,
    pub locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_plate_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_plate_thickness_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_plate_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plates: Option<Vec<LayerPlate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_field: Option<TerrainHeightfield>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseboardConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_polygon: Option<Vec<Vec2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triangle_type: Option<String>,
    pub width: f64,
    pub depth: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l_wing_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l_wing_depth: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u_left_wing_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u_right_wing_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u_back_depth: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t_bar_depth: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t_stem_width: Option<f64>,
    pub position_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_y: Option<f64>,
    pub style: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_dimensions: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MountainBrushNode {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model3D {
    pub id: String,
    pub name: String,
    pub category: String,
    pub transform: Transform,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub model_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mountain_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mountain_path: Option<Vec<MountainBrushNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ground_terrain_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_arc_str_serde")]
    pub file_data: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub name: String,
    pub scale: String,
    pub pieces: Vec<TrackPiece>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<Model3D>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed_voltage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catenary_feed_voltage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catenary_circuit_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loco_current_ma: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loco_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub train_current_amps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turnout_lantern_current_ma: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turnout_decoder_current_ma: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<Layer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_layer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseboard: Option<BaseboardConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseboards: Option<Vec<BaseboardConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_field: Option<TerrainHeightfield>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalConnector {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub angle: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauge: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_roadbed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_road: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub road_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_connectors: Option<Vec<LocalConnector>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub scale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gauge: Option<f64>,
    #[serde(default, alias = "isRoadSystem", skip_serializing_if = "Option::is_none")]
    pub is_road: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_roadbed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductors: Option<u8>,
    pub entries: Vec<CatalogEntry>,
}
