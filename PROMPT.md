# MASTER PROMPT — "PoeMate": a top-tier Path of Exile 1 build-guide overlay (Windows first)

You are a senior systems + UI engineer. Build a production-quality, GGG-policy-compliant, visually beautiful, extremely fast
in-game overlay for **Path of Exile 1** on **Windows 10/11**. It reads a **Path of Building (PoB) build** and, while the player
levels through the campaign (and later maps), subtly shows: where to go next (zone route / quest / waypoint / town tasks),
which passive nodes to take, which gems to link/buy, which gear to aim for, and level-based reminders. Think "Discord overlay":
translucent, minimal, unobtrusive, click-through by default, gorgeous typography and motion.

Work in phases. After each phase: build, run, self-test, summarize, and wait for nothing — continue to the next phase unless blocked.
Ask me only for decisions that are genuinely mine. Keep dependencies few. Prefer the simplest thing that fully works.

---------------------------------------------------------------------------------------------------
## 0. HARD COMPLIANCE RULES (non-negotiable — GGG Third-Party Policy)

Before coding, open and read the CURRENT policy (pathofexile.com/developer/docs -> "Policy on Third-Party Software" and the
forum "Third Party Policy" thread) and write `docs/COMPLIANCE.md` quoting the relevant rules with the date you read them.
Design so that every rule below is satisfied BY CONSTRUCTION:

ALLOWED approach (what we do):
- A separate, standalone top-level window drawn above the game (no hooking, no injection).
- Reading the game's own log file `<PoE install>\logs\Client.txt` (read-only, tail). GGG tolerates this; it is how Exile Leveling,
  Exile UI, etc. work.
- Official, documented web APIs (pathofexile.com/developer/docs, OAuth where required) with proper User-Agent + rate-limit respect
  if/when we add trade/character features. Public static data (skill tree export from github.com/grindinggear/skilltree-export).
- User-triggered actions only. Clipboard/screen may be read only once per manual key press; log reading requires the user to know what we do with it. We send no input to the game at all (GGG macro rules: manual, one function, one action).
- Show "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." in README, installer and settings.

FORBIDDEN (never implement, never "just for testing"):
- Reading/writing game process memory, DLL injection, hooking DirectX/Vulkan present, packet inspection/modification.
- Modifying game files (only GGG-sanctioned loot filters are OK, and we don't touch them).
- Automating gameplay: no auto-clicking, no macros chaining multiple actions, no auto-moving, no auto-flask/auto-trade-buy,
  no sending input to the game window on the player's behalf beyond what the user explicitly presses per key.
- Pixel-reading to *drive decisions that act in-game* (screen capture for OCR of OUR OWN overlay data is a grey area -> OFF by default,
  not in MVP; log-based detection is sufficient).
- Anything giving an unfair automated advantage (no map-layout/minimap hacks, no revealing hidden info).

Consequences for architecture: overlay = passive display + hotkeys that only toggle OUR UI. Add a visible "Compliance" panel in
settings stating exactly what the app does and doesn't do. Add a kill-switch hotkey that hides everything instantly.

Overlay visibility prerequisite: PoE must run in **Windowed** or **Windowed Fullscreen (borderless)**. In true exclusive fullscreen
a normal topmost window cannot show. Detect the PoE window (class `POEWindowClass`, title "Path of Exile") and warn the user
if it's exclusive fullscreen / not found. Support Steam and standalone client installs (auto-locate Client.txt via registry,
Steam library folders, running-process path, then manual picker).

---------------------------------------------------------------------------------------------------
## 1. TECH STACK (fast + small + beautiful)

Default decision (justify briefly in docs/ARCHITECTURE.md if you deviate):
- **Tauri 2 (Rust backend + WebView2 frontend)** — ~10 MB installer, ~30-60 MB RAM idle, native Win32 control. NOT Electron.
- Rust crates: `tauri`, `windows` (Win32: SetWindowLongPtrW, WS_EX_*, RegisterHotKey, SetWinEventHook for foreground tracking),
  `notify` or manual tail with `tokio` for Client.txt, `serde`, `quick-xml` (or `roxmltree`), `flate2`/`miniz_oxide`, `base64`,
  `reqwest` (rustls) for pobb.in / pastebin / poe.ninja fetch, `directories`, `tracing`. Keep the list short.
