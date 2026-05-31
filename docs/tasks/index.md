# qwert Phase 1 タスクインデックス

仕様書: `docs/qwert_仕様要件書_v10.md`（§21 Phase 1 対応）

> 本タスク群は `docs/tasks/fix.md` のレビュー指摘を反映済み（A群=全修正、C群=全修正、B群=B1: config 永続化を Phase 2 へ／B2: 自己トリガ抑制を Phase 1 実装／B3: watcher を qwert-core に分離／B4: 行番号・折り返し・タブ幅 UI を Phase 2 へ）。加えて、エディタのファイル切替時に自動保存が誤発火する不具合を `applyingRemote` フラグで抑制（t08 Editor）。さらに fix.md の推奨に従い、旧 t07/t08 をそれぞれ二分割し全10タスクに再採番した。

---

## 現在の実装状態（初期確認時点）

| ファイル | 状態 |
|--------|------|
| `Cargo.toml` (workspace) | 完了 — qwert-core + src-tauri の2クレート構成 |
| `crates/qwert-core/src/lib.rs` | スキャフォールド（add関数のみ、未実装） |
| `src-tauri/src/lib.rs` | スキャフォールド（greetコマンドのみ） |
| `src/App.tsx` | デフォルトテンプレート |
| `tsconfig.json` | `strict: true` あり、追加オプション未設定 |
| src/types/, src/stores/, src/components/ | 未作成（ディレクトリ不存在） |

---

## Phase 1 タスク一覧

| # | ファイル | 内容 | 前提タスク |
|---|--------|------|-----------|
| 01 | [p1-t01-project-setup.md](p1-t01-project-setup.md) | tsconfig厳格化 + TypeScript型基盤（Branded Types, as const） | なし |
| 02 | [p1-t02-qwert-core-vault-config.md](p1-t02-qwert-core-vault-config.md) | qwert-core: vault.rs（ディレクトリスキャン・アトミック読み書き）+ config.rs（TOML設定） | なし |
| 03 | [p1-t03-qwert-core-markdown.md](p1-t03-qwert-core-markdown.md) | qwert-core: markdown.rs（pulldown-cmark → HTML変換） | 02 |
| 04 | [p1-t04-qwert-core-appearance.md](p1-t04-qwert-core-appearance.md) | qwert-core: appearance.rs（appearance.toml読み込み + Rustサニタイザー） | 02 |
| 05 | [p1-t05-tauri-commands.md](p1-t05-tauri-commands.md) | Tauriコマンド層: commands/ + TypeScriptラッパー（Branded Types適用） | 01, 02, 03, 04 |
| 06 | [p1-t06-visual-foundation.md](p1-t06-visual-foundation.md) | 視覚設定基盤: --qw-* CSS変数 + CodeMirror 6テーマ + prefers-color-scheme対応 | 01 |
| 07 | [p1-t07-frontend-stores.md](p1-t07-frontend-stores.md) | フロントエンド・ストア層: vault / editor / settings / appearance ストア | 05 |
| 08 | [p1-t08-frontend-components.md](p1-t08-frontend-components.md) | フロントエンド・コンポーネント層: FileTree / Editor / Preview / StatusBar / Split View | 06, 07 |
| 09 | [p1-t09-integration-autosave-watch.md](p1-t09-integration-autosave-watch.md) | 結合①: 自動保存 + 外部変更検知（watcher・自己トリガ抑制・リロードダイアログ） | 08 |
| 10 | [p1-t10-integration-newfile-settings.md](p1-t10-integration-newfile-settings.md) | 結合②: 新規ファイル + 設定パネル + キーボードショートカット（App シェル完成） | 09 |

---

## 依存グラフ

```
01 ─┬───────────────────► 05 ─► 07 ─┐
    └────────► 06 ──────────────────┴─► 08 ─► 09 ─► 10
02 ─┬─► 03 ─────────────► 05         06 ┘（08 の前提）
    └─► 04 ─────────────► 05
```

- 05 の前提は **01・02・03・04 すべて**。
- 07（ストア）の前提は **05**。08（コンポーネント）の前提は **06・07**。
- 09（保存・監視）の前提は 08、10（新規・設定・ショートカット）の前提は 09。
- 01 と 02 は独立して並行実施可。06 は 01 完了後、05/07 と独立して進められる。

> t07/t08（旧 t07 を二分割）、t09/t10（旧 t08 を二分割）。1タスク=1レビュー単位に収まるよう分割した。

---

## 完了基準（Phase 1 全体）

- `pnpm tauri dev` で起動し、任意フォルダをVaultとして開ける
- ファイルツリーにて .md ファイルを選択するとエディタ + プレビューが表示される
- テキスト編集後 3 秒で自動保存される（自己書き込みは外部変更として誤検知しない）
- 外部変更を検知してリロード確認ダイアログが表示される（未保存時）／自動リロードされる（保存済み時）
- 新規ファイルを作成できる
- 設定画面で、構文ハイライト On/Off は**即時**、Vimバインド切替は**再起動後**に反映される
- デフォルトテーマがWCAG 2.2 Level AA準拠のコントラスト比を持つ
- `prefers-color-scheme: dark` に追従してダークテーマに切り替わる

> Phase 1 では設定値を config.toml に**永続化しない**（再起動で初期値に戻る）。永続化と、行番号/折り返し/タブ幅の設定 UI は **Phase 2**。
