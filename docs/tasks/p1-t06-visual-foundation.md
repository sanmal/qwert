# p1-t06: 視覚設定基盤 — CSS変数 + CodeMirror 6テーマ + prefers-color-scheme

仕様書参照: §10 視覚設定とアクセシビリティ、§21 Phase 1 タスク4

## 前提

- p1-t01 完了済み（TypeScript型基盤）
- pnpm でパッケージを追加する

## 追加パッケージ

```bash
pnpm add codemirror @codemirror/state @codemirror/view @codemirror/lang-markdown @codemirror/language-data @codemirror/language @codemirror/theme-one-dark @lezer/highlight solid-codemirror @replit/codemirror-vim
```

> A3: `highlight.ts` が import する `@codemirror/language`（`syntaxHighlighting`, `HighlightStyle`）と `@lezer/highlight`（`tags`）は**直接依存に必須**。pnpm は既定で hoisting しないため、直接依存に無いパッケージの import は解決失敗する。上記コマンドに両方含めてある。

## 作業内容

### 1. `src/styles/` ディレクトリ構成

```
src/styles/
  global.css
  tokens.css               # --qw-* CSS変数定義（全テーマ共通の変数名宣言）
  theme-default.css        # ライト背景・AA準拠
  theme-high-contrast.css  # ライト背景・AAA準拠（7:1以上）
  theme-dark.css           # ダーク背景・AA準拠
  theme-dark-high-contrast.css  # ダーク背景・AAA準拠
  editor.css
  preview.css
```

### 2. `src/styles/tokens.css` — CSS変数宣言

```css
:root {
  /* Typography */
  --qw-font-family: system-ui, sans-serif;
  --qw-font-size: 16px;
  --qw-font-weight: 400;
  --qw-line-height: 1.6;
  --qw-letter-spacing: 0em;
  --qw-word-spacing: 0em;
  --qw-paragraph-spacing: 1.5em;
  --qw-editor-max-width: 72ch;

  /* Colors — default (light, WCAG AA) */
  --qw-fg: #1a1a1a;
  --qw-bg: #ffffff;
  --qw-fg-muted: #6b7280;
  --qw-accent: #2563eb;

  /* Syntax highlight — Protanopia/Deuteranopia対応（明度差で区別） */
  --qw-cm-keyword: #7c3aed;
  --qw-cm-string: #059669;
  --qw-cm-comment: #9ca3af;
  --qw-cm-heading: #1e40af;
  --qw-cm-link: var(--qw-accent);
  --qw-cursor: var(--qw-fg);
  --qw-selection-bg: #dbeafe;

  /* UI */
  --qw-sidebar-width: 240px;
  --qw-border: #e5e7eb;
  --qw-surface: #f9fafb;
}
```

### 3. テーマCSS（プリセット4種）

#### `theme-default.css`
（tokens.css の `:root` がデフォルト値なので追加設定不要。ただし `data-theme="default"` セレクタで明示する）
```css
:root[data-theme="default"] {
  --qw-fg: #1a1a1a;
  --qw-bg: #ffffff;
  --qw-fg-muted: #6b7280;
  --qw-border: #e5e7eb;
  --qw-surface: #f9fafb;
}
```

コントラスト比確認: `#1a1a1a` on `#ffffff` = 約 16:1（AAA）。

#### `theme-high-contrast.css`
```css
:root[data-theme="high-contrast"] {
  --qw-fg: #000000;
  --qw-bg: #ffffff;
  --qw-fg-muted: #4b5563;
  --qw-accent: #1d4ed8;
  --qw-border: #6b7280;
  --qw-surface: #f3f4f6;
  --qw-cm-keyword: #5b21b6;
  --qw-cm-string: #065f46;
  --qw-cm-comment: #374151;
  --qw-cm-heading: #1e3a8a;
}
```

コントラスト比確認: `#000000` on `#ffffff` = 21:1（AAA）。

#### `theme-dark.css`
```css
:root[data-theme="dark"] {
  --qw-fg: #e5e7eb;
  --qw-bg: #1f2937;
  --qw-fg-muted: #9ca3af;
  --qw-accent: #60a5fa;
  --qw-border: #374151;
  --qw-surface: #111827;
  --qw-cm-keyword: #a78bfa;
  --qw-cm-string: #34d399;
  --qw-cm-comment: #6b7280;
  --qw-cm-heading: #93c5fd;
  --qw-selection-bg: #1e3a8a;
}
```

コントラスト比確認: `#e5e7eb` on `#1f2937` = 約 10:1（AAA）。

#### `theme-dark-high-contrast.css`
```css
:root[data-theme="dark-high-contrast"] {
  --qw-fg: #ffffff;
  --qw-bg: #000000;
  --qw-fg-muted: #d1d5db;
  --qw-accent: #93c5fd;
  --qw-border: #9ca3af;
  --qw-surface: #111827;
  --qw-cm-keyword: #c4b5fd;
  --qw-cm-string: #6ee7b7;
  --qw-cm-comment: #9ca3af;
  --qw-cm-heading: #bfdbfe;
  --qw-selection-bg: #1e40af;
}
```

