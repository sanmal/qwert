# p1-t04: qwert-core — appearance.rs

仕様書参照: §10 視覚設定とアクセシビリティ、§13 セキュリティ境界、§18 設定ファイル、§21 Phase 1（タスク群2「qwert-core 基盤」の appearance.rs + タスク群4「視覚設定基盤」の Rust サニタイザー）

## 前提

- p1-t02 完了済み（`error.rs`, `config.rs` が存在する）

## 追加する依存クレート

不要（既存の `serde`, `toml`, `thiserror`, `directories` で実装可能）。

## 作業内容

### 1. `crates/qwert-core/src/appearance.rs`

#### AppearanceConfig 構造体

`~/.config/qwert/appearance.toml`（グローバル）および `vault/.qwert/appearance.toml`（vaultスコープ）の両方を読む共通型。

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub text: TextConfig,
    pub color: ColorConfig,
    pub highlight: HighlightConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TextConfig {
    pub font_size: u32,              // default: 16
    pub font_family: String,         // default: "system-ui, sans-serif"
    pub line_height: f32,            // default: 1.6
    pub letter_spacing: f32,         // default: 0.0
    pub word_spacing: f32,           // default: 0.0
    pub editor_max_width: u32,       // default: 72 (ch unit)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    pub preset: Option<String>,      // "default" | "high-contrast" | "dark" | "dark-high-contrast"
    pub fg: Option<String>,          // hex color, must pair with bg
    pub bg: Option<String>,          // hex color, must pair with fg
    pub advanced: AdvancedColorConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedColorConfig {
    #[serde(rename = "cm-keyword")]
    pub cm_keyword: Option<String>,
    #[serde(rename = "cm-string")]
    pub cm_string: Option<String>,
    #[serde(rename = "cm-comment")]
    pub cm_comment: Option<String>,
    #[serde(rename = "cm-heading")]
    pub cm_heading: Option<String>,
    #[serde(rename = "cm-link")]
    pub cm_link: Option<String>,
    pub cursor: Option<String>,
    #[serde(rename = "selection-bg")]
    pub selection_bg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HighlightConfig {
    pub enabled: bool,   // default: true
}
```

**A5（重要）: `TextConfig` と `HighlightConfig` は手動で `impl Default` を書く。**

`AppearanceConfig` / `ColorConfig` / `AdvancedColorConfig` は全フィールドが `Option` か Default 済みなので `#[derive(Default)]` で正しい（`None` 既定で問題ない）。だが `TextConfig`（`font_size=16`, `line_height=1.6`, `editor_max_width=72` など非ゼロ）と `HighlightConfig`（`enabled=true`）は `#[derive(Default)]` だと `0`/`false`/`""` になり**仕様と異なる既定で静かに動く**（特に `highlight.enabled` が false 化、フォントサイズ 0 化）。`AppearanceConfig` の `#[derive(Default)]` は子の `Default` に従って合成されるため、下記が無いと既定が壊れる。

```rust
impl Default for TextConfig {
    fn default() -> Self {
        Self {
            font_size: 16,
            font_family: "system-ui, sans-serif".to_owned(),
            line_height: 1.6,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            editor_max_width: 72,
        }
    }
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
```

#### 主要関数

```rust
// appearance.toml を読む。存在しなければデフォルト返却。
pub fn load_global_appearance() -> crate::Result<AppearanceConfig>;

// vault スコープの appearance.toml を読む（Phase 3 でホットリロードを追加）
pub fn load_vault_appearance(vault_root: &std::path::Path) -> crate::Result<Option<AppearanceConfig>>;

// AppearanceConfig → CSS変数 HashMap に変換（サニタイズ済み）
// キー例: "--qw-font-size" → "16px"
pub fn to_css_vars(config: &AppearanceConfig) -> std::collections::HashMap<String, String>;
```

#### サニタイザー（セキュリティ境界 §13）

`to_css_vars` の実装でキー種別別の許可パターンを適用:

| キー種別 | 許可される値 |
|---------|-------------|
| 色（`fg`, `bg`, `cm-*`, `cursor`, `*-bg`, `*-fg`） | hex `#rrggbb` / `#rgb`, `rgb()`, `hsl()` |
| 寸法（`*spacing*`, `*height*`, `*size*`, `*width*`） | 数値 + px/em/rem/ch/% |
| フォント（`font-family`） | 英数字・スペース・ハイフン・カンマのみ |
| 上記以外 | 変換しない（スキップ） |

危険パターン（いずれか含む値は拒否）: `url(`, `expression(`, `javascript:`, `import`, `@`, `<`, `>`, `{`, `}`

検証は正規表現で行う。不正な値は無視してデフォルト値にフォールバック（ただしパースエラーは `CoreError` を返す）。

#### preset の扱い（A2、重要）

t06 のテーマ切替は **`:root[data-theme="dark"]` の属性セレクタ**で行う。一方フロントエンド（t07 appearanceStore）は受け取った map を `style.setProperty(key, value)` で流すだけ。したがって `to_css_vars` が preset を `--qw-preset` という **CSS 変数**として返すと、それを参照するルールが存在せず**テーマが切り替わらない**。

対策: **preset は CSS 変数ではなく、特例キー `data-theme` で返す**。フロントエンドは「`--` で始まらないキー」を特例として `document.documentElement.dataset.theme` にセットする（t07 appearanceStore で実装）。

```rust
// preset が Some("dark") のとき
//   map.insert("data-theme".to_owned(), "dark".to_owned());   // ← CSS 変数ではない特例キー
// preset が None かつ fg/bg が両方 Some のとき
//   map.insert("--qw-fg", ...); map.insert("--qw-bg", ...);   // data-theme は入れない
```

許可する preset 値は `"default" | "high-contrast" | "dark" | "dark-high-contrast"` のみ。これ以外はスキップ（不正値を `data-theme` に流さない）。

#### `to_css_vars` の出力例

```rust
// 入力: preset = "dark"
// → HashMap {
//     "data-theme": "dark",     // ← 特例キー（CSS 変数ではない）。frontend が dataset.theme にセット
// }

// 入力: fg = "#3a2418", bg = "#fdf6e3"（preset なし）
// → HashMap {
//     "--qw-fg": "#3a2418",
//     "--qw-bg": "#fdf6e3",
// }

// 入力: text.font_size = 18, text.line_height = 1.8
// → HashMap {
//     "--qw-font-size": "18px",
//     "--qw-line-height": "1.8",
//     ...
// }
```

preset と fg/bg の相互排他チェック: 両方が Some の場合は `CoreError` を返す（Phase 1 では警告ログのみ + preset 優先のフォールバックでも可）。

`load_vault_appearance` はファイルが存在しない場合は `Ok(None)` を返す。

### 2. `lib.rs` への追加

```rust
pub mod appearance;
```

### 3. テスト

- `to_css_vars` が危険パターンを含む値をスキップすること
- `to_css_vars` が `font-family` の英数字以外の値をスキップすること
- `to_css_vars` が hex カラー `#1a1a1a` を通過させること
- `to_css_vars` が `preset="dark"` を **`data-theme` キー**（`--qw-preset` ではない）で返すこと（A2）
- `to_css_vars` が不正な preset 値（例 `"neon"`）を `data-theme` に流さないこと
- preset + fg が両方 Some のとき相互排他チェックが働くこと

```bash
cargo test -p qwert-core
```

## 完了基準

- `crates/qwert-core/src/appearance.rs` が存在する
- `to_css_vars` が `url(`, `expression(`, `javascript:` を含む値を変換しない
- `cargo test -p qwert-core` がパスする
