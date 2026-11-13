# Credits

## Exile Leveling — campaign route data
- Source: https://github.com/HeartofPhos/exile-leveling (commit d6951db, copied 2026-11-13)
- License: MIT, Copyright (c) 2025 HeartofPhos. Full text: `data/LICENSE-exile-leveling.txt`
- Used files: `data/route/act-1.txt` … `act-10.txt` (route text), `data/areas.json` (zone ids, names, levels).
- Also used: `data/quests.json` (quest rewards and vendor offers per class) and `data/gems.json` (gem names). Upstream paths at that commit: `common/data/json/{areas,quests,gems}.json` and `common/data/routes/act-N.txt`.
- Checked 2026-11-13 against upstream: areas, quests and the route text are byte-identical apart from line endings. `gems.json` is trimmed to `name` and `is_support` (upstream also has `id`, `primary_attribute`, `required_level`).
- To refresh: copy those files from a newer upstream commit into `data/`, trim `gems.json` the same way, update the commit and date above, run `cd src-tauri && cargo test --lib` (the route and reward tests catch a broken format).
- PoeMate parses the route text format as is (see `docs/DATA_FORMAT.md`), so upstream route updates can be dropped in.
- Exile Leveling's author does not take route PRs and recommends users edit routes to taste; we do the same via custom routes (later).

## Path of Exile passive tree data
- https://github.com/grindinggear/skilltree-export (Grinding Gear Games), downloaded at runtime and cached locally.

## Disclaimer
This product isn't affiliated with or endorsed by Grinding Gear Games in any way.