### 4. `prefers-color-scheme` メディアクエリ（`tokens.css` に追記）

```css
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    --qw-fg: #e5e7eb;
    --qw-bg: #1f2937;
    --qw-fg-muted: #9ca3af;
    --qw-accent: #60a5fa;
    --qw-border: #374151;
    --qw-surface: #111827;
    --qw-cm-keyword: #a78bfa;
    --qw-cm-string: #34d399;
    --qw-cm-comment: #6b7280;
    --qw-cm-heading: #93c5fd;
    --qw-selection-bg: #1e3a8a;
  }
}

@media (prefers-contrast: more) {
  :root:not([data-theme]) {
    --qw-fg: #000000;
    --qw-bg: #ffffff;
    --qw-fg-muted: #374151;
  }
}
```

`data-theme` 属性が設定されている場合はメディアクエリを上書きしない（`:not([data-theme])`）。

### 5. `src/lib/codemirror/theme.ts` — CSS変数を参照する単一テーマ

```typescript
import { EditorView } from "@codemirror/view";

export const qwertTheme = EditorView.theme({
  "&": {
    backgroundColor: "var(--qw-bg)",
    color: "var(--qw-fg)",
    fontFamily: "var(--qw-font-family)",
    fontSize: "var(--qw-font-size)",
    lineHeight: "var(--qw-line-height)",
    maxWidth: "var(--qw-editor-max-width)",
  },
  ".cm-content": {
    caretColor: "var(--qw-cursor)",
    padding: "16px",
  },
  ".cm-cursor": { borderLeftColor: "var(--qw-cursor)" },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "var(--qw-selection-bg) !important",
  },
  ".cm-gutters": {
    backgroundColor: "var(--qw-surface)",
    color: "var(--qw-fg-muted)",
    border: "none",
  },
  "&.cm-focused": { outline: "none" },
}, { dark: false });
```

テーマの再構築は行わない。見た目の変更はCSS変数の値を切り替えるだけ。

### 6. `src/lib/codemirror/highlight.ts` — 構文ハイライト Compartment

```typescript
import { Compartment } from "@codemirror/state";
import { syntaxHighlighting, HighlightStyle } from "@codemirror/language";
import { tags } from "@lezer/highlight";

export const highlightCompartment = new Compartment();

export const qwertHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword,   color: "var(--qw-cm-keyword)" },
  { tag: tags.string,    color: "var(--qw-cm-string)" },
  { tag: tags.comment,   color: "var(--qw-cm-comment)", fontStyle: "italic" },
  { tag: tags.heading,   color: "var(--qw-cm-heading)", fontWeight: "bold" },
  { tag: tags.link,      color: "var(--qw-cm-link)" },
  { tag: tags.emphasis,  fontStyle: "italic" },
  { tag: tags.strong,    fontWeight: "bold" },
]);

// ハイライトOn（デフォルト）
export const highlightOn = highlightCompartment.of(
  syntaxHighlighting(qwertHighlightStyle)
);

// ハイライトOff
export const highlightOff = highlightCompartment.of([]);
```

### 7. `src/lib/codemirror/setup.ts` — CodeMirror 基本セットアップ

```typescript
import { basicSetup } from "codemirror";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { qwertTheme } from "./theme";
import { highlightOn } from "./highlight";

export function createBaseExtensions(vimMode: boolean) {
  const extensions = [
    basicSetup,
    markdown({ base: markdownLanguage, codeLanguages: languages }),
    qwertTheme,
    highlightOn,
  ];

  if (vimMode) {
    // vim() は動的インポートで遅延ロード（タスク08 Editor コンポーネントで実装）
  }

  return extensions;
}
```

### 8. `src/styles/global.css`

```css
@import "./tokens.css";
@import "./theme-default.css";
@import "./theme-high-contrast.css";
@import "./theme-dark.css";
@import "./theme-dark-high-contrast.css";

*, *::before, *::after {
  box-sizing: border-box;
}

html, body {
  margin: 0;
  padding: 0;
  background-color: var(--qw-bg);
  color: var(--qw-fg);
  font-family: var(--qw-font-family);
  font-size: var(--qw-font-size);
  line-height: var(--qw-line-height);
}
```

### 9. `src/index.tsx` の更新

```typescript
import "./styles/global.css";
```

## 完了基準

- `src/styles/` に7ファイルが存在する
- `src/lib/codemirror/` に `theme.ts`, `highlight.ts`, `setup.ts` が存在する
- デフォルトテーマのコントラスト比が WCAG AA（4.5:1）以上
- ダークテーマのコントラスト比が WCAG AA（4.5:1）以上
- ハイコントラストテーマのコントラスト比が WCAG AAA（7:1）以上
- `pnpm exec tsc --noEmit` がエラーなしで通る
