# Open questions (for the user) — updated as work continues

Newest first.

## Please test in the real game (I can't do these headlessly)
Start with `npm run tauri dev` while PoE runs (Windowed/Borderless). Your Ranger/Pconc build (pobb.in/Ig1GGqCA_Zp3) is already saved as the current build, so the cards should appear without importing.
1. Zone change: does the route card advance when you enter a new zone? Does it ever jump wrongly (e.g. after a town portal)?
2. Level up: does the HUD level and the passive card (stage "1) Level 1-12" → "2) Normal Lab" at 33 …) follow?
3. Ctrl+Shift+H (hide/show), Ctrl+Shift+←/→ (route step), Ctrl+Shift+↑/↓ (stage), the tray menu: do they work, and do any clash with your other tools or PoE keys?
4. Alt-tab: overlay hides when PoE isn't focused and returns when it is?
5. Readability over your game at your resolution/DPI; card size/position (they are fixed top-left for now; moving them is the widget framework, T20).
6. Does the route card text ("Hand in the quest" + the quest name below it) read well, or do you want the quest name inline?

## New since the passive budget
- The quest point table comes from the PoE wiki (read 2026-11-13) and a point counts as soon as the quest's route step is behind the cursor. Does the number match what `/passives` in game says for you?
- With all four cards the default left column reaches the life orb at the bottom-left (see screenshot). Drag cards in edit mode, or should I cap the column height / stack to two columns by default?
- The estimate assumes you follow the plan's order. If you take other points than the plan, "available now" may be off; a manual "points spent" correction (hotkey) is possible, want it?

## Decisions needed
1. **Stages without a level in their title** (e.g. "5) Pconc of Bouncing Swap", "6) ENDGAME SETUP") are never picked by level. For now Ctrl+Shift+↑/↓ switches stage manually. Better idea: a level you set per stage, or "switch at level N" in settings?
2. **Passive tree data is only available for the latest tree version** (GGG's export). Builds on older versions resolve against the latest tree; changed nodes may be named wrong. Warning in the UI enough, or bundle PoB's per-version tree data (bigger, license check)?
3. **Hotkeys**: defaults above. Any you want different? (Rebinding UI comes with settings, T27.)
4. **Dev behaviour when PoE isn't running**: overlay covers the primary monitor and stays visible. Fine, or hide when the game is absent?
5. **Flowbite Svelte** for the settings/onboarding window only (not the overlay): OK? Needs Tailwind.
6. **Route variant**: League-start route (extra detours like the Tidal Island / Medicine Chest) is off by default; "safe/plain" is the default. Which do you want as default, and should it be a setting?
7. **Quest passive points and the "Take now" gem logic**: gems come from the current + next build stage. Is that the right horizon?

## Known gaps / not done
- Widgets can't be moved yet (T20); settings, hotkey rebinding, onboarding wizard, i18n (T27/T28).
- Mini tree picture and the Ctrl+Shift+T passive panel (T22/T23).
- `tree::download` was exercised live once (6.7 MB downloaded fine) but isn't covered by an automated test.
- Not measured yet: idle CPU/RAM over a long session, FPS impact (T29).
- Only 3 real builds tested (all multi-stage); a single-spec build (auto-order path) is tested on the Ranger's endgame tree only.
- Hide hotkey and tray menu were not verified automatically.

## DECIDED
- Name: PoeMate. Exile Leveling data may be used (MIT, credited in docs/CREDITS.md).
