# Data formats

## Route text (`data/route/act-N.txt`)
One step per line, upstream format of Exile Leveling. Parsed by `src-tauri/src/route.rs`; unknown fragments are errors.

```
#section Act 1
Find and kill {kill|Hillock}
➞ {enter|1_1_town} #Lioneye's Watch       # trailing "#label" = zone/quest name
    #sub Go {dir|45}                       # hint of the step above
#ifdef LEAGUE_START                         # conditional block, also #ifndef; closed by #endif
    Get {waypoint_get}
#endif
```

Fragments: `enter|<area id>`, `area|<id>`, `waypoint|<id>`, `waypoint_get`, `kill|<boss>`, `quest|<id>[|<id>…]`, `quest_text|<text>`,
`dir|<0..315 step 45>` (0 = up, clockwise), `logout`, `trial`, `portal|set|use`, `crafting`, `generic|<text>`, `arena|<name>`, `ascend`.

Area ids are the ones `Client.txt` reports (`Generating level N area "1_1_town"`), so zone changes advance the route directly.

Flags used upstream: `LEAGUE_START`, `LIBRARY`, `BANDIT_ALIRA`, `BANDIT_OAK`, `BANDIT_KRAITYN`, `BANDIT_KILL`.

## Zones (`data/areas.json`)
Map of area id -> `{id, name, act, level, has_waypoint, is_town_area, …}`.
