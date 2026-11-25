use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use railbench_core::types::Catalog;

use crate::catalog::builtin_catalogs;

/// Tracks whether any custom or overridden catalog has been loaded or imported in this process.
/// When `true`, `catalog::is_builtin_slice` returns `false` so lookups always read the live
/// `catalogs` slice rather than the static compile-time `builtin_index`.
static HAS_CUSTOM_CATALOGS: AtomicBool = AtomicBool::new(false);

pub fn has_custom_catalogs() -> bool {
    HAS_CUSTOM_CATALOGS.load(Ordering::Relaxed)
}

pub fn mark_catalogs_customized() {
    HAS_CUSTOM_CATALOGS.store(true, Ordering::Relaxed);
}

#[derive(Debug, Clone)]
pub struct ImportCatalogResult {
    pub name: String,
    pub index: usize,
    pub is_road: bool,
    #[allow(dead_code)]
    pub replaced: bool,
    #[allow(dead_code)]
    pub persisted_path: Option<PathBuf>,
}

/// Returns the per-user directory where imported/updated track catalogs are persisted
/// (`$XDG_CONFIG_HOME/railbench/catalogs`, `%APPDATA%\railbench\catalogs` on Windows,
/// `~/Library/Application Support/railbench/catalogs` on macOS, or `$HOME/.config/railbench/catalogs`).
pub fn catalogs_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("railbench").join("catalogs"));
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(PathBuf::from(appdata).join("railbench").join("catalogs"));
        }
        return std::env::var_os("USERPROFILE")
            .map(|p| PathBuf::from(p).join(".config").join("railbench").join("catalogs"));
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("railbench")
                .join("catalogs")
        });
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".config").join("railbench").join("catalogs"))
    }
}

fn sanitize_catalog_filename(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "custom-catalog".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Ensures inherited fields (`id`, per-entry `gauge`, per-entry `is_road`, per-entry `has_roadbed`) match built-in defaults.
pub fn normalize_catalog(cat: &mut Catalog) {
    if cat.id.trim().is_empty() {
        let base = format!(
            "{}-{}-{}",
            cat.manufacturer.as_deref().unwrap_or("custom"),
            cat.system.as_deref().unwrap_or(&cat.name),
            if cat.scale.is_empty() { "h0" } else { &cat.scale }
        );
        cat.id = sanitize_catalog_filename(&base);
    }
    let default_gauge = cat.gauge;
    let is_road = cat.is_road == Some(true);
    let default_roadbed = cat.has_roadbed;
    for entry in &mut cat.entries {
        if entry.gauge.is_none() {
            entry.gauge = default_gauge;
        }
        if is_road {
            entry.is_road = Some(true);
        }
        if entry.has_roadbed.is_none() {
            entry.has_roadbed = default_roadbed;
        }
    }
}

fn normalize_kind_alias(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "gerade" | "gerades gleis" => "straight",
        "bogen" | "kurve" | "gebogen" | "curved" => "curve",
        "weiche-links" | "linksweiche" | "turnout_left" | "left-turnout" => "turnout-left",
        "weiche-rechts" | "rechtsweiche" | "turnout_right" | "right-turnout" => "turnout-right",
        "bogenweiche-links" | "bw-links" => "curved-turnout-left",
        "bogenweiche-rechts" | "bw-rechts" => "curved-turnout-right",
        "dreiwegweiche" | "threeway" => "three-way",
        "kreuzung" => "crossing",
        "dkw" | "doppelkreuzung" | "doppelkreuzungsweiche" | "doubleslip" => "double-slip",
        "flexgleis" => "flex",
        "prellbock" | "buffer" => "buffer-stop",
        "anschlussgleis" | "einspeisung" => "feeder",
        "trenngleis" | "isoliergleis" => "isolating",
        other => return other.to_string(),
    }
    .to_string()
}

