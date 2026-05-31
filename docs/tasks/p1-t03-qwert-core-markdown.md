# p1-t03: qwert-core — markdown.rs

仕様書参照: §3 Markdownプレビュー、§13 セキュリティ境界、§21 Phase 1 タスク2

## 前提

- p1-t02 完了済み（`error.rs`, `vault.rs`, `config.rs` が存在する）

## 追加する依存クレート（`crates/qwert-core/Cargo.toml`）

```toml
pulldown-cmark = { version = "0.12", features = ["simd", "html"] }
```

Phase 2 で math extension を有効化する際に `features` に `"math"` を追加する。

## 作業内容

### 1. `crates/qwert-core/src/markdown.rs`

#### 主要関数

```rust
// Markdown 文字列を sanitized HTML に変換する。
pub fn render_markdown(markdown: &str) -> String;
```

#### 実装詳細

**パーサーオプション**:
```rust
use pulldown_cmark::{Options, Parser, html};

let mut opts = Options::empty();
opts.insert(Options::ENABLE_TABLES);
opts.insert(Options::ENABLE_STRIKETHROUGH);
opts.insert(Options::ENABLE_TASKLISTS);
opts.insert(Options::ENABLE_FOOTNOTES);
opts.insert(Options::ENABLE_GFM);
// ENABLE_MATH は Phase 2 で追加
```

> C8: `Options::ENABLE_GFM` は pulldown-cmark 0.10+ に存在する（GFM の alert/admonition 等を有効化）。個別オプション（TABLES 等）と併用しても無害。万一コンパイルが通らない／GFM alert の挙動が不要なら、`ENABLE_GFM` 行を外して個別オプションのみで運用してよい（テーブル・取り消し線・タスクリストは個別オプションで担保される）。

**HTMLタグ除去（セキュリティ境界 §13）**:

pulldown-cmark の `Event::Html` と `Event::InlineHtml` イベントをフィルタして除去する。ユーザーが書いた生のHTMLタグ（`<script>`, `<iframe>`, `onclick=` 等）をレンダリング結果に含めない。

```rust
let parser = Parser::new_ext(markdown, opts)
    .filter(|event| !matches!(
        event,
        pulldown_cmark::Event::Html(_) | pulldown_cmark::Event::InlineHtml(_)
    ));
```

**HTML出力**:
```rust
let mut html_output = String::new();
html::push_html(&mut html_output, parser);
html_output
```

Phase 2 で DOMPurify による二重防御（フロントエンド側）を追加する。Phase 1 では上記のHTMLタグフィルタのみ。

### 2. `lib.rs` への追加

```rust
pub mod markdown;
```

### 3. テスト

`markdown.rs` にユニットテストを追加:
- GFM テーブルが `<table>` タグになること
- タスクリスト `- [ ]` が `<input type="checkbox">` になること
- 生の `<script>` タグが除去されること
- 生の `<iframe>` タグが除去されること
- `**bold**` が `<strong>bold</strong>` になること

```bash
cargo test -p qwert-core
```

## 完了基準

- `crates/qwert-core/src/markdown.rs` が存在する
- `render_markdown` が `<script>` / `<iframe>` / `<style>` タグを含まないHTMLを返す
- `cargo test -p qwert-core` がパスする
