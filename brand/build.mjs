// Builds every brand asset from one definition of the mark: node brand/build.mjs
// Needs Node 22+ and Chrome or Edge (headless, for the PNGs) and PowerShell (for the installer BMPs).
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "out");
mkdirSync(out, { recursive: true });

// ---- tokens (the same as the overlay's) ----
const C = { ground: "#121417", surface: "#1a1d21", ink: "#ece9e2", muted: "#8c8b85", sage: "#8fc9a6", inkOnLight: "#16181b", sageOnLight: "#3f8f68", paper: "#f4f2ed" };
const FONT = `"Segoe UI Variable Display", "Segoe UI", Inter, system-ui, sans-serif`;

// ---- the mark: a monoline P whose stem ends in a node (a route that arrives somewhere) ----
// 512 box, centred; stroke 44, bowl diameter 188, node radius 38.
const markShapes = (ink, node) => `<path d="M166 388V106H252a94 94 0 0 1 0 188H166" fill="none" stroke="${ink}" stroke-width="44" stroke-linecap="round" stroke-linejoin="round"/><circle cx="166" cy="388" r="38" fill="${node}"/>`;
const markSvg = (ink, node, bg = "") => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" role="img" aria-label="PoeMate">${bg}${markShapes(ink, node)}</svg>\n`;
const monoSvg = (c) => markSvg(c, c);

// the app icon: a rounded plate with the mark at 80%
const plate = (fill) => `<rect width="512" height="512" rx="112" fill="${fill}"/>`;
const iconSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" role="img" aria-label="PoeMate">${plate(C.ground)}<g transform="translate(51.2 51.2) scale(.8)">${markShapes(C.ink, C.sage)}</g></svg>\n`;

