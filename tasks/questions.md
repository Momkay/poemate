# Open questions (for the user) — updated as work continues

Newest first.

## Please test in the real game (I can't do these headlessly)
Start with `npm run tauri dev` while PoE runs (Windowed/Borderless). Your Ranger/Pconc build (pobb.in/Ig1GGqCA_Zp3) is already saved as the current build, so the cards should appear without importing.
1. Zone change: does the route card advance when you enter a new zone? Does it ever jump wrongly (e.g. after a town portal)?
2. Level up: does the HUD level and the passive card (stage "1) Level 1-12" → "2) Normal Lab" at 33 …) follow?
3. Ctrl+Shift+H (hide/show), Ctrl+Shift+←/→ (route step), Ctrl+Shift+↑/↓ (stage), the tray menu: do they work, and do any clash with your other tools or PoE keys?
4. Alt-tab: overlay hides when PoE isn't focused and returns when it is?
5. Readability over your game at your resolution/DPI; card size/position (cards can be dragged in edit mode).
6. Does the route card text ("Hand in the quest" + the quest name below it) read well, or do you want the quest name inline?

## New since the passive budget
- The quest point table comes from the PoE wiki (read 2026-11-13) and a point counts as soon as the quest's route step is behind the cursor. Does the number match what `/passives` in game says for you?
- Gems and gear now only show in town (or in the Ctrl+Shift+T panel / edit mode), the passive card lists 3 names, and the settings, hints and setup card sit bottom right above the mana orb. Is the left column calm enough now, and is the settings block in the right place for your resolution?
- The estimate assumes you follow the plan's order. If you take other points than the plan, "available now" may be off; a manual "points spent" correction (hotkey) is possible, want it?
- Readability: the default opacity (0.72) was hard to read over a busy background in my screenshots. Over the game, is it fine, or should the default go up?

## Decisions needed
1. **Stages without a level in their title** (e.g. "5) Pconc of Bouncing Swap", "6) ENDGAME SETUP") are never picked by level. For now Ctrl+Shift+↑/↓ switches stage manually. Better idea: a level you set per stage, or "switch at level N" in settings?
2. **Passive tree data is only available for the latest tree version** (GGG's export). Builds on older versions resolve against the latest tree; changed nodes may be named wrong. Warning in the UI enough, or bundle PoB's per-version tree data (bigger, license check)?
3. **Hotkeys**: rebinding exists now (T27). Any default you want different? The tray menu text now follows the bindings (check it after rebinding).
4. **Dev behaviour when PoE isn't running**: overlay covers the primary monitor and stays visible. Fine, or hide when the game is absent?
5. **Flowbite Svelte** for the settings/onboarding window only (not the overlay): OK? Needs Tailwind.
6. **Route variant**: now a setting, "Quests" in edit mode: *Essential* (default: the route for passive points and gems) or *Everything* (441 against 480 steps: the extra side quests and detours such as the Tidal Island / Medicine Chest). Both give the same passive points. Is that the split you meant, or do you want a third mode without gem quests (not built: nearly every quest in the route gives a gem or a point, filtering them hid only 2 of 441 steps)?
7. **Bandits (act 2)**: a setting now (from the build, kill all, Alira, Oak, Kraityn); without a build it means "kill all". Right default?
8. **Quest passive points and the "Take now" gem logic**: gems come from the current + next build stage. Is that the right horizon?

## Built since, please try by hand (edit mode = Ctrl+Shift+O)
- Drag a card, mouse wheel on a card to resize; does the layout survive a restart?
- Settings block (bottom right): Quests (Essential / Everything), Bandits, Language, Hotkeys. Switch Quests or Bandits mid-act: the route should stay on the same step. Do they survive a restart?
- Hotkeys list: click a key, press a new combination; a taken or bare key is refused with a message; swap two keys and restart: both should stay.
- Play without importing a build: the route card, the level and the zone should work on their own (act 2 shows the bandit steps, "kill all" by default).
- Remove build (edit box, only visible with a build loaded): the passive, gem and gear cards should disappear, the route position stay, and the build stay gone after a restart. Import it again to get it back.
- Client.txt picker: only appears when the log can't be found. To try: temporarily rename `Client.txt`, paste the path in edit mode, rename it back (the app finds it within 5 seconds).
- Route card now reads "Act 1 · 3 / 40" (position inside the act). Level-up toast. First-run "Welcome" card (you will see it once, press Done).
- Ctrl+Shift+T panel: mini tree, build notes card. Gear card: click an item, paste in the stash search.
- Set a manual stage (Ctrl+Shift+↑/↓), restart the app and change zone: the stage should stay (this was broken before the review fix).
- New look (cool blue-black, sea-glass accent): too plain, too dark, anything you miss? It was only seen over my own windows and once over PoE.

## Known gaps / not done
- Not built: reminders (T26), socket pips and swap-at-level (T24), vendor recipes and the bandit choice in the town chip (it counts gems only), a Client.txt browse button (you paste a path), replacing a wrong log path once a log was found, a "points spent" correction.
- `tree::download` was exercised live once; the refresh of an old tree cache (new league) is unit-tested but not seen live.
- The route, quests, gem rewards and the 23 passive-point quests are bundled data (Exile Leveling, MIT, plus the PoE wiki). A game patch that changes them needs a manual update: note the source and date in T31.
- Not measured yet: idle CPU/RAM over a long session, FPS impact (T29, after your playtest). The watcher now only rebuilds views when a log line arrives.
- Only 3 real builds tested; a single-spec build (auto-order path) is tested on the Ranger's endgame tree only.
- Hide hotkey and tray menu were not verified automatically.
- Security (T30): `cargo audit` and `npm audit` run on 2026-11-13: no vulnerabilities, 2 warnings (glib unsound, proc-macro-error unmaintained) from the Linux GTK stack of Tauri, which a Windows build does not use. Still open: no cargo-fuzz run, release-build CSP not checked.
- Not done by agreement: T29 performance and T31 installer/README, after everything else is finished.
- My test runs against a fake Client.txt stopped your own `npm run tauri dev` once (I matched processes by project path): restart it if the overlay is gone.

## DECIDED
- Name: PoeMate. Exile Leveling data may be used (MIT, credited in docs/CREDITS.md).
