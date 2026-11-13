# PoeMate

A subtle in-game overlay for Path of Exile 1 (Windows). Load a Path of Building build and it shows, while you level:
where to go next (campaign route and quests), which passive points to take and in which order per build stage,
which gems to take or buy, and which gear to look for.

> This product isn't affiliated with or endorsed by Grinding Gear Games in any way.

## What it does and doesn't do
It runs as a separate window above the game and only reads `Client.txt` (zone and level lines). It never touches the game
process, never sends input to the game and never reads the screen or clipboard in the background. Details and the GGG
rules it is built on: `docs/COMPLIANCE.md`. The game must run in **Windowed** or **Windowed Fullscreen**.

## Install
Build the installer with `npm run tauri build`; it ends up in `src-tauri/target/release/bundle/nsis/PoeMate_<version>_x64-setup.exe`
(per-user install, no admin rights). Needs WebView2, which Windows 11 has.

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
| Ctrl+Shift+T | passive panel (next points, mini tree, notes) |
| Ctrl+Shift+Right / Left | next / previous route step |
| Ctrl+Shift+Down / Up | next / previous build stage (for stages without a level in their title) |

All seven can be changed in Settings (edit mode, Keys tab).

Data is stored in `%APPDATA%\com.poemate.app`: `build.txt` (the imported build), `skilltree.json` (cached tree), `progress\<character>.json`, `layout.json` (widget positions and settings) and `hotkeys.json` (your key bindings).

## Credits
Campaign route, quest and gem data: Exile Leveling by HeartofPhos (MIT). Passive tree data: Grinding Gear Games' public skill tree export.
Build import format: Path of Building (Community). Quest passive points: the PoE wiki.
Sources, licenses and how to refresh the bundled data: `docs/CREDITS.md`; GGG rules and what PoeMate does with them: `docs/COMPLIANCE.md`. Plan and task list: `tasks/`.