- Frontend: **Svelte 5 or SolidJS + TypeScript + Vite**, hand-written CSS (CSS variables, `backdrop-filter`, `will-change`), no heavy UI kit.
  Animations via CSS/WAAPI (and `motion` only if truly needed). Icons: inline SVG sprite. Fonts: Inter/Geist + Fontin-like serif accent
  for the PoE vibe (bundle locally, no CDN).
- Tests: `cargo test` for parsers/state machine, Vitest for frontend logic, a fixture folder of real PoB codes + Client.txt excerpts.
- Packaging: `tauri build` -> signed-ready NSIS installer + portable zip. Auto-update optional (later).

Overlay window specifics (Win32, implement exactly):
- Transparent, frameless, `alwaysOnTop`, `skipTaskbar`, no shadow, no focus steal.
- Extended styles: `WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`, plus `WS_EX_TRANSPARENT` for click-through in "Play mode".
- Two modes toggled by global hotkey (default `Ctrl+Shift+O`, configurable):
  - **Play mode**: fully click-through, never steals focus/keyboard, auto-fades chrome, only shows needed info.
  - **Edit mode**: removes WS_EX_TRANSPARENT so you can drag/resize/scale widgets, open settings, paste PoB. Snap-to-edge + grid, per-monitor DPI aware.
- Auto-show only when the PoE window is foreground (SetWinEventHook EVENT_SYSTEM_FOREGROUND); hide/pause when alt-tabbed
  (also pause log parsing cost-free). Follow the PoE window rect for borderless on any monitor; handle multi-monitor + mixed DPI.
- Global hotkeys via `RegisterHotKey` (no low-level keyboard hooks, no keylogging). Never send keystrokes to the game.
- Rendering budget: overlay must idle at <1% CPU, <80 MB RAM, zero measurable FPS impact on the game. Redraw only on state change;
  no always-running animation loops; pause everything when PoE not foreground.

---------------------------------------------------------------------------------------------------
## 2. INPUT: THE BUILD (PoB)

Accept all of:
1. PoB **import code** pasted/clipboard (base64url -> replace `-`->`+`, `_`->`/` -> zlib inflate -> XML).
2. **pobb.in** link (fetch raw: `https://pobb.in/<id>/raw`), **pastebin.com** link (`/raw/<id>`), poe.ninja build link where feasible,
   maroon/other PoB hosts as pluggable adapters.
3. Local `.xml` PoB build file (drag & drop).
Clipboard import ONLY via an explicit hotkey/button, one clipboard read per press (GGG overlay rule) — never watch/poll the clipboard.

Parse (robustly, version-tolerant, fuzz-tested on malformed input — this is a trust boundary: size-cap inflate output, no XXE, timeouts):
- `<Build>`: class, ascendancy, level, main skill group, bandit choice, pantheon.
- `<Tree><Spec>` (multiple specs = **leveling progression**; titles like "Act 3", "Lvl 40", "Maps" -> treat as stages): `nodes`, `masteryEffects`,
  `treeVersion`, `classId`/`ascendClassId`, cluster jewel data, `<Sockets>`. Resolve node ids -> names/stats/positions using the matching
  skilltree-export data version (cache locally, version-keyed). Compute **diff between consecutive specs** = "nodes to allocate next".
- `<Skills>` with SkillSets: gem groups, slots, gem names/levels/quality/support links, enabled flags. Produce "buy/link now" and "swap at level X".
- `<Items>` + `<ItemSets>`: each slot's item (rarity, base, mods/implicit/influence/enchants, uniques), flag which are **leveling items vs end-game**.
- `<Notes>`: show free-form guide notes per stage (render safely as text/markdown, strip control codes).
- Build **Leveling Plan** model: timeline keyed by character level & act: {passives to take, gems to get/link, gear targets,
  pantheon/bandit reminders, notes}. If the PoB has only one spec, derive a sensible ordering (path from class start toward the final
  tree, prioritizing nodes by proximity/keystone/notables) and clearly label it "auto-generated order".

---------------------------------------------------------------------------------------------------
## 3. CAMPAIGN GUIDANCE (route + quests)

