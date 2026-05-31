import { createSignal } from "solid-js";

// C10/B1: Phase 1 は config.toml と接続しない。初期値はハードコード（仕様書 §18 の既定に合わせる）、
// 変更はセッション内メモリのみで永続化しない。config.toml からの load/save 配線は Phase 2。
const [vimMode, setVimMode] = createSignal(false);          // §18 [editor] vim_mode 既定 false
const [syntaxHighlight, setSyntaxHighlight] = createSignal(true);  // §10 [highlight] enabled 既定 true

export const settingsStore = {
  vimMode,
  setVimMode,
  syntaxHighlight,
  setSyntaxHighlight,
};
