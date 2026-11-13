# PoeMate

A subtle in-game overlay for Path of Exile 1 (Windows). Load a Path of Building build and it shows, while you level:
where to go next (campaign route and quests), which passive points to take and in which order per build stage,
which gems to take or buy, and which gear to look for.

> This product isn't affiliated with or endorsed by Grinding Gear Games in any way.

## What it does and doesn't do
It runs as a separate window above the game and only reads `Client.txt` (zone and level lines). It never touches the game
process, never sends input to the game and never reads the screen or clipboard in the background. Details and the GGG
rules it is built on: `docs/COMPLIANCE.md`. The game must run in **Windowed** or **Windowed Fullscreen**.

## Run (development)
```
npm install
npm run tauri dev
```
Needs Rust (MSVC toolchain) and WebView2. Tests: `cd src-tauri && cargo test --lib`.
Set `POEMATE_ALWAYS_SHOW=1` to keep the overlay visible when the game is not focused (screenshots, development).

## Use
1. Press **Ctrl+Shift+O** (edit mode), paste a pobb.in / pastebin link or a PoB code, press Import. Press the hotkey again to lock.
2. Play. The route follows your zone changes; the passive and gem cards follow your level and the build's stages.

| Hotkey | Action |
|---|---|
| Ctrl+Shift+O | edit mode on/off (overlay is click-through otherwise) |
| Ctrl+Shift+H | hide / show everything |
| Ctrl+Shift+Right / Left | next / previous route step |
| Ctrl+Shift+Down / Up | next / previous build stage (for stages without a level in their title) |

Data is stored in `%APPDATA%\com.poemate.app` (`build.txt`, `skilltree.json`, `progress\<character>.json`).

## Credits
Campaign route and quest data: Exile Leveling by HeartofPhos (MIT). Passive tree data: Grinding Gear Games' public skill tree export.
See `docs/CREDITS.md`. Plan and task list: `tasks/`.
