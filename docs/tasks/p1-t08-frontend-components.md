# p1-t08: SolidJSフロントエンド — コンポーネント層 + 初期レイアウト

仕様書参照: §1 ファイルツリー、§2 テキストエディタ、§3 Markdownプレビュー、§19 キーボードショートカット、§21 Phase 1（タスク群5 フロントエンド）

> 旧 t07 を二分割した後半。t07 のストアを参照する UI コンポーネントと、起動時に動く最小レイアウト（Vault を開く → ツリー → エディタ + プレビュー）を作る。自動保存・外部変更検知・設定・ショートカットの結合は t09/t10。

## 前提

- p1-t06 完了済み（`src/lib/codemirror/` のテーマ・ハイライト、`--qw-*` CSS変数が存在する）
- p1-t07 完了済み（`src/stores/` の4ストアが存在する）

## 作業内容

### 1. `src/components/FileTree.tsx`

- `vaultStore.fileTree()` を再帰的にツリー表示
- `.is_dir` が true なら `▶ / ▼` トグル付きフォルダ
- ファイルをクリックで `vaultStore.setSelectedFile(entry.path)`
- 選択中ファイルを `data-selected` でハイライト

```typescript
import { For, Show, createSignal } from "solid-js";
import type { FileEntry } from "../lib/tauri";
import { vaultStore } from "../stores/vault";

// C5: depth を伝播させてネストの字下げを効かせる。未使用の Props 型は作らない。
function FileTreeItem(props: { entry: FileEntry; depth: number }) {
  const [expanded, setExpanded] = createSignal(true);
  // entry.path は Rust 側が返す RelativePath（branded 済み）なので再キャスト不要。
  const isSelected = () => vaultStore.selectedFile() === props.entry.path;

  return (
    <div style={{ "padding-left": `${props.depth * 16}px` }}>
      <Show when={props.entry.is_dir}>
        <div class="tree-folder" onClick={() => setExpanded(v => !v)}>
          {expanded() ? "▼" : "▶"} {props.entry.name}
        </div>
        <Show when={expanded() && props.entry.children}>
          <For each={props.entry.children}>
            {child => <FileTreeItem entry={child} depth={props.depth + 1} />}
          </For>
        </Show>
      </Show>
      <Show when={!props.entry.is_dir}>
        <div
          class="tree-file"
          data-selected={isSelected()}
          onClick={() => vaultStore.setSelectedFile(props.entry.path)}
        >
          {props.entry.name}
        </div>
      </Show>
    </div>
  );
}

export function FileTree() {
  return (
    <div class="file-tree">
      <For each={vaultStore.fileTree()}>
        {entry => <FileTreeItem entry={entry} depth={0} />}
      </For>
    </div>
  );
}
```

> C5: `FileEntry.path` は t05 のラッパーで `RelativePath`（branded）型として定義済みなので、`setSelectedFile` に渡すときの `as RelativePath` キャストは不要。`vaultStore.setSelectedFile` の引数型も `RelativePath` に揃っている。
>
> 「+ 新規ファイル」ボタンは t10 で追加する（本タスクは表示のみ）。

### 2. `src/components/Editor.tsx`

- CodeMirror 6 を手動マウント
- `settingsStore.syntaxHighlight()` の変更を `highlightCompartment` でライブ反映
- `settingsStore.vimMode()` は **onMount 時の値で初期化**（ライブ切替はしない＝再起動反映）
- テキスト変更時に `editorStore.onChange(value)` を呼ぶ（プログラム的差し替え時は除外）

```typescript
import { onMount, onCleanup, createEffect } from "solid-js";
import { EditorView } from "@codemirror/view";
import { EditorState, type Extension } from "@codemirror/state";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { basicSetup } from "codemirror";
import { qwertTheme } from "../lib/codemirror/theme";
import {
  highlightCompartment,
  highlightOn,
  highlightOff,
} from "../lib/codemirror/highlight";
import { editorStore } from "../stores/editor";
import { settingsStore } from "../stores/settings";

export function Editor() {
  let containerRef!: HTMLDivElement;
  // C4: strict での未代入参照を避けるため undefined 許容にする。
  let view: EditorView | undefined;
  // プログラム的なコンテンツ差し替え（ファイル切替）中は onChange を発火させない。
  // これをしないと、読み込み直後に docChanged → UNSAVED → 自動保存が走り、
  // 開いた瞬間に「未保存」扱い＆自動上書きが発生する。
  let applyingRemote = false;

  onMount(async () => {
    // C4: vim() は basicSetup より「前」に置く（後ろに push するとキーバインド衝突の元）。
    const extensions: Extension[] = [];
    if (settingsStore.vimMode()) {
      const { vim } = await import("@replit/codemirror-vim");
      extensions.push(vim());
    }
    extensions.push(
      basicSetup,
      markdown({ base: markdownLanguage, codeLanguages: languages }),
      qwertTheme,
      settingsStore.syntaxHighlight() ? highlightOn : highlightOff,
      EditorView.updateListener.of(update => {
        if (update.docChanged && !applyingRemote) {
          editorStore.onChange(update.state.doc.toString());
        }
      }),
    );

    view = new EditorView({
      state: EditorState.create({ doc: editorStore.content(), extensions }),
      parent: containerRef,
    });
  });

  // syntaxHighlight 設定変更の追従（ライブ）
  createEffect(() => {
    const on = settingsStore.syntaxHighlight();
    if (!view) return;
    view.dispatch({
      effects: highlightCompartment.reconfigure(on ? highlightOn : highlightOff),
    });
  });

  // 外部からコンテンツが変わったとき（ファイル切替）に反映
  createEffect(() => {
    const doc = editorStore.content();
    if (!view) return;
    if (view.state.doc.toString() !== doc) {
      applyingRemote = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: doc },
      });
      applyingRemote = false;
    }
  });

  onCleanup(() => view?.destroy());

  return <div ref={containerRef} class="editor-container" />;
}
```

