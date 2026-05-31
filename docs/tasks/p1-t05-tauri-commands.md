# p1-t05: Tauriコマンド層 + TypeScriptラッパー

仕様書参照: §15 アーキテクチャ、§17 TypeScript型安全基盤、§21 Phase 1 タスク3

## 前提

- p1-t01 完了済み（`src/types/` が存在する）
- p1-t02, p1-t03, p1-t04 完了済み（qwert-core の全モジュールが存在する）

## 追加する依存クレート（`src-tauri/Cargo.toml`）

```toml
[dependencies]
tauri = { version = "2", features = [] }   # C7: 画像プレビューを入れる Phase 2 で features = ["protocol-asset"] を追加（Phase 1 は不要）
tauri-plugin-opener = "2"
tauri-plugin-dialog = "2"    # 追加: Vault選択ダイアログ
serde = { version = "1", features = ["derive"] }
serde_json = "1"
qwert-core = { path = "../crates/qwert-core" }
```

> B3: `notify` は **src-tauri に追加しない**。ファイル監視は qwert-core の `vault::watch_vault`（t02 で実装）を呼ぶ。src-tauri 側はそのコールバック内で Tauri イベントを emit するだけ（実際の配線は t09）。
>
> C7: 仕様書 §16 の `protocol-asset` feature はローカル画像表示用。Phase 1 は画像表示が無いため省略してよい（Phase 2 で追加）。

## 作業内容

### 1. `src-tauri/src/commands/` ディレクトリ構成

```
src-tauri/src/commands/
  mod.rs
  file.rs      # list_dir, read_file, write_file, create_file
  vault.rs     # open_vault, get_vault_root
  markdown.rs  # render_markdown
  appearance.rs # load_appearance
```

### 2. `src-tauri/src/commands/file.rs`

```rust
use qwert_core::vault;
use serde::{Deserialize, Serialize};
use tauri::State;

// アプリ状態（後述の AppState を参照）
use crate::AppState;

#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

#[tauri::command]
pub fn list_dir(state: State<'_, AppState>) -> Result<Vec<FileEntry>, String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::scan_vault(root)
        .map(|entries| entries.into_iter().map(FileEntry::from).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::read_file(root, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, content: String, state: State<'_, AppState>) -> Result<(), String> {
    // B2: 自己書き込み抑制 — notify イベントが書き込み完了より先に届くレースを避けるため、
    // 書き込みの「前」に記録する。watcher 側は一定時間内の同一パスを無視する（t09 で判定）。
    // 失敗時に残った記録は次の正当な外部変更を1回握り潰す可能性があるが、影響は軽微（窓は短い）。
    state
        .recent_writes
        .lock()
        .unwrap()
        .insert(path.clone(), std::time::Instant::now());

    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::write_file(root, &path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_file(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::create_file(root, &path).map_err(|e| e.to_string())
}
```

`FileEntry::from(qwert_core::vault::VaultEntry)` の変換を `impl From<...> for FileEntry` として実装する（再帰的に children を変換）。

### 3. `src-tauri/src/commands/vault.rs`

```rust
#[tauri::command]
pub fn get_vault_root(state: State<'_, AppState>) -> Option<String> {
    state.vault_root.lock().unwrap()
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
}

// tauri-plugin-dialog でフォルダ選択ダイアログを開く
#[tauri::command]
pub async fn open_vault_dialog(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    // A1: v2 の pick_folder はコールバック型で Future を返さない（.await できない）。
    // async コマンドでは blocking 版を使う。async コマンドは別スレッドで走るためブロックしてよい。
    let Some(fp) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    // A1: FilePath → PathBuf は .into_path()（Result を返す）。.to_path_buf() ではない。
    let path = fp.into_path().map_err(|e| e.to_string())?;
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    *state.vault_root.lock().unwrap() = Some(canonical.clone());
    Ok(Some(canonical.to_string_lossy().into_owned()))
}
```

> ファイル監視（`vault::watch_vault`）の起動は本コマンド成功時に行うのが自然だが、自己書き込み抑制（B2）と Tauri イベント emit を含む配線は **t09 で完成**させる。t05 では AppState に監視ハンドルと `recent_writes` の置き場を用意するところまで（下記 AppState 定義）。

### 4. `src-tauri/src/commands/markdown.rs`

```rust
#[tauri::command]
pub fn render_markdown(content: String) -> String {
    qwert_core::markdown::render_markdown(&content)
}
```

### 5. `src-tauri/src/commands/appearance.rs`

