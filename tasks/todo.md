# PoeMate — Task list
Sizes: XS 1 file · S 1-2 · M 3-5. Anything larger is split. Every task leaves the app building. "Verify" = focused command/manual check.
Commands assumed: `cargo test`, `npm run build`, `npm run tauri dev`.

## Phase 0 — Groundwork
- [x] **T1 Compliance doc (XS)** — Read CURRENT GGG third-party policy; write `docs/COMPLIANCE.md` with quotes + date + the app's allow/forbid list.
  - DONE 2026-11-13. Findings: clipboard/screen reads only once per manual key press (T10 changed); GGG disclaimer required; GGG API unused in MVP; forum policy thread not found, re-verify in T31.
  - AC: doc cites policy URL + read date; lists allowed (log read, own window, official API) and forbidden items; no code yet.
  - Verify: manual read-through. Deps: none.

## Phase 1 — Overlay shell
- [x] **T2 Scaffold (S)** (done 2026-11-13: `npm run build` + `cargo check` clean, dev window "PoeMate" opens) — Tauri 2 + Svelte 5 + TS project in repo root layout from PROMPT §7; demo glass card.
  - AC: `npm run tauri dev` opens a window with the card; `npm run build` + `cargo check` clean.
  - Files: `src-tauri/*`, `src/routes/+page.svelte`, config. Deps: none.
- [x] **T3 Overlay window + modes + hotkeys + tray (M)** (edit/play toggle verified via window styles; hide hotkey + tray menu NOT verified yet, test by hand) — Transparent/frameless/topmost/skip-taskbar; play = click-through + no focus, edit = interactive; `Ctrl+Shift+O` toggle, `Ctrl+Shift+H` panic-hide; tray with edit/quit.
  - AC: in play mode clicks pass to windows below and keyboard focus is never taken; edit mode shows dashed outline and is clickable; panic hides everything and a second press restores.
  - Verify: manual over Notepad/browser; hotkeys work while another app is focused. Deps: T2.
- [x] **T4 PoE window tracking (M)** (code + unit test for window/rect lookup pass; following a real PoE window, alt-tab hiding and CPU baseline still need a manual check) — Find `POEWindowClass`, follow client rect (physical px, multi-monitor/DPI), show only when PoE (or overlay) is foreground, dev fallback = cover primary monitor when no PoE; warn if not found / suspected exclusive fullscreen.
  - AC: overlay aligns with a windowed + borderless PoE; hides on alt-tab; idle CPU <1% measured; status event for "PoE not found".
  - Verify: manual with PoE (or test window with that class); Task Manager numbers recorded. Deps: T3.
- [ ] **Checkpoint 1** — builds clean; overlay verified over game/test window; CPU/RAM baseline noted in `docs/PERF.md`; review with human.

## Phase 2 — Client.txt heartbeat
- [x] **T5 Client.txt locator (S)** (5 tests green; manual picker UI comes with T28, saved path via `locator::save`) — Registry, Steam libraries, running-process path, manual picker fallback; persisted setting.
  - AC: returns path or typed "not found"; unit tests with fake directory trees. Verify: `cargo test locator`. Deps: T2.
- [x] **T6 Log tailer (M)** (5 tests green; the polling thread and "pause when PoE not foreground" are wired in T8) — Shared-read, seek-to-end start, partial lines, truncation/rotation, BOM; emits lines via channel; paused when PoE not foreground.
  - AC: tests append/truncate/rotate a temp file and receive exactly the new complete lines. Verify: `cargo test tailer`. Deps: T5.
- [x] **T7 Event parser + state machine (M)** (4 tests green; fixture is hand-written from the known format, NOT a real log: confirm against the real Client.txt in T8) — Regex for `Generating level N area "ID"`, `You have entered`, `is now level N`, slain; events `AreaChanged/LevelChanged/CharacterDetected`; chat/whisper lines dropped, never stored.
  - AC: real-format fixtures in `fixtures/client/` produce expected events; unknown lines ignored; no chat text retained.
  - Verify: `cargo test events`. Deps: T6.
