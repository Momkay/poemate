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
- With all four cards the default left column reaches the life orb at the bottom-left (see screenshot). Drag cards in edit mode, or should I cap the column height / stack to two columns by default?
- The estimate assumes you follow the plan's order. If you take other points than the plan, "available now" may be off; a manual "points spent" correction (hotkey) is possible, want it?

## Decisions needed
1. **Stages without a level in their title** (e.g. "5) Pconc of Bouncing Swap", "6) ENDGAME SETUP") are never picked by level. For now Ctrl+Shift+↑/↓ switches stage manually. Better idea: a level you set per stage, or "switch at level N" in settings?
2. **Passive tree data is only available for the latest tree version** (GGG's export). Builds on older versions resolve against the latest tree; changed nodes may be named wrong. Warning in the UI enough, or bundle PoB's per-version tree data (bigger, license check)?
3. **Hotkeys**: rebinding exists now (T27). Any default you want different?
4. **Dev behaviour when PoE isn't running**: overlay covers the primary monitor and stays visible. Fine, or hide when the game is absent?
5. **Flowbite Svelte** for the settings/onboarding window only (not the overlay): OK? Needs Tailwind.
6. **Route variant**: League-start route (extra detours like the Tidal Island / Medicine Chest) is off by default; "safe/plain" is the default. Which do you want as default, and should it be a setting?
7. **Quest passive points and the "Take now" gem logic**: gems come from the current + next build stage. Is that the right horizon?

## Built since, please try by hand (edit mode = Ctrl+Shift+O)
- Drag a card, mouse wheel on a card to resize; does the layout survive a restart?
- Hotkeys list (edit box): click a key, press a new combination; a taken or bare key should be refused with a message; does the new key work and survive a restart?
- Language switch EN/NL; first-run "Welcome" card (you will see it once, press Done).
- Ctrl+Shift+T panel: mini tree, build notes card. Gear card: click an item, paste in the stash search.
- Level-up toast, "Town tasks" chip, step animation (the zone/level part was replayed against a fake Client.txt on 2026-11-13 and followed correctly).

## Known gaps / not done
- Manual Client.txt picker (only auto-detect and a saved path), reminders in T26, socket pips / swap-at-level in T24.
- `tree::download` was exercised live once but isn't covered by an automated test.
- Not measured yet: idle CPU/RAM over a long session, FPS impact (T29, after your playtest).
- Only 3 real builds tested; a single-spec build (auto-order path) is tested on the Ranger's endgame tree only.
- Hide hotkey and tray menu were not verified automatically; tray labels and the footer still name the default keys in the tray.
- Not done yet by agreement: T29 performance and T31 installer/README, after everything else is finished.
- Security: no cargo-fuzz run, no `cargo audit`, release-build CSP not checked (T30).

## DECIDED
- Name: PoeMate. Exile Leveling data may be used (MIT, credited in docs/CREDITS.md).