fn normalize_entry_json(val: &mut serde_json::Value) {
    let Some(obj) = val.as_object_mut() else {
        return;
    };
    for (alias, canon) in [
        ("nummer", "id"),
        ("artikelnummer", "id"),
        ("artikelNr", "id"),
        ("artNr", "id"),
        ("articleNumber", "id"),
        ("trackId", "id"),
        ("titel", "name"),
        ("bezeichnung", "name"),
        ("typ", "kind"),
        ("type", "kind"),
        ("kategorie", "category"),
        ("beschreibung", "description"),
        ("laenge", "length"),
        ("winkel", "angle"),
        ("abzweigRadius", "branchRadius"),
        ("abzweigWinkel", "branchAngle"),
        ("abzweigLaenge", "branchLength"),
        ("spurweite", "gauge"),
        ("bettung", "hasRoadbed"),
        ("strasse", "isRoad"),
    ] {
        if !obj.contains_key(canon) {
            if let Some(v) = obj.remove(alias) {
                obj.insert(canon.to_string(), v);
            }
        }
    }
    if let Some(k) = obj.get("kind").and_then(|v| v.as_str()).map(normalize_kind_alias) {
        obj.insert("kind".into(), serde_json::Value::String(k));
    }
    if !obj.contains_key("name") {
        if let Some(id_s) = obj.get("id").and_then(|v| v.as_str()) {
            obj.insert("name".into(), serde_json::Value::String(id_s.to_string()));
        }
    }
}

/// Parses and validates a track/road catalog JSON string (stripping UTF-8 BOM, detecting layout
/// files, accepting top-level entry arrays, and mapping German/alias keys & track kinds).
pub fn parse_catalog_json(raw: &str) -> Result<Catalog, String> {
    let clean = raw.trim().trim_start_matches('\u{feff}');
    let mut val: serde_json::Value = serde_json::from_str(clean).map_err(|e| e.to_string())?;

    if let Some(arr) = val.as_array_mut() {
        for item in arr.iter_mut() {
            normalize_entry_json(item);
        }
        val = serde_json::json!({
            "id": "custom-imported",
            "name": "Custom Catalog",
            "scale": "H0",
            "entries": val,
        });
    } else if let Some(obj) = val.as_object_mut() {
        if obj.contains_key("pieces") && !obj.contains_key("entries") && !obj.contains_key("gleise") {
            return Err("This file appears to be a track layout (.railbench), not a track catalog.".into());
        }
        for (alias, canon) in [
            ("gleise", "entries"),
            ("tracks", "entries"),
            ("hersteller", "manufacturer"),
            ("massstab", "scale"),
            ("spur", "scale"),
            ("spurweite", "gauge"),
            ("bettung", "hasRoadbed"),
            ("titel", "name"),
        ] {
            if !obj.contains_key(canon) {
                if let Some(v) = obj.remove(alias) {
                    obj.insert(canon.to_string(), v);
                }
            }
        }
        if !obj.contains_key("id") {
            obj.insert("id".into(), serde_json::Value::String(String::new()));
        }
        if !obj.contains_key("name") {
            obj.insert("name".into(), serde_json::Value::String("Custom Catalog".into()));
        }
        if let Some(entries) = obj.get_mut("entries").and_then(|v| v.as_array_mut()) {
            for item in entries.iter_mut() {
                normalize_entry_json(item);
            }
        }
    }

    let mut cat: Catalog = serde_json::from_value(val).map_err(|e| e.to_string())?;
    if cat.entries.is_empty() {
        return Err("Catalog contains no track entries".into());
    }
    normalize_catalog(&mut cat);
    Ok(cat)
}

pub fn export_catalog_json(cat: &Catalog) -> String {
    serde_json::to_string_pretty(cat).unwrap_or_else(|_| "{}".into())
}

/// Replaces an existing catalog matching by `id` (primary) or `name` + `scale` (fallback),
/// or appends it as a new catalog. Returns `(index, replaced)`.
pub fn upsert_catalog(catalogs: &mut Vec<Catalog>, cat: Catalog) -> (usize, bool) {
    let existing_idx = catalogs
        .iter()
        .position(|c| !cat.id.is_empty() && c.id.eq_ignore_ascii_case(&cat.id))
        .or_else(|| {
            catalogs.iter().position(|c| {
                c.name.eq_ignore_ascii_case(&cat.name) && c.scale.eq_ignore_ascii_case(&cat.scale)
            })
        });

    if let Some(idx) = existing_idx {
        catalogs[idx] = cat;
        (idx, true)
    } else {
        catalogs.push(cat);
        (catalogs.len() - 1, false)
    }
}

/// Persists a catalog into `dir/<sanitized-id>.json`.
pub fn save_catalog_to_dir(dir: &Path, cat: &Catalog) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create catalog dir: {e}"))?;
    let path = dir.join(format!("{}.json", sanitize_catalog_filename(&cat.id)));
    let json = serde_json::to_string_pretty(cat).map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write catalog file: {e}"))?;
    Ok(path)
}

