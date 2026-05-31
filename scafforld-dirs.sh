#!/usr/bin/env bash
set -euo pipefail

# qwert-core のソース置き場（中身は Claude Code が生成）
mkdir -p crates/qwert-core/src

# src-tauri の追加モジュール
mkdir -p src-tauri/src/cli src-tauri/src/commands
mkdir -p src-tauri/capabilities src-tauri/icons

# フロントエンド構造
mkdir -p src/components src/stores src/types
mkdir -p src/lib/codemirror src/styles

# 設定と仕様
mkdir -p config docs

echo "scaffold dirs created."
