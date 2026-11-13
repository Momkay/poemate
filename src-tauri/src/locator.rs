//! Finds `<PoE install>\logs\Client.txt`: saved choice -> running game -> registry -> Steam libraries -> default folders.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Saved,
    RunningGame,
    Registry,
    Steam,
    DefaultFolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub path: PathBuf,
    pub source: Source,
}

const SETTING_FILE: &str = "client_log_path.txt";

fn log_in(install: &Path) -> Option<PathBuf> {
    let p = install.join("logs").join("Client.txt");
    p.is_file().then_some(p)
}

/// First existing log wins, in the order given. `None` = not found: the UI must offer a manual picker.
pub fn locate(saved: Option<PathBuf>, roots: Vec<(Source, PathBuf)>) -> Option<Located> {
    if let Some(p) = saved.filter(|p| p.is_file()) {
        return Some(Located { path: p, source: Source::Saved });
    }
    roots
        .into_iter()
        .find_map(|(source, dir)| log_in(&dir).map(|path| Located { path, source }))
}

/// `"path"  "D:\\SteamLibrary"` lines of Steam's libraryfolders.vdf.
pub fn steam_library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|l| {
            let mut q = l.split('"').skip(1).step_by(2);
            (q.next()? == "path").then(|| q.next().map(|v| PathBuf::from(v.replace("\\\\", "\\"))))?
        })
        .collect()
}

pub fn load_saved(config_dir: &Path) -> Option<PathBuf> {
    let s = std::fs::read_to_string(config_dir.join(SETTING_FILE)).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| PathBuf::from(s))
}

/// A path typed or pasted by the user (Explorer's "Copy as path" adds quotes): an existing file named like the game's log.
pub fn validate(input: &str) -> Result<PathBuf, String> {
    if input.len() > 520 {
        return Err("That path is too long".into());
    }
    let p = PathBuf::from(input.trim().trim_matches('"'));
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !name.to_ascii_lowercase().ends_with("client.txt") {
        return Err("That isn't a Client.txt file: it is in the logs folder of the game".into());
    }
    if !p.is_file() {
        return Err("That file doesn't exist".into());
    }
    Ok(p)
}

pub fn save(config_dir: &Path, log: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(config_dir.join(SETTING_FILE), log.to_string_lossy().as_bytes())
}

/// Install folders worth checking on this machine, most specific first.
#[cfg(windows)]
pub fn install_roots() -> Vec<(Source, PathBuf)> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    let mut roots = Vec::new();
    if let Some(dir) = crate::poe_window::install_dir() {
        roots.push((Source::RunningGame, dir));
    }
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        if let Ok(k) = RegKey::predef(hive).open_subkey(r"Software\GrindingGearGames\Path of Exile") {
            if let Ok(v) = k.get_value::<String, _>("InstallLocation") {
                roots.push((Source::Registry, PathBuf::from(v)));
            }
        }
    }
    let steam = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .and_then(|k| k.get_value::<String, _>("SteamPath"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files (x86)\Steam"));
    let mut libs = vec![steam.clone()];
    if let Ok(vdf) = std::fs::read_to_string(steam.join(r"steamapps\libraryfolders.vdf")) {
        libs.extend(steam_library_paths(&vdf));
    }
    for lib in libs {
        roots.push((Source::Steam, lib.join(r"steamapps\common\Path of Exile")));
    }
    roots.push((Source::DefaultFolder, PathBuf::from(r"C:\Program Files (x86)\Grinding Gear Games\Path of Exile")));
    roots
}

#[cfg(not(windows))]
pub fn install_roots() -> Vec<(Source, PathBuf)> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_install(base: &Path, name: &str) -> PathBuf {
        let dir = base.join(name);
        std::fs::create_dir_all(dir.join("logs")).unwrap();
        std::fs::write(dir.join("logs").join("Client.txt"), "").unwrap();
        dir
    }

    #[test]
    fn a_pasted_path_must_be_an_existing_client_log() {
        let t = tempfile::tempdir().unwrap();
        let dir = fake_install(t.path(), "poe");
        let log = dir.join("logs").join("Client.txt");
        let quoted = format!("  \"{}\"  ", log.display());
        assert_eq!(validate(&quoted).unwrap(), log);
        assert!(validate(&dir.join("logs").join("Other.txt").display().to_string()).is_err());
        assert!(validate(&dir.join("logs").join("Missing-Client.txt").display().to_string()).unwrap_err().contains("doesn't exist"));
        assert!(validate("").is_err());
        assert!(validate(&"a".repeat(600)).is_err());
        // the saved path is what the watcher uses next
        save(t.path(), &log).unwrap();
        assert_eq!(load_saved(t.path()), Some(log));
    }

    #[test]
    fn first_existing_install_wins_in_order() {
        let t = tempfile::tempdir().unwrap();
        let missing = t.path().join("nope");
        let steam = fake_install(t.path(), "steam");
        let default = fake_install(t.path(), "default");
        let got = locate(None, vec![(Source::Registry, missing), (Source::Steam, steam.clone()), (Source::DefaultFolder, default)]).unwrap();
        assert_eq!(got, Located { path: steam.join("logs").join("Client.txt"), source: Source::Steam });
    }

    #[test]
    fn saved_choice_beats_detection_but_only_if_it_still_exists() {
        let t = tempfile::tempdir().unwrap();
        let install = fake_install(t.path(), "a");
        let saved = fake_install(t.path(), "b").join("logs").join("Client.txt");
        let roots = vec![(Source::Steam, install.clone())];
        assert_eq!(locate(Some(saved.clone()), roots.clone()).unwrap().source, Source::Saved);
        assert_eq!(locate(Some(t.path().join("gone.txt")), roots).unwrap().source, Source::Steam);
    }

    #[test]
    fn not_found_is_none() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(locate(None, vec![(Source::Steam, t.path().to_path_buf())]), None);
    }

    #[test]
    fn parses_steam_library_vdf() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t\t\"label\"\t\t\"\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}";
        assert_eq!(
            steam_library_paths(vdf),
            vec![PathBuf::from(r"C:\Program Files (x86)\Steam"), PathBuf::from(r"D:\SteamLibrary")]
        );
    }

    #[test]
    fn saved_setting_roundtrips() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(load_saved(t.path()), None);
        let p = PathBuf::from(r"D:\Games\PoE\logs\Client.txt");
        save(&t.path().join("cfg"), &p).unwrap();
        assert_eq!(load_saved(&t.path().join("cfg")), Some(p));
    }
}
