# p1-t09: 結合① — 自動保存 + 外部変更検知

仕様書参照: §2 テキストエディタ（自動保存）、§12 外部変更検知と並行アクセス設計、§21 Phase 1（タスク群6 結合）

> 旧 t08 を二分割した前半。保存と外部変更検知という「データ保全」系の結合に絞る。新規ファイル・設定・ショートカットは t10。本タスクでは keydown ハンドラは追加しない（全ショートカットは t10 が単一ハンドラで持つ＝C6）。

## 前提

- p1-t08 完了済み（コンポーネントと初期 App.tsx が存在する）
- p1-t05 / p1-t02 完了済み（`write_file` コマンドと `vault::watch_vault` が存在する）

## 作業内容

### 1. 自動保存の配線

`editorStore`（t07）は `registerSaveCallback` / `scheduleAutosave` / `saveCurrentFile` を既に持つ。あとは **App.tsx の `onMount` で保存コールバックを登録**するだけ。

```typescript
// App.tsx の onMount 内
import { editorStore } from "./stores/editor";
import { vaultStore } from "./stores/vault";
import * as tauri from "./lib/tauri";

editorStore.registerSaveCallback(async () => {
  const file = vaultStore.selectedFile();
  if (file) await tauri.writeFile(file, editorStore.content());
});
```

これで `editorStore.onChange`（t08 の Editor が呼ぶ）→ 3秒後に `saveCallback` 実行、という自動保存が完成する。`Ctrl+S` による即時保存（`saveCurrentFile`）の**ショートカット登録は t10**で行う。

### 2. 外部変更検知（qwert-core watch → 自己トリガ抑制 → Tauriイベント → ダイアログ）

> **B3**: `notify` を src-tauri に持たず、qwert-core の `vault::watch_vault`（t02 実装）を使う。src-tauri は「コールバックで受けたパスを、自己書き込みでなければ emit する」グルーに徹する（§15 準拠）。独自の `src-tauri/src/watcher.rs` は作らない。
>
> **B2**: 自動保存のアトミック書き込み（tempfile + rename）自体が watcher を発火させ、自分の書き込みを「外部変更」として誤検知する。`write_file` コマンドが（書き込み前に）記録した `recent_writes`（t05）を見て、直近 N ms 以内の自己書き込みパスは emit しない。

#### Rust側: `open_vault_dialog` 成功時に監視を開始（`src-tauri/src/commands/vault.rs`）

t05 の `open_vault_dialog` の末尾（vault_root 確定後）に、監視を起動して guard を AppState に格納する処理を足す:

```rust
use std::time::Duration;
use tauri::Emitter;   // app.emit に必要

// open_vault_dialog 内、*state.vault_root.lock() に canonical をセットした後:
{
    let app_for_cb = app.clone();
    let recent = state.recent_writes.clone();        // Arc<Mutex<HashMap<String, Instant>>>
    let guard = qwert_core::vault::watch_vault(&canonical, move |relative: String| {
        // B2: 直近 1500ms 以内に自分が書いたパスなら自己トリガとみなして無視。
        // 1回の保存で notify が複数イベント（Create + Modify 等）を出し得るため、
        // 即削除せず「窓が切れるまで」抑制し、古いエントリは retain で掃除する。
        const SELF_WRITE_WINDOW: Duration = Duration::from_millis(1500);
        {
            let mut map = recent.lock().unwrap();
            map.retain(|_, t| t.elapsed() < SELF_WRITE_WINDOW);   // 期限切れ掃除
            if map.contains_key(&relative) {
                return;   // 窓内の自己書き込み → 無視
            }
        }
        let _ = app_for_cb.emit("file-changed", relative);
    }).map_err(|e| e.to_string())?;

    // guard を保持（drop すると監視停止）。前の vault の guard はここで置き換えて落とす。
    *state.watch_guard.lock().unwrap() = Some(guard);
}
```

ポイント:
- `watch_vault` は vault 相対パス（`/` 区切り）を渡してくる（t02 で変換済み）。`write_file` が `recent_writes` に入れるキーも同じ vault 相対パスなので突き合わせが成立する。
- vault を開き直すと古い `WatchGuard` が `Some(_)` 置き換えで drop され、前の監視は自動停止する。

#### TypeScript側: `App.tsx` に `file-changed` リスナーを追加

App に外部変更ダイアログ用の signal を定義し、`onMount` でリスナーを張る:

```typescript
import { createSignal, onMount } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { SAVE_STATE } from "./types/constants";

const [showExternalChangeDialog, setShowExternalChangeDialog] = createSignal(false);
const [externalChangeFile, setExternalChangeFile] = createSignal<string>("");

onMount(() => {
  void listen<string>("file-changed", (event) => {
    const changedPath = event.payload;
    const currentFile = vaultStore.selectedFile();

    // ファイルツリーは常に更新
    void vaultStore.refreshFileTree();

    if (currentFile && currentFile === changedPath) {
      if (editorStore.saveState() === SAVE_STATE.UNSAVED) {
        setExternalChangeFile(changedPath);
        setShowExternalChangeDialog(true);   // 未保存 → ダイアログ
      } else {
        void editorStore.loadFile(currentFile);   // 保存済み → 自動リロード
      }
    }
  });
});
```

> これらの signal（`showExternalChangeDialog` 等）と下記ダイアログの描画は、t10 の最終 App JSX 組み立てに引き継ぐ。本タスクでは「未保存時にダイアログが出る／保存済み時に自動リロードされる」ことを確認できる最小描画を入れておく（`<Show when={showExternalChangeDialog()}>...`）。

#### `src/components/ExternalChangeDialog.tsx`

```typescript
interface Props {
  fileName: string;
  onReload: () => void;
  onKeep: () => void;
}

export function ExternalChangeDialog(props: Props) {
  return (
    <div class="dialog-overlay">
      <div class="dialog">
        <p>「{props.fileName}」が外部で変更されました。</p>
        <button onClick={props.onReload}>外部の変更を読み込む</button>
        <button onClick={props.onKeep}>自分の変更を保持する</button>
      </div>
    </div>
  );
}
```

App でのハンドラ（reload は対象ファイルを読み直す）:
```tsx
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
```

### 3. `capabilities/default.json` の確認

外部変更検知は Tauri の `emit`/`listen` を使う。`core:event:*` は `core:default` に含まれるため追加不要なはず。ビルド時の Permission 静的検証でエラーが出たら、メッセージが示す識別子を追加する。

## 完了基準

- テキスト編集後 3 秒で選択中ファイルへ自動保存される
- 自動保存による自分の書き込みが「外部変更」として誤検知されない（B2 抑制が効く）
- 外部エディタ等での変更を検知し、未保存時はダイアログ、保存済み時は自動リロードされる
- `notify` は src-tauri の依存に追加されていない（qwert-core の `watch_vault` のみ使用 = B3）
- `cargo build --manifest-path src-tauri/Cargo.toml` が通る
