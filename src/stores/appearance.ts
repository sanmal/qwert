import { createSignal } from "solid-js";
import * as tauri from "../lib/tauri";

const [loaded, setLoaded] = createSignal(false);
// C9: increments on every applyVars call; StatusBar uses it to trigger IPC refetch.
const [appearanceVersion, setAppearanceVersion] = createSignal(1);
// C3: non-null when appearance.toml had a parse/conflict error.
const [currentWarning, setCurrentWarning] = createSignal<string | null>(null);

// Remove every qwert-managed inline custom property and the data-theme
// attribute so a re-apply starts from a clean slate (e.g. switching from custom
// fg/bg to a preset must not leave a stale --qw-fg behind).
function clearManagedVars() {
  const el = document.documentElement;
  const toRemove: string[] = [];
  for (let i = 0; i < el.style.length; i++) {
    const prop = el.style[i];
    if (prop && prop.startsWith("--qw-")) toRemove.push(prop);
  }
  for (const prop of toRemove) el.style.removeProperty(prop);
  delete el.dataset.theme;
}

// Apply a resolved CSS-vars map to the document root. Shared by the initial
// load and hot-reload so both follow the exact same code path.
function applyVars(map: Record<string, string>) {
  clearManagedVars();
  for (const [key, value] of Object.entries(map)) {
    if (key.startsWith("--")) {
      document.documentElement.style.setProperty(key, value);
    } else if (key === "data-theme") {
      // A2: preset → data-theme attribute; theme CSS rules (:[data-theme="..."]) take effect.
      document.documentElement.dataset.theme = value;
    }
    // Unknown keys are ignored.
  }

  // C9: bump version so StatusBar's createResource refetches from Rust IPC.
  setAppearanceVersion((v) => v + 1);
}

async function applyAppearance() {
  if (loaded()) return;
  const map = await tauri.loadAppearance();
  applyVars(map);
  setLoaded(true);
}

// C2: hot-reload entry point. Re-applies on every appearance-changed event —
// deliberately bypasses the loaded() guard (which is initial-load only).
// Also clears any outstanding C3 warning (config is valid again).
function reapplyAppearance(map: Record<string, string>) {
  applyVars(map);
  setCurrentWarning(null);
}

// C3: record a warning from a failed parse / conflict. The previous visual
// state is NOT changed — the caller must not call applyVars on error.
function setAppearanceWarning(msg: string) {
  setCurrentWarning(msg);
}

export const appearanceStore = {
  applyAppearance,
  reapplyAppearance,
  setAppearanceWarning,
  loaded,
  appearanceVersion,
  currentWarning,
};
