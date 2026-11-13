# Implementation Plan: PoeMate (PoE1 build-guide overlay, Windows)

Source: `PROMPT.md`. Task details + checkboxes: `tasks/todo.md` (default task list target).

## Overview
Standalone Tauri 2 overlay window (click-through, topmost) that reads a PoB build and `Client.txt`, and shows the next
campaign step, passive nodes, gems, gear and reminders by character level/zone. Compliance with GGG's third-party policy
is a design constraint: passive display only, no hooks/injection/memory reads/input automation.

## Architecture decisions
- **Tauri 2 + Svelte 5 + TS**, not Electron: small footprint, native Win32 access; Rust owns window, log tail, PoB parsing, plan engine, storage.
- **Core logic is pure Rust modules with fixtures** (log parser, PoB decode/parse, plan engine, route engine). UI only renders state -> logic testable without the game.
- **Event-driven**: Client.txt tail -> state machine -> Tauri events -> UI. No polling renders; foreground check is cheap and pauses everything when PoE isn't focused.
- **Vertical slices**: each phase ends with something runnable (overlay shell -> live HUD -> build loaded -> route running -> full UI).
- **Data in versioned JSON** (`data/`): route dataset, vendor/gem tables, tree export cache keyed by treeVersion. Human-editable.
- **Persistence**: plain JSON per character in app data dir (SQLite only if JSON proves insufficient).
- Win32 via `windows` crate; hotkeys via `RegisterHotKey` (tauri global-shortcut); click-through via `WS_EX_TRANSPARENT` toggle.

## Phases (see todo.md for tasks)
0. Compliance groundwork (T1)
1. Overlay shell (T2-T4) — **checkpoint 1**
2. Client.txt heartbeat (T5-T8) — **checkpoint 2**
3. PoB import + tree data (T9-T12) — **checkpoint 3**
4. Plan engine (T13-T14)
5. Route + persistence (T15-T18) — **checkpoint 4: end-to-end MVP loop**
6. Widgets + polish (T19-T28) — **checkpoint 5**
7. Hardening + release (T29-T31) — **checkpoint 6**

## Risks and mitigations
| Risk | Impact | Mitigation |
|---|---|---|
| GGG policy changed / overlay deemed non-compliant | High | T1 reads current policy first; no memory/hook/input code ever; panel in settings states what app does; re-check in T31 |
| Exclusive fullscreen hides overlay | High | Detect + warn; document Windowed/Borderless requirement |
| Route dataset is large manual work, license unclear (Exile Leveling) | High | T15 verifies license before copying; ship Act 1-3 first, remaining acts as data-only tasks; keep schema simple |
| Auto-ordering passives for single-spec PoB gives bad paths | Med | Isolated in T14, labelled "auto-generated", stage-based specs preferred |
| Client.txt format/locale changes | Med | Fail soft with visible "can't read log" status; fixtures for each line type |
| Mixed-DPI / multi-monitor positioning bugs | Med | Physical-pixel math in T4; manual test matrix in checkpoint 1 and 5 |
| WebView2 RAM/CPU over budget | Med | Measure early (checkpoint 1) not only in T29; no animation loops |
| Tree data size/version mismatch | Med | T12 version-keyed cache, fall back to nearest version with warning |
| PoB input is untrusted (zip bomb, XXE) | Med | Size-capped inflate, no DTD/entities, timeouts (T9, T30) |

## Open questions (human decisions)
- Final app name: PROMPT now says "PoeMate", repo folder is "PoeOverlya" — which to use for product/identifier?
- Include current-league content (Act 10 / newest campaign) in MVP routes, or only acts 1-10 of the classic campaign?
- Is use of Exile Leveling data acceptable if license permits (attribution), or must the route be written from scratch?
- Dutch UI strings: machine-draft OK or hand-written?
- Distribution: GitHub releases only, or also a public page? (affects signing, T31)