- Data: maintain a versioned, human-editable **route dataset** (JSON/YAML) for the full campaign (all acts incl. Act 10 / current content):
  per area: area id (`1_1_town`, `1_1_1`, ...), name, act, recommended level, ordered steps (e.g. "Kill Hillock, take Medicine Chest quest",
  "Waypoint -> The Southern Forest", "Use portal", "Take Quest reward: Skill Point / gem"), optional simple map hints (direction words like
  "exit is usually left/west" / "follow road"), town tasks (talk to Nessa for reward gem, vendor recipe), side areas to skip/do, lab/trial notes,
  bandit decision step with the PoB bandit shown. Reference/seed from the open-source **Exile Leveling** (HeartofPhos, check license) and
  community route data — respect licenses, credit them in `docs/CREDITS.md`, don't copy unlicensed text.
- Quest-reward intelligence: for each quest reward, tell the player which gem from their PoB/class they should pick or buy
  (vendor availability per class/act). Show "Take now / buy at vendor / not available yet" using the gem list from the PoB.
- Render route as a compact **step list with check-offs** (auto-advanced by log events, manually tickable via hotkeys), current step
  highlighted, next 2-3 steps dimmed. Optional tiny "next area" arrow/glyph. No map overlay drawing over the game (keep it a calm list/card).
- Support custom per-user routes (import/export JSON), plus "league start / speedrun / safe" route variants.

---------------------------------------------------------------------------------------------------
## 4. GAME-STATE DETECTION VIA Client.txt (the heartbeat)

Tail `Client.txt` (open with shared read, seek to end at start, handle rotation/truncation, UTF-8/BOM, partial lines). Parse with
precompiled regex/state machine (unit-tested against fixtures):
- Area enter: `Generating level N area "AREA_ID" with seed S` -> current area id + area level (gives zone reliably).
- `: You have entered <Area Name>.` -> display name, town vs zone.
- `: <Character> (<Class>) is now level N` -> level-up triggers plan update (new nodes/gems/notes for level N).
- Death, `has been slain`, instance/region lines, trade/whisper lines -> ignored except optional death counter (local only; never log private chat).
- Don't store or transmit chat content. Privacy: everything local, no telemetry by default.
State machine emits events: `AreaChanged`, `LevelChanged`, `QuestStepCompleted?` (heuristic from area transitions/known kill messages),
`CharacterDetected`. Persist per-character progress (current act/step, checked steps, selected build) in a local SQLite/JSON file.
Multi-character: switch build/progress per character name.

---------------------------------------------------------------------------------------------------
## 5. THE UI / UX — "subtle, beautiful, Discord-overlay-like"

Design principles: the game is the hero. Overlay is glanceable (<1 second to read), minimal text, strong hierarchy, zero clutter.
- Visual language: dark glass cards (`rgba(14,15,20,.55)` + `backdrop-filter: blur(14px) saturate(1.2)`), 1px inner hairline border,
  soft shadow, 10-14px radius, accent color derived from the build's ascendancy/main-skill element (fire/cold/lightning/chaos/phys)
  — subtle accent glow only on the active step. Adjustable global opacity (20-100%), scale (70-150%), font size, density (compact/cozy).
- Typography: clean sans for body (Inter/Geist), small-caps/serif touch for headers (PoE feel), tabular numerals for levels.
  Contrast checked (WCAG AA) on varied game backgrounds; text-shadow/outline fallback for bright scenes; optional colorblind-safe palette.
- Widgets (each independent, draggable in Edit mode, pinned positions saved per monitor/resolution):
  1. **Route card**: current zone name + act + level badge; current step big; next steps small; check-off animation; "town tasks" chip.
  2. **Passive card**: "Take next: <node name> (+stat)" with tiny node-type icon (notable/keystone/mastery/jewel/ascendancy), count remaining,
     optional **mini tree preview** (pre-rendered minimap crop with the next nodes highlighted — lightweight canvas/SVG, lazy).
  3. **Gems card**: current links as colored socket pips (R/G/B), "get at vendor / quest reward / swap at lvl X", support gem order.
  4. **Gear card**: what to look for now (slot, base, key mods/prices/links), uniques to buy, upgrade-from-leveling-gear reminders; copy-to-clipboard
     search string button in Edit mode (user-initiated only).
  5. **Notes/Reminders**: build notes, pantheon, bandits, lab/ascendancy trial reminders; level-based toasts with gentle slide/fade (auto-dismiss 6s,
     queued, never stacking more than 2).
  6. **Mini HUD bar**: tiny pill (level • area • step N/M) that's all you see when idle; expands on hover in Edit mode or hotkey.
