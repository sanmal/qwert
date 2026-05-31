# p1-t02: qwert-core — vault.rs + config.rs

仕様書参照: §18 設定ファイル、§21 Phase 1 タスク2

## 前提

- `crates/qwert-core/` ディレクトリと `Cargo.toml` が存在する
- 現在 `src/lib.rs` には `add()` のスキャフォールドのみ

## 追加する依存クレート（`crates/qwert-core/Cargo.toml`）

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"
walkdir = "2"
notify = "7"          # B3: ファイル監視は qwert-core 側に置く（§15 準拠）。本タスクで watch API を実装
directories = "5"
tempfile = "3"
```

> B3（watcher の配置）: 仕様書 §15 は vault.rs（qwert-core）が「スキャン・**ウォッチ**・アトミック書き込み」を担うと定める。これに従い `notify` は **qwert-core 側にのみ**置き、本タスクで `vault::watch_vault` を実装する。src-tauri 側（t05/t09）は qwert-core の watch API を呼んで Tauri イベントを emit するグルーに徹し、`notify` を **重複宣言しない**。

## 作業内容

### 1. `crates/qwert-core/src/error.rs`

Phase 1 で必要な最小限のエラー型（Phase 2 でフィールド追加予定）:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Path traversal detected: {0}")]
    PathTraversal(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
```

### 2. `crates/qwert-core/src/vault.rs`

#### VaultEntry 型

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub name: String,
    pub path: String,       // vault-relative path (forward slash)
    pub is_dir: bool,
    pub children: Option<Vec<VaultEntry>>,
}
```

#### パストラバーサル防止

**重要（A4）**: `canonicalize()` は **実在するパスにしか効かない**。既存ファイル用の `resolve_path` だけでは新規作成（`create_file`、新規パスへの `write_file`）でトラバーサル検証ができない（`canonicalize` が NotFound を返す）。そこで **2 経路**に分ける。

既存パス用（read / 既存 write）:
```rust
pub fn resolve_path(vault_root: &Path, relative: &str) -> crate::Result<PathBuf> {
    let joined = vault_root.join(relative);
    let canonical = joined.canonicalize().map_err(|_| {
        crate::CoreError::NotFound(relative.to_owned())
    })?;
    if !canonical.starts_with(vault_root) {
        return Err(crate::CoreError::PathTraversal(relative.to_owned()));
    }
    Ok(canonical)
}
```

新規パス用（create / 新規 write）— 親ディレクトリを canonicalize して vault 配下か検証し、ファイル名を結合する:
```rust
pub fn resolve_new_path(vault_root: &Path, relative: &str) -> crate::Result<PathBuf> {
    // 1. lexical 検証: `..` 成分・絶対パスを拒否
    let rel = Path::new(relative);
    if rel.is_absolute()
        || rel.components().any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(crate::CoreError::PathTraversal(relative.to_owned()));
    }
    let joined = vault_root.join(rel);
    // 2. 親ディレクトリ（実在する最も近い祖先）を canonicalize して vault 配下か検証
    let parent = joined.parent().ok_or_else(|| {
        crate::CoreError::PathTraversal(relative.to_owned())
    })?;
    // 親がまだ無ければ create_dir_all してから canonicalize する（create_file 側で実施）
    let parent_canonical = parent.canonicalize().map_err(|_| {
        crate::CoreError::NotFound(parent.to_string_lossy().into_owned())
    })?;
    if !parent_canonical.starts_with(vault_root) {
        return Err(crate::CoreError::PathTraversal(relative.to_owned()));
    }
    let file_name = joined.file_name().ok_or_else(|| {
        crate::CoreError::PathTraversal(relative.to_owned())
    })?;
    Ok(parent_canonical.join(file_name))
}
```

使い分け:
- `read_file` / 既存ファイルへの `write_file` → `resolve_path`
- `create_file` / 新規ファイルへの `write_file` → 親を `create_dir_all` で用意してから `resolve_new_path`

`vault_root` 自体も `canonicalize()` 済みである前提。Tauri コマンド層（タスク05）でvault開始時に一度正規化する。

#### 主要関数

```rust
// .md ファイルのツリースキャン（再帰）
pub fn scan_vault(vault_root: &Path) -> crate::Result<Vec<VaultEntry>>;

