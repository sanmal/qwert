import { createSignal } from "solid-js";
import * as tauri from "../lib/tauri";

const [loaded, setLoaded] = createSignal(false);
const [currentFg, setCurrentFg] = createSignal<string | null>(null);
const [currentBg, setCurrentBg] = createSignal<string | null>(null);

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

  // Read resolved CSS vars so StatusBar can display the contrast ratio.
  // getComputedStyle reads the full cascade (including [data-theme] attribute rules).
  const style = getComputedStyle(document.documentElement);
  const fg = style.getPropertyValue("--qw-fg").trim();
  const bg = style.getPropertyValue("--qw-bg").trim();
  setCurrentFg(fg || null);
  setCurrentBg(bg || null);
}

async function applyAppearance() {
  if (loaded()) return;
  const map = await tauri.loadAppearance();
  applyVars(map);
  setLoaded(true);
}

// C2: hot-reload entry point. Re-applies on every appearance-changed event —
// deliberately bypasses the loaded() guard (which is initial-load only).
function reapplyAppearance(map: Record<string, string>) {
  applyVars(map);
}

export const appearanceStore = {
  applyAppearance,
  reapplyAppearance,
  loaded,
  currentFg,
  currentBg,
};
