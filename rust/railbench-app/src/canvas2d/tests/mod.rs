use super::*;
use crate::catalog::{builtin_catalogs, create_piece, find_entry};
use railbench_core::types::Transform;

fn make_layout_with_pieces(pieces: Vec<TrackPiece>) -> Layout {
    Layout {
        name: "CanvasTest".into(),
        scale: "H0".into(),
        pieces,
        models: None,
        feed_voltage: None,
        catenary_feed_voltage: None,
        catenary_circuit_mode: None,
        loco_current_ma: None,
        loco_count: None,
        train_current_amps: None,
        turnout_lantern_current_ma: None,
        turnout_decoder_current_ma: None,
        layers: None,
        active_layer_id: None,
        baseboard: None,
        baseboards: None,
        terrain_field: None,
    }
}


mod cache_and_snapping;
mod editing_and_terrain;
