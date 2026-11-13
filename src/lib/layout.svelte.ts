import { invoke } from "@tauri-apps/api/core";

export type Pos = { x: number; y: number; scale: number };
export type Settings = { opacity: number; density: "cozy" | "compact" };

export const GRID = 8;
const EDGE = 12;

/** Positions are kept per overlay size, so a different resolution or window size has its own layout. */
export const layout = $state({
  settings: { opacity: 0.72, density: "cozy" } as Settings,
  all: {} as Record<string, Record<string, Pos>>,
  key: "",
});

let timer: ReturnType<typeof setTimeout> | undefined;

export function sizeKey() {
  return `${window.innerWidth}x${window.innerHeight}`;
}

export async function loadLayout() {
  layout.key = sizeKey();
  try {
    const l = await invoke<{ settings: Settings; all: typeof layout.all }>("layout_load");
    layout.settings = l.settings;
    layout.all = l.all;
  } catch {
    /* defaults */
  }
}

export function saveSoon() {
  clearTimeout(timer);
  timer = setTimeout(() => {
    invoke("layout_save", { layout: { settings: $state.snapshot(layout.settings), all: $state.snapshot(layout.all) } }).catch(() => {});
  }, 400);
}

export function posOf(id: string): Pos | undefined {
  return layout.all[layout.key]?.[id];
}

export function setPos(id: string, p: Pos) {
  (layout.all[layout.key] ??= {})[id] = p;
}

export function resetLayout() {
  delete layout.all[layout.key];
  saveSoon();
}

/** Round to the grid and stick to the window edges. */
export function snap(v: number, size: number, max: number) {
  let s = Math.round(v / GRID) * GRID;
  if (Math.abs(s) < EDGE) s = 0;
  const far = max - size;
  if (Math.abs(s - far) < EDGE) s = far;
  return Math.max(0, Math.min(s, Math.max(0, far)));
}
