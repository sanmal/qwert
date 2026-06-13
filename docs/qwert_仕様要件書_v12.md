Rust製軽量Markdownノートアプリケーション。

最終更新: 2026-06-13(v12)

---

## プロジェクト概要

Obsidianの重さ（Electron起因）を解消するため、Tauri 2.0 + SolidJS による軽量Markdownノートアプリを開発する。ローカルファイルのみを扱い、フォルダ/ファイル管理を中心とした最小限の機能セットに絞る。将来的にAndroidアプリとしても動作させ、PC-スマホ間でSyncthingによるローカルファイル同期を実現する。

**仮称**: `qwert`（旧称 nota は既存サービスと重複のため変更）

**目標**: デスクトップ版は起動1秒以内、メモリ使用量100MB以下

### qwertの立ち位置（v10で明確化）

qwert は AI エージェントと、それを扱う人間向けの Markdown ノート
アプリケーションである。SDD（仕様書駆動開発）ワークフローにおいて、
AI エージェントがローカルの仕様書を読み・編集・改訂する基盤を提供し、
CLI / MCP / hook を介した決定論的な vault 操作を実現する。

汎用 PKM（Personal Knowledge Management）アプリ（Obsidian /
VS Code / Notion 等）とは差別化される点が三つある:

1. AI エージェント向け決定論的 vault 操作 CLI
   - Noun-Verb 体系、セマンティック終了コード、構造化出力
   - wikilink の AST 解析による Revision システム
   - MCP stdio サーバーモード
2. 不可視文字検出ツール
   - 間接プロンプトインジェクション素材の構造的検出
   - 第1層: Unicode Tag / Null byte / C0+C1 制御文字
3. WCAG 準拠 Markdown リーダー
   - WCAG 2.2 Level AA 必須、AAA 部分対応
   - vault スコープ設定による即時テーマ反映（Phase 3。Phase 1〜2 はグローバル設定の起動時読み込みのみ）

「AI を使う前提」を仕様の柱に据えるが、AI を持たないユーザーも
CLI 経路で全機能にアクセス可能とする（AI は推奨経路であって唯一の
経路ではない）。

### qwertの位置づけ（v9で明文化）

qwertはMarkdownエディタとして、SDD（仕様書駆動開発）の仕様書を記述・運用する基盤を提供する。AIエージェントが仕様書を読み、CLI/MCP経由で安全に操作できるインターフェイスを備える。視覚的表現の上限はコードブロック内Mermaidまでとし、ピクセル精度のUI表現は外部サービス（Figma/Penpot等）に委託する。「全部入り」のMarkdownアプリケーションを求めるユーザーはObsidianやVS Codeを選ぶべきであり、qwertはあえてその対極、すなわちスコープを絞った小道具として機能する。

### 設計哲学（v9で5層well構成へ再整理）

"Just a small tool in the garage, hook and MCP as spare keys."

qwertが well に行うことを5層に展開する:

- **Note well**: CodeMirror 6によるMarkdown編集、Vim切替、自動保存、外部変更検知
- **File well**: wikilink整合性、Revisionシステム、アトミック書き込み、vault状態管理、ファイル品質保証（不可視文字検出）
- **Show well**: テキスト判別性（WCAG 2.2 AA / 部分AAA）、コードブロック内Mermaid / KaTeXによる構造化図表
- 配色のカスタマイズ手段として、qwert は GUI の色ピッカーやリアルタイム
コントラスト表示 UI を持たない。代わりに以下の二経路を提供する:

- AI 経路（推奨）: appearance.toml のコメントに埋め込んだ機械可読指示
  テンプレートに従い、AI エージェントがユーザーの自然言語の要望
  （「暖色系に」「夜間に目が疲れない配色に」等）を WCAG 準拠の
  fg/bg ペアへ変換する
- CLI 経路: `qwert appearance contrast` でコントラスト比と WCAG レベルを
  検証、`qwert appearance set` で検証付き書き込み

ここで qwert が書くのは「AI 向けの機械可読指示テンプレート」であり、
「人間向けの配色ガイドブック（色の選び方・WCAG 解説・カスタマイズ手順）」
ではない。後者は引き続き提供しない。この区別が「テープを巻ける表面を
残しておく。ただしテープの巻き方のガイドブックは書かない」の v10 における
解釈となる——AI に対する操作プロトコルの提供は、人間向けガイドブックの
不提供と両立する。

- **Secure well**: 外部リソース構造的拒否、プラグイン不採用、Tauri capabilities最小化、間接プロンプトインジェクション耐性
- **Agent well**: CLI Noun-Verb体系、MCP stdio、セマンティック終了コード、確定的wikilink操作

Note well と File well は機能カテゴリ、Show well は表現上限、Secure well と Agent well は AIエージェント時代の差別化要因として機能する。プラグインシステムや同期機能はアプリに組み込まず、外部ツール（Syncthing, jj, neovim, Figma等）との協調を前提とする。

### Show well の方針

華やかな見た目ではなく、テキストの認識しやすさ（text discriminability）を道具としての品質として追求する。WCAG 2.2準拠（Level AA必須、Level AAA部分対応）、CSS Custom Propertiesベースの軽量な視覚設定、構文ハイライトOn/Off切替のみ（配色カスタマイズUIは持たない）。「テープを巻ける表面を残しておく。ただしテープの巻き方のガイドブックは書かない」— UIに露出しない設定はファイル経由で許容するが、保証はしない。

視覚表現の上限はコードブロック内のMermaidとKaTeXまで。ピクセル精度のUI表現、手書き風自由描画、PDF・Canvas・動画等のバイナリレンダリングはqwertのスコープ外。これらはFigma / Penpot等の外部サービスに委託し、合意時点のスナップショットが必要な場合のみローカルエクスポート画像をvault内に保持する運用を推奨する。

---

## 確定技術スタック

