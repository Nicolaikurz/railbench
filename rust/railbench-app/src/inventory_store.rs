use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::export::PieceListSummary;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InventoryEntry {
    pub owned: usize,
    pub unit_price: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackInventoryStore {
    pub currency: String,
    pub items: BTreeMap<String, InventoryEntry>,
}

impl Default for TrackInventoryStore {
    fn default() -> Self {
        Self {
            currency: "EUR".to_string(),
            items: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BomBudgetSummary {
    pub total_needed: usize,
    pub total_owned_used: usize,
    pub total_to_buy: usize,
    pub total_layout_cost: f64,
    pub remaining_buy_cost: f64,
}

impl TrackInventoryStore {
    pub fn inventory_path() -> Option<PathBuf> {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("railbench").join("inventory.json"));
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                return Some(PathBuf::from(appdata).join("railbench").join("inventory.json"));
            }
            return std::env::var_os("USERPROFILE")
                .map(|p| PathBuf::from(p).join(".config").join("railbench").join("inventory.json"));
        }
        #[cfg(target_os = "macos")]
        {
            return std::env::var_os("HOME").map(|h| {
                PathBuf::from(h)
                    .join("Library")
                    .join("Application Support")
                    .join("railbench")
                    .join("inventory.json")
            });
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".config").join("railbench").join("inventory.json"))
        }
    }

    pub fn load_from_path(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(val) = serde_json::from_str::<Value>(&raw) else {
            return Self::default();
        };
        let mut out = Self::default();
        if let Some(cur) = val.get("currency").and_then(|v| v.as_str()) {
            out.currency = cur.to_string();
        }
        if let Some(items_obj) = val.get("items").and_then(|v| v.as_object()) {
            for (k, entry_v) in items_obj {
                let owned = entry_v.get("owned").and_then(|n| n.as_u64()).unwrap_or(0) as usize;
                let unit_price = entry_v
                    .get("unitPrice")
                    .and_then(|n| n.as_f64())
                    .unwrap_or(0.0)
                    .max(0.0);
                out.set_entry(k, owned, unit_price);
            }
        }
        out
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Inventory dir error: {e}"))?;
        }
        let mut items_map = Map::new();
        for (k, entry) in &self.items {
            items_map.insert(
                k.clone(),
                serde_json::json!({
                    "owned": entry.owned,
                    "unitPrice": entry.unit_price,
                }),
            );
        }
        let root = serde_json::json!({
            "currency": self.currency,
            "items": Value::Object(items_map),
        });
        let json = serde_json::to_string_pretty(&root)
            .map_err(|e| format!("Inventory JSON error: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("Inventory write error: {e}"))
    }

    pub fn load_from_disk() -> Self {
        Self::inventory_path()
            .map(|p| Self::load_from_path(&p))
            .unwrap_or_default()
    }

    pub fn save_to_disk(&self) {
        if let Some(path) = Self::inventory_path() {
            let _ = self.save_to_path(&path);
        }
    }

    pub fn currency_symbol(&self) -> &str {
        match self.currency.as_str() {
            "USD" => "$",
            "CHF" => "CHF",
            "GBP" => "GBP",
            _ => "EUR",
        }
    }

    pub fn owned(&self, catalog_id: &str) -> usize {
        self.items.get(catalog_id).map(|e| e.owned).unwrap_or(0)
    }

    pub fn unit_price(&self, catalog_id: &str) -> f64 {
        self.items
            .get(catalog_id)
            .map(|e| e.unit_price.max(0.0))
            .unwrap_or(0.0)
    }

    pub fn set_entry(&mut self, catalog_id: &str, owned: usize, unit_price: f64) {
        let price = unit_price.max(0.0);
        if owned == 0 && price <= 0.0001 {
            self.items.remove(catalog_id);
        } else {
            self.items.insert(
                catalog_id.to_string(),
                InventoryEntry {
                    owned,
                    unit_price: price,
                },
            );
        }
    }

    pub fn compute_bom_budget(&self, sum: &PieceListSummary) -> BomBudgetSummary {
        let mut out = BomBudgetSummary::default();
        for item in sum.tracks.items.iter().chain(sum.roads.items.iter()) {
            let needed = item.count;
            let stock = self.owned(&item.catalog_id);
            let used = needed.min(stock);
            let to_buy = needed.saturating_sub(used);
            let price = self.unit_price(&item.catalog_id);
            out.total_needed += needed;
            out.total_owned_used += used;
            out.total_to_buy += to_buy;
            out.total_layout_cost += needed as f64 * price;
            out.remaining_buy_cost += to_buy as f64 * price;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{PieceCategorySummary, PieceListSummary, PieceSummaryItem};

    #[test]
    fn inventory_roundtrip_and_bom_budget_calculation() {
        let mut store = TrackInventoryStore {
            currency: "EUR".into(),
            items: BTreeMap::new(),
        };
        store.set_entry("24188", 4, 2.50);
        store.set_entry("24611", 0, 24.00);
        store.set_entry("zero", 0, 0.0);
        assert_eq!(store.items.len(), 2);

        let sum = PieceListSummary {
            total_pieces: 12,
            total_length_meters: 2.0,
            total_objects: 12,
            tracks: PieceCategorySummary {
                items: vec![
                    PieceSummaryItem {
                        catalog_id: "24188".into(),
                        name: "Gerade".into(),
                        category: "Geraden".into(),
                        kind: "straight".into(),
                        count: 10,
                        specification: "188 mm".into(),
                        total_length_meters: 1.88,
                    },
                    PieceSummaryItem {
                        catalog_id: "24611".into(),
                        name: "Weiche L".into(),
                        category: "Weichen".into(),
                        kind: "turnout-left".into(),
                        count: 2,
                        specification: "24.3 deg".into(),
                        total_length_meters: 0.37,
                    },
                ],
                total_pieces: 12,
                total_length_meters: 2.25,
            },
            roads: PieceCategorySummary {
                items: Vec::new(),
                total_pieces: 0,
                total_length_meters: 0.0,
            },
            buildings: Vec::new(),
            total_buildings: 0,
        };

        let budget = store.compute_bom_budget(&sum);
        assert_eq!(budget.total_needed, 12);
        assert_eq!(budget.total_owned_used, 4);
        assert_eq!(budget.total_to_buy, 8);
        assert!((budget.total_layout_cost - (10.0 * 2.50 + 2.0 * 24.00)).abs() < 1e-6);
        assert!((budget.remaining_buy_cost - (6.0 * 2.50 + 2.0 * 24.00)).abs() < 1e-6);

        let dir = std::env::temp_dir().join("railbench_inv_test_roundtrip");
        let path = dir.join("inventory.json");
        store.save_to_path(&path).unwrap();
        let reloaded = TrackInventoryStore::load_from_path(&path);
        assert_eq!(store, reloaded);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
