//! Rebindable global hotkeys: action id -> shortcut text, saved as hotkeys.json next to layout.json.
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri_plugin_global_shortcut::Shortcut;

/// (id, label, default). Shortcut text uses the key names the browser reports as `KeyboardEvent.code`.
pub const ACTIONS: [(&str, &str, &str); 7] = [
    ("edit", "Edit mode", "Ctrl+Shift+KeyO"),
    ("hide", "Hide overlay", "Ctrl+Shift+KeyH"),
    ("panel", "Passive panel", "Ctrl+Shift+KeyT"),
    ("step_next", "Next step", "Ctrl+Shift+ArrowRight"),
    ("step_prev", "Previous step", "Ctrl+Shift+ArrowLeft"),
    ("stage_next", "Next stage", "Ctrl+Shift+ArrowDown"),
    ("stage_prev", "Previous stage", "Ctrl+Shift+ArrowUp"),
];

static BOUND: Mutex<Vec<(&'static str, Shortcut)>> = Mutex::new(Vec::new());

#[derive(Serialize)]
pub struct Binding {
    pub id: &'static str,
    pub label: &'static str,
    pub keys: String,
}

fn file(dir: &Path) -> std::path::PathBuf {
    dir.join("hotkeys.json")
}

/// Defaults overlaid with the saved choices; a missing, corrupt or unparsable entry falls back to its default.
pub fn load(dir: &Path) -> BTreeMap<&'static str, String> {
    let saved: BTreeMap<String, String> = std::fs::read_to_string(file(dir)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let mut out: BTreeMap<_, _> = ACTIONS.iter().map(|a| (a.0, a.2.to_string())).collect();
    for (id, keys) in saved {
        let Some(slot) = ACTIONS.iter().find(|a| a.0 == id) else { continue };
        let others: BTreeMap<_, _> = out.iter().filter(|(k, _)| **k != slot.0).map(|(k, v)| (*k, v.clone())).collect();
        if check(&others, &keys).is_ok() {
            out.insert(slot.0, keys);
        }
    }
    out
}

pub fn save(dir: &Path, map: &BTreeMap<&'static str, String>) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join("hotkeys.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(map)?)?;
    std::fs::rename(tmp, file(dir))
}

/// A hotkey must parse, carry a modifier (a bare key would be stolen from every app) and not equal another action's.
pub fn check(others: &BTreeMap<&'static str, String>, keys: &str) -> Result<Shortcut, String> {
    let sc: Shortcut = keys.parse().map_err(|_| format!("Can't use \"{keys}\" as a hotkey"))?;
    if sc.mods.is_empty() {
        return Err("Add Ctrl, Alt or Shift: a bare key would be taken from every app".into());
    }
    for (id, other) in others {
        if other.parse::<Shortcut>().is_ok_and(|o| o == sc) {
            let label = ACTIONS.iter().find(|a| a.0 == *id).map_or(*id, |a| a.1);
            return Err(format!("Already used by \"{label}\""));
        }
    }
    Ok(sc)
}

pub fn set_bound(list: Vec<(&'static str, Shortcut)>) {
    *BOUND.lock().unwrap() = list;
}

pub fn action_of(sc: &Shortcut) -> Option<&'static str> {
    BOUND.lock().unwrap().iter().find(|(_, s)| s == sc).map(|(id, _)| *id)
}

pub fn view(map: &BTreeMap<&'static str, String>) -> Vec<Binding> {
    ACTIONS.iter().map(|a| Binding { id: a.0, label: a.1, keys: map[a.0].clone() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> BTreeMap<&'static str, String> {
        ACTIONS.iter().map(|a| (a.0, a.2.to_string())).collect()
    }

    #[test]
    fn defaults_parse_and_are_distinct() {
        let d = defaults();
        for (id, keys) in &d {
            let others: BTreeMap<_, _> = d.iter().filter(|(k, _)| *k != id).map(|(k, v)| (*k, v.clone())).collect();
            assert!(check(&others, keys).is_ok(), "{id}: {keys}");
        }
    }

    #[test]
    fn conflicts_garbage_and_bare_keys_are_rejected() {
        let d = defaults();
        assert!(check(&d, "Ctrl+Shift+KeyO").unwrap_err().contains("Edit mode"));
        assert!(check(&d, "KeyQ").is_err());
        assert!(check(&d, "Ctrl+Nonsense").is_err());
        assert!(check(&d, "Ctrl+Alt+KeyQ").is_ok());
    }

    #[test]
    fn corrupt_or_conflicting_saved_entries_fall_back() {
        let dir = std::env::temp_dir().join(format!("poemate-hk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(file(&dir), r#"{"edit":"Ctrl+Alt+KeyQ","hide":"Ctrl+Alt+KeyQ","panel":"???","bogus":"Ctrl+KeyZ"}"#).unwrap();
        let m = load(&dir);
        assert_eq!(m["edit"], "Ctrl+Alt+KeyQ");
        assert_eq!(m["hide"], "Ctrl+Shift+KeyH"); // same keys as edit
        assert_eq!(m["panel"], "Ctrl+Shift+KeyT");
        save(&dir, &m).unwrap();
        assert_eq!(load(&dir), m);
        let _ = std::fs::remove_dir_all(dir);
    }
}
