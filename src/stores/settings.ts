import { createSignal } from "solid-js";
import type { Keybindings } from "../lib/tauri";
import * as tauri from "../lib/tauri";

// §18 既定値。config.toml と一致させること（Rust 側 KeybindingsConfig::default() が正）。
const KB_DEFAULTS: Keybindings = {
  save: "Ctrl+S",
  new_note: "Ctrl+N",
  command_palette: "Ctrl+P",
  full_search: "Ctrl+Shift+F",
  view_mode_toggle: "Ctrl+E",
  sidebar_toggle: "Ctrl+B",
  settings: "Ctrl+,",
};

// C10/B1: Phase 1 は config.toml と接続しない。初期値はハードコード（仕様書 §18 の既定に合わせる）、
// 変更はセッション内メモリのみで永続化しない。config.toml からの load/save 配線は Phase 2。
const [vimMode, setVimMode] = createSignal(false);          // §18 [editor] vim_mode 既定 false
const [syntaxHighlight, setSyntaxHighlight] = createSignal(true);  // §10 [highlight] enabled 既定 true
const [keybindings, setKeybindings] = createSignal<Keybindings>({ ...KB_DEFAULTS });

async function loadKeybindings(): Promise<void> {
  try {
    const kb = await tauri.getKeybindings();
    setKeybindings(kb);
  } catch {
    // keep in-memory defaults on error (e.g. no vault / no config.toml yet)
  }
}

export const settingsStore = {
  vimMode,
  setVimMode,
  syntaxHighlight,
  setSyntaxHighlight,
  keybindings,
  setKeybindings,
  loadKeybindings,
};
