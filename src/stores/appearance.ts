import { createSignal } from "solid-js";
import * as tauri from "../lib/tauri";

const [loaded, setLoaded] = createSignal(false);

async function applyAppearance() {
  if (loaded()) return;
  const map = await tauri.loadAppearance();
  for (const [key, value] of Object.entries(map)) {
    if (key.startsWith("--")) {
      // CSS 変数はそのまま適用
      document.documentElement.style.setProperty(key, value);
    } else if (key === "data-theme") {
      // A2: preset は CSS 変数ではなく data-theme 属性として適用する。
      // t06 のテーマCSS（:root[data-theme="..."]）が初めて効く。
      document.documentElement.dataset.theme = value;
    }
    // それ以外の未知キーは無視
  }
  setLoaded(true);
}

export const appearanceStore = { applyAppearance, loaded };