- Motion: 120-180 ms ease-out transitions, spring for card entrance, no layout thrash; respect `prefers-reduced-motion`.
- Hotkeys (all rebindable, conflict-checked against PoE defaults): toggle overlay, toggle edit mode, next/prev step, hide everything (panic),
  cycle widget density, show/hide mini tree. No hotkey may send input to PoE.
- Onboarding: first-run wizard (find Client.txt -> detect display mode -> import build -> pick route variant -> position widgets with live preview)
  in <60 seconds. Tray icon with quick menu. Dark theme only for MVP; themes later.
- Localization-ready (i18n keys), start with English + Dutch.

---------------------------------------------------------------------------------------------------
## 6. PERFORMANCE & QUALITY BARS (measure, don't assume)

- Cold start < 1.5 s; build import + parse < 300 ms for a typical PoB; Client.txt event -> UI update < 50 ms.
- Idle CPU < 1%, RAM < 80 MB (WebView2 included target; report actual numbers). Verify with a profiling pass and include results in `docs/PERF.md`.
- Passive tree data cached on disk; lazy-load mini tree; no network calls in the hot path; network only on user action (import/update) with timeouts + offline fallback.
- Robustness: malformed PoB, huge files, log rotation, game crash/relaunch, PoE patch changing log format (fail soft, show "can't read log" status), clock/locale differences.
- Security: no `eval`, strict CSP, Tauri allowlist minimal (no broad fs/shell), validate every external input, never execute downloaded content, size/time caps, no secrets in logs.
- Accessibility: keyboard-operable settings, screen-reader labels in settings UI (overlay itself is display-only).

---------------------------------------------------------------------------------------------------
## 7. PROJECT STRUCTURE & DELIVERABLES

```
poemate/
  src-tauri/            Rust: window/Win32, log tailer, pob parser, plan engine, storage
  src/                  Svelte/Solid UI: widgets, settings, onboarding
  data/                 route dataset, gem/vendor tables, tree export cache (versioned)
  fixtures/             sample PoB codes, XML, Client.txt excerpts
  docs/                 COMPLIANCE.md, ARCHITECTURE.md, DATA_FORMAT.md, CREDITS.md, PERF.md
  README.md
```
Phases (do in order, each ends runnable):
1. Scaffold Tauri app + transparent click-through topmost overlay + edit/play toggle + foreground tracking + hotkeys + tray. Demo card.
2. Client.txt locator + tailer + event state machine + tests (fixtures). Show live current area/level in mini HUD.
3. PoB importer (code / pobb.in / pastebin / file) -> typed Build model + tree data resolver + spec diffing + tests.
4. Plan engine: level/act-keyed Leveling Plan (passives, gems, gear, notes) + auto-order fallback.
5. Route dataset + route card + step auto-advance + manual override + persistence per character.
6. Full widget set + polish: motion, theming by element, density, DPI/multi-monitor, onboarding wizard, settings, i18n (EN/NL).
7. Perf pass, security pass, installer, docs, compliance re-check, README with GIFs/screens.
Post-MVP backlog (don't build now, list in README): map-mod warnings, poe.ninja price hints, trade-search string helper (user-triggered, official API),
atlas tree guidance, Linux/Steam Deck, overlay themes, community route/build sharing.

## 8. WORKING STYLE
- Smallest complete implementation per phase; no speculative abstraction; match existing code style; comments only for non-obvious "why".
- Every non-trivial parser/state-machine has tests with real-world fixtures. Verify by actually running the app and checking the overlay
  over a borderless-window PoE (or a test window if PoE isn't installed) — report what you verified and what you could not.
- When data is missing or a rule is uncertain, say so explicitly and pick the compliant/safer default.
- Finish each phase with: what works, what's skipped, known risks.
