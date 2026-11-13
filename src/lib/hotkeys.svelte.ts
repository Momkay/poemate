import { invoke } from "@tauri-apps/api/core";

export type Binding = { id: string; label: string; keys: string };

export const hk = $state({ binds: [] as Binding[] });

/** "Ctrl+Shift+KeyO" -> "Ctrl+Shift+O": what people expect to read. */
export const pretty = (keys: string) => keys.replace(/Key|Digit|Arrow/g, "");
export const keyOf = (id: string) => pretty(hk.binds.find((b) => b.id === id)?.keys ?? "");

export const loadBinds = () => invoke<Binding[]>("hotkeys_get").then((b) => (hk.binds = b));
export const resetBinds = () => invoke<Binding[]>("hotkeys_reset").then((b) => (hk.binds = b));

/** The text the backend parses for a key press, or null while only a modifier is down. */
export function comboOf(e: KeyboardEvent): string | null {
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return null;
  return [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.code].filter(Boolean).join("+");
}

/** "" when it worked, otherwise the message to show (taken key, no modifier, ...). */
export async function setBind(id: string, keys: string): Promise<string> {
  try {
    hk.binds = await invoke<Binding[]>("hotkeys_set", { id, keys });
    return "";
  } catch (e) {
    return String(e);
  }
}
