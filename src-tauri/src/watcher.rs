//! Background loop: locate Client.txt, tail it, parse events, feed the session, publish a snapshot to the UI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::events::{Parser, State};
use crate::locator;
use crate::session::{self, Views};
use crate::tailer::{recent_lines, Tailer};

/// True while the game is focused (or not running at all, for development). The tailer sleeps otherwise.
pub static ACTIVE: AtomicBool = AtomicBool::new(true);

#[derive(Serialize, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub state: State,
    /// "searching" | "ok" | "not-found" | "error"
    pub log: String,
    pub views: Views,
}

static SNAP: LazyLock<Mutex<Snapshot>> = LazyLock::new(|| Mutex::new(Snapshot { log: "searching".into(), ..Default::default() }));

#[tauri::command]
pub fn snapshot() -> Snapshot {
    let s = SNAP.lock().unwrap().clone();
    Snapshot { views: session::views(&s.state), ..s }
}

fn publish(app: &AppHandle, state: &State, log: &str) {
    let next = Snapshot { state: state.clone(), log: log.into(), views: session::views(state) };
    let mut cur = SNAP.lock().unwrap();
    if *cur != next {
        *cur = next.clone();
        let _ = app.emit("snapshot", next);
    }
}

/// Re-publish after the session changed outside the log (import, manual step, manual stage).
pub fn refresh(app: &AppHandle) {
    let (state, log) = {
        let s = SNAP.lock().unwrap();
        (s.state.clone(), s.log.clone())
    };
    publish(app, &state, &log);
}

pub fn current_level() -> u32 {
    SNAP.lock().unwrap().state.level
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let cfg = app.path().app_config_dir().unwrap_or_default();
        let mut parser = Parser::default();

        let path = loop {
            if let Some(found) = locator::locate(locator::load_saved(&cfg), locator::install_roots()) {
                break found.path;
            }
            publish(&app, &parser.state, "not-found");
            std::thread::sleep(Duration::from_secs(5));
        };

        // learn the current zone/level from the end of the existing log; those events are not replayed into the route
        for line in recent_lines(&path, 256 * 1024) {
            parser.feed(&line);
        }
        session::sync_to_current_area(&parser.state);
        let mut tail = Tailer::new(path);
        publish(&app, &parser.state, "ok");

        loop {
            std::thread::sleep(Duration::from_millis(250));
            if !ACTIVE.load(Ordering::Relaxed) {
                continue;
            }
            match tail.poll() {
                Ok(lines) => {
                    let events: Vec<_> = lines.iter().flat_map(|l| parser.feed(l)).collect();
                    if !events.is_empty() {
                        session::on_events(&events, &parser.state);
                    }
                    publish(&app, &parser.state, "ok");
                }
                Err(_) => publish(&app, &parser.state, "error"),
            }
        }
    });
}
