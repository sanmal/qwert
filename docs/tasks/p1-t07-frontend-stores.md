# p1-t07: SolidJSフロントエンド — ストア層

仕様書参照: §1 ファイルツリー、§2 テキストエディタ、§10 視覚設定、§21 Phase 1（タスク群5 フロントエンド）

> 旧 t07 を「ストア層（本タスク）」と「コンポーネント層（t08）」に二分割した前半。状態管理だけを先に固め、コンポーネント（t08）から参照する。

## 前提

- p1-t05 完了済み（`src/lib/tauri.ts` に全コマンドラッパーが存在する）
- p1-t01 完了済み（`src/types/` の brand / constants が存在する）

## 作業内容

`src/stores/` に4ストアを作成する。SolidJS の `createSignal` をモジュールスコープで持ち、シングルトンとして export する。

### `src/stores/vault.ts`

```typescript
import { createSignal } from "solid-js";
import type { FileEntry } from "../lib/tauri";
import type { RelativePath, AbsolutePath } from "../types/brand";
import * as tauri from "../lib/tauri";

const [vaultRoot, setVaultRoot] = createSignal<AbsolutePath | null>(null);
const [fileTree, setFileTree] = createSignal<FileEntry[]>([]);
const [selectedFile, setSelectedFile] = createSignal<RelativePath | null>(null);

async function openVault() {
  const root = await tauri.openVaultDialog();
  if (root) {
    setVaultRoot(root);
    await refreshFileTree();
  }
}

async function refreshFileTree() {
  const entries = await tauri.listDir();
  setFileTree(entries);
}

export const vaultStore = {
  vaultRoot,
  fileTree,
  selectedFile,
  setSelectedFile,
  openVault,
  refreshFileTree,
};
```

### `src/stores/editor.ts`

```typescript
import { createSignal } from "solid-js";
import type { RelativePath } from "../types/brand";
import type { SaveState } from "../types/constants";
import { SAVE_STATE } from "../types/constants";
import * as tauri from "../lib/tauri";

const [content, setContent] = createSignal("");
const [saveState, setSaveState] = createSignal<SaveState>(SAVE_STATE.SAVED);
let autosaveTimer: ReturnType<typeof setTimeout> | undefined;
let saveCallback: (() => Promise<void>) | undefined;

async function loadFile(path: RelativePath) {
  const text = await tauri.readFile(path);
  setContent(text);
  setSaveState(SAVE_STATE.SAVED);
}

function onChange(newContent: string) {
  setContent(newContent);
  setSaveState(SAVE_STATE.UNSAVED);
  scheduleAutosave();
}

// 実際の書き込み処理は App.tsx 側で登録する（t09 で配線）。
function registerSaveCallback(cb: () => Promise<void>) {
  saveCallback = cb;
}

function scheduleAutosave(delayMs = 3000) {
  clearTimeout(autosaveTimer);
  autosaveTimer = setTimeout(async () => {
    if (!saveCallback) return;
    setSaveState(SAVE_STATE.SAVING);
    await saveCallback();
    setSaveState(SAVE_STATE.SAVED);
  }, delayMs);
}

// 即時保存（Ctrl+S 用）。タイマーをクリアして即座に書き込む。
async function saveCurrentFile() {
  clearTimeout(autosaveTimer);
  if (!saveCallback) return;
  setSaveState(SAVE_STATE.SAVING);
  await saveCallback();
  setSaveState(SAVE_STATE.SAVED);
}

export const editorStore = {
  content,
  saveState,
  loadFile,
  onChange,
  registerSaveCallback,
  scheduleAutosave,
  saveCurrentFile,
};
```

> autosave のデフォルト遅延は 3000ms（仕様書 §18 `autosave_delay_ms`）をハードコード。config.toml からの読み込みは Phase 2（B1）。

### `src/stores/settings.ts`

```typescript
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
```

### `src/stores/appearance.ts`

```typescript
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
```

> A2: Rust の `to_css_vars`（t04）は preset を `--qw-preset` ではなく特例キー `data-theme` で返す。ここで `dataset.theme` にセットすることで t06 の属性セレクタ式テーマが切り替わる。`--` で始まるキーだけ `setProperty` する分岐が必須。

## 完了基準

- `src/stores/` に `vault.ts` / `editor.ts` / `settings.ts` / `appearance.ts` が存在する
- 各ストアが branded 型（`RelativePath` / `AbsolutePath`）を正しく使っている
- `editorStore` が `registerSaveCallback` / `scheduleAutosave` / `saveCurrentFile` を export している（t09 で配線）
- `pnpm exec tsc --noEmit` がエラーなしで通る（コンポーネント未作成でもストア単体で型が通る）
