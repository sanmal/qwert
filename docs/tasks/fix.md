qwert Phase 1 タスク分割の評価と改善点

評価日: 2026-05-30
対象: index.md + p1-t01〜t08（Sonnet 作成）を仕様書 v10 と突き合わせて再評価。

---

## 総合評価

- 分割の粒度・依存関係・完了基準の立て方は良好で、おおむねこのまま実用できる。
- ただし **このまま Claude Code に渡すとコンパイル不能/機能不全になる箇所が5件（A群）**ある。
  着手前に各 .md を修正しておくのが望ましい。
- 加えて、設計の断絶・Phase 1 完了基準とのギャップ（B群）が4件、品質改善（C群）が複数。

優先度: A 要修正（壊れる）／ B 要検討（仕様の意図とずれる）／ C 直すと良い。

---

## 分割度合いの評価

8 分割は §21 Phase 1（6 タスク群）を素直に展開しており妥当。1 タスク = 1 レイヤ or 1 クレートの
モジュール群に収まっていて、Claude Code に渡すサイズとして適切。

- 最重量は t07（4 ストア + 5 コンポーネント + App.tsx + CSS）と t08（自動保存 + watcher +
  新規作成 + 設定 + ショートカット）。どちらも 1 セッションにはやや大きい。
  - t08 は (a) 自動保存 + 外部変更検知 / (b) 新規ファイル + 設定パネル + ショートカット
    に二分すると、レビュー単位として安全。t07 はストアとコンポーネントで割ってもよい。
- t02 は vault + config で大きめだが、t03/t04 が t02 をひとまとめに依存するので分けない方が自然。

依存グラフは表が正しく、ASCII 図は 01→05 の線が省かれているなど多少不正確（実害なし）。

---

## index.md の基準の評価

完了基準は具体的で良い。ただし2点、実装と基準がずれる:

- 「Vimバインド切替…変更できる」: t07 は Vim 切替を**再起動必須**としている（動的切替は未実装）。
  基準はライブ切替を示唆するので、基準側に「再起動後に反映」と注記するか、起動時に
  settings から Vim を読む実装にする。構文ハイライト On/Off は Compartment でライブ切替できる。
- 「設定画面から変更できる」: 後述 B1 のとおり config.toml に**永続化されない**（セッション内のみ）。
  基準に「永続化は Phase 2」と注記するか、load/save コマンドを足す。

---

## A群: 実装が壊れる/動かない（要修正）

### A1. t05 dialog API が v2 と不一致（コンパイル不能）

`app.dialog().file().pick_folder().await` は誤り。tauri-plugin-dialog v2 の `pick_folder` は
**コールバック型**で Future を返さないため `.await` できない。async コマンドでは
`blocking_pick_folder()`（await 不要）を使う。さらに `FilePath → PathBuf` は `.to_path_buf()` では
なく `.into_path()`（Result を返す）。

```rust
#[tauri::command]
async fn open_vault_dialog(app: tauri::AppHandle, state: State<'_, AppState>)
    -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(fp) = app.dialog().file().blocking_pick_folder() else { return Ok(None) };
    let path = fp.into_path().map_err(|e| e.to_string())?;
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    *state.vault_root.lock().unwrap() = Some(canonical.clone());
    Ok(Some(canonical.to_string_lossy().into_owned()))
}
```

### A2. preset がテーマ切替に効かない（t04 ↔ t06/t07 の不整合）

t04 の `to_css_vars` は `preset="dark"` を `--qw-preset: dark` という CSS 変数として出力する。
だが t06 のテーマ切替は `:root[data-theme="dark"]` という**属性セレクタ**で行い、`--qw-preset`
変数を参照するルールは存在しない。t07 の appearanceStore は全エントリを
`style.setProperty(key, value)` で流すだけなので、preset は素通りして**テーマが切り替わらない**。

修正案（どちらか）:
- appearanceStore で `--qw-preset` を特例扱いし、値を `document.documentElement.dataset.theme`
  にセットする（推奨。Rust 側は preset を別チャネルで返すか、キー名で判別）。
