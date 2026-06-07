import { createSignal } from "solid-js";
import * as tauri from "../lib/tauri";

const [loaded, setLoaded] = createSignal(false);
const [currentFg, setCurrentFg] = createSignal<string | null>(null);
const [currentBg, setCurrentBg] = createSignal<string | null>(null);

async function applyAppearance() {
  if (loaded()) return;
  const map = await tauri.loadAppearance();
  for (const [key, value] of Object.entries(map)) {
    if (key.startsWith("--")) {
      document.documentElement.style.setProperty(key, value);
    } else if (key === "data-theme") {
      // A2: preset → data-theme attribute; theme CSS rules (:[data-theme="..."]) take effect.
      document.documentElement.dataset.theme = value;
    }
    // Unknown keys are ignored.
  }
  setLoaded(true);

  // Read resolved CSS vars so StatusBar can display the contrast ratio.
  // getComputedStyle reads the full cascade (including [data-theme] attribute rules).
  const style = getComputedStyle(document.documentElement);
  const fg = style.getPropertyValue("--qw-fg").trim();
  const bg = style.getPropertyValue("--qw-bg").trim();
  if (fg) setCurrentFg(fg);
  if (bg) setCurrentBg(bg);
}

export const appearanceStore = { applyAppearance, loaded, currentFg, currentBg };
