use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

mod events;
mod hotkeys;
mod layout;
mod locator;
mod poe_window;
mod plan;
mod points;
mod pob;
mod pob_model;
mod pob_source;
mod rewards;
mod route;
mod route_engine;
mod session;
mod tailer;
mod tree;
mod watcher;

static EDIT: AtomicBool = AtomicBool::new(false);
static HIDDEN: AtomicBool = AtomicBool::new(false);

/// Play mode = click-through and never takes focus; edit mode = interactive.
fn set_edit(w: &WebviewWindow, on: bool) {
    EDIT.store(on, Ordering::Relaxed);
    let _ = w.set_ignore_cursor_events(!on);
    let _ = w.set_focusable(on);
    let _ = w.emit("mode", on);
}

fn toggle_edit(w: &WebviewWindow) {
    if !HIDDEN.load(Ordering::Relaxed) {
        set_edit(w, !EDIT.load(Ordering::Relaxed));
    }
}

/// Kill switch: hides everything and drops back to play mode.
fn toggle_hidden(w: &WebviewWindow) {
    if HIDDEN.fetch_xor(true, Ordering::Relaxed) {
        let _ = w.show();
    } else {
        set_edit(w, false);
        let _ = w.hide();
    }
}

#[tauri::command]
async fn import_build(app: tauri::AppHandle, input: String) -> Result<session::ImportSummary, String> {
    // blocking network and parsing work: keep it off the async runtime threads
    let out = tauri::async_runtime::spawn_blocking(move || session::import(&input)).await.map_err(|e| e.to_string())??;
    watcher::refresh(&app);
    Ok(out)
}

#[tauri::command]
fn layout_load(app: tauri::AppHandle) -> layout::Layout {
    layout::load(&app.path().app_config_dir().unwrap_or_default())
}

#[tauri::command]
fn layout_save(app: tauri::AppHandle, layout: layout::Layout) -> Result<(), String> {
    layout::save(&app.path().app_config_dir().unwrap_or_default(), layout).map_err(|e| e.to_string())
}

#[tauri::command]
fn hotkeys_get(app: tauri::AppHandle) -> Vec<hotkeys::Binding> {
    hotkeys::view(&hotkeys::load(&app.path().app_config_dir().unwrap_or_default()))
}

#[tauri::command]
fn hotkeys_set(app: tauri::AppHandle, id: String, keys: String) -> Result<Vec<hotkeys::Binding>, String> {
    let dir = app.path().app_config_dir().unwrap_or_default();
    let mut map = hotkeys::load(&dir);
    let Some(slot) = hotkeys::ACTIONS.iter().find(|a| a.0 == id) else { return Err("Unknown action".into()) };
    let others = map.iter().filter(|(k, _)| **k != slot.0).map(|(k, v)| (*k, v.clone())).collect();
    let new = hotkeys::check(&others, &keys)?;
    let gs = app.global_shortcut();
    let old: Shortcut = map[slot.0].parse().map_err(|_| "Stored hotkey is invalid".to_string())?;
    if new != old {
        // the OS refuses keys another app owns; the old binding stays until this one is in
        gs.register(new).map_err(|_| "Another app already uses that key".to_string())?;
        let _ = gs.unregister(old);
    }
    map.insert(slot.0, keys);
    hotkeys::save(&dir, &map).map_err(|e| e.to_string())?;
    hotkeys::set_bound(map.iter().filter_map(|(k, v)| Some((*k, v.parse().ok()?))).collect());
    Ok(hotkeys::view(&map))
}

#[tauri::command]
fn quest_mode(app: tauri::AppHandle, all_quests: bool) {
    session::set_all_quests(all_quests);
    watcher::refresh(&app);
}

#[tauri::command]
fn route_step(app: tauri::AppHandle, delta: i32) {
    session::step(delta);
    watcher::refresh(&app);
}

#[tauri::command]
fn plan_stage(app: tauri::AppHandle, delta: i32) {
    session::stage(delta, watcher::current_level());
    watcher::refresh(&app);
}

