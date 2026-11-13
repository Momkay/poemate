# PoeMate — GGG third-party compliance

Read on **2026-11-13** from:
- https://www.pathofexile.com/developer/docs (sections `#policy`, `#guidelines`, `#ratelimits`) — no last-updated date shown on the page
- https://www.pathofexile.com/developer/docs/authorization
- https://www.pathofexile.com/legal/terms-of-use-and-privacy-policy (Terms of Use, clauses 7 and 22)

Caveats: quotes were retrieved through a fetch agent, not read by hand. The forum "Third Party Policy" thread could not be located, so
this document rests on the developer docs and Terms of Use only. Re-read the primary pages before release (task T31).
The docs page also states "We are currently unable to process new applications", so PoeMate does **not** use GGG's API in the MVP.

## Rules that apply to us (verbatim)

**Apps that run separate from the game**
> "Ordinarily we only allow the creation of applications that run completely separate from the game. We don't encourage players to download or run "tools" on their machine as these are never guaranteed to be free from malware or nefarious practices."
> "While not encouraged, these are permitted. They must use a public OAuth client if interacting with our APIs."

**Overlay rules (screen, clipboard, game logs)**
> "Apps that use information sourced from the screen, the clipboard, or the game's logs **must** follow our overlay rules:"
> "You may only read this information as long as the user is aware of what you are doing with that data."
> "Reading from the screen or clipboard may only be performed once per manual key press."

**Macros / input automation**
> "Automation of key-strokes (or other inputs) that affect the game in any way **must** follow our macro rules:"
> "Macros must be invoked manually by the user (automated invocations such as but not limited to: timers, reacting to file changes, or from reading the screen are not allowed)."
> "Each macro invocation must have one set function (it cannot cycle between actions)."
> "The resulting function may only perform one action that interacts with the game (sending a single chat message or command counts as one action)."

**Apps that interact with the game or its files**
> "This behaviour is strictly against our Terms of Use (sections 7b, 7c, 7i)."
> "Creation or use of these type of applications will result in immediate account termination."

**Terms of Use, clause 7 (excerpts)**
> 7.2 "Modify or adapt (including through third parties and third-party tools) the game client or its data, other than in the normal course of PoE gameplay as permitted in accordance with the Licence."
> 7.3 "Utilise any automated software or 'bots' in relation to your access or use of the Website, Materials or Services."
> 7.5 "Connect to the Servers through any software other than the authorised game client software."
> 7.9 "Reverse engineer, de-compile or disassemble the Website, Materials or Services …"

**Branding / disclaimer**
> "You may not present your project or tool in a way that could be construed as it being developed or officially authorised by Grinding Gear Games."
> Required statement: "This product isn't affiliated with or endorsed by Grinding Gear Games in any way."

**If we ever use the API (not in MVP)**
> "Any application that interacts with our API must set an identifiable User Agent header prefixed using the following format: User-Agent: OAuth {$clientId}/{$version} (contact: {$contact}) ..."
> Rate limits: "For this reason we include headers in response headers that should be parsed and followed by your application." / "Exceeding these limits frequently will result in your application access being revoked."
> Public clients "Must only use the Authorization Code (with PKCE) grant type and must use a local redirect URI"; "Don't embed keys or credentials in a distributed binary."

## What PoeMate does

| Behaviour | Status | Basis |
|---|---|---|
| Separate top-level window above the game | Allowed | runs "completely separate from the game" |
| Reads `Client.txt` (read-only tail) | Allowed with disclosure | game-log reading needs the user to know what we do with it -> first-run + settings disclosure (T27/T28) |
| Reads clipboard | **Only on a manual hotkey press, once per press** | overlay rule; no background clipboard watching |
| Reads the screen (capture/OCR/pixels) | **Not implemented** | rule allows once per key press only; out of MVP scope |
| Sends keys/mouse to the game, macros | **Never** | macro rules; hotkeys only control PoeMate's own UI |
| Reads game memory, injects DLLs, hooks D3D/Vulkan, touches game files | **Never** | Terms 7.2, "interact with the game or its files" |
| Network | Only on user action: PoB hosts (pobb.in, pastebin), skilltree-export data. No GGG API in MVP | rate/User-Agent rules apply if added |
| Telemetry | None; chat lines in `Client.txt` are discarded, never stored | privacy |

## Required in the product
1. Disclaimer "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." in README, installer and settings.
2. Settings "Compliance" panel listing the table above in plain language (T27).
3. First-run notice that `Client.txt` is read and what is derived from it (area, level) (T28).
4. Panic hotkey hides the overlay instantly (T3).
5. No GGG logos/branding presented as official.

## Third-party data and credits
Shown in Settings > About. Licenses checked on GitHub 2026-11-13.

| Source | License | Used for |
|---|---|---|
| [exile-leveling](https://github.com/HeartofPhos/exile-leveling) (HeartofPhos) | MIT | route, quest and gem data in `data/` |
| [skilltree-export](https://github.com/grindinggear/skilltree-export) (GGG) | none stated; GGG's own public export | passive tree data, fetched on user action |
| [Path of Building](https://github.com/PathOfBuildingCommunity/PathOfBuilding) | custom license file, not read yet | we only parse the build import format; no PoB code or data bundled |
| PoE wiki | CC BY-NC-SA | quest passive points |

Looked at and **not used**: poe-vendor-string (AGPL-3.0: copying it would make PoeMate AGPL), poe2.re (no license, PoE2), Path of Building PoE2 (PoE2), Exile-UI Act-Decoder images (code is MIT, but the zone images may be GGG material: check per file before bundling).

## Plan changes caused by this review
- T10: clipboard auto-sniffing removed; replaced by a "Import from clipboard" hotkey/button (one read per press).
- PROMPT.md §0 and §2 updated accordingly.
