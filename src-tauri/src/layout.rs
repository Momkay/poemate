//! Widget positions per screen size, plus the look settings (opacity, density). Stored as JSON, sanitized on load.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE: &str = "layout.json";
const MAX_KEYS: usize = 32;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WidgetPos {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub opacity: f32,
    /// "cozy" | "compact"
    pub density: String,
    /// "en" | "nl"
    pub lang: String,
    /// the first-run setup card was finished
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { opacity: 0.72, density: "cozy".into(), lang: "en".into(), onboarded: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Layout {
    pub settings: Settings,
    /// "<width>x<height>" of the overlay -> widget id -> position
    pub all: HashMap<String, HashMap<String, WidgetPos>>,
}

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v.is_finite() { v.clamp(lo, hi) } else { lo }
}

impl Layout {
    /// The file is user-editable and the UI is untrusted input: keep every value in a sane range.
    pub fn sanitized(mut self) -> Self {
        self.settings.opacity = clamp(self.settings.opacity, 0.2, 1.0);
        if !["cozy", "compact"].contains(&self.settings.density.as_str()) {
            self.settings.density = "cozy".into();
        }
        if !["en", "nl"].contains(&self.settings.lang.as_str()) {
            self.settings.lang = "en".into();
        }
        self.all.retain(|k, _| k.len() <= 16);
        trim(&mut self.all, MAX_KEYS);
        for widgets in self.all.values_mut() {
            widgets.retain(|id, _| id.len() <= 32);
            trim(widgets, 32);
            for p in widgets.values_mut() {
                p.x = clamp(p.x, -2000.0, 8000.0);
                p.y = clamp(p.y, -2000.0, 8000.0);
                p.scale = clamp(p.scale, 0.5, 2.0);
            }
        }
        self
    }
}

/// Drops arbitrary entries until at most `n` remain.
fn trim<V>(m: &mut HashMap<String, V>, n: usize) {
    while m.len() > n {
        let k = m.keys().next().cloned().unwrap();
        m.remove(&k);
    }
}

pub fn load(dir: &Path) -> Layout {
    std::fs::read(dir.join(FILE)).ok().and_then(|b| serde_json::from_slice::<Layout>(&b).ok()).unwrap_or_default().sanitized()
}

pub fn save(dir: &Path, layout: Layout) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(FILE);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(&layout.sanitized())?)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32, y: f32, scale: f32) -> WidgetPos {
        WidgetPos { x, y, scale }
    }

    #[test]
    fn roundtrips_and_missing_or_corrupt_files_give_defaults() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(load(t.path()), Layout::default());
        let mut l = Layout::default();
        l.all.insert("1920x1080".into(), HashMap::from([("route".to_string(), pos(18.0, 56.0, 1.0))]));
        l.settings.density = "compact".into();
        save(t.path(), l.clone()).unwrap();
        assert_eq!(load(t.path()), l);
        std::fs::write(t.path().join(FILE), "{nope").unwrap();
        assert_eq!(load(t.path()), Layout::default());
    }

    #[test]
    fn values_are_clamped_and_junk_is_dropped() {
        let mut l = Layout::default();
        l.settings = Settings { opacity: 7.0, density: "gigantic".into(), lang: "xx".into(), ..Settings::default() };
        l.all.insert("a_very_long_resolution_key".into(), HashMap::new());
        l.all.insert("800x600".into(), HashMap::from([("w".to_string(), pos(f32::NAN, 1e9, -3.0))]));
        let s = l.sanitized();
        assert_eq!((s.settings.opacity, s.settings.density.as_str(), s.settings.lang.as_str()), (1.0, "cozy", "en"));
        assert_eq!(s.all.len(), 1);
        let p = &s.all["800x600"]["w"];
        assert_eq!((p.x, p.y, p.scale), (-2000.0, 8000.0, 0.5));
    }

    #[test]
    fn partial_files_fill_in_defaults() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join(FILE), r#"{"all":{"1x1":{"w":{"x":1,"y":2,"scale":1}}}}"#).unwrap();
        let l = load(t.path());
        assert_eq!(l.settings, Settings::default());
        assert_eq!(l.all["1x1"]["w"], pos(1.0, 2.0, 1.0));
    }
}
