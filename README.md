# qwert — プロジェクトメモリ

Rust(Tauri 2) + SolidJS の軽量 Markdown ノートアプリ。AI エージェントと
それを扱う人間向けの SDD（仕様書駆動開発）基盤。
完全な仕様は docs/qwert_仕様要件書_v10.md 本書中の §N 参照）。本ファイルと docs/qwert_仕様要件書_v10.md が食い違う場合は
docs/qwert_仕様要件書_v10.md を正とし、不明点は勝手に解釈せず必ず質問する。

## 5層 well（設計の柱）
- Note well: CodeMirror 6 編集 / Vim切替 / 自動保存 / 外部変更検知
- File well: wikilink整合 / Revision / アトミック書込 / 不可視文字検出
- Show well: 視認性(WCAG 2.2 AA必須/AAA部分)。表現上限は mermaid コードブロックと KaTeX まで
- Secure well: 下記「構造的に禁止」を厳守
- Agent well: CLI Noun-Verb / MCP stdio / セマンティック終了コード

## 構造的に禁止（実装しない＝攻撃面を作らない）
- 外部URL画像 / iframe / 外部 script・style・font / javascript: / 画像以外の data:
- 任意コード実行・プラグイン機構・shell 実行経路
- カスタムCSS注入（appearance は Rust 側サニタイザ経由のみ）
- Mermaid/KaTeX を超える描画（PDF/Canvas/動画/ピクセル精度UI）

## CLI 規約
- Noun-Verb canonical（file/note/vault/appearance）。短縮形は人間向けの裏口。
- JSON は必ず top-level エンベロープ（schema_version, kind, ペイロードは top-level）。
  data ラッパー禁止。
- 終了コード: 0 Success / 1 General / 2 Usage / 3 NotFound / 4 Conflict / 5 Validation。
- 非対話時は破壊的操作に --yes 必須。エラーは next_step/candidates を含める。
- vault-level state（sync-conflict 等）は exit code に昇格させず vault status で扱う。

## 型安全（Liminia Type Safety v1）
- tsconfig strict + noUncheckedIndexedAccess + verbatimModuleSyntax 前提。
- Branded Types(RelativePath/AbsolutePath)、定数は as const。EXIT_CODE は Rust と一致。

## VCS は jj（git ではない）
- git add / git commit / git push を実行しないこと。バージョン管理は人間が jj で行う。
- 変更後は要約だけ伝える。読み取り専用の git status 程度は可。

## ビルド/確認コマンド
- 開発: cargo tauri dev / フロント: pnpm install, pnpm dev
- Rust: cargo test, cargo clippy -- -D warnings, cargo fmt
- TS: pnpm tsc --noEmit, pnpm test（vitest）
- man: cargo run --bin qwert -- generate-man（hidden）

## 進め方
- 依頼された Phase / タスク番号（§21）だけを実装する。将来フェーズを先取りしない。
- 決定論的ロジック（Revision の wikilink 特定、不可視文字検出、コントラスト計算）は
  ユニットテスト必須。
