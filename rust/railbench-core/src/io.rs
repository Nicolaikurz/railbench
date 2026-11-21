use crate::types::{Catalog, Layout};

/// Parses a RAILbench `.railbench` / `.gleisplan` JSON string into a `Layout`.
pub fn load_layout_json(json: &str) -> Result<Layout, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serializes a `Layout` to `.railbench` JSON.
/// Uses pretty-printed JSON for small/medium layouts (<= 250 pieces) and compact JSON
/// for larger layouts to keep file sizes small and saves fast.
pub fn save_layout_json(layout: &Layout) -> Result<String, serde_json::Error> {
    if layout.pieces.len() <= 250 {
        serde_json::to_string_pretty(layout)
    } else {
        serde_json::to_string(layout)
    }
}

/// Serializes a `Layout` to compact (single-line) JSON for fast network sync and large files.
pub fn save_layout_json_compact(layout: &Layout) -> Result<String, serde_json::Error> {
    serde_json::to_string(layout)
}

/// Parses a track catalog JSON string into a `Catalog`.
pub fn load_catalog_json(json: &str) -> Result<Catalog, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_minimal_layout() {
        let json = r#"{
            "name": "Test Plan",
            "scale": "H0",
            "pieces": [
                {
                    "id": "p1",
                    "catalogId": "24188",
                    "transform": { "x": 100.0, "y": 50.0, "rotation": 0.0 },
                    "connectors": [
                        { "x": 100.0, "y": 50.0, "angle": 180.0 },
                        { "x": 288.3, "y": 50.0, "angle": 0.0 }
                    ],
                    "powerFeed": true
                }
            ]
        }"#;
        let layout = load_layout_json(json).expect("valid layout JSON");
        assert_eq!(layout.name, "Test Plan");
        assert_eq!(layout.pieces.len(), 1);
        assert_eq!(layout.pieces[0].catalog_id, "24188");
        assert_eq!(layout.pieces[0].power_feed, Some(true));

        let saved = save_layout_json(&layout).expect("serialize layout");
        let reparsed = load_layout_json(&saved).expect("reparse layout");
        assert_eq!(layout, reparsed);

        let compact = save_layout_json_compact(&layout).expect("serialize compact layout");
        assert!(!compact.contains('\n'));
        assert!(compact.len() < saved.len());
        let reparsed_compact = load_layout_json(&compact).expect("reparse compact layout");
        assert_eq!(layout, reparsed_compact);
    }
}