// ファイル読み取り（パストラバーサル検証付き）
pub fn read_file(vault_root: &Path, relative_path: &str) -> crate::Result<String>;

// アトミック書き込み（tmp → rename）
pub fn write_file(vault_root: &Path, relative_path: &str, content: &str) -> crate::Result<()>;

// ファイル新規作成（すでに存在する場合はエラー）
pub fn create_file(vault_root: &Path, relative_path: &str) -> crate::Result<()>;
```

#### ファイル監視 API（B3、§15 準拠）

`notify` を qwert-core に閉じ込め、変更を**コールバックで受け渡す**だけにする（Tauri 依存を持ち込まない）。返り値の guard を保持している間だけ監視が継続する（drop で停止）。

```rust
use std::path::PathBuf;

/// 監視ハンドル。drop されると監視スレッドと watcher が停止する。
pub struct WatchGuard {
    _watcher: notify::RecommendedWatcher,
    // 監視スレッドは guard 内の watcher が drop されると mpsc が閉じて自然終了する
}

/// vault_root 配下を再帰監視し、変更のあった .md ファイルの
/// vault 相対パス（`/` 区切り）ごとに callback を呼ぶ。
/// callback はバックグラウンドスレッドから呼ばれる（Send + 'static 必須）。
pub fn watch_vault<F>(vault_root: &Path, callback: F) -> crate::Result<WatchGuard>
where
    F: Fn(String) + Send + 'static;
```

- src-tauri 側（t09）はこの `callback` の中で「自己書き込み抑制（B2）」を判定し、通過したものだけ `app.emit("file-changed", relative)` する。
- qwert-core は「どのパスが変わったか」を伝えるのみで、emit も抑制判定も知らない。

#### 実装詳細

- `scan_vault`: `walkdir::WalkDir` で再帰走査。`VaultEntry.path` はvaultルートからの相対パス（`/` 区切り）。`.md` および `.markdown` 拡張子のみを含める。隠しディレクトリ（`.git`, `.jj`, `.qwert/cache`, `node_modules`）はスキップ。
- `read_file`: `resolve_path`（既存パス用）で検証してから読む。
- `write_file`: 既存パスは `resolve_path`、新規パスは親を用意してから `resolve_new_path` で検証。`tempfile::NamedTempFile` で同一ディレクトリに書き込み、`persist()` で rename。
- `create_file`: `resolve_new_path` で検証。親ディレクトリが存在しなければ `create_dir_all` で作成後、`OpenOptions::new().create_new(true)` で作成。
- `watch_vault`: `notify::RecommendedWatcher` を生成して `RecursiveMode::Recursive` で監視。内部スレッドで mpsc を回し、`.md`/`.markdown` のイベントだけ vault 相対パスに変換して callback を呼ぶ。watcher を `WatchGuard` に格納して返す（呼び出し側が保持しないと即停止する点に注意）。

### 3. `crates/qwert-core/src/config.rs`

> **B1（重要）: Phase 1 では config.toml の読み書きはしない。** 本タスクで作るのは **型定義 + 手動 `impl Default` + そのユニットテスト**まで。`load_config` / `save_config` / `config_path` および Tauri コマンド配線は **Phase 2 へ繰り延べ**る（フロントエンドの設定はセッション内メモリのみで動かす。詳細は t07 ストア・t10 設定パネルを参照）。これにより「設定機構を作ったのにアプリから読み書きされない」断絶を避ける。

#### Config 構造体（Phase 1 必要最小限）

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub editor: EditorConfig,
    pub preview: PreviewConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub restore_last_vault: bool,
    pub autosave_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub tab_size: u32,
    pub use_spaces: bool,
    pub vim_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    pub default_view: String,  // "editor" | "split" | "preview"
    pub sync_scroll: bool,
    pub render_mermaid: bool,
    pub render_math: bool,
}
```