- もしくは to_css_vars が preset を返さず、preset 名を別フィールドで返して frontend が data-theme に適用。

### A3. t06 の pnpm phantom dependency（解決失敗）

`src/lib/codemirror/highlight.ts` は `@codemirror/language`（syntaxHighlighting, HighlightStyle）
と `@lezer/highlight`（tags）を import するが、t06 の `pnpm add` 一覧に**両方とも無い**。
pnpm は既定で hoisting しないため、直接依存に無いパッケージの import は解決失敗する。

```bash
# t06 の pnpm add に追記
pnpm add @codemirror/language @lezer/highlight
```

### A4. resolve_path が新規ファイルに使えない（create_file のトラバーサル検証が破綻）

t02 の `resolve_path` は `canonicalize()` を使うが、これは**実在するパスにしか効かない**。
`create_file`（新規）や write_file の新規パスでは `canonicalize` が NotFound を返す。結果、
create_file は resolve_path を通せず、トラバーサル検証なしで作成されるか、エラーで作れない。

修正案: 新規パス用に「親ディレクトリを canonicalize → vault 配下か検証 → ファイル名を結合」
という別経路を用意する（lexical 検証 + 親の canonicalize）。読み取り/既存ファイルは現行の
resolve_path、新規作成は親基準の検証、と分ける。

### A5. 非ゼロ既定には手動 impl Default が必須（サイレント破壊）

t02 の Config 系・t04 の TextConfig / HighlightConfig は `#[serde(default)]` だが、既定値が
非ゼロ（autosave 3000ms, tab_size 4, font_size 16, line_height 1.6, editor_max_width 72,
default_view "split"）。`#[derive(Default)]` だと 0/false/"" になり**仕様と異なる既定で静かに動く**。

特に危険なのが `HighlightConfig.enabled`（既定 true のはずが derive だと false）と
default_view（""）。これらは**手動 `impl Default`** が必須。t04 は AppearanceConfig に
`derive(Default)` を付けているが、内包する TextConfig / HighlightConfig が手動 Default を
持たないと正しい既定にならない（derive の合成は子の Default に従うため）。各 .md に
「derive ではなく手動 impl Default」と明記すべき。

---

## B群: 設計の断絶・完了基準とのギャップ（要検討）

### B1. config.toml がフロントエンドに繋がっていない

t02 で `load_config`/`save_config` を作るが、**それを公開する Tauri コマンドが無い**。t07 の
settingsStore は `vim_mode=false`/`syntaxHighlight=true` をハードコード初期化し、autosave も
3000ms 直書き。t08 は `invoke("save_settings", ...)` に言及するが、そのコマンドは t05 で
**未定義**。つまり config.toml の機構を作るのに、走るアプリからは読み書きされない。

対応（どちらか）:
- `load_config`/`save_config` の Tauri コマンドを t05 に足し、settingsStore を初期化時に
  load、変更時に save する。
- もしくは「config.toml の読み書きは Phase 2」と明示し、t02 の config.rs は型定義 + テストのみ、
  完了基準にも永続化しない旨を注記。t08 の save_settings 参照は削除。

### B2. 外部変更検知の自己トリガが未対策

t08 の watcher は vault 配下の .md 変更すべてに `file-changed` を emit する。だが**自動保存の
アトミック書き込み（tempfile + rename）自体が watcher を発火**させ、自分の書き込みを「外部変更」
として検知してしまう。保存済み扱い→自動リロードのループや、カーソル飛びの原因になる。
直近の自己書き込みを無視する窓（保存直後 N ms）や、書き込んだ mtime を覚えて一致なら無視する
抑制が必要（§12 の趣旨）。

### B3. watcher の配置が §15 とずれ、notify が二重宣言

§15 は vault.rs（qwert-core）が「スキャン・**ウォッチ**・アトミック書き込み」を担うとする。
t08 は src-tauri/watcher.rs に置く。AppHandle で emit する都合上 glue は src-tauri で良いが、
§15 準拠なら「qwert-core が watch + コールバック、src-tauri が emit」に分けるのが筋。
現状 notify は t02 で qwert-core に、t05 で src-tauri に**両方宣言**され、qwert-core 側は未使用。
どちらかに寄せる（watch を qwert-core に置くなら t02 の notify が活き、src-tauri から呼ぶ）。

