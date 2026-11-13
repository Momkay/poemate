# PoeMate performance (T29, partial)

Measured 2026-11-13 on the dev machine (Windows 11), release build (`npm run tauri build -- --no-bundle`), `POEMATE_ALWAYS_SHOW=1`, the game not running, a saved build loaded.

| What | Result | How |
|---|---|---|
| Cold start to a window handle | ~0.26 s | polling `MainWindowHandle`; the first paint may come a little later |
| Idle CPU | 0.31 % of one core | CPU time of poemate.exe and its direct children over 15 s |
| RAM | ~157 MB | working set of the same processes; the WebView2 helper processes are children of the child, so this undercounts |
| Installer | 2.7 MB | NSIS |

Not measured yet: import time, event-to-UI latency while a game writes Client.txt, FPS impact on PoE (needs the game running), RAM including all WebView2 processes. T29 stays open until those are done.