**A5（重要）: `#[derive(Default)]` は使わない。手動で `impl Default` を書く。**

既定値が非ゼロ（`autosave_delay_ms=3000`, `tab_size=4`, `default_view="split"` 等）かつ bool が `true` 既定（`show_line_numbers`, `word_wrap`, `use_spaces`, `vim_mode=false`, `render_*`, `sync_scroll`, `restore_last_vault`）。`#[derive(Default)]` だと `0` / `false` / `""` になり、**仕様と異なる既定で静かに動く**（特に `default_view=""` は表示モード判定を壊す）。`#[serde(default)]`（コンテナ属性）は欠落フィールドを各型の `Default` で埋めるため、下記の手動実装が必須。

```rust
impl Default for GeneralConfig {
    fn default() -> Self {
        Self { restore_last_vault: true, autosave_delay_ms: 3000 }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            word_wrap: true,
            tab_size: 4,
            use_spaces: true,
            vim_mode: false,
        }
    }
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            default_view: "split".to_owned(),
            sync_scroll: true,
            render_mermaid: true,
            render_math: true,
        }
    }
}

// Config 自体は全フィールドが Default を持つので #[derive(Default)] で可。
// ただし上記の子の手動 Default に依存する点に注意。
impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            editor: EditorConfig::default(),
            preview: PreviewConfig::default(),
        }
    }
}
```

#### 主要関数

> **B1: Phase 1 では実装しない。** 以下は **Phase 2** で実装する（XDG 読み書き + Tauri コマンド配線）。Phase 1 ではシグネチャも実装も追加しない。

```rust
// --- 以下はすべて Phase 2 ---
// pub fn load_config() -> crate::Result<Config>;          // ~/.config/qwert/config.toml を読む
// pub fn save_config(config: &Config) -> crate::Result<()>;
// pub fn config_path() -> Option<std::path::PathBuf>;     // directories::ProjectDirs 使用
```

### 4. `crates/qwert-core/src/lib.rs` の更新

```rust
pub mod config;
pub mod error;
pub mod vault;

pub use error::{CoreError, Result};
```

### 5. テスト

各モジュールにユニットテストを追加:
- `vault.rs`: `resolve_path` がパストラバーサル（`../` 脱出）を拒否することを確認
- `vault.rs`: `resolve_new_path` が `..`・絶対パスを拒否し、vault 配下の新規パスを通すことを確認（A4）
- `vault.rs`: `scan_vault` が `.md` のみ返すことを確認（`tempdir` を使用）
- `vault.rs`: `write_file` がアトミックに書き込まれることを確認
- `vault.rs`: `create_file` が既存ファイルに対してエラーを返すことを確認
- `config.rs`: 手動 `impl Default` の各値が仕様どおり（`autosave_delay_ms=3000`, `tab_size=4`, `default_view="split"`, 各 bool）であることを確認（A5）
- `config.rs`: 空 TOML 文字列を `toml::from_str::<Config>("")` でパースしても上記既定値になることを確認（`#[serde(default)]` の検証）

> `watch_vault` のテストは inotify のタイミング依存で不安定になりやすいため Phase 1 では必須としない（手動確認は t09 の結合で行う）。

```bash
cargo test -p qwert-core
```

## 完了基準

- `cargo test -p qwert-core` がパスする
- `crates/qwert-core/src/` に `error.rs`, `vault.rs`, `config.rs` が存在する
- `vault.rs` に `resolve_path` / `resolve_new_path` / `watch_vault`（WatchGuard 返却）が存在する
- `config.rs` は **型定義 + 手動 impl Default + テストのみ**（`load_config`/`save_config` は未実装 = Phase 2）
- `lib.rs` が `config` / `error` / `vault` をエクスポートしている
- `cargo build -p qwert-core` が警告なしでビルドできる
- **設定の永続化は Phase 1 ではしない**（config.toml の読み書きは Phase 2）