トレードオフ: src-tauri 集約は実装が単純だが §15 と乖離。qwert-core 分離は §15 準拠で
CLI/MCP からも監視ロジックを再利用できるが、コールバック設計のぶん手数が増える。

### B4. エディタ設定（行番号/折り返し/タブ幅）が未配線

§2 必須機能の「行番号表示（トグル可能）」「テキストの折り返し設定」「タブ/インデント操作」は
config.rs（t02）に定義はあるが、CodeMirror（t06/t07 は basicSetup 任せ）にも設定パネル（t08 は
Vim + ハイライトのみ）にも繋がっていない。Phase 1 完了基準には含まれないが §2 必須の一部なので、
Phase 1 で拾うか「Phase 2 へ繰り延べ」と明記しておくと取りこぼしを防げる。

---

## C群: 品質・軽微（直すと良い）

- C1. t01: `erasableSyntaxOnly` は **TypeScript 5.8+**（"5.5+" は誤り）。
- C2. t01: brand.ts のコンストラクタが検証なしの `as` キャスト。spec は `/* validation */` を意図。
  Phase 1 は可だが「検証は未実装（TODO）」と明記。
- C3. t01: spec §17 にある `esModuleInterop` が欠落。`references: tsconfig.node.json` は
  テンプレートが該当ファイルを生成している前提——存在を確認（無ければ references を外す）。
- C4. t07 Editor: `vim()` は **basicSetup より前**に置く（末尾 push はキーバインド衝突の元）。
  `let view: EditorView` は `EditorView | undefined` にして strict の未代入参照を回避。
- C5. t07 FileTree: `depth` が常に 0 でネストの字下げが効かない。`Props` も未使用。depth を
  FileTreeItem に伝播させる。
- C6. t08: keydown ハンドラが section 1/4/5 に分散（Ctrl+S・Ctrl+, が重複登録され得る）。
  **1 つのハンドラに集約**し onCleanup で解除。`showSidebar` 信号 + 条件描画、外部変更ダイアログの
  信号配線（setShowExternalChangeDialog 等の定義と描画）が本文に無いので補う。
- C7. t05: `tauri = { features = [] }` で spec §16 の `protocol-asset` が省略（Phase 1 は画像表示が
  無いので可、将来必要）。dialog 権限 `dialog:allow-open` の識別子は v2 で要確認。
- C8. t03: `Options::ENABLE_GFM` が pulldown-cmark 0.12 に存在するか・個別オプションと重複しても
  無害かを確認（概ね問題ないが、GFM alert 等の挙動差に注意）。
- C9. §21 タスク番号参照の軽微なズレ（t01「タスク1・4」、t04「タスク2・4」の "・4" が曖昧）。
- C10. settingsStore は config.toml 由来でなくハードコード初期値（B1 と同根）。

---

## 改善提案のまとめ

着手前に各 .md へ反映すべき優先順:

1. A1〜A5 を修正（コンパイル/機能が直接壊れる）。特に A2（preset）と A4（create_file 検証）と
   A5（手動 Default）は気づきにくいので明記が効く。
2. B1 を方針決定（config を Phase 1 で配線するか、Phase 2 へ繰り延べて基準に注記するか）。
   B2（自己トリガ抑制）は外部変更検知の体験に直結するので Phase 1 で入れる価値が高い。
3. B3・B4 は §15/§2 との整合。少なくとも「どこに置く/いつやる」を .md に明記。
4. C 群は Claude Code が実装中に踏みやすい落とし穴なので、該当 .md の「注意」に一行ずつ足す。

分割そのものは良くできているので、上記を各タスク .md に追記すれば、Claude Code に
1 タスクずつ安心して渡せる状態になる。t07/t08 だけは分量的に二分割を検討するとレビューが楽。