```rust
use std::collections::HashMap;

#[tauri::command]
pub fn load_appearance(state: State<'_, AppState>) -> HashMap<String, String> {
    let config = qwert_core::appearance::load_global_appearance()
        .unwrap_or_default();

    // vault スコープが存在すればそちらを優先（グローバルは無視）
    let vault_root = state.vault_root.lock().unwrap().clone();
    if let Some(root) = vault_root {
        if let Ok(Some(vault_cfg)) = qwert_core::appearance::load_vault_appearance(&root) {
            return qwert_core::appearance::to_css_vars(&vault_cfg);
        }
    }

    qwert_core::appearance::to_css_vars(&config)
}
```

### 6. `src-tauri/src/commands/mod.rs`

```rust
pub mod appearance;
pub mod file;
pub mod markdown;
pub mod vault;
```

### 7. `src-tauri/src/lib.rs` の更新

#### AppState 定義

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct AppState {
    pub vault_root: Mutex<Option<PathBuf>>,
    // B2: 直近の自己書き込み（vault 相対パス → 書き込み時刻）。watcher の自己トリガ抑制に使う。
    // watcher のバックグラウンドスレッドへ clone して渡すため Arc。
    pub recent_writes: Arc<Mutex<HashMap<String, Instant>>>,
    // B3: 監視ハンドル。保持している間だけ監視が続く（t09 で設定）。
    pub watch_guard: Mutex<Option<qwert_core::vault::WatchGuard>>,
}
```

#### Tauri builder

```rust
mod commands;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            vault_root: Mutex::new(None),
            recent_writes: Arc::new(Mutex::new(HashMap::new())),
            watch_guard: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::file::list_dir,
            commands::file::read_file,
            commands::file::write_file,
            commands::file::create_file,
            commands::vault::get_vault_root,
            commands::vault::open_vault_dialog,
            commands::markdown::render_markdown,
            commands::appearance::load_appearance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`tauri_plugin_dialog` を `Cargo.toml` に追加したら `src-tauri/capabilities/default.json` にも権限を追加する:

```json
{
  "identifier": "default",
  "description": "Default capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default"
  ]
}
```

> C7: フォルダ選択は `tauri-plugin-dialog` の既定権限に含まれる `dialog:default` で足りる（個別の `dialog:allow-open` 識別子は版により名称が異なるため、まず `dialog:default` を使い、ビルド時の Permission 静的検証で過不足を確認する）。Tauri 2.0 は権限不足をビルド時に検出するため、エラーが出たらメッセージが示す識別子を追加する。

### 8. `src/lib/tauri.ts` — TypeScript ラッパー（Branded Types適用）

`src/lib/tauri.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { RelativePath, AbsolutePath } from "../types/brand";

export interface FileEntry {
  name: string;
  path: RelativePath;
  is_dir: boolean;
  children?: FileEntry[];
}

export async function listDir(): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("list_dir");
}

export async function readFile(path: RelativePath): Promise<string> {
  return invoke<string>("read_file", { path });
}

export async function writeFile(path: RelativePath, content: string): Promise<void> {
  return invoke<void>("write_file", { path, content });
}

export async function createFile(path: RelativePath): Promise<void> {
  return invoke<void>("create_file", { path });
}

export async function getVaultRoot(): Promise<AbsolutePath | null> {
  const result = await invoke<string | null>("get_vault_root");
  return result as AbsolutePath | null;
}

export async function openVaultDialog(): Promise<AbsolutePath | null> {
  const result = await invoke<string | null>("open_vault_dialog");
  return result as AbsolutePath | null;
}

export async function renderMarkdown(content: string): Promise<string> {
  return invoke<string>("render_markdown", { content });
}

export async function loadAppearance(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("load_appearance");
}
```

### 9. `src/types/models.ts` への追記

```typescript
export type { FileEntry } from "../lib/tauri";
```

## ビルド確認

```bash
cargo build --manifest-path src-tauri/Cargo.toml
```

## 完了基準

- `src-tauri/src/commands/` に4ファイルが存在する
- `cargo build --manifest-path src-tauri/Cargo.toml` が通る
- `src/lib/tauri.ts` に全コマンドのラッパーが存在する
- `list_dir`, `read_file`, `write_file`, `create_file`, `open_vault_dialog`, `render_markdown`, `load_appearance` が `invoke_handler` に登録されている

> B1: 設定の読み書きコマンド（`load_config` / `save_config` / `save_settings` 等）は **Phase 1 では作らない**（Phase 2）。Phase 1 の settingsStore（vim/ハイライト）はセッション内メモリのみで動かし、永続化しない。