/// Loads all built-in catalogs and applies any persisted `.json` overrides/additions from `dir`.
pub fn load_all_catalogs_from_dir(dir: &Path) -> Vec<Catalog> {
    let mut catalogs = builtin_catalogs();
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return catalogs;
    };

    let mut files: Vec<PathBuf> = read_dir
        .filter_map(|e| e.ok().map(|ent| ent.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|ext| ext.to_str())
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect();
    files.sort();

    let mut loaded_any = false;
    for path in files {
        match std::fs::read_to_string(&path).and_then(|raw| {
            parse_catalog_json(&raw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }) {
            Ok(cat) => {
                upsert_catalog(&mut catalogs, cat);
                loaded_any = true;
            }
            Err(err) => {
                eprintln!("[catalog_store] failed to load {}: {err}", path.display());
            }
        }
    }
    if loaded_any {
        mark_catalogs_customized();
    }
    catalogs
}

/// Loads built-in catalogs overlaid with any saved user catalog files from `catalogs_dir()`.
pub fn load_all_catalogs() -> Vec<Catalog> {
    match catalogs_dir() {
        Some(dir) => load_all_catalogs_from_dir(&dir),
        None => builtin_catalogs(),
    }
}

/// Parses, upserts (replacing a matching built-in catalog by `id` or `name`+`scale`),
/// and persists the catalog JSON to `dir`.
pub fn import_and_persist_catalog_in_dir(
    catalogs: &mut Vec<Catalog>,
    raw: &str,
    dir: Option<&Path>,
) -> Result<ImportCatalogResult, String> {
    let cat = parse_catalog_json(raw)?;
    let name = cat.name.clone();
    let is_road = cat.is_road == Some(true);
    let persisted_path = match dir {
        Some(d) => Some(save_catalog_to_dir(d, &cat)?),
        None => None,
    };
    let (index, replaced) = upsert_catalog(catalogs, cat);
    mark_catalogs_customized();
    Ok(ImportCatalogResult {
        name,
        index,
        is_road,
        replaced,
        persisted_path,
    })
}

/// Imports a catalog JSON into the active catalog list and saves it to the user's config folder
/// so it persists across restarts and continues overriding any matching built-in catalog.
pub fn import_and_persist_catalog(
    catalogs: &mut Vec<Catalog>,
    raw: &str,
) -> Result<ImportCatalogResult, String> {
    let dir = catalogs_dir();
    import_and_persist_catalog_in_dir(catalogs, raw, dir.as_deref())
}

/// Returns `true` if a persisted override/custom file exists for `catalog_id` in `catalogs_dir()`.
#[allow(dead_code)]
pub fn has_saved_override(catalog_id: &str) -> bool {
    let Some(dir) = catalogs_dir() else {
        return false;
    };
    dir.join(format!("{}.json", sanitize_catalog_filename(catalog_id)))
        .is_file()
}

/// Removes a saved catalog file in `dir` and restores the original built-in catalog if one exists
/// (or removes the custom catalog if it has no built-in counterpart).
#[allow(dead_code)]
pub fn reset_catalog_to_builtin_in_dir(
    catalogs: &mut Vec<Catalog>,
    catalog_id: &str,
    dir: &Path,
) -> Result<bool, String> {
    let file_path = dir.join(format!("{}.json", sanitize_catalog_filename(catalog_id)));
    let removed_file = if file_path.is_file() {
        std::fs::remove_file(&file_path).map_err(|e| format!("Failed to remove override: {e}"))?;
        true
    } else {
        false
    };

    if let Some(orig) = builtin_catalogs()
        .into_iter()
        .find(|c| c.id.eq_ignore_ascii_case(catalog_id))
    {
        upsert_catalog(catalogs, orig);
        Ok(true)
    } else {
        let before = catalogs.len();
        catalogs.retain(|c| !c.id.eq_ignore_ascii_case(catalog_id));
        Ok(removed_file || catalogs.len() != before)
    }
}

/// Removes a persisted override for `catalog_id` in `catalogs_dir()` and restores the built-in catalog.
#[allow(dead_code)]
pub fn reset_catalog_to_builtin(
    catalogs: &mut Vec<Catalog>,
    catalog_id: &str,
) -> Result<bool, String> {
    let Some(dir) = catalogs_dir() else {
        return Ok(false);
    };
    reset_catalog_to_builtin_in_dir(catalogs, catalog_id, &dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::find_entry;

    fn temp_catalog_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "railbench_catalog_store_{tag}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn import_replaces_builtin_by_id_persists_and_resets() {
        let dir = temp_catalog_dir("replace_id");
        let mut catalogs = builtin_catalogs();
        let initial_len = catalogs.len();

        // Take a built-in catalog, keep the same id & entry count, rename a track and tweak length.
        let mut updated = catalogs
            .iter()
            .find(|c| c.id == "maerklin-c-h0")
            .expect("maerklin-c-h0 built-in")
            .clone();
        updated.name = "Maerklin H0 C-Gleis (Updated v2)".into();
        let first_track_id = updated.entries[0].id.clone();
        updated.entries[0].name = "Updated Custom Straight".into();
        updated.entries[0].length = Some(999.0);
        let raw = serde_json::to_string(&updated).unwrap();

        let res = import_and_persist_catalog_in_dir(&mut catalogs, &raw, Some(&dir)).unwrap();
        assert!(res.replaced, "should replace matching built-in by id");
        assert_eq!(catalogs.len(), initial_len, "should not duplicate built-in catalog");
        assert!(res.persisted_path.as_ref().is_some_and(|p| p.is_file()));

        // Live find_entry must see the updated entry even when id + entry count stayed identical.
        let found = find_entry(&catalogs, &first_track_id).expect("entry present");
        assert_eq!(found.name, "Updated Custom Straight");
        assert_eq!(found.length, Some(999.0));

        // Reloading from disk must still apply the persisted override over built-in catalogs.
        let reloaded = load_all_catalogs_from_dir(&dir);
        assert_eq!(reloaded.len(), initial_len);
        let reloaded_cat = reloaded.iter().find(|c| c.id == "maerklin-c-h0").unwrap();
        assert_eq!(reloaded_cat.name, "Maerklin H0 C-Gleis (Updated v2)");
        assert_eq!(
            find_entry(&reloaded, &first_track_id).unwrap().length,
            Some(999.0)
        );

        // Resetting restores the original built-in catalog and deletes the override file.
        let mut restored = reloaded;
        let did_reset =
            reset_catalog_to_builtin_in_dir(&mut restored, "maerklin-c-h0", &dir).unwrap();
        assert!(did_reset);
        assert!(!dir.join("maerklin-c-h0.json").exists());
        let orig_entry = find_entry(&restored, &first_track_id).unwrap();
        assert_ne!(orig_entry.length, Some(999.0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_new_custom_catalog_appends_and_survives_reload() {
        let dir = temp_catalog_dir("append_new");
        let mut catalogs = builtin_catalogs();
        let initial_len = catalogs.len();

        let raw = r#"{
            "id": "my-custom-h0",
            "name": "My Custom H0 Tracks",
            "scale": "H0",
            "manufacturer": "MyBrand",
            "system": "Custom",
            "gauge": 16.5,
            "entries": [
                { "id": "MC-100", "name": "Straight 100mm", "kind": "straight", "length": 100.0 }
            ]
        }"#;

        let res = import_and_persist_catalog_in_dir(&mut catalogs, raw, Some(&dir)).unwrap();
        assert!(!res.replaced);
        assert_eq!(catalogs.len(), initial_len + 1);
        assert_eq!(find_entry(&catalogs, "MC-100").unwrap().gauge, Some(16.5));

        let reloaded = load_all_catalogs_from_dir(&dir);
        assert_eq!(reloaded.len(), initial_len + 1);
        assert_eq!(find_entry(&reloaded, "MC-100").unwrap().length, Some(100.0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_german_aliases_array_export_and_reject_layout() {
        let de_json = r#"{
            "name": "Eigenbau H0",
            "spur": "H0",
            "spurweite": 16.5,
            "gleise": [
                { "artikelnummer": "E-200", "bezeichnung": "Gerade 200", "typ": "straight", "laenge": 200 },
                { "id": "E-R2", "name": "Bogen", "type": "curve", "radius": 420, "winkel": 30 }
            ]
        }"#;
        let parsed = parse_catalog_json(de_json).expect("german alias catalog parses");
        assert_eq!(parsed.scale, "H0");
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].id, "E-200");
        assert_eq!(parsed.entries[0].length, Some(200.0));

        let arr_json = r#"[{ "id": "ARR-1", "name": "Straight", "type": "straight", "length": 150 }]"#;
        let parsed_arr = parse_catalog_json(arr_json).expect("array catalog parses");
        assert_eq!(parsed_arr.entries.len(), 1);

        let exported = export_catalog_json(&parsed);
        let reparsed = parse_catalog_json(&exported).expect("exported catalog round-trips");
        assert_eq!(reparsed.entries.len(), 2);

        let layout_like = r#"{ "name": "My Plan", "pieces": [{ "id": "p1" }] }"#;
        assert!(parse_catalog_json(layout_like).is_err());
    }
}
