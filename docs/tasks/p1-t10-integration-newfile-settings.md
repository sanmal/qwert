# p1-t10: 結合② — 新規ファイル + 設定パネル + キーボードショートカット

仕様書参照: §1 ファイルツリー（新規作成）、§19 キーボードショートカット、§21 Phase 1（タスク群6 結合）

> 旧 t08 を二分割した後半。App シェルの最終組み立て（signal 定義・単一 keydown ハンドラ・条件描画）をここで完成させる。これが Phase 1 の最終タスク。

## 前提

- p1-t09 完了済み（自動保存・外部変更検知が動作し、`editorStore` の保存系が配線済み）

## 作業内容

### 1. 新規ファイル作成（`src/components/FileTree.tsx`）

ファイルツリー上部に「+ 新規ファイル」ボタンを追加:

```typescript
import type { RelativePath } from "../types/brand";
import * as tauri from "../lib/tauri";
import { vaultStore } from "../stores/vault";

async function handleNewFile() {
  const name = prompt("ファイル名（拡張子 .md は自動付加）:");
  if (!name) return;
  const path = (name.endsWith(".md") ? name : `${name}.md`) as RelativePath;
  await tauri.createFile(path);
  await vaultStore.refreshFileTree();
  vaultStore.setSelectedFile(path);
}
```

`window.prompt` を使用（Phase 3 でカスタム UI に置換可能）。ボタンは `FileTree` の export するルート要素の先頭に置く。

### 2. 設定パネル（`src/components/SettingsPanel.tsx`）

```typescript
import { settingsStore } from "../stores/settings";

export function SettingsPanel() {
  return (
    <div class="settings-panel">
      <h2>設定</h2>
      <label>
        <input
          type="checkbox"
          checked={settingsStore.vimMode()}
          onChange={(e) => settingsStore.setVimMode(e.currentTarget.checked)}
        />
        Vim バインド（変更後は再起動が必要）
      </label>
      <label>
        <input
          type="checkbox"
          checked={settingsStore.syntaxHighlight()}
          onChange={(e) => settingsStore.setSyntaxHighlight(e.currentTarget.checked)}
        />
        構文ハイライト
      </label>
    </div>
  );
}
```

- 構文ハイライト On/Off は t08 の Editor が `createEffect` でライブ反映する。
- Vim 切替は再起動後反映（t08 で明記済み）。

> **B1/C10**: 設定の `config.toml` 永続化は **Phase 1 では行わない**（Phase 2）。Phase 1 は settingsStore のメモリ値のみ。アプリ再起動で初期値に戻る。`invoke("save_settings", ...)` は t05 に存在しないので呼ばない。
>
> **B4**: §2 必須機能のうち「行番号表示トグル」「テキスト折り返し設定」「タブ幅/インデント」は、CodeMirror 設定（t06/t08 は `basicSetup` 任せ）にも本パネルにも **Phase 1 では配線しない＝Phase 2 へ繰り延べ**る。Phase 2 で config.toml 配線（B1）と同時に CodeMirror の `Compartment` 経由で切り替える項目を追加する。Phase 1 のパネルは Vim 切替と構文ハイライト On/Off の2項目のみ。

### 3. App シェルの最終組み立て（`src/App.tsx`）

t08 の初期 App と t09 の追加（保存コールバック登録・file-changed リスナー・外部変更 signal）に、本タスクで **残りの signal・単一 keydown ハンドラ・条件描画**を足して完成させる。

#### signal 定義（App() 内）

t09 で定義した `showExternalChangeDialog` / `externalChangeFile` に加えて:

```typescript
import { createSignal } from "solid-js";
import type { ViewMode } from "./types/constants";
import { VIEW_MODE } from "./types/constants";

const [viewMode, setViewMode] = createSignal<ViewMode>(VIEW_MODE.SPLIT);
const [showSettings, setShowSettings] = createSignal(false);
const [showSidebar, setShowSidebar] = createSignal(true);
```

（t08 の `const [viewMode] = createSignal(...)` は setter 付きに置き換える。）

#### 単一 keydown ハンドラ（C6）

> **C6: keydown ハンドラは1つだけ登録し、`onCleanup` で解除する**（Ctrl+S・Ctrl+, の重複登録を防ぐ）。t09 までで keydown を一切足していないため、ここが唯一の登録箇所になる。

```typescript
import { onMount, onCleanup } from "solid-js";

onMount(() => {
  const onKey = (e: KeyboardEvent) => {
    if (!e.ctrlKey) return;
    switch (e.key) {
      case "s":
        e.preventDefault();
        void editorStore.saveCurrentFile();   // 即時保存（タイマーをクリアして書き込み）
        break;
      case ",":
        e.preventDefault();
        setShowSettings(v => !v);
        break;
      case "b":
        e.preventDefault();
        setShowSidebar(v => !v);
        break;
      case "e":
        e.preventDefault();
        setViewMode(current => {
          const modes = [VIEW_MODE.EDITOR, VIEW_MODE.SPLIT, VIEW_MODE.PREVIEW] as const;
          const idx = modes.indexOf(current);
          return modes[(idx + 1) % modes.length] ?? VIEW_MODE.SPLIT;
        });
        break;
    }
  };
  document.addEventListener("keydown", onKey);
  onCleanup(() => document.removeEventListener("keydown", onKey));
});
```

#### 最終 JSX

```tsx
import { Show } from "solid-js";
import { SettingsPanel } from "./components/SettingsPanel";
import { ExternalChangeDialog } from "./components/ExternalChangeDialog";

return (
  <div class="app-layout">
    <Show when={showSidebar()}>
      <div class="sidebar">
        <button onClick={vaultStore.openVault}>Vault を開く</button>
        <FileTree />
      </div>
    </Show>
    <div class="main-content">
      <Show when={viewMode() !== VIEW_MODE.PREVIEW}><Editor /></Show>
      <Show when={viewMode() !== VIEW_MODE.EDITOR}><Preview /></Show>
    </div>
    <StatusBar />

    <Show when={showSettings()}><SettingsPanel /></Show>

    <Show when={showExternalChangeDialog()}>
      <ExternalChangeDialog
        fileName={externalChangeFile()}
        onReload={() => {
          const f = vaultStore.selectedFile();
          if (f) void editorStore.loadFile(f);
          setShowExternalChangeDialog(false);
        }}
        onKeep={() => setShowExternalChangeDialog(false)}
      />
    </Show>
  </div>
);
```

複数の `onMount`（appearance 適用・保存コールバック登録・file-changed リスナー・keydown）は併存して構わない（Solid は全て実行する）。可読性のため1つの `onMount` にまとめてもよい。

## 完了基準（Phase 1 全体の到達点）

- 「+ 新規ファイル」でファイルを作成し、作成直後に選択状態になる
- `Ctrl+,` で設定パネルが開閉する。構文ハイライト On/Off は**即時**、Vim バインドは**再起動後**に反映される
- `Ctrl+B` でサイドバーの表示/非表示が切り替わる
- `Ctrl+E` で Editor / Split / Preview の表示モードが切り替わる
- `Ctrl+S` で即時保存される
- keydown ハンドラは単一登録で `onCleanup` 解除される（C6）
- 設定値は永続化されない（再起動で初期値 = B1）。行番号/折り返し/タブ幅 UI は Phase 2（B4）