| レイヤー          | 技術                     | 備考                                |
| ------------- | ---------------------- | --------------------------------- |
| アプリフレームワーク    | Tauri 2.0              | デスクトップ + Android対応                |
| バックエンド        | Rust                   | Tauriコマンド + CLIサブコマンド + MCPサーバー   |
| フロントエンド       | SolidJS + TypeScript   | fine-grained reactivity、仮想DOMなし   |
| テキストエディタ      | CodeMirror 6           | solid-codemirror でSolidJS統合       |
| Vimバインド       | @replit/codemirror-vim | 設定で切替可能（Phase 1）                  |
| Markdownパース   | pulldown-cmark (Rust側） | GFM拡張サポート、WikiLink対応              |
| Markdownプレビュー | HTML/CSS描画             | pulldown-cmark → HTML → WebView描画 |
| Mermaidダイアグラム | mermaid.js （フロントエンド）   | Phase 2、コードブロック内のみ                |
| 数式レンダリング      | KaTeX （フロントエンド）        | Phase 2、インライン + ブロック対応            |
| ビルドツール        | Vite                   | SolidJS標準のバンドラー                   |
| パッケージマネージャー   | pnpm                   | npm より高速・ディスク効率的                  |

---

## ターゲット環境

**デスクトップ（Phase 1〜）**:
- OS: Pop!_OS 24 LTS / Ubuntu 24 LTS（Linux優先）
- ディスプレイサーバー: X11 / Wayland両対応
- 必要パッケージ: webkit2gtk-4.1-dev, libssl-dev 等

**モバイル（Phase 4〜）**:
- OS: Android（Nothing Phone 3a / Snapdragon 7s Gen3 で検証）
- 同期: Syncthing経由でPC上のVaultフォルダとAndroid上のフォルダを双方向同期

**共通**:
- 想定ノート数: 数千ファイル規模
- ファイル形式: `.md`（CommonMark + GFM拡張）

---

## コア機能仕様

> 節番号の規約: 本節の `### 1.`〜`### 11.` が §1〜§11 に対応し、以降の各章見出し `## §12`〜`## §25` が §12 以降に対応する（本書中の参照はすべて §N 形式で行う）。

### 1. ファイルツリー（サイドバー）

任意のフォルダを「Vault」として開き、その配下の `.md` ファイルをツリー表示する。

**必須機能**:
- フォルダの再帰的スキャンとツリー表示
- フォルダの展開/折りたたみ
- ファイルの新規作成・リネーム・削除
- フォルダの新規作成・リネーム・削除
- `.md` ファイルのみ表示（フィルタ設定可能）
- ファイル変更の自動検知（Rust側 notify crate → Tauriイベントでフロントエンドに通知）
- ドラッグ&ドロップによるファイル移動（Phase 2）

### 2. テキストエディタ

CodeMirror 6 ベースの Markdown エディタ。

**必須機能**:
- Markdown構文ハイライト（`@codemirror/lang-markdown` + `@codemirror/language-data`）
- Vimバインド切替（`@replit/codemirror-vim`、設定でトグル）
- 行番号表示（トグル可能）
- 自動保存（変更検知後の遅延保存、デフォルト3秒）
- Undo/Redo（CodeMirror組み込み）
- 基本的なキーボードショートカット
- タブ/インデント操作
- テキストの折り返し設定
- ダーク/ライトテーマ（`@codemirror/theme-one-dark` 等）

**Vimバインド実装**:
```typescript
import { vim } from "@replit/codemirror-vim";

const extensions = [
  basicSetup,
  markdown({ base: markdownLanguage, codeLanguages: languages }),
  ...(settings.vim_mode ? [vim()] : []),
];
```

### 3. Markdownプレビュー

エディタの右側にレンダリング済みプレビューを表示するスプリットビュー。

**必須機能**:
- CommonMark + GFM準拠のレンダリング
- 見出し、太字、イタリック、取り消し線
- 順序付き/なしリスト、チェックボックス
- コードブロック（構文ハイライト付き）
- テーブル
- `[[wikilink]]` のリンク表示（Phase 2）
- 画像表示（ローカルファイル参照、qwert 独自 URI スキーム（Rust ゲートキーパー経由）Phase 4。外部URL画像 `![](https://...)` は描画しない）
- Mermaidダイアグラム描画（Phase 2、コードブロック内のみ）
- 数式レンダリング（Phase 2、KaTeX、インライン `$...$` とブロック `$$...$$`）
- エディタとプレビューのスクロール同期
- 3つの表示モード: Editor / Split / Preview

### 4. `[[wikilink]]` ノート間リンク（Phase 2）

Obsidian互換のwikilink記法をサポート。

**必須機能**:
- `[[ノート名]]` / `[[ノート名|表示テキスト]]` / `[[ノート名#見出し]]`
- リンク先へのジャンプ（Ctrl+Click）
- リンク入力時のオートコンプリート（CodeMirror拡張）
- バックリンク表示

### 5. 全文検索

`ignore` crate + `regex` crate による grep 的アプローチを採用する（素朴方針。Phase 2 タスク6 で実装）。tantivy 導入は Phase 2 以降、ノート数増加に応じて必要性を見て検討する（素朴方針で足りる限り導入しない）。

**JSON 出力例**（`--format json`、エンベロープは top-level 展開）:
```json
{
  "schema_version": "v1",
  "kind": "search_results",
  "query": "TODO",
  "hits": [
    {"path": "specs/auth.md", "line": 42, "snippet": "- [ ] TODO: トークン失効の検討"},
    {"path": "daily/2026-03-08.md", "line": 7, "snippet": "TODO: バックリンクのテスト"}
  ],
  "total_hits": 2
}
```
`hits` 配列は top-level に置かれ、`jq -r '.hits[].path'` で各ヒットのパスを抽出できる。各要素は `path` / `line` / `snippet` を持つ。

### 6. Mermaidダイアグラム（Phase 2）

フロントエンド側で mermaid.js を使用。

**レンダリング対象（スコープ限定）**:
- ` ```mermaid ` コードブロック（情報文字列が `mermaid`）の内部のみを描画対象とする。
- インライン記法・独自拡張はサポートしない。コードブロック外に Mermaid 構文が出現しても誤検出しない。
- パースエラー時は元のコードブロックをそのまま表示する（プレビューを壊さない）。
- dynamic import による遅延ロード（src/lib/mermaid.ts）。Mermaid を含まないノートでは常時ペナルティゼロ。

**コードブロック内SVG描画拡張（Phase 3以降の要検討事項）**:
` ```svg ` コードブロックの直接埋め込み描画をPhase 3以降で検討する。実装する場合は DOMPurify による二重防御（Rustコア側マーカー化 + フロント側サニタイズ）が必須となり、初期バンドルへの追加は約20KB（DOMPurifyのみ）に抑える。判断基準は「Mermaidで表現できない図解の頻度」と「外部ツール（Figma/Excalidraw）でのSVGエクスポート→vault配置運用の煩わしさ」のバランス。

### 7. 数式レンダリング KaTeX（Phase 2、v8新設）

Markdown内のLaTeX記法数式をKaTeXで描画する。インライン数式とブロック数式の両方をサポートする。

**採用技術の判断: KaTeX を採用、Temml / MathMLネイティブ描画は不採用**:

qwertはTauri 2.0でOSごとに異なるWebViewエンジン（Linux: WebKitGTK / Android: Chromium WebView）に描画を委ねる。MathMLネイティブ描画の品質はWebViewエンジンとOpenType MATHフォントの可用性に強く依存し、プラットフォーム間で見た目が変わる（行列括弧の高さ、ストレッチ演算子の描画バグ等が報告されている）。

KaTeXはHTML+CSSベースで描画し数学フォントを自前バンドルするため、WebKitGTK / Chromium WebView間で描画結果がほぼ同一になる。「Show well」の意味は **「最良環境で美しい」ではなく「どの環境でも確実に読める」** であり、KaTeXの自前描画モデルがこの設計哲学に適合する。

| 評価軸 | KaTeX | Temml（参考） |
|--------|-------|--------------|
| クロスプラットフォーム描画一貫性 | ◎（自前描画） | △（エンジン依存） |
| フォント環境依存 | なし（バンドル） | あり（数学フォント必要） |
| WebViewバージョン変動耐性 | ◎ | △ |
| アクセシビリティ安定性 | ○（hidden MathML併用） | △（全面依存） |
| バンドルサイズ | 約350KB | 10〜380KB |
| 描画品質（最悪環境） | ○（変わらない） | ×（壊れうる） |

**サポート記法**:
- インライン数式: `$a^2 + b^2 = c^2$`
- ブロック数式: `$$\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}$$`
- 通貨記号等との誤検出を避けるため、`$` の前後に空白がある場合はインライン数式として扱わないヒューリスティクスをパーサ側で適用する。

**実装方針**:
- パース: pulldown-cmark の math extension（`ENABLE_MATH` フィーチャーフラグ）でMath Eventを識別。
- レンダリング: KaTeX（フロントエンド、mhchem等の追加拡張なし）。
- フォント: KaTeX同梱の `KaTeX_Main`, `KaTeX_Math`, `KaTeX_AMS` 等を自前バンドル（合計約350KB）。
- 遅延ロード: 数式を含むノートを開いた時のみKaTeX JSとフォントをロードする。数式を含まないノートでは常時ペナルティゼロ。
- hidden MathML: KaTeXのデフォルト出力を使用（スクリーンリーダー対応）。
- エラー時の挙動: `throwOnError: false` 設定により、不正な数式は元のLaTeXソースを赤字で表示（プレビューが壊れない）。

**スコープ外**:
- MathJax（バンドルサイズ大、起動が遅い、サーバーサイドレンダリング前提設計）。
- Temml / ブラウザネイティブMathML（WebViewエンジン依存の描画差異）。
- mhchem等のKaTeX拡張（化学反応式等、Phase 5以降で要望ベース検討）。
- 数式エディタUI、プレビュー中の数式クリック編集。

**設定（config.toml）**:
```toml
[preview]
render_math = true          # v8追加: KaTeX描画の有効/無効
```

**将来検討**:
- **katex-rs**（Rust純粋実装）の成熟を待つ。成熟すればLaTeX→HTML+MathML変換をqwert-core側で完結させフロントエンドのJSバンドル増加をゼロにできる。現時点ではスクリーンショットテストの完全一致に至っておらず採用は時期尚早。
- **MathML Coreクロスブラウザ品質の安定**（2-3年後想定）後に Temml / MathMLネイティブ描画への移行を再検討する。パーサーレイヤー（pulldown-cmark math extension）は共通なので、レンダラー差し替えは局所的変更で済む。

### 8. Revisionシステム（Phase 2）

SDD（仕様駆動開発）ワークフローにおいて、ドキュメントの改訂（リビジョン）に伴うVault全体のwikilink参照を自動更新する機能。

**設計原則**: "File well" の範疇。qwert-coreに実装し、GUI / CLI / MCP すべてから利用可能。AIエージェントへのwikilink更新委任はトークン消費・精度の両面で不適切であり、決定論的なツールとして実装する。

**対象wikilinkパターン**（完全一致のみ、前方一致は対象外）:
- `[[A]]` → `[[A_2]]`
- `[[A|表示テキスト]]` → `[[A_2|表示テキスト]]`（表示テキストは保持）
- `[[A#見出し]]` → `[[A_2#見出し]]`
- `[[A#見出し|表示テキスト]]` → `[[A_2#見出し|表示テキスト]]`
- `![[A]]`（埋め込み）→ `![[A_2]]`
- **対象外**: `[[AB]]`（前方一致）、コードブロック内の `[[A]]`、HTMLコメント内の `[[A]]`、frontmatter内のテキスト

**命名規則**（設定可能、デフォルト: increment）:

| 方式 | 例 | 説明 |
|------|----|------|
| increment | A → A_2, A_2 → A_3 | 末尾の数値をインクリメント |
| date | A → A_20260309 | 改訂日付を付与 |
| semver | A → A_1.1.0 | セマンティックバージョニング |
| manual | A → （ユーザー入力） | 任意の名前を指定 |

**実行フロー**:
1. ユーザーがGUI/CLI/MCPでRevisionを実行（対象ファイルと新名称を指定）
2. システムが新ファイル名を提示し、影響範囲をプレビュー
3. `--dry-run` 指定時: 計画のみ出力し、ファイルは変更しない
4. 本実行時は **アトミックに** 以下を実行:
   a. 対象ファイルをリネーム（旧名 → 新名）
   b. Vault全体の `.md` ファイルをスキャン（rayon並列処理）
   c. pulldown-cmark AST解析でコードブロック外の対象wikilinkを特定
   d. 全更新対象ファイルをバッチアトミック書き込み（WALパターン）
   e. `on-revise` フック呼び出し

**--dry-run の出力仕様**（v6新設）:

dry-run は「影響範囲の可視化」を担うが、ユーザーとエージェント双方にとっての検証材料として diff 出力を別層で提供する。これはpulldown-cmark AST解析の誤検出（HTMLコメント内、ネストされたコードフェンス等のエッジケース）を事前に発見するための決定的な手段となる。

```bash
# デフォルト: 計画サマリをJSONで返す
$ qwert note revision specs/auth.md --dry-run
{
  "schema_version": "v1",
  "kind": "revision_plan",
  "dry_run": true,
  "old_name": "auth",
  "new_name": "auth_2",
  "old_path": "specs/auth.md",
  "new_path": "specs/auth_2.md",
  "affected_files": [
    {"path": "specs/index.md", "wikilink_count": 2},
    {"path": "daily/2026-03-08.md", "wikilink_count": 1}
  ],
  "total_wikilinks": 3
}

# diff ファイルを生成（パスをJSONで返す）
$ qwert note revision specs/auth.md --dry-run --diff
# → 上記JSONに "diff_path": "/tmp/qwert-diff-xxxxxxxx.patch" を追加
# → 別ファイルとして unified diff を書き出し

# diff を直接 stdout へ（JSON なし、patch(1) / git apply 互換）
$ qwert note revision specs/auth.md --dry-run --format diff
--- a/specs/index.md
+++ b/specs/index.md
@@ -12,7 +12,7 @@
 ## 参考ドキュメント
-- [[auth]] - 認証仕様
+- [[auth_2]] - 認証仕様
...
```

**on-revise フック**:
```bash
# ~/.config/qwert/hooks/on-revise
# 引数: $1=旧パス $2=新パス
# 環境変数: $QWERT_VAULT $QWERT_REV_COUNT（更新したリンク数）
#!/bin/sh
jj commit -m "revise: $(basename $1) → $(basename $2) (${QWERT_REV_COUNT} refs updated)"
```

**WALパターン（複数ファイルの整合性保証）**:
```
1. 全一時ファイルを作成・書き込み・fsync
2. インテントログ（.qwert/pending-revision.json）作成
3. 高速連続rename（POSIX renameat: アトミック）
4. インテントログ削除（コミット完了）
起動時チェック: pending-revision.json残存 → ロールバックまたは再実行
```

**大文字小文字の扱い**:
- リンク解決: `unicase` crateによるcase-insensitive比較
- ファイルシステムの挙動に従う（Linux=case-sensitive、macOS/Windows=case-insensitive）
- `unicode-normalization` crateによるNFC/NFD正規化

**設定（config.toml）**:
```toml
[revision]
naming = "increment"     # "increment" | "date" | "semver" | "manual"
confirm_before_execute = true
excluded_dirs = []       # v9追加: Revision対象外ディレクトリ
```

**対象外ディレクトリ（v9新設）**:

`excluded_dirs` 設定で指定したディレクトリ配下のファイルは、Revision操作の対象外とする。

- デフォルト: `[]`（空配列）
- 除外ディレクトリ内のファイルへの参照（wikilink）も、Vault全体スキャン時の更新対象に含めない
- ユーザー側でADR等の不変性が必要な文書を運用する場合は `["docs/decisions"]` 等を設定する
- ADR運用そのものはqwertの責務範囲外であり、採番・テンプレート展開・supersedes管理は外部ツール（chezmoi管理のシェルスクリプト等）に委ねる

この設定の存在により、ADR（Architecture Decision Record）のような「旧記録を保存したまま、新記録への参照を追加する後進的参照モデル」と、Revisionの「ファイルをリネームし、参照側を新名に追従させる前進的改訂モデル」の概念的競合を構造的に回避する。

**改訂内容の妥当性検証は責務外（v9で明文化）**:

qwertが保証するのは「ファイルが存在するか」「Markdownとして読めるか」「パスが有効か」までであり、ファイル内容の妥当性（改訂前後の類似度等）はユーザーの判断領域とする。VCS（jj/git等）による履歴管理を前提とし、qwertは改訂結果の意味的検証を行わない。

ドライバのビット規格（物理形状による二値判定）と異なり、改訂の妥当性は本質的に閾値問題（連続値の判定）であり、閾値の決定にはポリシーが必要、ポリシーにはカスタマイズが必要となる。これらを抱え込むとqwert-coreが太るため、「形状が合うか」レベルの客観的・二値的判定（ファイルの存在、Markdownとして読めるか、パスの有効性）のみをqwertが担う。

**GUIでのUX**:
- ファイルツリーの右クリックメニューに「Revision...」を追加
- 確認ダイアログ: 新ファイル名のプレビュー + 影響を受けるファイル数 + diff プレビュー
- 完了後トースト: `「A → A_2」完了: 3ファイルの[[A]]を更新しました`

**画像ファイルへの拡張（v9新設、Phase 3以降の要検討事項）**:

「視覚表現の上限はMermaidまで」を原則としつつ、SDDワークフローでFigmaから静的PNG/SVGをvaultに取り込む運用の実需頻度を見て、画像ファイルのRevision対応をPhase 3以降で検討する。

検討対象の操作は3つの異なる意図に分かれており、`.md` のRevisionとは異なる意味論的設計が必要となる:

1. **Rename**: ファイル名を変えて、参照側を追従させる（`mockup.png` → `mockup-v2.png`）
2. **Replace**: 中身だけ差し替えて、ファイル名と参照は変えない（`mockup.png` の中身だけ更新）
3. **Archive**: 旧版を別名で残しつつ新版を旧名に配置する（履歴保持）

実装する場合の暫定方針:
- `qwert asset rename` を別コマンドとして追加（`note revision` の意味論を保つ）
- `note revision --include-assets` でノートと付随画像のセット改訂を可能とする（Phase 4以降）
- 共有画像（複数ノートから参照）は `--include-assets` の対象外、明示的に `asset rename` で扱う

判断基準は「Figmaエクスポート画像のローカル保管運用の頻度」と「jjによる画像履歴管理で十分かどうか」。VCSとの責務重複（特にArchive操作）を慎重に評価する必要がある。

### 9. Vault状態レポート（v6新設）

vault全体の状態（操作結果とは独立）を報告するための専用コマンド `qwert vault status` を提供する。個別ファイル操作の成否とは**層を分けて**扱う。

**報告対象**:
- Syncthing競合ファイル（`*.sync-conflict-*.md`）の存在
- pending-revision.json の残存（前回Revisionの異常終了検知）
- 未保存のGUI編集セッション（将来拡張）
- vault整合性チェック結果

**出力例**:
```bash
$ qwert vault status --format json
{
  "schema_version": "v1",
  "kind": "vault_status",
  "vault": "/home/user/notes",
  "sync_conflicts": [
    {
      "base": "specs/auth.md",
      "conflict_file": "specs/auth.sync-conflict-20260422-123456-ABC123.md"
    }
  ],
  "pending_revision": null,
  "healthy": false,
  "warnings": [
    "1 sync-conflict file(s) detected. Resolve manually before continuing."
  ]
}
```

**設計上の重要な境界**:
- sync-conflict ファイルの存在は vault-level state であり、個別コマンド（read/write/revision等）の exit code には**昇格させない**。
- 検出のみがqwertの責務。解決は利用者の責務（任意のdiffツールやエディタで対応）。
- CRDT未採用の判断と整合。Syncthing競合は「発生前提の事実」として扱う。

### 10. 視覚設定とアクセシビリティ（v7で統合）

Markdownテキストを扱うノートアプリとして「Show well」を実現するため、テキスト判別性（text discriminability）のユーザーカスタマイズを提供する。詳細仕様は別途 `docs/appearance-spec.md` に分離（本セクションはサマリ）。

**スコープ内**:
- テキストの前景色・背景色、フォントサイズ・ファミリー、行間・文字間・単語間スペーシング、エディタ幅
- 構文ハイライトのOn/Off切替
- プリセットテーマ4種（default / high-contrast / dark / dark-high-contrast）
- OS設定の自動追従（`prefers-color-scheme`, `prefers-contrast`）

**スコープ外**:
- カスタムCSS注入（Obsidianスニペット相当）
- テーママーケットプレイス・プラグインシステム
- 構文ハイライトの色カスタマイズUI
- フォント同梱・配布
- アニメーション・トランジション（`prefers-reduced-motion` 尊重、基本的に導入しない）

**WCAG 2.2 対応**:

| Level | 基準 | 実装方針 |
|-------|------|---------|
| AA（必須） | 1.4.1 Use of Color | Markdown構文記号自体が構造を表現、色なしでも構造認知可能 |
| AA（必須） | 1.4.3 Contrast (Minimum) 4.5:1 | デフォルトテーマが保証 |
| AA（必須） | 1.4.4 Resize Text 200% | CodeMirror 6のリフロー対応 |
| AA（必須） | 1.4.11 Non-text Contrast 3:1 | サイドバー・ボタン・フォーカスリング等 |
| AA（必須） | 1.4.12 Text Spacing | CSS変数ベースレイアウトで対応 |
| AAA（部分） | 1.4.6 Contrast (Enhanced) 7:1 | ハイコントラストプリセット提供 |
| AAA（部分） | 1.4.8 Visual Presentation | 前景/背景色選択、行幅制限、行間1.5倍以上 |

**構文ハイライト**:
- On/Offの二値切替のみ。トークン別色カスタマイズUIは提供しない。
- デフォルト配色はProtanopia（1型色覚）/ Deuteranopia（2型色覚）対応（赤-緑対比非依存、色相だけでなく明度差で区別）
- CodeMirror 6 の `Compartment` を用いた動的切替

```typescript
import { Compartment } from "@codemirror/state";
import { syntaxHighlighting } from "@codemirror/language";

const highlightCompartment = new Compartment();
// On:  highlightCompartment.of(syntaxHighlighting(qwertHighlightStyle))
// Off: highlightCompartment.of([])
```

**プリセットテーマ**:

| プリセット名 | 説明 | コントラスト比目標 |
|-------------|------|-----------------|
| `default` | ライト背景 | AA（4.5:1以上） |
| `high-contrast` | ライト背景・高コントラスト | AAA（7:1以上） |
| `dark` | ダーク背景 | AA（4.5:1以上） |
| `dark-high-contrast` | ダーク背景・高コントラスト | AAA（7:1以上） |

**CSS Custom Properties**:
全CSS変数は `--qw-` プレフィックスを持つ。CodeMirror 6のテーマは1つだけ定義し、見た目の変更はCSS変数の値切替で行う（theme extension再構築はしない）。

```css
:root {
  --qw-font-family: system-ui, sans-serif;
  --qw-font-size: 16px;
  --qw-font-weight: 400;
  --qw-fg: #1a1a1a;
  --qw-bg: #ffffff;
  --qw-fg-muted: #6b7280;
  --qw-accent: #2563eb;
  --qw-line-height: 1.6;
  --qw-letter-spacing: 0em;
  --qw-word-spacing: 0em;
  --qw-paragraph-spacing: 1.5em;
  --qw-editor-max-width: 72ch;
  --qw-cm-keyword: /* Protanopia/Deuteranopia対応値 */;
  --qw-cm-string: /* 同上 */;
  --qw-cm-comment: /* 同上 */;
  --qw-cm-heading: /* 同上 */;
  --qw-cm-link: var(--qw-accent);
  --qw-cursor: var(--qw-fg);
  --qw-selection-bg: #dbeafe;
}
```

**設定の2軸構造（スコープ層 / 露出層）**:

**設定のスコープ層**:

| スコープ | パス | 反映タイミング | 関係 |
|---------|------|--------------|------|
| グローバル | `~/.config/qwert/appearance.toml` | 起動時1回 | 全 vault 共通のデフォルト |
| vault スコープ | `vault/.qwert/appearance.toml` | 即時反映 | この vault でグローバルを上書き |

vault スコープが存在すればそちらのみを使用し、グローバルは無視する
（マージはしない）。vault スコープが存在しなければグローバルのみが効く。

**設定の露出層**（各スコープ層内で共通）:

| 露出層 | 内容 | 保証 |
|--------|------|------|
| 第1露出層 | UIに表示される設定（`[text]`, `[color]`, `[highlight]`） | 公式サポート、後方互換維持 |
| 第2露出層 | 設定ファイルのコメントアウト済みキー（`[color.advanced]`） | 自己責任、バージョン間の動作非保証 |
| 第3露出層 | 任意CSS注入 | qwertは提供しない（スコープ外） |

**セキュリティモデル**:
- データフロー: `appearance.toml → Rust（読み込み + バリデーション） → IPC → Frontend（CSS変数セット）` の一方向
- Rustバックエンドがゲートキーパー。フロントエンドは設定ファイルを直接読まない
- 拒否パターン: `url()`, `expression()`, `javascript:`, `import`, `@`, `<`, `>`, `{`, `}` を含む値
- キー種別別の許可パターン:

| キー種別 | 許可される値の形式 |
|---------|------------------|
| 色（`fg`, `bg`, `cm-*`, `cursor`, `*-bg`, `*-fg`） | hex（`#rrggbb`, `#rgb`）, `rgb()`, `hsl()` |
| 寸法（`*spacing*`, `*height*`, `*size*`, `*width*`） | 数値, px, em, rem, ch, % |
| フォント（`font-family`） | 英数字, スペース, ハイフン, カンマのみ |
| 上記以外 | 拒否 |

**設定の反映タイミング**:

- グローバル設定: アプリ起動時の1回のみ読み込み。変更は次回起動時に反映
- vault スコープ設定: 即時反映（ホットリロード）

vault スコープに限定して即時反映を提供する理由は、配色の試行錯誤体験を
優先するため。グローバルを起動時1回に保つことで、ファイルウォッチの対象を
vault スコープの単一ファイルに限定し、CSS変数の競合状態の考慮範囲を最小化する。

**即時反映の対象範囲**: vault スコープファイルの全セクション
（`[text]` / `[color]` / `[highlight]` / `[color.advanced]`）を即時反映の対象とする。
いずれも最終的には CSS 変数の差し替えに帰着するため、セクションによる特例を設けない。

**即時反映の実装**:
- notify crate で `vault/.qwert/appearance.toml` の単一ファイルを監視
- 変更検知 → debounce 300ms → Rust 側でサニタイズ → IPC で frontend へ key-value 送信
  → `document.documentElement.style.setProperty()` 実行
- アトミック書き込み（tmp → rename）を経由する CLI/MCP 書き込みでは中間状態が見えない。
  直接編集時のレース条件は debounce + TOML パース成功までのリトライで吸収する
- 既存の一方向データフロー（toml → Rust → IPC → frontend）を維持。
  フロント側に新しい責務は増えない

**不正な設定ファイルの扱い**:
- 起動時に vault スコープファイルが不正な TOML → グローバル設定にフォールバック
  + 起動時通知/ステータスバー警告。グローバルも無ければビルトインデフォルト
- ホットリロード中に不正 → 直前の有効な状態を保持（グローバルには戻さない）
  + 一時的な警告（`appearance.toml: syntax error at line N, keeping current theme`）
- preset と fg/bg がファイル内で同居 → 相互排他違反として上記と同じフォールバック挙動
  + 競合キーを名指しした警告（`appearance.toml: 'preset' and 'fg'/'bg' are mutually exclusive`）
- Syncthing 競合ファイル（`.qwert/appearance.sync-conflict-*.toml`）は無視（本体のみ読む）、
  `qwert vault status` で報告。§9 の vault-level state 方針に乗せる

カスタム前景・背景色指定時は両方同時指定必須（WCAG 1.4.8 Failure F24準拠）。
片方のみの指定はバリデーションエラーとして拒否する。
この拒否は CLI（`appearance set`）経路だけでなく、appearance.toml のファイル読み込み経路（raw `qwert file write` / AI 直接編集を含む全経路）にも適用する。
fg・bg の片方のみが指定された設定ファイルは、preset と fg/bg の相互排他違反と同じくフォールバック挙動（起動時=グローバルへ、ホットリロード時=直前保持）+ 警告で扱い、片側だけを CSS 変数へ反映してはならない。
これにより §13 の「`qwert appearance status` で異常なコントラスト比（fail）を検知できる」境界が、片側指定のケースでも構造的に成立する（片側指定は反映前に拒否されるため）。

コントラスト比の検証は GUI のリアルタイム表示 UI ではなく、以下で提供する:
- CLI `qwert appearance contrast --fg <hex> --bg <hex>` で対話的に検証
- appearance.toml 読み込み時に Rust 側で WCAG 2.x 相対輝度計算を実行し、
  結果をステータスバーに `Contrast: N:N (AA|AAA|fail)` として表示
- fail の場合は起動時通知（グローバル）または一時警告（vault スコープ）を併発
- AI 経路では、appearance.toml のコメントに埋め込んだプロトコルに従い
  AI が WCAG 準拠の fg/bg を提案

preset と fg/bg は相互排他とする。両方を指定した場合は明示的なエラーを返し、
「変更したつもりが反映されない（理由不明）」状態を構造的に避ける。

**将来拡張**: W3C Design Tokens Format Module 2025.10（Stable版）への対応準備として、設定読み込みをadapter patternで実装する。MVPでは未対応、Phase 5以降で検討。

```rust
trait AppearanceSource {
    fn load(&self) -> Result<HashMap<String, String>>;
}
struct TomlSource { path: PathBuf }
// 将来: struct DtcgSource { path: PathBuf }
```

### 11. ファイル品質保証 - 不可視文字検出（v9新設、Phase 2）

Markdownファイルにおける「あるはずのないもの」の検出と通知。"File well" の範疇として、ファイル内容の品質保証層を提供する。意味的な攻撃判定（プロンプトインジェクション分類など）はqwertの責務外であり、あくまで「素材としての異物検出」のみを担う。

**設計原則**:

qwertが行うのは「このファイルにあるはずのない文字が存在する」という構造的事実の通知であり、「これはプロンプトインジェクションです」という意味的判定ではない。ドライバのビット規格と同じく、客観的・二値的に判定できる範囲のみを扱う。攻撃の意図解釈はMCP利用側（Claude等のAIエージェント）の責務とする。

§13 セキュリティ境界が定める間接プロンプトインジェクション耐性の素材検出として位置づけられ、AIエージェントがvault内のノートを読む前段で「このファイルには不可視文字が含まれる」という事実を伝える経路として機能する。

**検出対象の3層構造**:

| 層 | 内容 | 判断 | Phase |
|---|------|-----|------|
| 第1層 | Unicode Tag（U+E0000〜E007F）、Null byte（U+0000）、C0制御文字のうちTab/LF/CR以外（U+0001〜0008, 000B, 000C, 000E〜001F）、C1制御文字（U+0080〜009F） | 正当な理由ゼロ、無条件で検出・警告 | **Phase 2 で実装** |
| 第2層 | ZWJ（U+200D）、BiDi制御文字（U+202A〜202E, U+2066〜2069）、BOM（U+FEFF、文中のみ）、Variation Selector（U+FE00〜FE0F） | 文脈依存、絵文字シーケンス・多言語テキストでは正当 | Phase 5 発展機能で段階的に対応 |
| 第3層 | 不正なUTF-8シーケンス（オーバーロングエンコーディング等） | `read_to_string` のエラーハンドリング拡充として自然対応 | Phase 2 |

**Phase 2 実装範囲（A + B のみ、サニタイズは含まない）**:

A. **検出のみ**: ファイル読み込み時に第1層該当文字を検出し、警告を表示（StatusBar / ファイルツリーアイコン等）。ファイル内容は変更しない。

B. **CLIスキャンコマンド**: `qwert vault scan` 等で vault 全体の検出結果を一覧出力。

C. **サニタイズオプション（Phase 3 以降の検討事項）**: `qwert note sanitize <path>` で能動的に第1層文字を除去する破壊的操作。「qwertは事実通知のみ、サニタイズは別」原則からPhase 2では実装しない。Phase 3以降で必要性を判断する。

**実装方針**:

- 既存依存（`unicode-normalization`, `regex`）のみで実装可能、新規crate追加なし
- pulldown-cmark AST と組み合わせて「コードブロック外のテキストノードに含まれる第1層文字のみ報告」する精度向上は、Phase 5 で第2層対応と同時に検討
- qwert-core/sanitize.rs に検出関数を実装、GUI / CLI / MCP すべてから呼び出し可能とする
- MCPツールの `read` レスポンスに `invisible_char_warnings` フィールドを追加し、Claude等が間接プロンプトインジェクションのリスクを認識できるようにする（Phase 3）

**API設計（qwert-core/sanitize.rs）**:

```rust
pub struct InvisibleCharFinding {
    pub line: usize,
    pub column: usize,
    pub char_value: char,
    pub category: InvisibleCharCategory,
}

pub enum InvisibleCharCategory {
    UnicodeTag,      // U+E0000〜U+E007F
    NullByte,        // U+0000
    C0Control,       // U+0001〜0008, 000E〜001F
    C1Control,       // U+0080〜009F
}

pub fn detect_invisible_chars(content: &str) -> Vec<InvisibleCharFinding>;
// Phase 2 ではdetectのみ。strip_invisible_chars はPhase 3以降の検討事項。
```

**設定（config.toml、v9新設）**:

```toml
[sanitize]
warn_invisible_chars = true       # 第1層検出の有効/無効、デフォルトON
# strip_on_save = false           # Phase 3以降の検討事項。Phase 2では未実装
```

**第2層・第3層の扱い（Phase 5 発展機能）**:

第2層（文脈依存の不可視文字）はPhase 5でpulldown-cmark AST連携を前提に段階的に対応する。「文脈判定をせず、存在を報告するだけ」のアプローチと「ASTのノード種別と組み合わせて文脈判定する」アプローチがあり、Phase 5 でユーザー需要を見て選択する。

第3層は `read_to_string` が `Err` を返すケースをqwert側で「このファイルには不正なバイトシーケンスがあります（位置: N byte目）」と詳細表示する範囲で対応する。Phase 2 で対応可能。

**スコープ外**:

- プロンプトインジェクションの意味的検出（sibylline-clean的アプローチ）。MCP利用側の責務とする
- ホモグリフ攻撃検出（似た見た目の異字検出）
- raw HTML（`<script>`, `<iframe>`等）の検出。これはレンダリング安全性の問題であり、§13 セキュリティ境界のCSP/サニタイズ層で扱う

---

## §12 外部変更検知と並行アクセス設計

qwert GUIと外部プロセス（qwert CLI、qwert mcp、Syncthing、neovim等）が同一ファイルに同時アクセスするケースに対応する。

### プロセス構成

```
┌─ qwert GUI ─────────┐     ┌─ qwert mcp ────────────┐
│ Tauri プロセス        │     │ 別プロセス（Claude起動） │
│ qwert-core           │     │ qwert-core              │
│   read / write       │     │   read / write          │
└──────────┬───────────┘     └──────────┬──────────────┘
           │                            │
           ▼                            ▼
    ┌──────────────────────────────────────────┐
    │           ファイルシステム                 │
    │    (Syncthing / neovim / 他ツールも参加)   │
    └──────────────────────────────────────────┘
```

### 3段階の防御

**Level 1: notify による外部変更検知 + リロード確認（Phase 1）**

`notify` crateのファイルウォッチャーが外部書き込みを inotify(Linux) で検知し、Tauriイベント経由でGUIフロントエンドに通知する。

- エディタが未変更（saved）: 自動リロード
- エディタが変更中（unsaved）: ダイアログ表示
  - 「外部の変更を読み込む」→ エディタ内容をディスク内容で置換
  - 「自分の変更を保持する」→ 編集続行（次回保存で上書き）

これはMCP経由の書き込み、Syncthing同期、neovim等の外部エディタ、すべての外部変更に対して統一的に機能する。

**Android における制約（v12新設）**: Android の共有ストレージ（FUSE / emulated 層）上では inotify が外部変更を取りこぼす場合がある。Android では notify によるリアルタイム検知に加え、アプリのフォアグラウンド復帰時（onResume 相当）に vault を再スキャンするフォールバックを併用する。

**Level 2: mtime ベースの楽観的ロック（Phase 2）**

書き込み前にファイルの最終更新日時（mtime）を確認し、読み込み時と変わっていたら競合として扱う。CLIでは `--if-match <mtime>` フラグで明示的にロック要求する。

```rust
pub fn write_file_safe(
    path: &Path,
    content: &str,
    expected_mtime: SystemTime,
) -> Result<WriteResult> {
    let current_mtime = fs::metadata(path)?.modified()?;
    if current_mtime != expected_mtime {
        return Ok(WriteResult::Conflict { /* ... */ });
    }
    atomic_write(path, content)?;
    Ok(WriteResult::Success)
}
```

mtime不一致は exit code 4（Conflict）を返す。

**Level 3: MCPツールへの編集中ヒント（Phase 3）**

MCPサーバーのツールに「このファイルは現在GUIで編集中」フラグを返し、Claudeが編集中ファイルを上書きしないよう誘導する。

### CRDTは不要

qwertではCRDT（Yjs/Automerge等）は導入しない。CRDTはリアルタイム同時編集（Google Docs等）向けであり、Claude Desktopからの書き込みは低頻度のため、notify + 楽観的ロックで十分。Obsidianも外部変更に対してCRDTは未採用。

---

## §13 セキュリティ境界（v9新設）

qwertはローカルMarkdownエディタとして、AIエージェントに開発権限を付与する場面で攻撃面最小化を設計目標に含める。設計哲学の Secure well 層に対応するセクションであり、以下の境界を構造的に実装する。

### 構造的に拒否する機能

以下は「機能として存在しない」ため攻撃経路にならない。Obsidian / VS Code / Notion 等の「全機能」ツールが多層防御で守ろうとする領域を、qwertは最初から引き受けない選択を取る。

- 外部URL画像 `![](https://...)` の取得・描画
- iframe `<iframe>` の描画
- 外部スクリプト・スタイルシート・フォントの読み込み
- JavaScript URI（`javascript:`）およびデータURI（画像以外）
- 任意コード実行（`<script>` タグ、HTMLイベントハンドラ）
- Markdown以外のファイル形式のネイティブ閲覧（PDF, Canvas, 動画等）
- プラグインによる任意コードロード
- カスタムCSS注入（appearance.toml の第2層設定はRust側ゲートキーパーでサニタイズされた範囲のみ）
- グローバル設定のホットリロード（vault スコープのみ即時反映。反映対象は Rust ゲートキーパーでサニタイズ済みの CSS 変数差し替えに限定され、コード実行・外部リソース読み込みの経路は持たない）

これらは v9 の「視覚表現の上限はMermaidまで」「グローバル設定は起動時1回読み込み」「プラグインシステム不採用」といった他の方針と整合する形で、qwert全体の設計の一貫性を構成する。

### CSP方針（tauri.conf.json）

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' data: qwert-img: http://qwert-img.localhost;
connect-src 'self';
object-src 'none';
frame-src 'none';
```

`'unsafe-inline'` style-src はMermaidが内部でインラインstyleを生成するため現実的に必要。ただし `unsafe-eval` は含めない。`asset:` は img-src から除外する。ローカル画像配信は asset protocol ではなく独自スキーム `qwert-img` に一本化したため（理由は下記「ローカル画像配信の境界」）。Android では convertFileSrc 同様にスキームが `http://qwert-img.localhost` 形式へ写るため両形式を許可する。

### Tauri capabilities最小化

| capability | 設定 | 根拠 |
|-----------|------|------|
| `fs` | vault配下のみ許可 | 独自コマンドに集約|
| `http` | デフォルト無効 | qwertは外部URL取得機能を持たない |
| `shell` | 全面不許可 | 任意コマンド実行経路を構造的に閉じる |
| `dialog` | ファイル選択のみ許可 | Vault選択時の OpenDialog のみ |
| 独自スキーム `qwert-img` | `register_uri_scheme_protocol` で登録、ハンドラ内で `resolve_path` 検証 | ローカル画像プレビュー用。asset protocol は不採用（scope glob の推測と Android での配信不具合を避け、Rustゲートキーパーで検証を一元化） |

設定ミスがランタイムではなくビルド時に検出される（Tauri 2.0 の Permission静的検証）。

### ローカル画像配信の境界（v12新設）

vault 内のローカル画像を WebView へ配信する経路は、独自 URI スキーム `qwert-img` で実装する。Markdown レンダリング時に Rust 側（markdown.rs の `Event::Image`）が画像参照を解決し、以下の境界を構造的に適用する。

- 外部参照拒否: `http` / `https` / `data` スキームの画像は描画しない（§13 の外部URL画像拒否方針と整合）。vault 相対パスのみを受理する。
- パス検証の二重化: markdown.rs で `resolve_path` により vault 配下を検証して URL を生成し、`qwert-img` プロトコルハンドラでも受信パスを再度 `resolve_path` で検証する（多層防御）。vault 外・不存在は壊れリンク表示とし、プレビューを壊さない。
- 一方向データフロー維持: バイト読み出しは Rust の `std::fs` で行い、フロントエンドはファイルを直接読まない。
- svg の扱い: `qwert-img` ハンドラは svg に `image/svg+xml` を厳格付与し、`<img>` 要素専用とする（インライン展開しない）。`<img src>` で読み込まれた svg はブラウザが画像として扱いスクリプトを無効化するため比較的安全だが、必要に応じて DOMPurify を通す余地を残す。
- 画像 Revision（参照追従）は行わない（§8 のとおり画像 Revision は不採用）。`move_file` で画像を移動できるが、ノート内の `![](相対パス)` は自動追従しない。

### vault スコープ設定ファイルの信頼境界（v10）

vault スコープ設定ファイル（vault/.qwert/appearance.toml）は、グローバル設定と
同一のサニタイズパイプライン（§10 のキー種別別許可パターン、危険値拒否）を通る。
`qwert appearance set` 経由でも raw `qwert file write` 経由でも、また AI が直接
ファイルを編集した場合でも、フロントエンドへ到達する前に必ず Rust 側ゲートキーパーで
検証される。信頼境界はすべての経路で同一。

appearance.toml を MCP/CLI 経由で書き込めることのリスク評価:
- 書き込み内容は必ずサニタイザーを通るため、悪意ある書き込みが成功しても最悪ケースは
  「サニタイズを通過した有効だが見にくいテーマ」止まり。コード実行・外部リソース読み込みの
  ベクターは構造的に存在しない
- 間接プロンプトインジェクションで「テーマを真っ白にして文字を見えなくする」程度の
  嫌がらせは理論上可能だが、ホットリロードなので次の正常な指示で即座に戻せ、
  `qwert appearance status` で異常なコントラスト比（fail）を検知できる
- vault スコープファイルへの書き込みは Tauri fs capability の vault 配下スコープ内に収まる
  （.qwert は vault 配下）。vault 外への書き込み経路は構造的に存在しない

### プロンプトインジェクション耐性

AIエージェントがvault内の悪意あるノートを読んで指示に従うリスク（間接プロンプトインジェクション）に対して、qwertの機能面では以下により攻撃経路が構造的に存在しない:

- shellアクセスなし（任意コマンド実行不可）
- 外部URL取得機能なし（抽出データの外部送信経路なし）
- wikilink操作は確定的AST解析で処理（pulldown-cmark）し、ノート内容を指示として実行しない
- Revisionシステムはコードブロック・HTMLコメント・frontmatter内のwikilinkを除外し、注入された見せかけのwikilinkを書き換えない

ただしAIエージェント側（Claude Code等）が悪意ある指示を解釈して `qwert` コマンドを発行する可能性は排除できない。vault配下への書き込み以上の影響は発生しない設計となっている点が、qwertの提供する境界となる。

§11 ファイル品質保証（不可視文字検出）は、この境界の前段で「ノート内に通常存在しない文字が含まれる」事実をユーザー・AIエージェント双方に伝える素材検出層として機能する。

### 派生プロジェクトの位置づけ

qwert本体への機能追加要望は原則として外部ツール連携（hook / MCP）で対応する。本体を小さく保ったうえで、機能拡張ニーズを別バイナリとして受け止める方針:

- `qwert-figma-preview`: Figma URLのサムネイル取得（hook経由で起動）
- `qwert-canvas-bridge`: Excalidraw / tldraw 等との連携
- `qwert-pdf-renderer`: PDFサムネイル
- その他、qwert-coreライブラリ公開後に他プロジェクトが depend する形での独自CLI/GUI

これらはqwert本体に機能追加するのではなく、ユーザーが必要な道具だけを追加できるUnix哲学に沿った形で提供する。qwert本体の Secure well を維持しつつ、利便性は派生プロジェクトで吸収する分業モデル。

---

## §14 CLIサブコマンド + MCPサーバー（v6で体系刷新）

qwertはGUIモード以外に、CLIサブコマンドとMCPサーバーモードを持つ。すべてqwert-coreの同一関数を呼ぶ。

### 設計原則

CLIを先に実装し、MCPは薄いラッパーとして後から被せる。エージェントフレンドリーCLIの8原則（構造化出力 / セマンティック終了コード / 非対話モード / Noun-Verb文法 / アクション可能エラー / dry-run / コンポーザビリティ）に準拠する。

### サブコマンド体系（Noun-Verb canonical form）

リソースを名詞とし、その配下に動詞を配置する。エージェントが命名規則から次の操作を予測できる構造とする。

```bash
# GUI・サーバーモード / メタコマンド（Noun-Verb ではない）
qwert                               # GUIモード（Tauri起動）
qwert mcp --vault <path>            # MCPサーバーモード（stdio JSON-RPC）
qwert generate-man                  # man ページ生成（hidden。--help/describe には非表示。§22参照）

# file リソース（raw ファイル操作）
qwert file read <path>              # ファイル読み取り → stdout
qwert file write <path>             # stdin → ファイル書き込み（アトミック）
qwert file list [--tree]            # vault内の.mdファイル一覧

# note リソース（Markdownとして認識される操作）
qwert note render <path>            # Markdown → HTML → stdout
qwert note backlinks <path>         # バックリンク一覧
qwert note revision <path> [opts]   # Revisionシステム
qwert note scan <path>              # v9: 不可視文字検出（第1層、Phase 2）

# vault リソース（vault全体への操作）
qwert vault search <query>          # 全文検索
qwert vault status                  # vault状態レポート
qwert vault scan                    # v9: vault全体の不可視文字スキャン（Phase 2）

# appearance リソース（視覚設定への操作）
qwert appearance contrast --fg <hex> --bg <hex>  # v10: コントラスト比検証
qwert appearance set [opts]                       # v10: 検証付き設定書き込み
qwert appearance status                           # v10: 現在の有効設定レポート
```

### メタコマンドの扱い（v10で明文化）

`mcp` / `describe` / `generate-man` は Noun-Verb ではないメタコマンドである。とくに `generate-man` は man ページ生成専用で、clap 側では `#[command(hide = true)]` を付与し `--help` / `describe` の出力には現さない。「明記する」対象と「隠す」対象は異なる——仕様書（本書）にはメタコマンドとして明記して完全性を保つ一方、実行時出力ではエージェント向けの予測可能性（canonical な Noun-Verb のみ提示）を損なわないよう隠す。`generate-man` はリリースビルド / CI からのみ呼ぶ（§22 ビルドとインストール参照）。Phase 2 は hidden A（実行時サブコマンド + `hide`）で整合を取り、将来パッケージ配布を始める段で build.rs によるビルド時生成（B）への移行を検討する。

### 短縮エイリアス（人間のUX向け）

`qwert describe` および `qwert --help` の出力では canonical form のみを提示するが、人間がターミナルから叩く際の利便性のため短縮エイリアスを提供する。

| 短縮形                      | canonical form                |
| ------------------------ | ----------------------------- |
| `qwert read <path>`      | `qwert file read <path>`      |
| `qwert write <path>`     | `qwert file write <path>`     |
| `qwert list`             | `qwert file list`             |
| `qwert render <path>`    | `qwert note render <path>`    |
| `qwert backlinks <path>` | `qwert note backlinks <path>` |
| `qwert revision <path>`  | `qwert note revision <path>`  |
| `qwert search <query>`   | `qwert vault search <query>`  |
| `qwert status`           | `qwert vault status`          |

エージェント向けのドキュメント（`--help` / SKILL.md / MCP tool description）では canonical form を正とする。短縮形は人間専用の「裏口」として機能する。

appearance 系コマンド（contrast / set / status）には短縮エイリアスを提供しない。
低頻度操作であり、エージェント向けの予測可能性を優先して canonical form のみとする。

### 出力フォーマット規約

全コマンドは `--format` フラグで出力形式を切替可能。デフォルトはコマンドの性質に応じて決定する。

| 形式 | 用途 | 採用コマンド |
|------|------|-------------|
| `json` | 構造化データ、エージェント向け | 全コマンドで対応 |
| `path` | 改行区切りパスのみ、xargs親和 | list / search / backlinks |
| `text` | 人間向け整形 | 各コマンド（デフォルト動作） |
| `raw` | 加工なし原文 | read / render |
| `diff` | unified diff | revision --dry-run のみ |

**JSONエンベロープ**（全JSON出力に必須）:
```
{
  "schema_version": "v1",
  "kind": "<コマンド固有の種別>",
  /* コマンド固有のペイロードを top-level に展開 */
}
```

**コマンド別デフォルト**:

| コマンド | デフォルト | 代替形式 |
|---------|-----------|---------|
| `file read` | raw | json |
| `file write` | (stdin入力のみ） | json（結果レポート） |
| `file list` | json | path / text |
| `note render` | raw（HTML） | json |
| `note backlinks` | json | path / text |
| `note revision` | text（確認プロンプト） | json / diff |
| `note revision --dry-run` | json | diff |
| `vault search` | json | path / text |
| `vault status` | json | text |
| `appearance contrast` | text | json |
| `appearance set` | text | json |
| `appearance status` | json | text |

**stdout / stderr 分離**:
- stdout: データのみ
- stderr: 進捗・警告・エラーメッセージ

### パイプ親和性の具体例

```bash
# vault内の全ノードから TODO を grep（xargs経由）
qwert file list --format path | xargs grep -l "TODO"

# 特定ノートのバックリンクを全て開く（neovimで）
qwert note backlinks specs/auth.md --format path | xargs nvim

# revision の diff を jj に直接 apply（試行）
qwert note revision old.md --dry-run --format diff | patch -p1 --dry-run

# JSON から特定フィールド抽出
qwert vault search "TODO" --format json | jq -r '.hits[].path'
```

### 終了コード規約（v6新設）

セマンティック終了コードにより、エージェントが `$?` を見て次の一手を判断できる。

| コード | カテゴリ | 意味 | 該当例 |
|-------|---------|------|--------|
| 0 | Success | 正常終了 | - |
| 1 | General | 分類不能なエラー | 予期しないI/Oエラー、パニック |
| 2 | Usage | 引数・構文エラー | 必須フラグ欠落、型不一致 |
| 3 | NotFound | リソース未発見 | vault/ファイルが存在しない |
| 4 | Conflict | 操作実行不能な競合 | mtime不一致、pending-revision残存、ロック取得失敗 |
| 5 | Validation | 入力値検証エラー | wikilink形式不正、命名規則違反、不正なUTF-8 |

**重要な境界条件**:
- 終了コード4は**当該コマンドの実行結果**として返す。vault内にsync-conflictファイルが存在するだけでは exit 4 を返さない（それは `qwert vault status` の守備範囲）。
- 認証カテゴリは qwert には存在しない（ローカル専用のため）。

### 非対話モード規約（v6新設）

エージェント起動時にプロンプトで停止しないよう、TTY判定により挙動を変える。

- **TTY検知時**（人間利用）: `confirm_before_execute = true` に従い確認プロンプトを出す
- **非TTY時**（パイプ/スクリプト/エージェント経由）: プロンプトを出さない。代わりに:
  - `--yes` が指定されていれば実行
  - `--yes` なしでは exit code 2（Usage）で拒否
  - stderr に `error: non-interactive context requires --yes for destructive operations` を出力

### アクション可能なエラー（v6新設）

エラー出力には「次に何をすべきか」を明示的に含める。qwert-coreの`Error`構造体に以下のフィールドを持たせ、CLIレイヤーはそれをJSON化してstderrに出す。

**エラーJSON構造**:
```json
{
  "schema_version": "v1",
  "kind": "error",
  "category": "validation",
  "exit_code": 5,
  "message": "wikilink target 'auth' matches multiple files",
  "candidates": [
    {"value": "specs/auth.md", "label": "specs/auth.md"},
    {"value": "draft/auth.md", "label": "draft/auth.md"}
  ],
  "required_args": [],
  "next_step": "Disambiguate with full path: qwert note revision specs/auth.md"
}
```

**エラーパターン別の next_step 例**:

| エラー | category | next_step |
|-------|---------|-----------|
| vault未発見 | NotFound | `Run qwert with --vault <PATH> or cd into a vault` |
| wikilink解決失敗 | Validation | `Use candidates above or check --format path for full paths` |
| mtime競合 | Conflict | `Re-read with qwert file read <path>, merge, then retry` |
| pending-revision残存 | Conflict | `Run qwert vault status to inspect, then resolve manually` |
| 無効なrevision名 | Validation | `Use one of: increment \| date \| semver \| manual` |

### describe コマンド（Phase 3）

`qwert describe <subcommand> --format json` により、各コマンドの引数スキーマをJSONで返す。MCPのtool description生成にも同一スキーマを用いる（DRY）。Phase 2では `clap_mangen` による man ページ生成で代替する。

### `qwert revision` の動作例

```bash
# 新名称を自動生成（incrementルール）+ dry-run でプレビュー
$ qwert note revision specs/auth.md --dry-run
{
  "schema_version":"v1","kind":"revision_plan","dry_run":true,
  "old_name":"auth","new_name":"auth_2",
  "affected_files":[...],"total_wikilinks":5
}

# diff で実際の変更内容を確認
$ qwert note revision specs/auth.md --dry-run --format diff
--- a/specs/index.md
+++ b/specs/index.md
@@ -12,7 +12,7 @@
-- [[auth]] - 認証仕様
+- [[auth_2]] - 認証仕様

# 人間向け: 確認あり実行
$ qwert note revision specs/auth.md
Revision: auth.md → auth_2.md
Affects: 3 files (5 wikilinks)
Execute? [y/N] y
Done: auth.md → auth_2.md (5 refs updated in 3 files)

# スクリプト/エージェント向け: 非対話実行
$ qwert note revision specs/auth.md --yes --format json

# 名称を手動指定
$ qwert note revision specs/auth.md --name auth_v2_jwt --yes
```

### `qwert appearance contrast` の動作仕様（v10）

性質: 純粋な情報コマンド。色のペアを受け取りコントラスト比と WCAG レベルを返す。
デフォルトでは判定結果に関わらず正常終了（exit 0）する。

```bash
# text 出力（TTY時デフォルト）
$ qwert appearance contrast --fg "#1a1a1a" --bg "#ffffff"
Contrast: 16.1:1
Normal text: AAA (threshold 7:1)
Large text:  AAA (threshold 4.5:1)

# JSON 出力
$ qwert appearance contrast --fg "#1a1a1a" --bg "#ffffff" --format json
{
  "schema_version": "v1",
  "kind": "contrast_result",
  "fg": "#1a1a1a",
  "bg": "#ffffff",
  "ratio": 16.1,
  "level_normal": "AAA",
  "level_large": "AAA"
}

# ビルトインプリセットの検証
$ qwert appearance contrast --preset dark-high-contrast --format json

# アサートモード（CI/エージェント向け）: 閾値未満なら exit 5
$ qwert appearance contrast --fg "#888888" --bg "#dddddd" --assert-aa
error: contrast 2.7:1 below WCAG AA (requires 4.5:1 normal text)
$ echo $?    # → 5
```

フィールド定義:
- ratio: 相対輝度比（小数第1位）
- level_normal / level_large: "AAA" | "AA" | "fail" の三値
- 大テキスト閾値: 24px 以上、または 18.66px(≈19px) bold 以上

終了コード:
| 状況 | exit code |
|------|-----------|
| 正常に計算完了（判定結果は問わない） | 0 (Success) |
| 不正な hex / fg・bg ペア不成立 | 5 (Validation) |
| --assert-aa 指定 + AA 未満 | 5 (Validation) |
| --assert-aaa 指定 + AAA 未満 | 5 (Validation) |
| 引数不足・型不一致 | 2 (Usage) |

「計算は成功したが閾値を満たさない」は本質的にエラーではないため、デフォルトは exit 0。
ゲートとして使う場合のみ --assert-aa / --assert-aaa で exit 5 にする。
exit 1（一般エラー）への割当は避ける（セマンティック終了コード表との整合）。
--assert-* は normal text 閾値で判定する（Markdown 本文は通常テキスト適用が主のため）。

### `qwert appearance set` の動作仕様（v10、推奨書き込み経路）

```bash
# vault スコープにカスタム色を書き込み（デフォルトで vault スコープ）
$ qwert appearance set --fg "#3a2418" --bg "#fdf6e3"
Set fg=#3a2418 bg=#fdf6e3 (contrast 7.3:1, AAA) in vault/.qwert/appearance.toml

# グローバルに書き込み（即時反映されず次回起動で適用）
$ qwert appearance set --preset dark --scope global --yes
Set preset=dark in ~/.config/qwert/appearance.toml. Restart to apply.

# 非対話（エージェント向け）
$ qwert appearance set --fg "#3a2418" --bg "#fdf6e3" --yes --format json
```

挙動:
- fg/bg はペア必須。片方のみ指定は exit 5（F24）、next_step に "Specify both --fg and --bg"
- preset と fg/bg は相互排他。両方指定は exit 5、next_step に
  "Use either --preset or --fg/--bg, not both"
- --scope のデフォルト: vault がアクティブなら vault スコープ、なければ global
  （Phase 2 では vault スコープ未実装のため暫定で global 既定。Phase 3 の vault スコープ化以降に vault 優先へ）
- 書き込み前にコントラストを計算し結果を出力に含める
- AA 未満でも書き込みは行う（警告は出すが拒否しない＝ユーザーの自律性を尊重）。
  --require-aa 指定時のみ AA 未満を exit 5 で拒否
- アトミック書き込み（tmp → rename）。vault スコープ書き込み後は notify watcher が
  即時反映を発火。global 書き込みは次回起動で反映（出力で "Restart to apply" を明示）
- 非対話時（非TTY）は --yes がなければ exit 2（破壊的操作のため）


### Claude Desktop MCP設定例

```json
{
  "mcpServers": {
    "qwert": {
      "command": "/home/user/.cargo/bin/qwert",
      "args": ["mcp", "--vault", "/home/user/notes"]
    }
  }
}
```

### 追加依存

```toml
# CLI
clap = { version = "4", features = ["derive"] }
clap_mangen = "0.2"          # man ページ生成（Phase 2）
is-terminal = "0.4"          # TTY 判定

# diff 生成
similar = "2"                # unified diff for --dry-run --diff

# MCP サーバー（Phase 3、実装済み）
rmcp = { version = "1", features = ["server", "transport-io", "macros"] }
tokio = { version = "1", features = ["full"] }
```

---

## §15 アーキテクチャ

### qwert-core の位置づけ

```
┌─────────────────────────────────────────────────────┐
│                  qwert バイナリ                       │
│                                                      │
│  ┌── GUI モード ──┐  ┌── CLI ──┐  ┌── MCP ────────┐ │
│  │ Tauri + SolidJS│  │ clap    │  │ rmcp (stdio)  │ │
│  │ WebView UI     │  │ stdout  │  │ JSON-RPC      │ │
│  └───────┬────────┘  └────┬────┘  └──────┬────────┘ │
│          │                │              │           │
│          └────────────────┼──────────────┘           │
│                           │                          │
│              ┌────────────┴────────────┐             │
│              │       qwert-core        │             │
│              │  vault.rs  markdown.rs  │             │
│              │  search.rs link_index.rs│             │
│              │  revision.rs config.rs  │             │
│              │  error.rs  status.rs    │             │
│              │  sanitize.rs (v9)       │             │
│              └─────────────────────────┘             │
└──────────────────────────────────────────────────────┘
```

### ディレクトリ構成

```
qwert/
├── Cargo.toml                    # Workspaceルート
├── crates/
│   └── qwert-core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── vault.rs          # Vault管理（スキャン、ウォッチ、アトミック書き込み）
│           ├── markdown.rs       # pulldown-cmark パース、HTML変換、math extension対応（v8）
│           ├── search.rs         # 全文検索（ignore + regex）
│           ├── link_index.rs     # wikilink インデックス、バックリンク
│           ├── revision.rs       # Revisionシステム（wikilink自動更新）
│           ├── revision_diff.rs  # Phase 2: dry-run 用 diff 生成
│           ├── status.rs         # Phase 2: vault状態レポート
│           ├── appearance.rs     # v7: appearance.toml 読み込み + サニタイザー
│           ├── sanitize.rs       # v9: 不可視文字検出（Phase 2）
│           ├── error.rs          # Phase 2: next_step/candidates 付きエラー型
│           └── config.rs         # TOML設定ファイル管理
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   └── src/
│       ├── main.rs               # エントリポイント（GUI / CLI / MCP 分岐）
│       ├── lib.rs
│       ├── cli/                  # Phase 2: CLI サブコマンド
│       │   ├── mod.rs
│       │   ├── file.rs           # file read/write/list
│       │   ├── note.rs           # note render/backlinks/revision
│       │   ├── vault.rs          # vault search/status
│       │   ├── format.rs         # --format 切替 + JSONエンベロープ
│       │   ├── exit_code.rs      # セマンティック終了コード定義
│       │   └── tty.rs            # 非対話判定
│       ├── mcp.rs                # Phase 3: MCP サーバー
│       └── commands/             # Tauriコマンド（GUI用 IPC）
│           ├── mod.rs
│           ├── file.rs
│           ├── vault.rs
│           ├── search.rs
│           ├── markdown.rs
│           ├── link.rs
│           ├── revision.rs
│           ├── status.rs         # Phase 2
│           ├── sanitize.rs       # v9: 不可視文字検出結果取得（Phase 2）
│           └── appearance.rs     # v7: load_appearance コマンド
├── src/                          # フロントエンド（SolidJS）
│   ├── index.html
│   ├── index.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── FileTree.tsx
│   │   ├── Editor.tsx
│   │   ├── Preview.tsx
│   │   ├── SearchPanel.tsx
│   │   ├── CommandPalette.tsx
│   │   ├── RevisionDialog.tsx
│   │   ├── VaultStatusBanner.tsx # Phase 2: sync-conflict等の通知
│   │   └── StatusBar.tsx
│   ├── stores/
│   │   ├── vault.ts
│   │   ├── editor.ts
│   │   ├── settings.ts
│   │   └── appearance.ts         # v7: CSS変数適用ストア
│   ├── types/
│   │   ├── brand.ts              # Branded Types（RelativePath / AbsolutePath）
│   │   ├── constants.ts          # as const 定数
│   │   └── models.ts             # Tauriコマンド応答型
│   ├── lib/
│   │   ├── tauri.ts
│   │   ├── autosave.ts
│   │   ├── appearance.ts         # v7: document.documentElement.style.setProperty ラッパー
│   │   ├── math.ts               # v8: KaTeX 遅延ロード + 描画ラッパー
│   │   ├── mermaid.ts            # v8: mermaid.js 遅延ロード + 描画ラッパー
│   │   └── codemirror/
│   │       ├── setup.ts
│   │       ├── markdown.ts
│   │       ├── theme.ts          # v7: CSS変数参照の単一テーマ定義
│   │       ├── highlight.ts      # v7: 構文ハイライトCompartment
│   │       └── wikilink.ts
│   └── styles/
│       ├── global.css
│       ├── tokens.css            # v7: --qw-* CSS変数定義
│       ├── theme-default.css     # v7: ライト背景プリセット
│       ├── theme-high-contrast.css  # v7
│       ├── theme-dark.css        # v7
│       ├── theme-dark-high-contrast.css  # v7
│       ├── editor.css
│       └── preview.css
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
└── config/
    └── default.toml
```

---

## §16 主要依存一覧

### Rust ツールチェーン

`qwert-core` は edition 2024 + let-chains を使用するため **Rust 1.88 以上**が必須。
`mise.toml` で `rust = "1.88"` を pin する（C-6, ビルド再現性）。

### Rust (Cargo.toml)

```toml
# qwert-core
[dependencies]
pulldown-cmark = { version = "0.12", features = ["simd", "html"] }  # v8: math extension を使用
notify = "7"
walkdir = "2"
ignore = "0.4"
regex = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
directories = "5"
thiserror = "2"

# Phase 2: Revisionシステム
tempfile = "3"
rayon = "1"
unicase = "2"
unicode-normalization = "0.1"
similar = "2"                     # v6追加: unified diff

# src-tauri
[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-dialog = "2"
qwert-core = { path = "../crates/qwert-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }      # Phase 2
clap_mangen = "0.2"                                  # v6追加: man 生成
is-terminal = "0.4"                                  # v6追加: TTY 判定
rmcp = { version = "1", features = ["server", "transport-io", "macros"] }  # Phase 3: MCP（実装済み）
tokio = { version = "1", features = ["full"] }                              # Phase 3: MCP（実装済み）
```

### JavaScript (package.json)

```json
{
  "dependencies": {
    "solid-js": "^1.9",
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-fs": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "solid-codemirror": "^2",
    "@codemirror/state": "^6",
    "@codemirror/view": "^6",
    "@codemirror/lang-markdown": "^6",
    "@codemirror/language-data": "^6",
    "@codemirror/theme-one-dark": "^6",
    "@replit/codemirror-vim": "^6",
    "codemirror": "^6"
  },
  "devDependencies": {
    "@solidjs/testing-library": "^0.8",
    "vitest": "^2",
    "vite": "^6",
    "vite-plugin-solid": "^2",
    "typescript": "^5",
    "@tauri-apps/cli": "^2"
  }
}
```

Phase 2で追加:
```json
{
  "mermaid": "^11",
  "highlight.js": "^11",
  "katex": "^0.16",
  "dompurify": "^3"
}
```

※ `katex` はJSライブラリとCSS、フォント一式を含む。Viteで `import "katex/dist/katex.min.css"` および `import "katex"` し、遅延ロードはdynamic importで実現する。

---

## §17 TypeScript型安全基盤（Liminia Type Safety v1 準拠）

### tsconfig.json

```jsonc
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "erasableSyntaxOnly": true,
    "verbatimModuleSyntax": true,
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "noEmit": true,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "useDefineForClassFields": true,
    "jsx": "preserve",
    "jsxImportSource": "solid-js"
  }
}
```

### Branded Types（`src/types/brand.ts`）

```typescript
declare const __brand: unique symbol;
export type Brand<T, B extends string> = T & { readonly [__brand]: B };

export type RelativePath = Brand<string, 'RelativePath'>;
export type AbsolutePath = Brand<string, 'AbsolutePath'>;

export function relativePath(s: string): RelativePath { /* validation */ }
export function absolutePath(s: string): AbsolutePath { /* validation */ }
```

### as const パターン（`src/types/constants.ts`）

```typescript
export const VIEW_MODE = { EDITOR: 'editor', SPLIT: 'split', PREVIEW: 'preview' } as const;
export type ViewMode = (typeof VIEW_MODE)[keyof typeof VIEW_MODE];

export const THEME = { DARK: 'dark', LIGHT: 'light' } as const;
export type Theme = (typeof THEME)[keyof typeof THEME];

export const SAVE_STATE = { SAVED: 'saved', UNSAVED: 'unsaved', SAVING: 'saving' } as const;
export type SaveState = (typeof SAVE_STATE)[keyof typeof SAVE_STATE];

export const EXIT_CODE = {
  SUCCESS: 0, GENERAL: 1, USAGE: 2,
  NOT_FOUND: 3, CONFLICT: 4, VALIDATION: 5,
} as const;
export type ExitCode = (typeof EXIT_CODE)[keyof typeof EXIT_CODE];
```

---

## §18 設定ファイル

設定は用途別に2ファイルに分離する。

### config.toml（動作設定）

`~/.config/qwert/config.toml`（XDG準拠）

```toml
[general]
restore_last_vault = true
autosave_delay_ms = 3000

[editor]
show_line_numbers = true
word_wrap = true
tab_size = 4
use_spaces = true
vim_mode = false

[preview]
default_view = "split"
sync_scroll = true
render_mermaid = true
render_math = true               # v8追加: KaTeX描画の有効/無効

[file_tree]
show_hidden = false
file_extensions = ["md", "markdown"]
ignore_dirs = [".git", ".jj", "node_modules", ".obsidian", ".qwert/cache"]

[search]
case_sensitive = false
max_results = 100
context_lines = 2

[sync]
atomic_writes = true
detect_conflicts = true

[revision]
naming = "increment"
confirm_before_execute = true    # false でスクリプト向け無確認実行
dry_run_diff_default = false     # v6追加: --dry-run時に常にdiff生成するか
excluded_dirs = []               # v9追加: Revision対象外ディレクトリ
                                 # 例: ["docs/decisions"] でADR保護

[sanitize]
warn_invisible_chars = true      # v9追加: 第1層不可視文字検出
                                 # Unicode Tag, Null byte, C0/C1制御文字
# strip_on_save = false          # Phase 3以降の検討事項。Phase 2では未実装

[cli]
default_format = "auto"          # v6追加: "auto" | "json" | "text"
                                 # "auto": 各コマンドの既定フォーマットに従う
                                 #   （read/render=raw, list/search/status等=json,
                                 #    revision=text。「コマンド別デフォルト」表が正）
                                 # "json"/"text": 全コマンドの出力を明示的に上書き
```

### appearance.toml（視覚設定）

`~/.config/qwert/appearance.toml`（XDG準拠）

初回起動時は全行コメントアウトのテンプレートを生成する。値が未指定の項目にはqwertビルトインのデフォルトが適用される。

```toml
# qwert appearance configuration (global)
#
# This file is loaded once at startup. Changes take
# effect on next launch.
# For per-vault settings with immediate reflection,
# place vault/.qwert/appearance.toml instead.
#
# Use EITHER [color] preset OR fg+bg, not both
# (they are mutually exclusive).
#
# Keys under [color.advanced] map directly to
# CSS Custom Properties (--qw-*). Not exposed in the
# UI, not guaranteed stable across versions.

[text]
# font_size = 16
# font_family = "system-ui, sans-serif"
# line_height = 1.6
# letter_spacing = 0.0
# word_spacing = 0.0
# editor_max_width = 72

[color]
# preset = "default"          # "default" | "high-contrast" | "dark" | "dark-high-contrast"
# fg = "#1a1a1a"              # カスタム前景色（bgとセットで指定必須）
# bg = "#ffffff"              # カスタム背景色（fgとセットで指定必須）

[highlight]
# enabled = true

[color.advanced]
# cm-keyword = "#7c3aed"
# cm-string = "#059669"
# cm-comment = "#9ca3af"
# cm-heading = "#1e40af"
# cm-link = "#2563eb"
# cursor = "#1a1a1a"
# selection-bg = "#dbeafe"
```

**v6からの変更点**: v6 では `[general] theme = "dark" | "light"` と `[editor] font_family / font_size` を config.toml に持っていたが、v7 では視覚設定一式を appearance.toml に分離。テーマ選択は `[color] preset` に統合され、4種プリセット + カスタムに拡張された。

vault スコープテンプレート(vault/.qwert/appearance.toml)
```toml
# qwert appearance configuration (vault scope)
#
# This file overrides ~/.config/qwert/appearance.toml
# for this vault only. Changes here are reflected
# immediately (no restart needed).
#
# Use EITHER [color] preset OR fg+bg, not both
# (they are mutually exclusive).
#
# ===== For AI assistants =====
# When the user asks to customize the appearance,
# follow this protocol:
#
# 1. Understand the user's intent (e.g. "warm tone",
#    "easy on the eyes at night", "high contrast").
# 2. Propose `fg` and `bg` as a hex pair satisfying
#    WCAG 2.2 contrast:
#    - AA  (required):  ratio >= 4.5:1 for normal text
#    - AAA (preferred): ratio >= 7:1
# 3. Compute the ratio using WCAG relative luminance.
#    Never propose one of fg/bg without the other
#    (WCAG 1.4.8 Failure F24).
# 4. Add a comment above the values stating the
#    achieved ratio and WCAG level, e.g.:
#      # contrast 7.3:1 (AAA), warm tone
#      fg = "#3a2418"
#      bg = "#fdf6e3"
# 5. To verify locally, the user can run:
#      qwert appearance contrast --fg <hex> --bg <hex>
#
# Answer me in English.  # <- edit to your preferred language (e.g. "Japanese", "日本語", "Français")
#
# ===== Configuration =====
#
# [text]
# font_size = 16
# font_family = "system-ui, sans-serif"
# line_height = 1.6
# letter_spacing = 0.0
# word_spacing = 0.0
# editor_max_width = 72
#
# [color]
# preset = "default"
# fg = "#1a1a1a"
# bg = "#ffffff"
#
# [highlight]
# enabled = true
#
# [color.advanced]
# cm-keyword = "#7c3aed"
# cm-string = "#059669"
# cm-comment = "#9ca3af"
# cm-heading = "#1e40af"
# cm-link = "#2563eb"
# cursor = "#1a1a1a"
# selection-bg = "#dbeafe"
```

---

## §19 キーボードショートカット

| 操作 | デスクトップ | Android | 備考 |
|------|------------|---------|------|
| 保存 | `Ctrl+S` | 自動保存のみ | |
| 新規ノート | `Ctrl+N` | FABボタン | |
| コマンドパレット | `Ctrl+P` | 検索バー | ファイル名ファジー検索 |
| 全文検索 | `Ctrl+Shift+F` | 検索バー | |
| 表示モード切替 | `Ctrl+E` | タブ切替 | Editor ↔ Split ↔ Preview |
| サイドバー切替 | `Ctrl+B` | ハンバーガーメニュー | |
| 設定画面 | `Ctrl+,` | 設定アイコン | |
| Undo | `Ctrl+Z` | CodeMirror標準 | Vim有効時は `u` |
| Redo | `Ctrl+Shift+Z` | CodeMirror標準 | Vim有効時は `Ctrl+R` |
| wikilinkジャンプ | `Ctrl+Click` | タップ | Phase 2 |

---

## §20 Syncthing連携設計

アプリ自体は同期機能を持たず、Syncthing互換を保証する。

**設計方針**:
- アトミック書き込み（tmp → rename パターン）でSyncthing競合を最小化
- `.stignore` テンプレート提供（`.qwert/cache/`、`.qwert/pending-revision.json`、`.qwert/editing_state.json` を除外。後者2つは揮発的・デバイスローカルな状態であり同期不要。editing_state.json を同期すると別デバイスの編集中フラグが混入する）
- notify crateでSyncthingが持ち込んだ変更をリアルタイム検知 → Tauriイベントでフロントエンド通知
- `*.sync-conflict-*` ファイルの検知 → `qwert vault status` で報告 + GUI上部バナー表示（VaultStatusBanner.tsx）
- `.qwert/appearance.toml` はデフォルトで同期対象とする（vault スコープテーマを
PC-Android 間で共有する想定）。デバイスごとに異なるテーマを使いたい場合は、
利用者が `.stignore` に `.qwert/appearance.toml` を追加して除外する。
これは「Syncthing 競合の責務は qwert にない」方針（§9）と整合する。

**責務境界（v6で明文化）**:

| 層 | 責務 | qwertの関与 |
|----|------|-----------|
| 同時編集の防止 | 利用者 | なし |
| 競合ファイルの生成 | Syncthing | なし |
| 競合ファイルの**検出・通知** | qwert | あり |
| 競合ファイルの解決 | 利用者（任意ツール） | なし |

sync-conflict の存在は個別コマンドの exit code には昇格しない（`vault status` の守備範囲）。

**vault 配置モデルと Android 権限（v12新設）**:

qwert はアプリ専用ディレクトリを作らず、ユーザーが指定した任意のディレクトリを vault とする。デスクトップ（§1）と同一モデルを Android にも適用し、両プラットフォームで「ユーザー指定ディレクトリ = vault」を統一する。

Android では共有ストレージ上の任意ディレクトリ（例: `/storage/emulated/0/notes`）を vault とするため `MANAGE_EXTERNAL_STORAGE`（全ファイルアクセス）権限を用いる。理由は二つ:

- qwert-core が `std::fs` で直接ファイルを読む設計であり、SAF（content:// 経由）では成立しない。
- Syncthing 等の共有ストレージ依存の同期ツールとの併用が、スコープドストレージ（SAF）では不可能（外部変更の監視ができず、ファイルアクセスごとの権限チェックで著しく遅くなるため）。

配布は §21 の APK/AAB（F-Droid・直接配布）を前提とし、`MANAGE_EXTERNAL_STORAGE` を許容する。Google Play 配布を将来検討する場合は、権限の正当化（document management / on-device file search カテゴリ）が別途必要となる。

**PC-Android同期フロー**:
```
[PC: Pop!_OS]                    [Android: Nothing Phone 3a]
  ~/notes/  ← Syncthing同期 →  /storage/emulated/0/notes/
    ├── daily/                      ├── daily/
    ├── projects/                   ├── projects/
    └── .qwert/                     └── .qwert/
         └── config.toml                 └── config.toml
```

---

## §21 開発フェーズ

### Phase 1: MVP（デスクトップ）- 推定 3〜4週間

1. プロジェクトセットアップ
   - `cargo create-tauri-app qwert -- --template solid-ts`
   - Workspace構成（qwert-core crate追加）
   - tsconfig厳格化（Liminia Type Safety v1 準拠）
   - TypeScript型基盤（Branded Types, as const 定数）
2. Rust側: qwert-core 基盤
   - vault.rs: ディレクトリスキャン、ファイル読み書き（アトミック）
   - config.rs: TOML設定読み書き
   - markdown.rs: pulldown-cmark パース → HTML変換
   - **appearance.rs: appearance.toml 読み込み + Rustサニタイザー（v7）**
3. Tauriコマンド層 + TypeScriptラッパー（Branded Types適用）
   - list_dir, read_file, write_file, render_markdown
   - **load_appearance（v7: サニタイズ済みkey-valueペアを返す）**
4. **視覚設定基盤（v7で前倒し）**
   - `--qw-*` CSS変数定義（tokens.css）
   - 単一のCodeMirror 6テーマ（CSS変数参照）
   - デフォルトテーマのWCAG AA準拠検証
   - `prefers-color-scheme` / `prefers-contrast` メディアクエリ対応
   - 構文ハイライトCompartment（On/Off切替準備）
5. SolidJS フロントエンド
   - FileTree, Editor (CodeMirror 6 + Vim切替）, Preview
   - Split View レイアウト
   - appearance store（起動時1回のCSS変数適用）
6. 基本機能の結合
   - ファイル選択 → エディタ表示 → プレビュー連動
   - 自動保存 + データ保全
   - 外部変更検知（notify → リロード確認ダイアログ）
   - 新規ファイル作成
   - 設定画面（Vimバインド切替、構文ハイライトOn/Off）

### Phase 2: コア機能完成 + CLI基盤 - 推定 3〜4週間

**CLI基盤（v6 で追加・前倒し）**:
1. **CLIサブコマンド（clap, Noun-Verb canonical + 短縮エイリアス）**
   - file read/write/list, note render/backlinks, vault search/status
   - 出力フォーマット基盤（json/path/text/raw、エンベロープ生成）
   - セマンティック終了コード（EXIT_CODE 定数の共有）
   - 非対話モード（is-terminal によるTTY判定）
   - エラー型（qwert-core/error.rs: next_step/candidates付き）
2. **vault status コマンド**
   - sync-conflict検出、pending-revision検査
3. **man ページ自動生成**（clap_mangen）

**Markdown / リンク機能**:
4. `[[wikilink]]` パース・リンクジャンプ・オートコンプリート
5. バックリンク表示パネル
6. 全文検索（ignore + regex）
7. コマンドパレット（ファイル名ファジー検索）

**Revisionシステム（CLI優先で実装）**:
8. qwert-core/revision.rs + revision_diff.rs
   - pulldown-cmark AST解析によるwikilink特定（コードブロック・HTMLコメント・frontmatter除外）
   - rayon並列スキャン + WALパターンバッチアトミック書き込み
   - on-revise フック呼び出し
9. `qwert note revision --dry-run`（JSON出力）
10. `qwert note revision --dry-run --diff` / `--format diff`（similar crate）
11. `qwert note revision --yes`（非対話実行）
12. GUI: ファイルツリー右クリック + RevisionDialog.tsx（diff プレビュー含む）

**同期 / 並行アクセス**:
13. Syncthing競合ファイル検知・通知（VaultStatusBanner.tsx）
14. mtime ベースの楽観的ロック（write_file_safe）
15. `qwert file write --if-match <mtime>`

**リッチコンテンツ描画（v8新設）**:
16. Markdown math extension有効化（qwert-core/markdown.rs、pulldown-cmark の `ENABLE_MATH`）
17. **KaTeX 数式レンダリング**:
    - `$...$` インライン数式、`$$...$$` ブロック数式
    - KaTeX CSS・フォント同梱、dynamic importによる遅延ロード（src/lib/math.ts）
    - `throwOnError: false` 設定によるフォールバック挙動
    - 誤検出防止ヒューリスティクス（`$` 前後の空白判定）
18. **Mermaidダイアグラム描画**:
    - コードブロック（情報文字列 `mermaid`）のみを対象
    - dynamic importによる遅延ロード（src/lib/mermaid.ts）
    - パースエラー時は元のコードブロックをそのまま表示
19. コードブロックのクライアント側ハイライト（highlight.js、遅延ロード）

**視覚設定（v7基盤 + v10改訂）**:
20. プリセットテーマ4種実装（default / high-contrast / dark / dark-high-contrast）
21. グローバル appearance.toml の読み込み + Rustサニタイザー（起動時1回）
22. CLI `qwert appearance contrast`（WCAG 2.x 相対輝度計算、--assert-aa/aaa）
23. CLI `qwert appearance set --scope global`（検証付き書き込み、preset/fg-bg相互排他）
24. グローバル appearance.toml テンプレート生成（AI向けプロトコルは含めない）
25. ステータスバーへのコントラスト比表示（appearance status の data source）

**ファイル品質保証（v9新設）**:
26. qwert-core/sanitize.rs 実装（第1層検出のみ、A + B範囲）
    - Unicode Tag（U+E0000〜E007F）、Null byte、C0/C1制御文字の検出
    - InvisibleCharFinding 構造体、detect_invisible_chars 関数
    - pulldown-cmark AST との結合は第1層のみ無条件適用（Phase 5で第2層対応時にコードブロック外限定化を検討）
27. CLI `qwert note scan <path>`、`qwert vault scan` 実装
    - JSON出力（行・列・カテゴリ・該当文字を含む findings 配列）
    - text出力（人間向け、ファイル名と件数のサマリ）
28. GUI: ファイル読み込み時のStatusBar警告表示、ファイルツリーアイコン上の警告マーカー
29. MCPツール拡張: `read` レスポンスに `invisible_char_warnings` フィールド追加（Phase 3 と連動）
30. config.toml `[sanitize] warn_invisible_chars` の設定読み込み・反映
31. 第3層（不正なUTF-8シーケンス）のエラーハンドリング拡充: `read_to_string` のErrを「位置: N byte目に不正なバイト」形式で詳細化

**セキュリティ境界実装（v9新設）**:
32. tauri.conf.json の CSP 設定（default-src 'self', frame-src 'none' 等）
33. Tauri capabilities/default.json の最小権限設定
    - fs: vault配下のみ scope 設定
    - http: 無効
    - shell: 全面不許可
    - dialog: openDialog のみ許可
34. パストラバーサル防止の `resolve_path` 関数を vault.rs の全ファイルアクセス入口に配置
35. Markdownレンダリング時の HTML タグ除去フィルタ（pulldown-cmark の `Event::Html` / `Event::InlineHtml` を filter）
36. プレビュー描画時の DOMPurify 適用（XSS二重防御）

### Phase 3: 品質向上 + 外部連携 - 推定 2〜3週間

1. MCPサーバーモード（rmcp: `qwert mcp --vault <path>`）
   - CLIと同一のqwert-core関数を呼ぶ薄いラッパー
   - `note revision` をMCPエンドポイントとして公開
   - `note scan` / `vault scan` をMCPエンドポイントとして公開（v9）
2. MCPツールに「GUI編集中」フラグを返す仕組み
3. `qwert describe <subcommand> --format json`（スキーマ自己記述）
4. ドラッグ&ドロップ（ファイル移動）
5. パフォーマンス最適化（大量ファイル、大きなノート）
6. tantivy導入検討（ノート数が増えた場合）
7. カスタムキーバインド設定
8. TSV出力形式（要望ベース）

**v9 要検討事項（実装可否は運用頻度を見て判断）**:
9. **コードブロック内SVG描画拡張**: ` ```svg ` 直接埋め込みの実装可否を、運用中の仕様書内Canvas利用頻度を踏まえて判断。実装する場合は DOMPurify 二重防御（Rustコア側マーカー化 + フロント側サニタイズ）が必須。「視覚表現の上限はMermaidまで」原則と緊張するため、Mermaidで表現できない図解の頻度を実測してから判断
10. **画像Revisionシステム**: `qwert asset rename` 別コマンドとしての実装可否を、運用中のFigma引用頻度・vault内画像のリネーム発生頻度を踏まえて判断。Rename / Replace / Archive の3つの意図分離設計が必要
11. **不可視文字サニタイズ機能**: `qwert note sanitize <path>` で第1層文字を能動的に除去する破壊的操作の実装可否を判断。「qwertは事実通知のみ」原則からPhase 2では実装しないが、SDD運用での修正フロー需要を見て検討

**視覚設定 vault スコープ化（v10新設）**:
12. vault スコープ appearance.toml（vault/.qwert/）の二層化実装
    - vault スコープ存在時はグローバルを無視（マージしない）
13. ホットリロード（notify 単一ファイル監視 + debounce 300ms + パース成功までリトライ）
    - 即時反映: 全セクション対象（CSS変数差し替え）
    - 不正時: 起動時=グローバルにフォールバック、ホットリロード時=直前保持
14. CLI `qwert appearance set --scope vault`（即時反映発火）、`qwert appearance status`
15. vault スコープ appearance.toml テンプレート生成（AI向けプロトコル付き、言語切替コメント）
16. §9 vault status の appearance.toml 競合検出拡張
17. §13 vault スコープサニタイズの信頼境界実装

### Phase 4: Android対応 - 推定 3〜4週間

1. Tauri 2.0 Android ビルド環境構築
2. レスポンシブUI（モバイル向けレイアウト、タッチ操作）
3. Android ファイルアクセス・権限処理（**SAF ではなく `MANAGE_EXTERNAL_STORAGE`**。AndroidManifest への権限宣言 + 初回 vault 選択時に `ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION` で設定画面へ誘導する自前実装 + 未許可時の明確なエラーUI）
4. 外部変更検知フォールバック（共有ストレージで inotify が効かない場合の、フォアグラウンド復帰時 vault 再スキャン。§12）
5. Syncthing連携テスト（Nothing Phone 3a実機）
6. ソフトキーボード最適化
7. APK/AABパッケージング・配布

**ローカル画像対応（v12新設、デスクトップ先行実装可）**:
8. qwert-core/markdown.rs: `Event::Image` を捕捉。http/https/data スキームは外部参照として描画拒否、vault 相対パスは `resolve_path` 検証後 `qwert-img://localhost/<相対パス>` に書き換え、検証失敗は壊れリンク表示。`loading="lazy"` を付与
9. src-tauri: `register_uri_scheme_protocol("qwert-img", ...)` を登録。ハンドラ内で AppState の vault を取得 → `resolve_path` 再検証 → `std::fs` 読み出し → 拡張子から MIME 判定（png/jpeg/webp/gif/svg）。svg は `image/svg+xml` を厳格付与・インライン展開禁止
10. tauri.conf.json: CSP `img-src` に独自スキーム（デスクトップ形式 + Android `http://qwert-img.localhost`）を追加、`assetProtocol` は無効化
11. テスト: パストラバーサル拒否（`![](../../etc/passwd)`）、外部URL拒否、svg の Content-Type、不存在画像の壊れ表示

注: タスク 3・4 は画像機能の前提（vault をそもそも読めるか）であり、画像タスク 8〜11 のうちデスクトップ向けは Android 権限まわりを待たずに先行実装できる。

### Phase 5: 発展機能（任意）

- ~~グラフビュー（ノート間リンク可視化）~~ →グラフビューは集積情報の多寡を自己満足的に認識するトロフィー的な機能として不採用とする(update 2026/05/27)
- タグ管理
- テンプレート機能
- エクスポート（PDF、HTML）
- 日本語全文検索高度化（lindera tokenizer）
- Mermaid対応図種拡充
- **W3C Design Tokens Format Module 2025.10 インポート対応（v7）**: appearance.rsを`AppearanceSource` trait化、DTCG JSONからCSS変数への変換実装（色空間変換 Oklch/Display P3 → sRGB hex、トークンのフラット化）
- **katex-rs 評価と段階的移行（v8新設）**: Rust純粋実装のkatex-rsの成熟度を継続評価。スクリーンショットテストの完全一致が達成できた段階で qwert-core 側のマークダウン→HTML変換パイプラインに統合し、フロントエンドのKaTeX JSバンドルを削減する。
- **mhchem等のKaTeX拡張（v8新設）**: 化学反応式・物理単位記法等の専門記法は、要望ベースでKaTeXプラグインとして追加検討。
- **MathML Core 再評価（v8新設）**: 2〜3年後にクロスブラウザ品質が安定した段階で Temml / MathMLネイティブ描画への移行を再検討。パーサーは共通なのでレンダラー差し替えは局所的変更で済む。
- **不可視文字検出 第2層対応（v9新設）**: ZWJ、BiDi制御文字、BOM、Variation Selector等の文脈依存文字を、pulldown-cmark AST との結合により「コードブロック外のテキストノードに含まれる場合のみ報告」する精度向上。絵文字シーケンスや多言語テキストでの偽陽性を回避する。Phase 2 の第1層実装後、運用中の偽陽性発生頻度を踏まえて段階的に対応
- **不可視文字検出 第3層拡充（v9新設）**: 不正なUTF-8シーケンスのエラー詳細表示はPhase 2で対応するが、ホモグリフ攻撃検出（似た見た目の異字検出）等のさらなる拡張は要望ベースで検討
- **派生プロジェクト群（v9新設）**: `qwert-figma-preview`（Figma URLサムネイル）、`qwert-canvas-bridge`（Excalidraw/tldraw連携）、`qwert-pdf-renderer`（PDFサムネイル）等を別バイナリとして開発。qwert本体ではなくhook経由で起動する設計とし、qwert-core を Rust ライブラリとして公開する形で実装基盤を提供する

---

## §22 ビルドとインストール

### 開発環境セットアップ

```bash
# Rustツールチェーン（mise経由）
mise use rust@latest

# Node.js（mise経由）
mise use node@lts

# pnpmインストール
corepack enable && corepack prepare pnpm@latest --activate

# Tauri CLI
cargo install tauri-cli --version "^2"

# Linux依存パッケージ（Pop!_OS / Ubuntu）
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev \
  librsvg2-dev

# プロジェクト作成
cargo create-tauri-app qwert -- --template solid-ts
cd qwert
pnpm install
```

### 開発

```bash
cargo tauri dev              # デスクトップ開発（ホットリロード）

jj init                      # バージョン管理初期化
jj commit -m "initial project setup"
```

### リリースビルド

```bash
cargo tauri build            # デスクトップ
cargo run --bin qwert -- --help  # CLI動作確認

# man ページ生成（Phase 2）
cargo run --bin qwert -- generate-man > qwert.1

# Android（Phase 4）
cargo tauri android init
cargo tauri android build
```

### Cargo.toml リリースプロファイル

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

---

## §23 Obsidianとの互換性

**互換性あり**:
- `.md` ファイルの読み書き
- `[[wikilink]]` 記法
- フォルダ構造
- CommonMark / GFM Markdown
- Mermaidコードブロック（Phase 2）

**非互換（対応しない）**:
- `.obsidian/` 内のプラグイン設定・テーマ
- Obsidian独自のメタデータ（YAML frontmatterは表示のみ対応）
- Obsidianプラグイン（Dataview、Templater等）
- Canvas機能
- Obsidian Sync / Publish（Syncthingで代替）

---

## §24 制約と前提条件

- Claude Codeでの開発を前提とし、AIが理解しやすいモジュール分割を意識する
- 同期機能はアプリに組み込まない（Syncthing等に委ねる）
- プラグインシステムは持たない（シンプルさ優先）
- Electronは使用しない
- OSSライセンス構成（MIT or Apache-2.0）
- コアロジック（qwert-core）はUI非依存のRustライブラリとして分離し、GUI / CLI / MCP すべてから共有する
- Revisionシステムはqwert-coreに実装し、hookシステムは外部hookのみを提供する
- **CLIはエージェントフレンドリーCLI8原則のうち、必須4原則（構造化出力 / 終了コード / 非対話 / Noun-Verb）とアクション可能エラー、Revision --dry-run + diff を Phase 2 で実装する**
- **vault-level state（sync-conflict等）と operation-level result（exit code）を層として分離する。前者は `qwert vault status`、後者は各コマンドの終了コードで扱う**
- **視覚設定はWCAG 2.2 Level AA準拠必須、AAA部分対応。CSS Custom Propertiesベースで軽量。構文ハイライトはOn/Off二値のみ、配色カスタマイズUIは提供しない。カスタムCSS注入およびテーママーケットプレイスは提供しない**
- **視覚設定はグローバルが起動時1回読み込み、vault スコープのみ即時反映（ホットリロード）。設定ファイルはRust側ゲートキーパーでサニタイズし、フロントエンドは直接ファイルを読まない（一方向データフロー）**
- **数式レンダリングはKaTeXを採用し、Temml / ブラウザネイティブMathMLは不採用とする（v8）。WebViewエンジン依存の描画差異を排除し、プラットフォーム間の一貫性を優先する。フォント込み約350KBのバンドルは遅延ロードで常時ペナルティをゼロにする**
- **Mermaidは ` ```mermaid ` コードブロック内のみで描画し、インライン記法や独自拡張は提供しない（v8）。コードブロック外にMermaid構文が出現しても誤検出しない**

**v9で明文化された制約**:

- **設計哲学は5層well構成（Note / File / Show / Secure / Agent）として整理する**。各層は機能カテゴリと差別化要因の両方を表現し、特に Secure well と Agent well の組み合わせがAIエージェント時代の差別化要因として機能する
- **qwertはSDD（仕様書駆動開発）の仕様書を記述・運用する基盤として位置づける**。AIエージェントが仕様書を読み、CLI/MCP経由で安全に操作できるインターフェイスを提供する
- **視覚表現の上限はコードブロック内のMermaidとKaTeXまでとする**。ピクセル精度のUI表現、手書き風自由描画、PDF・Canvas・動画等のバイナリレンダリングはqwertのスコープ外。Figma / Penpot等の外部サービスをURL参照で活用し、合意時点のスナップショットが必要な場合のみローカルエクスポート画像をvault内に保持する運用を推奨する
- **「全部入り」のMarkdownアプリケーションを求めるユーザーはObsidianやVS Codeを選ぶべきであり、qwertはあえてその対極、すなわちスコープを絞った小道具として機能する**。Obsidian / VS Code / Notion等の「全機能」ツールが避けて通れない複雑性（プラグインCVE追従、複数レンダラー維持、外部リソース防御の多層化）を最初から引き受けない選択を取る
- **セキュリティによる差別化を設計目標に含める**。外部URL取得、任意コード実行、プラグイン機構、複数ファイル形式サポートを構造的に不採用とすることで、間接プロンプトインジェクション攻撃経路を機能レベルで遮断する。CSP方針とTauri capabilities最小化はセキュリティ境界セクションに規定する
- **改訂内容の妥当性検証は責務外**。File well の責任境界は「ファイルの存在・形式・パスの有効性」までとし、ファイルの中身が意図通りかどうかの検証はユーザー責任とする。VCS（jj/git等）による履歴管理を前提とする
- **ADR等の不変性を要する文書運用はqwertの責務範囲外**。qwertはMarkdownエディタ/ビューアとして振る舞い、ファイル集合の特殊な命名規則・採番・不変性保証は外部ツール（chezmoi管理のシェルスクリプト等）に委ねる。Revisionシステムは `excluded_dirs` 設定で特定ディレクトリを保護できる
- **不可視文字検出（Phase 2）は事実通知のみ**。Unicode Tag、Null byte、C0/C1制御文字（第1層）の検出と警告を行うが、能動的なサニタイズ（除去）は含まない。プロンプトインジェクションの意味的検出はqwertの責務外であり、MCP利用側（AIエージェント）の責任とする
- **派生プロジェクトは別バイナリとして提供する**。qwert本体への機能追加要望はhook / MCPで対応し、機能拡張ニーズは `qwert-figma-preview`、`qwert-canvas-bridge`、`qwert-pdf-renderer` 等の派生バイナリとしてユーザーが必要な道具だけ追加できるUnix哲学に沿った形で提供する
- **画像Revisionとコードブロック内SVG描画はPhase 3以降の要検討事項**。「視覚表現の上限はMermaid」を原則としつつ、運用中の実需頻度を見て最終的な実装判断を行う

**v12で明文化された制約**:

- **ローカル画像配信は独自 URI スキーム `qwert-img` で行い、asset protocol は不採用**。Rust 側で `resolve_path` 検証を一元化し（多層防御）、外部URL画像は描画しない。svg は `image/svg+xml` を厳格付与し `<img>` 専用とする。画像参照（`![](相対パス)`）の自動追従（画像 Revision）は行わない
- **vault はアプリ専用ディレクトリを作らずユーザー指定ディレクトリとし、デスクトップ/Android で統一する**。Android では共有ストレージ上の vault を扱うため `MANAGE_EXTERNAL_STORAGE` を用い、配布は APK/AAB（F-Droid・直接配布）を前提とする
- **appearance.toml の fg/bg 片側指定は全経路（CLI / raw file write / AI 直接編集）で拒否する**（F24）。CLI だけでなくファイル読み込み経路にも適用し、片側のみは CSS 変数へ反映しない
---

## §25 変更履歴

| Version | Date | Changes |
|---------|------|---------|
|  v12 | 2026-06-13 | Phase 3 実装完了の反映と Phase 4 ローカル画像対応の確定。（1) §16/§14 の rmcp を 0.16 → 1.x（features: server / transport-io / macros）へ更新（実装に追随）。（2) §10 に F24（fg/bg 片側指定の拒否）がファイル読み込み経路（raw file write / AI 直接編集を含む全経路）にも適用される旨を明記。§13 の「appearance status で fail 検知」境界が片側指定でも構造的に成立することを保証。（3) §3 の画像表示を asset protocol → 独自 URI スキーム `qwert-img` に変更（Phase 4）。（4) §13 に「ローカル画像配信の境界」を新設。独自スキーム + resolve_path 二重検証 + 外部URL拒否 + svg の Content-Type 厳格化・インライン展開禁止。CSP img-src を `qwert-img:` / `http://qwert-img.localhost` に変更し asset: を除外。capabilities 表の protocol-asset 行を独自スキーム行へ置換。（5) §20 に「vault 配置モデルと Android 権限」を新設。アプリ専用ディレクトリを作らずユーザー指定ディレクトリを vault とする統一モデルを採用し、Android では MANAGE_EXTERNAL_STORAGE を用いる（std::fs 直接読み出し + Syncthing 併用のため SAF 不採用）。配布は APK/AAB 前提。（6) §20 の .stignore に `.qwert/editing_state.json` を除外追加（デバイスローカルな揮発状態の同期回避）。（7) §12 に Android 共有ストレージでの inotify 取りこぼしと、フォアグラウンド復帰時再スキャンのフォールバックを追記。（8) §21 Phase 4 のタスク 3 を SAF → MANAGE_EXTERNAL_STORAGE 権限処理へ修正、外部変更検知フォールバックを追加、ローカル画像対応タスク（markdown.rs Event::Image 書き換え / qwert-img プロトコル登録 / CSP / テスト）を新設。デスクトップ先行実装可と注記。（9) §24 制約に v12 確定事項3項目を追加。 以下はハウスキーピング（実装整合）の反映: （10) C-1: workspace メンバー `src-tauri` 配下の重複 `Cargo.lock`（残骸）を削除し root の `Cargo.lock` を唯一の正とする。`.gitignore` に `/src-tauri/Cargo.lock` を追加。（11) C-4: グローバル `appearance.toml` テンプレート（`APPEARANCE_TEMPLATE`）を §18 どおり全行コメントアウト化し、初回生成ファイルがビルトイン既定値へ解決するよう修正（将来の既定値変更時に旧値を固定しない）。（12) C-5: `save_global_appearance` を `std::fs::write` 直書きから tmp→rename のアトミック書込へ変更し、`save_vault_appearance`（vault 版）と書込方式を統一。（13) C-6: `mise.toml` に `rust = "1.88"` を pin（`qwert-core` は edition 2024 + let-chains を使用するため Rust 1.88 以上が必須）。ビルド再現性を担保。 |
| v11 | 2026-06-10 | 軽微な変更|
| v10 | 2026-05-28 | 視覚設定のAI連携・vaultスコープ即時反映・appearance CLIコマンド族を確定。立ち位置を「AIエージェントとそれを扱う人間向け」に明確化。（1) プロジェクト概要に立ち位置宣言を追加。AIエージェント向け決定論的vault操作CLI / 不可視文字検出ツール / WCAG準拠Markdownリーダー の3差別化軸を宣言。AIを使う前提を柱に据えつつCLI経路で非AIユーザーもアクセス可能とする。（2) Show well の方針を改訂。GUI色ピッカー・リアルタイムコントラスト表示UIを持たず、AI経路（appearance.tomlコメントの機械可読指示テンプレート）とCLI経路（appearance contrast/set）で配色カスタマイズを提供。「機械可読指示テンプレート」と「人間向けガイドブック」を別概念として区別し、前者は書き後者は書かない。（3) §10視覚設定を二層化。グローバル（~/.config、起動時1回）+ vaultスコープ（vault/.qwert/、即時反映、グローバルを上書き、マージしない）。設定の3層構造を「スコープ層」と「露出層」の二軸に再構造化。（4) 即時反映の対象を全セクションとし、notify単一ファイル監視 + debounce 300ms + パース成功までリトライで実装。不正TOML時は起動時=グローバルにフォールバック+警告、ホットリロード時=直前保持+一時警告。preset/fg-bg同居も相互排他違反として同挙動。（5) §10のリアルタイムコントラスト表示と「設定は起動時1回読み込み」の矛盾を解消。コントラスト検証はGUIではなくCLIとステータスバー表示で提供。（6) §11にappearance名詞族（contrast/set/status）を追加。JSONエンベロープをtop-level統一（dataラッパー削除、既存実例と整合）。preset/fg-bg相互排他で明示エラー、--scopeデフォルトはvault優先、短縮エイリアスなし。appearance contrastはデフォルトexit 0、--assert-aa/aaaでexit 5ゲート化。（7) §13にvaultスコープ設定の信頼境界を追記。全経路（appearance set / raw file write / AI直接編集）が同一サニタイズパイプラインを通る。MCP経由書き込みの最悪ケースは「サニタイズ通過済みの見にくいテーマ」止まりと評価。（8) §9 vault statusにappearance.toml競合（.qwert/appearance.sync-conflict-*.toml）検出を追加。（9) appearance.tomlを二層テンプレート化。グローバル版（AIプロトコルなし）とvaultスコープ版（AI向けプロトコル + 言語切替インラインコメント付き）。（10) §Syncthingに.qwert/appearance.tomlのデフォルト同期と除外運用を注記。（11) Phase 2の視覚設定UIタスクをCLI経路 + ステータスバー表示に差し替え（GUI色ピッカー削除）。Phase 3にvaultスコープ化・ホットリロード・appearance set/statusを追加。（12) docs/appearance-spec.mdの内容を本体§10へ統合（分離記述を廃止）。 |
| v9 | 2026-05-02 | 設計哲学を5層well構成へ再整理し、SDD基盤としての位置づけを明文化、セキュリティ境界の独立セクション化、Revisionシステムの責務境界明確化、ファイル品質保証層の新設を実施。（1) プロジェクト概要の設計哲学を Note / File / Show / Secure / Agent の5層 well 構成へ再整理。Note well と File well は機能カテゴリ、Show well は表現上限、Secure well と Agent well は AIエージェント時代の差別化要因として位置づけ。（2) qwertの位置づけを「SDD（仕様書駆動開発）の仕様書を記述・運用する基盤」として明文化。「全部入り」を求めるユーザーはObsidian/VS Codeを選ぶべきであり、qwertはあえてその対極の小道具として機能することを宣言。（3) 視覚表現の上限を「コードブロック内のMermaidとKaTeXまで」と明記。ピクセル精度UI、手書き風描画、PDF/Canvas/動画等のバイナリレンダリングはスコープ外、Figma/Penpot等の外部サービスをURL参照で活用する運用を推奨。（4) §セキュリティ境界 を独立セクションとして新設。構造的に拒否する機能の列挙（外部URL画像、iframe、外部スクリプト、JavaScript URI、任意コード実行、プラグインによるコードロード等）、CSP方針、Tauri capabilities最小化、間接プロンプトインジェクション耐性、派生プロジェクト方針を統合的に規定。（5) §8 Revisionシステムに `excluded_dirs` 設定を追加。ADR等の不変性が必要な文書をユーザー側で運用可能にする。ADR運用そのものはqwertの責務範囲外と明示。（6) §8 Revisionシステムに「改訂内容の妥当性検証は責務外」を明文化。File well の責任境界は客観的・二値的判定（ファイル存在、Markdown読込可、パス有効性）までとし、内容の妥当性はVCSで管理する利用者責務とする。（7) §11「ファイル品質保証 - 不可視文字検出」を新設（Phase 2）。第1層（Unicode Tag、Null byte、C0/C1制御文字）の検出と警告のみを実装（A: 検出のみ、B: CLIスキャン）。サニタイズ機能（C）はPhase 3以降の検討事項。第2層（ZWJ、BiDi制御等の文脈依存文字）はPhase 5発展機能、第3層（不正UTF-8）はPhase 2のエラーハンドリング拡充で対応。（8) §6 Mermaidに「コードブロック内SVG描画拡張」をPhase 3以降の要検討事項として追記。「視覚表現の上限はMermaid」原則を維持しつつ、運用中のCanvas利用頻度を見て判断。（9) §8 Revisionに「画像ファイルへの拡張」をPhase 3以降の要検討事項として追記。Rename/Replace/Archive の3つの意図分離設計が必要、運用中のFigma引用頻度を見て判断。（10) config.tomlに `[revision] excluded_dirs` と `[sanitize] warn_invisible_chars` を追加。（11) ディレクトリ構成に qwert-core/sanitize.rs、commands/sanitize.rs を追加。（12) CLI に `qwert note scan` と `qwert vault scan` サブコマンドを追加。（13) Phase 2 タスクにファイル品質保証（タスク24-29）とセキュリティ境界実装（タスク30-34）を追加。（14) Phase 3 にv9要検討事項3項目（SVG拡張、画像Revision、サニタイズ機能）を追加。（15) Phase 5 に第2層・第3層不可視文字検出、派生プロジェクト群を追加。（16) 制約と前提条件に「v9で明文化された制約」セクションとして10項目を追加。 |
| v8 | 2026-04-23 | Markdownリッチコンテンツ描画方針を確定。（1) コア機能仕様§7「数式レンダリング KaTeX」を新設。KaTeX採用、Temml/ブラウザネイティブMathML不採用を明記（WebViewエンジン依存の描画差異を排除するため）。インライン `$...$` とブロック `$$...$$` の両対応、フォント込み約350KBを遅延ロード、hidden MathMLでアクセシビリティ担保、`throwOnError: false` によるフォールバック挙動、誤検出防止ヒューリスティクスを規定。（2) §6 Mermaidダイアグラムのスコープを ` ```mermaid ` コードブロック内のみに明示的限定。インライン記法・独自拡張はサポートしない。パースエラー時は元のコードブロックをそのまま表示。遅延ロード方針を明記。（3) Markdownプレビュー§3の必須機能に数式レンダリングを追加。（4) 技術スタック表にKaTeX行を追加、Mermaid備考を「コードブロック内のみ」に更新。（5) config.tomlの`[preview]`に`render_math = true`を追加。（6) 後続セクション§7→§8、§8→§9、§9→§10に繰り下げ。（7) 依存: pulldown-cmark の math extension 有効化、JSに `katex ^0.16` を追加。（8) ディレクトリ構成に `src/lib/math.ts`、`src/lib/mermaid.ts`（いずれも遅延ロードラッパー）を追加。qwert-core/markdown.rs のコメントに math extension 対応を追記。（9) Phase 2 タスクを「リッチコンテンツ描画」項に再編（math extension 有効化、KaTeX描画、Mermaid限定描画、highlight.js）。（10) Phase 5 に katex-rs 移行評価、mhchem等の拡張検討、MathML Core 再評価（2〜3年後）を追加。（11) 制約に数式はKaTeX採用・MathML不採用、Mermaidはコードブロック内限定を明記。 |
| v7 | 2026-04-23 | 視覚設定（Appearance Specification）を本体仕様に統合。（1) コア機能仕様§9「視覚設定とアクセシビリティ」を新設、WCAG 2.2 Level AA必須・AAA部分対応を明示。（2) 設定ファイルを config.toml（動作）と appearance.toml（視覚）の2ファイル構成に分離。v6の `[general] theme` と `[editor] font_family/font_size` を appearance.toml の `[color] preset` / `[text]` に統合。（3) プリセットテーマ4種（default / high-contrast / dark / dark-high-contrast）を規定。（4) 構文ハイライトをOn/Off二値のみに限定、Protanopia/Deuteranopia対応デフォルト配色。（5) CSS Custom Properties（`--qw-*`プレフィックス）ベースの軽量設計。CodeMirror 6は単一テーマ + CSS変数参照で再構築回避。（6) Rust側ゲートキーパーによるサニタイズ（キー種別別許可パターン、危険値パターン拒否）と一方向データフロー。（7) 設定は起動時1回読み込み、ホットリロード非提供。（8) 3層構造（UI公式 / ファイル経由自己責任 / スコープ外）を明文化。（9) W3C Design Tokens Format Module 2025.10 対応を将来拡張としてPhase 5に追加、`AppearanceSource` traitでadapter pattern準備。（10) ディレクトリ構成に qwert-core/appearance.rs、commands/appearance.rs、stores/appearance.ts、styles/tokens.css および theme-*.css 4種、codemirror/theme.ts・highlight.ts を追加。（11) Phase 1 に視覚設定基盤（CSS変数、WCAG AA検証、prefers-color-scheme対応、構文ハイライトCompartment）を組み込み、推定期間を 2〜3週間 → 3〜4週間 に修正。Phase 2 にプリセット4種・カスタム色・コントラスト比表示を追加。 |
| v6 | 2026-04-22 | エージェントフレンドリーCLI原則の統合。（1) CLIサブコマンドをNoun-Verb canonical form（`qwert file read`等）に再設計、短縮エイリアスを併設。（2) セマンティック終了コード規約（0-5の6カテゴリ、authをconflictに差し替え）追加。（3) JSONエンベロープ（schema_version/kind）と`--format json/path/text/raw/diff`規約追加。（4) 非対話モード規約（is-terminalによるTTY判定、`--yes`強制）追加。（5) アクション可能エラー（qwert-core/error.rsにnext_step/candidates/required_args）追加。（6) `qwert note revision --dry-run`と`--dry-run --diff`/`--format diff`追加（similar crate）。（7) `qwert vault status`コマンド新設。Syncthing競合検出を operation exit code から分離し、vault-level state として扱う。（8) Phase 2にCLI基盤とman ページ生成（clap_mangen）を追加。（9) ディレクトリ構成にsrc-tauri/cli/、qwert-core/error.rs/status.rs/revision_diff.rsを追加。 |
| v5 | 2026-03-09 | Revisionシステム追加。pulldown-cmark AST解析 + rayon並列スキャン + WALパターンによるアトミック一括更新。on-revise hookシステム。`qwert revision` CLIサブコマンド追加。 |
| v4 | 2026-03-08 | nota → qwert 改称。CLI + MCP方針追加。Vimバインド切替をPhase 1に前倒し。並行アクセス設計追加。TypeScript型安全基盤組み込み。 |
| v3 | 2026-03-08 | 確定版。Tauri 2.0 + SolidJS + CodeMirror 6。Android対応方針。 |
| v2 | 2026-03-08 | Android対応要件追加。egui vs Tauri の Option A/B 比較。Mermaid対応。 |
| v1 | 2026-03-08 | 初版。egui前提。 |
