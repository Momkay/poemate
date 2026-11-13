# PoeMate brand

## The idea
A route that ends in a node. The mark is a **monoline P**: one stroke that runs down the stem and ends in a **sage dot**. It is the line you follow and the point you reach, the same picture as the ringed "next node" in the overlay's mini tree. One shape, one accent, no ornament, so it still reads at 16 px and sits quietly in a game.

## Files (`brand/`)
| File | Use |
| --- | --- |
| `mark.svg` | The mark on dark grounds. Ink plus sage. |
| `mark-on-light.svg` | The mark on light grounds. Darker ink and a deeper sage for contrast. |
| `mark-mono-white.svg`, `mark-mono-black.svg` | One colour only: stamps, embossing, single-colour print, watermarks. |
| `logo-horizontal.svg`, `logo-horizontal-on-light.svg` | Mark plus name. The name is live text (see Type). |
| `app-icon.svg` | The mark on its rounded plate. Source of every app icon. |
| `out/app-icon-1024.png` | 1024 px source for `tauri icon`. |
| `out/mark-1024.png` | The mark on transparent. |
| `out/readme-banner.png` | 1600x480, top of the README. |
| `out/social-preview.png` | 1280x640, the GitHub social preview. |
| `out/installer-sidebar.bmp`, `out/installer-header.bmp` | 24-bit, 164x314 and 150x57, for the NSIS installer. |
| `out/brand-sheet.png` | One page showing the whole system. |
| `build.mjs` | Rebuilds everything above from one definition: `node brand/build.mjs`. Needs Node 22+, Chrome or Edge, PowerShell. |

The app icons (`src-tauri/icons/*`, `.ico`, `.icns`, the Windows Store sizes) and `static/favicon.*` come from `app-icon.svg` through `npm run tauri icon brand/out/app-icon-1024.png`.

## Colour
| Name | Hex | Use |
| --- | --- | --- |
| Ground | `#121417` | Backgrounds, the app icon plate. |
| Ink | `#ece9e2` | Text and the mark on dark. |
| Sage | `#8fc9a6` | The accent: "now" and "gain". One dot per view. |
| Muted | `#8c8b85` | Secondary text. |
| Paper | `#f4f2ed` | Light surfaces. |
| Ink on light | `#16181b` | Text and the mark on paper. |
| Sage on light | `#3f8f68` | The accent on paper (the light sage fails contrast there). |

Amber `#d9b36a` and red `#d9755f` exist only for warnings and keystones in the overlay. They are not brand colours.

## Type
Segoe UI Variable Display: **Poe** in 600, **Mate** in 300, tracking about -0.02 em. Body text is Segoe UI Variable Text. The wordmark in the SVG files is live text with the stack `"Segoe UI Variable Display", "Segoe UI", Inter, system-ui`; on a machine without Segoe it falls back. For print or a store listing, export the lockup to outlines in a vector editor first.

## Clear space and size
- Clear space around the mark or lockup: the diameter of the sage dot on every side.
- Smallest sizes: the mark 16 px, the app icon 16 px, the lockup 96 px wide.
- Never: recolour the sage, put the sage on a light ground (use sage on light), stretch or rotate the mark, add a shadow, outline or gradient, set the name in another face.

## Legal
PoeMate is not affiliated with or endorsed by Grinding Gear Games in any way. Do not use GGG's logos, art or fonts in the brand. "Path of Exile" is named only to say what the tool is for.

## Still to wire (T31)
The installer art is made but not yet referenced. In `src-tauri/tauri.conf.json`, under `bundle.windows.nsis`, set `installerIcon`, `headerImage` (`brand/out/installer-header.bmp`) and `sidebarImage` (`brand/out/installer-sidebar.bmp`), then check one installer build.