/// shortcut: polls every 400ms instead of SetWinEventHook; switch if profiling shows idle CPU.
fn spawn_tracker(w: WebviewWindow) {
    std::thread::spawn(move || {
        let ours = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        // development aid (screenshots, tests): keep the overlay visible even when the game isn't focused
        let always_show = std::env::var_os("POEMATE_ALWAYS_SHOW").is_some();
        let mut visible = None;
        let mut found = None;
        let mut rect = None;
        loop {
            std::thread::sleep(Duration::from_millis(400));
            let poe = poe_window::probe();
            watcher::ACTIVE.store(
                always_show || poe.as_ref().is_none_or(|p| p.foreground || p.foreground_is(ours)),
                Ordering::Relaxed,
            );
            let want = !HIDDEN.load(Ordering::Relaxed)
                && (always_show
                    || match &poe {
                    // no game running: stay visible so the overlay can be developed and demoed
                    None => true,
                    Some(p) => p.foreground || p.foreground_is(ours) || EDIT.load(Ordering::Relaxed),
                });
            if visible != Some(want) {
                visible = Some(want);
                let _ = if want { w.show() } else { w.hide() };
            }
            if found != Some(poe.is_some()) {
                found = Some(poe.is_some());
                let _ = w.emit("poe-found", poe.is_some());
            }
            if let Some(p) = poe {
                if rect != Some(p.rect) && p.rect.2 > 0 && p.rect.3 > 0 {
                    rect = Some(p.rect);
                    let (x, y, cw, ch) = p.rect;
                    let _ = w.set_position(PhysicalPosition::new(x, y));
                    let _ = w.set_size(PhysicalSize::new(cw, ch));
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, sc, ev| {
                    if ev.state() != ShortcutState::Pressed {
                        return;
                    }
                    // the step/stage/panel keys only move our own UI; nothing is sent to the game
                    match hotkeys::action_of(sc) {
                        Some("step_next") => session::step(1),
                        Some("step_prev") => session::step(-1),
                        Some("stage_next") => session::stage(1, watcher::current_level()),
                        Some("stage_prev") => session::stage(-1, watcher::current_level()),
                        Some("panel") => {
                            let _ = app.emit("passive-panel", ());
                            return;
                        }
                        Some(id @ ("edit" | "hide")) => {
                            if let Some(w) = app.get_webview_window("main") {
                                if id == "edit" { toggle_edit(&w) } else { toggle_hidden(&w) }
                            }
                            return;
                        }
                        _ => return,
                    }
                    watcher::refresh(app);
                })
                .build(),
        )
        .setup(move |app| {
            let w = app.get_webview_window("main").expect("main window");
            // demo fallback: cover the primary monitor until the game window is found
            if let Ok(Some(m)) = w.primary_monitor() {
                let _ = w.set_position(*m.position());
                let _ = w.set_size(*m.size());
            }
            set_edit(&w, false);
            let mut bound = Vec::new();
            for (id, keys) in hotkeys::load(&app.path().app_config_dir().unwrap_or_default()) {
                let Ok(sc) = keys.parse::<Shortcut>() else { continue };
                // a key another app owns just stays unbound; it can be changed in edit mode
                if app.global_shortcut().register(sc).is_ok() {
                    bound.push((id, sc));
                }
            }
            hotkeys::set_bound(bound);

            let edit = MenuItem::with_id(app, "edit", "Toggle edit mode (Ctrl+Shift+O)", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide / show overlay (Ctrl+Shift+H)", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("window icon").clone())
                .menu(&Menu::with_items(app, &[&edit, &hide, &quit])?)
                .on_menu_event(|app, ev| {
                    let Some(w) = app.get_webview_window("main") else { return };
                    match ev.id.as_ref() {
                        "edit" => toggle_edit(&w),
                        "hide" => toggle_hidden(&w),
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .build(app)?;
            spawn_tracker(w);
            session::init(app.path().app_config_dir().unwrap_or_default());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                session::restore(); // may download the tree data on first run
                watcher::refresh(&handle);
            });
            watcher::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![watcher::snapshot, import_build, route_step, plan_stage, quest_mode, hotkeys_get, hotkeys_set, layout_load, layout_save])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