// horizontal lockup: mark, then the name in two weights (the font is a live text: see brand-guide.md)
const lockup = (ink, node, textInk) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 320" role="img" aria-label="PoeMate">
<g transform="translate(-94 -31) scale(.75)">${markShapes(ink, node)}</g>
<text x="224" y="222" font-family='${FONT}' font-size="176" letter-spacing="-4" fill="${textInk}"><tspan font-weight="600">Poe</tspan><tspan font-weight="300">Mate</tspan></text>
</svg>\n`;

const write = (name, text) => writeFileSync(join(here, name), text);
write("mark.svg", markSvg(C.ink, C.sage));
write("mark-on-light.svg", markSvg(C.inkOnLight, C.sageOnLight));
write("mark-mono-white.svg", monoSvg("#ffffff"));
write("mark-mono-black.svg", monoSvg("#000000"));
write("app-icon.svg", iconSvg);
write("logo-horizontal.svg", lockup(C.ink, C.sage, C.ink));
write("logo-horizontal-on-light.svg", lockup(C.inkOnLight, C.sageOnLight, C.inkOnLight));

// ---- rendering through headless Chrome ----
const browsers = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
];
const chrome = browsers.find(existsSync);
if (!chrome) throw new Error("No Chrome or Edge found for the PNG renders");

function render(name, w, h, html, transparent = false, bg = C.ground) {
  const page = join(out, `${name}.html`);
  writeFileSync(page, `<!doctype html><meta charset="utf-8"><style>html,body{margin:0;width:${w}px;height:${h}px;overflow:hidden;background:${transparent ? "transparent" : bg};font-family:${FONT};color:${C.ink}}</style>${html}`);
  execFileSync(chrome, [
    "--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
    `--window-size=${w},${h}`, ...(transparent ? ["--default-background-color=00000000"] : []),
    `--screenshot=${join(out, name + ".png")}`, pathToFileURL(page).href,
  ], { stdio: "ignore" });
}
const inline = (svg, style = "") => `<div style="${style}">${svg.replace("<svg ", `<svg style="display:block;width:100%;height:100%" `)}</div>`;

// app icon (source for `tauri icon`), the mark alone, and a favicon
render("app-icon-1024", 1024, 1024, inline(iconSvg, "width:1024px;height:1024px"), true);
render("mark-1024", 1024, 1024, inline(markSvg(C.ink, C.sage), "width:1024px;height:1024px"), true);

// GitHub social preview 1280x640 and README banner 1600x480
const claim = "A quiet route guide for Path of Exile leveling.";
const banner = (w, h, mark, title, sub) => `<div style="position:absolute;left:${w * 0.07}px;top:${(h - mark) / 2}px;width:${mark}px;height:${mark}px">${markSvg(C.ink, C.sage).replace("<svg ", `<svg style="display:block;width:100%;height:100%" `)}</div>
<div style="position:absolute;left:${w * 0.07 + mark * 0.78 + w * 0.02}px;top:${h / 2 - title * 0.82}px"><div style="font-size:${title}px;letter-spacing:-.03em;line-height:1"><b style="font-weight:600">Poe</b><span style="font-weight:300">Mate</span></div><div style="margin-top:${title * 0.28}px;font-size:${title * 0.27}px;color:${C.muted};max-width:${w * 0.55}px;line-height:1.35">${sub}</div></div>`;
render("social-preview", 1280, 640, banner(1280, 640, 400, 124, claim));
render("readme-banner", 1600, 480, banner(1600, 480, 340, 112, claim));

// installer art: NSIS wants 24-bit BMPs, sidebar 164x314 and header 150x57
render("installer-sidebar", 164, 314, `<div style="position:absolute;left:34px;top:46px;width:96px;height:96px">${markSvg(C.ink, C.sage).replace("<svg ", `<svg style="display:block;width:100%;height:100%" `)}</div><div style="position:absolute;left:0;width:164px;top:158px;text-align:center;font-size:23px;letter-spacing:-.02em"><b style="font-weight:600">Poe</b><span style="font-weight:300">Mate</span></div>`);
render("installer-header", 150, 57, `<div style="position:absolute;left:12px;top:11px;width:35px;height:35px">${markSvg(C.ink, C.sage).replace("<svg ", `<svg style="display:block;width:100%;height:100%" `)}</div><div style="position:absolute;left:56px;top:16px;font-size:21px;letter-spacing:-.02em"><b style="font-weight:600">Poe</b><span style="font-weight:300">Mate</span></div>`);
const ps = (f) => `Add-Type -AssemblyName System.Drawing; $s=[System.Drawing.Image]::FromFile('${join(out, f + ".png").replaceAll("/", "\\")}'); $b=New-Object System.Drawing.Bitmap $s.Width,$s.Height,([System.Drawing.Imaging.PixelFormat]::Format24bppRgb); $g=[System.Drawing.Graphics]::FromImage($b); $g.DrawImage($s,0,0,$s.Width,$s.Height); $b.Save('${join(out, f + ".bmp").replaceAll("/", "\\")}',[System.Drawing.Imaging.ImageFormat]::Bmp); $g.Dispose(); $b.Dispose(); $s.Dispose()`;
for (const f of ["installer-sidebar", "installer-header"]) execFileSync("powershell", ["-NoProfile", "-Command", ps(f)], { stdio: "ignore" });

// brand sheet: one page that shows the system
const sw = (name, hex, use, text) => `<div style="background:${hex};color:${text};padding:18px 20px;border-radius:6px;min-height:84px"><div style="font-size:18px;font-weight:600">${name}</div><div style="font-size:14px;opacity:.75;margin-top:4px">${hex} · ${use}</div></div>`;
const small = (svg, px, bg) => `<div style="width:${px}px;height:${px}px;background:${bg};border-radius:${px * 0.22}px;display:grid;place-items:center">${svg.replace("<svg ", `<svg style="display:block;width:100%;height:100%" `)}</div>`;
render("brand-sheet", 1600, 1180, `<div style="padding:64px 72px;display:grid;gap:56px">
<div style="display:grid;grid-template-columns:1fr 1fr;gap:24px">
 <div style="background:${C.surface};padding:56px;border-radius:8px;display:grid;place-items:center;height:240px">${lockup(C.ink, C.sage, C.ink).replace("<svg ", `<svg style="width:480px" `)}</div>
 <div style="background:${C.paper};padding:56px;border-radius:8px;display:grid;place-items:center;height:240px">${lockup(C.inkOnLight, C.sageOnLight, C.inkOnLight).replace("<svg ", `<svg style="width:480px" `)}</div>
</div>
<div style="display:grid;grid-template-columns:repeat(5,1fr);gap:16px">
 ${sw("Ground", C.ground, "backgrounds", C.ink)}${sw("Ink", C.ink, "text, the mark", C.inkOnLight)}${sw("Sage", C.sage, "now, gain: the accent", "#10231a")}${sw("Muted", C.muted, "secondary text", C.ground)}${sw("Paper", C.paper, "light surfaces", C.inkOnLight)}
</div>
<div style="display:grid;grid-template-columns:auto 1fr;gap:56px;align-items:center">
 <div style="display:flex;gap:20px;align-items:flex-end">${small(markSvg(C.ink, C.sage), 128, C.ground)}${small(iconSvg, 128, "transparent")}${small(iconSvg, 64, "transparent")}${small(iconSvg, 32, "transparent")}${small(iconSvg, 16, "transparent")}</div>
 <div style="font-size:20px;line-height:1.5;color:${C.muted};max-width:720px"><div style="font-size:36px;letter-spacing:-.02em;color:${C.ink};margin-bottom:10px"><b style="font-weight:600">Poe</b><span style="font-weight:300">Mate</span></div>A route that ends in a node: the line you follow and the point you reach. Quiet by default, one accent for where you are now. Type is Segoe UI Variable Display, 600 and 300.</div>
</div></div>`, false, "#0a0b0d");
console.log("brand assets written to", resolve(out));