注意:
- **Vim モード切替は Phase 1 ではアプリ再起動が必要**（動的な付け外しは実装コストが高い）。`onMount` 時の `settingsStore.vimMode()` の値で初期化する。
- 自己 onChange 抑制は `applyingRemote` フラグで行う。より堅牢にするなら CodeMirror の `Annotation` を使う方法もあるが、Phase 1 はフラグ方式で十分（dispatch は同期的なため競合しない）。

### 3. `src/components/Preview.tsx`

- `editorStore.content()` を `renderMarkdown` で HTML 化し表示
- `innerHTML` で注入（Rust 側で生 HTML タグ除去済み）
- 150ms デバウンスで再レンダリング

```typescript
import { createSignal, createEffect } from "solid-js";
import { editorStore } from "../stores/editor";
import * as tauri from "../lib/tauri";

export function Preview() {
  const [html, setHtml] = createSignal("");
  let renderTimer: ReturnType<typeof setTimeout>;

  createEffect(() => {
    const md = editorStore.content();
    clearTimeout(renderTimer);
    renderTimer = setTimeout(async () => {
      const rendered = await tauri.renderMarkdown(md);
      setHtml(rendered);
    }, 150);
  });

  return <div class="preview-container" innerHTML={html()} />;
}
```

### 4. `src/components/StatusBar.tsx`

```typescript
import { SAVE_STATE } from "../types/constants";
import { editorStore } from "../stores/editor";
import { vaultStore } from "../stores/vault";

export function StatusBar() {
  const saveLabel = () => {
    switch (editorStore.saveState()) {
      case SAVE_STATE.SAVING: return "保存中...";
      case SAVE_STATE.UNSAVED: return "未保存";
      default: return "保存済み";
    }
  };

  return (
    <div class="status-bar">
      <span>{vaultStore.selectedFile() ?? ""}</span>
      <span>{saveLabel()}</span>
    </div>
  );
}
```

### 5. `src/App.tsx`（初期レイアウト）

Split View（サイドバー + エディタ + プレビュー）。自動保存・外部変更検知・設定・ショートカットは t09/t10 で足すので、本タスクは表示と選択→ロードのみ。

```typescript
import { createEffect, createSignal, onMount, Show } from "solid-js";
import type { ViewMode } from "./types/constants";
import { VIEW_MODE } from "./types/constants";
import { FileTree } from "./components/FileTree";
import { Editor } from "./components/Editor";
import { Preview } from "./components/Preview";
import { StatusBar } from "./components/StatusBar";
import { vaultStore } from "./stores/vault";
import { editorStore } from "./stores/editor";
import { appearanceStore } from "./stores/appearance";

export default function App() {
  const [viewMode] = createSignal<ViewMode>(VIEW_MODE.SPLIT);

  onMount(() => {
    appearanceStore.applyAppearance();
  });

  // ファイル選択時にエディタへロード
  createEffect(() => {
    const file = vaultStore.selectedFile();
    if (file) editorStore.loadFile(file);
  });

  return (
    <div class="app-layout">
      <div class="sidebar">
        <button onClick={vaultStore.openVault}>Vault を開く</button>
        <FileTree />
      </div>
      <div class="main-content">
        <Show when={viewMode() !== VIEW_MODE.PREVIEW}><Editor /></Show>
        <Show when={viewMode() !== VIEW_MODE.EDITOR}><Preview /></Show>
      </div>
      <StatusBar />
    </div>
  );
}
```

> t10 で `viewMode` の setter・`showSidebar`・`showSettings`・外部変更ダイアログの signal とキーボード操作を追加し、この JSX を拡張する。

### 6. CSS

**`src/styles/editor.css`**:
```css
.editor-container { flex: 1; overflow: auto; background-color: var(--qw-bg); }
.editor-container .cm-editor { height: 100%; }
```

**`src/styles/preview.css`**:
```css
.preview-container {
  flex: 1;
  overflow: auto;
  padding: 16px;
  background-color: var(--qw-bg);
  color: var(--qw-fg);
  font-family: var(--qw-font-family);
  font-size: var(--qw-font-size);
  line-height: var(--qw-line-height);
  max-width: var(--qw-editor-max-width);
}
```

**`src/App.css`**（レイアウト）:
```css
.app-layout { display: flex; flex-direction: column; height: 100vh; }
.sidebar {
  width: var(--qw-sidebar-width);
  overflow-y: auto;
  background: var(--qw-surface);
  border-right: 1px solid var(--qw-border);
}
.main-content { display: flex; flex: 1; overflow: hidden; }
.status-bar {
  display: flex;
  justify-content: space-between;
  padding: 4px 16px;
  background: var(--qw-surface);
  border-top: 1px solid var(--qw-border);
  font-size: 12px;
  color: var(--qw-fg-muted);
}
```

`editor.css` / `preview.css` は `global.css`（t06）に `@import` 追加するか、各コンポーネントで import する。

## 完了基準

- `pnpm tauri dev` でアプリが起動する
- 「Vault を開く」でフォルダ選択ダイアログが開き、選択するとファイルツリーが表示される
- ファイルをクリックするとエディタにコンテンツが表示される
- Split View でエディタ左・プレビュー右が並ぶ
- エディタで入力するとプレビューが更新される
- StatusBar に選択ファイル名と保存状態が表示される
- `prefers-color-scheme: dark` の環境でダークテーマになる
- `pnpm exec tsc --noEmit` がエラーなしで通る