- [x] **T8 Live mini HUD (S)** (verified against the real Client.txt on 2026-11-13: startup state correct; live update on zone change still to be seen in play; real format fix: area line is `[DEBUG Client]`) — Pill shows level • area • step placeholder; "can't read log" status state.
  - AC: appending lines to a real/fake Client.txt updates HUD <50 ms; error state visible. Verify: manual with scripted appender. Deps: T7, T3.
- [x] **Checkpoint 2** (confirmed by user on the real game: HUD shows the right zone, also after changing zone) — tests green; live HUD follows a real play session or replayed log; human review.

## Phase 3 — PoB import + tree data
- [x] **T9 PoB decode (S)** (3 tests green; tests use a self-made encoder, no real PoB code fixture yet: T10/T11 add real ones from pobb.in) — base64url -> zlib -> XML string with output size cap; clear error types.
  - AC: valid code decodes; garbage/oversized/zip-bomb input rejected without panic; fixtures `fixtures/pob/`. Verify: `cargo test pob_decode`. Deps: T2.
- [x] **T10 Fetchers + clipboard detect (M)** (7 tests green incl. a live pobb.in fetch of the user's build over HTTPS; real fixture `fixtures/pob/real1.txt`. Deviation: no clipboard hotkey, the future UI uses a paste box, which needs no clipboard read; drag/drop wiring comes in T28) — pobb.in, pastebin, local .xml (drag/drop); timeouts, size caps, only on user action; clipboard import only via explicit hotkey/button, one read per press (no clipboard watching — GGG overlay rule).
  - AC: each source returns decoded XML in tests (mock server) ; offline = friendly error. Verify: `cargo test fetch`. Deps: T9.
- [x] **T11 XML -> Build model (M)** (7 tests green on 3 real builds: Pathfinder, Elementalist with cluster jewels, Luminary with "lv 24+" style titles; a single-spec build is still missing) — Build (class/asc/level/bandit/pantheon), Specs (titles, nodes, masteries, treeVersion), Skills/SkillSets, Items/ItemSets, Notes; no DTD/entities; tolerant of missing fields.
  - AC: 3+ real PoB fixtures (single-spec, multi-spec leveling, cluster jewels) parse to expected summaries; malformed XML errors cleanly. Verify: `cargo test pob_model`. Deps: T9.
- [x] **T12 Tree data + resolver + spec diff (M)** (5 tests green. GGG's export only exists for the latest tree: older treeVersions are resolved against it; cluster jewel nodes (>65535) are reported as "Cluster jewel passive"; allocation order is reconstructed because PoB does not store it; not yet downloaded live: `tree::download` untested against the network) — Download/cache skilltree-export per treeVersion; node id -> name/type/stats; diff consecutive specs -> "take next" list.
  - AC: known fixture diff returns expected named nodes; missing version falls back with warning; cache reused offline. Verify: `cargo test tree`. Deps: T11.
- [ ] **Checkpoint 3** — pasting a PoB link yields a printed/loggable Build + stage diffs; human review.

## Phase 4 — Plan engine
- [x] **T13 Leveling plan from stages (M)** (4 tests green; stages without a level in the title (e.g. "ENDGAME SETUP") are never picked automatically, they need a manual switch) — Timeline keyed by level/act: passives (spec diffs), gems (skill sets), gear targets, bandit/pantheon, notes; query "what's relevant at level N".
  - AC: given fixture build, `plan.at(level)` returns expected passives/gems/notes; stage titles ("Act 3", "Lvl 40") parsed. Verify: `cargo test plan`. Deps: T11, T12.
- [x] **T14 Auto-order fallback (M)** (done together with T12's ordering: road-to-nearest-notable first; `Plan.auto_order` flag; property test on the Ranger's endgame tree; ignores quest passive points) — Single-spec builds: derive node order (path from class start, prioritize notables/keystones by distance), labelled "auto-generated".
  - AC: produced order is connected (each node reachable from earlier allocated set), covers all final nodes; label flag set. Verify: property-style test on fixtures. Deps: T13.

## Phase 5 — Route + persistence
- [x] **T15 Route dataset + license check (M)** (Exile Leveling is MIT, credited; all 10 acts used as is instead of a custom schema; 6 tests green; quests/gems tables come with T18) — Verify Exile Leveling license; define JSON schema (area id, steps, hints, town tasks, quest rewards, variants); seed Acts 1-3; `docs/DATA_FORMAT.md`, `docs/CREDITS.md`.
  - AC: schema validated by a test loading all data; license outcome recorded; Acts 1-3 complete. Verify: `cargo test route_data`. Deps: T7.
- [x] **T16 Route engine (M)** (6 tests green; towns only count when the enter step is within 4 steps, so a portal to town doesn't skip the route; replaying all enters walks acts 1-10) — Area events -> current/next steps, auto-advance, manual next/prev/check hotkeys (own UI only), variants.
  - AC: replay of an Act 1 fixture log advances steps correctly; manual override sticks; going back to town doesn't lose progress. Verify: `cargo test route_engine`. Deps: T15, T7.
- [x] **T17 Per-character persistence (S)** (4 session tests; build saved as build.txt, progress per character in progress/<name>.json, atomic writes, corrupt file ignored. Also wired the whole loop: watcher -> session -> snapshot -> route/passive/gem/gear cards, hotkeys Ctrl+Shift+arrows, import box in edit mode) — JSON per character: build, progress, checked steps, layout later; atomic writes.
  - AC: restart restores state; corrupt file falls back to defaults. Verify: `cargo test store`. Deps: T16, T11.
- [x] **T18 Quest-reward gem intelligence (M)** (5 tests; quest steps show "Take/Buy <gem> · <npc>" for gems of the current and next stage; the Ranger's real build gets Caustic Arrow, Sniper's Mark, Shrapnel Ballista from Tarkleigh and Burning Arrow from Nessa) — Map quest rewards/vendors by class+act to gems in the PoB: "take now / buy / not yet".
  - AC: fixture build + Act 1 rewards give correct recommendation; unknown gems handled. Verify: `cargo test rewards`. Deps: T15, T13.
- [x] **T19 Remaining acts data (M, data-only, repeatable)** (covered by T15: upstream already has acts 1-10) — Acts 4-10 (split per act if large).
  - AC: dataset test passes; each area has ≥1 step. Deps: T15.
- [~] **Checkpoint 4 (MVP loop)** (built and screenshot-verified with the user's Ranger build and a live tree download; the user still has to play a session with it) — import PoB -> play with Client.txt -> HUD + route card update + persistence survives restart; human demo.

## Phase 6 — Widgets & polish
- [x] **T20 Widget framework (M)** (layout.json per "<w>x<h>" key with 3 Rust tests; seeded layout verified in a screenshot over the live game: positions, scale 1.3, opacity and compact density applied. Dragging with the mouse, the wheel resize and the settings controls are NOT verified: need a hand test. The HUD pill is not movable yet. 2 monitors / 150% DPI not tested) — Draggable/scalable widgets in edit mode, snap/grid, positions per monitor+resolution, tokens for opacity/scale/density/accent.
  - AC: layout persists; works at 100%/150% DPI and 2 monitors. Verify: manual matrix. Deps: T3, T17.
- [ ] **T21 Route card + HUD polish (S)** — Step list, check-off animation, town-task chip, reduced-motion.
  - AC: matches PROMPT §5 spec; no animation loops when idle. Deps: T20, T16.
- [x] **T22a Passive point budget (added by the user's request)** — quest passive points table from the PoE wiki (23 points, +1 for killing all bandits; each quest counted once when its route step is passed), budget = level - 1 + quests; the passive card marks the points you can afford now ("2/12 now", the rest dimmed) and shows "+1 passive point" on quest steps. 5 points tests + 1 session test; verified in a screenshot over the live game (level 3 -> 2 points). It estimates from the plan: it can't know the points you really spent (needs screen reading).
- [x] **T22 Passive card (S)** (done 2026-11-13: Ctrl+Shift+T toggles a 560px two-column panel with all nodes of the stage, counts per node type and the stage list with the current one marked; stages switch with Ctrl+Shift+↑/↓. svelte-check + 71 tests green; the hotkey and layout are NOT verified by hand yet) — "Take next" list with node type icons + count, in the order to allocate them, grouped per build stage ("Normal Lab", "lv 24+" …). Own toggle hotkey (default Ctrl+Shift+T) shows a larger passive panel next to the game tree. The overlay cannot detect the P key / the open tree (would need screen or memory reading, forbidden) and does not draw on the game's tree (needs pixel alignment); the mini tree (T23) is a separate picture. Deps: T20, T13.
- [x] **T23 Mini tree preview (M)** (done 2026-11-13: SVG in the Ctrl+Shift+T panel, cropped around the points to take next; nodes get coordinates from the export's groups/orbits, each step knows the allocated neighbour it hangs off; earlier stages dim, affordable ones lit by type, the next one ringed. 72 tests + a positions test; seen once in a screenshot, zoom/pan not built, ascendancy nodes left out) — Lazy canvas/SVG crop with next nodes highlighted. Deps: T22, T12.
  - AC: opens <200 ms, adds no idle cost when hidden.
- [ ] **T24 Gems card (S)** — Socket pips, buy/reward/swap-at-level, support order. Deps: T20, T18.
- [ ] **T25 Gear card (S)** — Slot targets, uniques, leveling vs endgame; copy search string (user click only). Deps: T20, T13.
- [ ] **T26 Notes + toasts (S)** — Build notes, reminders, level-up toasts (max 2, 6 s). Deps: T20, T13.
- [ ] **T27 Settings + compliance panel + hotkey rebinding (M)** — Opacity/scale/density, hotkeys with conflict check, compliance text from T1.
  - AC: rebinding persists; conflicting binding rejected. Deps: T20, T1.
- [ ] **T28 Onboarding wizard + i18n EN/NL (M)** — Locate log -> display-mode check -> import build -> route variant -> position widgets in <60 s; i18n keys everywhere.
  - AC: fresh profile reaches working overlay in <60 s; language switch works. Deps: T27, T5, T10.
- [ ] **Checkpoint 5** — full-session manual playtest (campaign act 1-3), contrast on varied backgrounds, human review.

## Phase 7 — Hardening & release
- [ ] **T29 Performance pass (S)** — Measure cold start, import time, event->UI latency, idle CPU/RAM, FPS impact; results in `docs/PERF.md`; fix regressions.
  - AC: budgets from PROMPT §6 met or documented exceptions. Deps: Checkpoint 5.
- [ ] **T30 Security pass (S)** — CSP, minimal Tauri capabilities, fuzz PoB decode/parse, size/timeout caps, no secrets/chat in logs.
  - AC: fuzz run without panics; capability list reviewed. Deps: Checkpoint 5.
- [ ] **T31 Installer, README, compliance re-check (M)** — NSIS + portable zip, README with screenshots/GIFs, backlog list, re-read GGG policy and update `docs/COMPLIANCE.md`.
  - AC: clean-machine install runs; docs complete. Deps: T29, T30.
- [ ] **Checkpoint 6 (Release)** — all ACs met, human sign-off.

## Parallelizable
After T2: T5-T8 (log) and T9-T12 (PoB) are independent. T19 (route data) can run alongside Phase 6. T22-T26 widgets are independent once T20 exists.
