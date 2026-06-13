import { createEffect, createSignal, onCleanup, onMount, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import type { ViewMode } from "./types/constants";
import { VIEW_MODE, SAVE_STATE } from "./types/constants";
import { FileTree } from "./components/FileTree";
import { Editor } from "./components/Editor";
import { Preview } from "./components/Preview";
import { StatusBar } from "./components/StatusBar";
import { SettingsPanel } from "./components/SettingsPanel";
import { ExternalChangeDialog } from "./components/ExternalChangeDialog";
import { BacklinksPanel } from "./components/BacklinksPanel";
import { CommandPalette } from "./components/CommandPalette";
import { VaultStatusBanner } from "./components/VaultStatusBanner";
import { vaultStore } from "./stores/vault";
import { editorStore } from "./stores/editor";
import { appearanceStore } from "./stores/appearance";
import { settingsStore } from "./stores/settings";
import * as tauri from "./lib/tauri";
import type { RelativePath } from "./types/brand";
import "./App.css";

/** Returns true when `e` matches a key spec like "Ctrl+S" or "Ctrl+Shift+F". */
function matchesSpec(e: KeyboardEvent, spec: string): boolean {
  const parts = spec.split("+");
  const key = parts.at(-1) ?? "";
  const mods = parts.slice(0, -1);
  if (e.ctrlKey !== mods.includes("Ctrl")) return false;
  if (e.altKey !== mods.includes("Alt")) return false;
  if (e.shiftKey !== mods.includes("Shift")) return false;
  if (e.metaKey !== mods.includes("Meta")) return false;
  return e.key.toLowerCase() === key.toLowerCase();
}

export default function App() {
  const [viewMode, setViewMode] = createSignal<ViewMode>(VIEW_MODE.SPLIT);
  const [showSettings, setShowSettings] = createSignal(false);
  const [showSidebar, setShowSidebar] = createSignal(true);
  const [showPalette, setShowPalette] = createSignal(false);
  const [showExternalChangeDialog, setShowExternalChangeDialog] = createSignal(false);
  const [externalChangeFile, setExternalChangeFile] = createSignal<string>("");

  onMount(() => {
    appearanceStore.applyAppearance();
    void settingsStore.loadKeybindings();

    editorStore.registerSaveCallback(async () => {
      const file = vaultStore.selectedFile();
      if (file) await tauri.writeFile(file, editorStore.content());
    });

    // C2: vault appearance.toml の直接編集 → 300ms debounce 後に CSS 変数を再適用。
    void listen<Record<string, string>>("appearance-changed", (event) => {
      appearanceStore.reapplyAppearance(event.payload);
    });

    // C3: 不正 TOML / 相互排他違反 → 直前の見た目を維持し、StatusBar に警告を表示。
    void listen<string>("appearance-warning", (event) => {
      appearanceStore.setAppearanceWarning(event.payload);
    });

    void listen<string>("file-changed", (event) => {
      const changedPath = event.payload;
      const currentFile = vaultStore.selectedFile();

      void vaultStore.refreshFileTree();

      if (currentFile && currentFile === changedPath) {
        if (editorStore.saveState() === SAVE_STATE.UNSAVED) {
          setExternalChangeFile(changedPath);
          setShowExternalChangeDialog(true);
        } else {
          void editorStore.loadFile(currentFile);
        }
      }
    });

    // C6: keydown ハンドラは1つだけ登録し、onCleanup で解除する。
    // キー仕様は settingsStore.keybindings() から動的に読む（再割当に即時対応）。
    const kb = settingsStore.keybindings;
    const onKey = (e: KeyboardEvent) => {
      if (matchesSpec(e, kb().save)) {
        e.preventDefault();
        void editorStore.saveCurrentFile();
      } else if (matchesSpec(e, kb().new_note)) {
        e.preventDefault();
        const name = prompt("ファイル名（拡張子 .md は自動付加）:");
        if (!name) return;
        const path = (name.endsWith(".md") ? name : `${name}.md`) as RelativePath;
        void tauri.createFile(path).then(async () => {
          await vaultStore.refreshFileTree();
          vaultStore.setSelectedFile(path);
        });
      } else if (matchesSpec(e, kb().command_palette)) {
        e.preventDefault();
        setShowPalette(v => !v);
      } else if (matchesSpec(e, kb().full_search)) {
        e.preventDefault();
        // 全文検索は未実装（Phase 3 以降）
      } else if (matchesSpec(e, kb().view_mode_toggle)) {
        e.preventDefault();
        setViewMode(current => {
          const modes = [VIEW_MODE.EDITOR, VIEW_MODE.SPLIT, VIEW_MODE.PREVIEW] as const;
          const idx = modes.indexOf(current);
          return modes[(idx + 1) % modes.length] ?? VIEW_MODE.SPLIT;
        });
      } else if (matchesSpec(e, kb().sidebar_toggle)) {
        e.preventDefault();
        setShowSidebar(v => !v);
      } else if (matchesSpec(e, kb().settings)) {
        e.preventDefault();
        setShowSettings(v => !v);
      }
    };
    document.addEventListener("keydown", onKey);
    onCleanup(() => document.removeEventListener("keydown", onKey));
  });

  // ファイル選択時にエディタへロード
  createEffect(() => {
    const file = vaultStore.selectedFile();
    if (file) editorStore.loadFile(file);
  });

  // Level 3: 保存状態変化をバックエンドに通知（MCP 編集中ヒント用）
  createEffect(() => {
    const file = vaultStore.selectedFile();
    const state = editorStore.saveState();
    if (file) {
      void tauri.setEditingState(file, state !== SAVE_STATE.SAVED);
    }
  });

  return (
    <div class="app-layout">
      <VaultStatusBanner />
      <Show when={showSidebar()}>
        <div class="sidebar">
          <button onClick={vaultStore.openVault}>Vault を開く</button>
          <FileTree />
          <BacklinksPanel />
        </div>
      </Show>
      <div class="main-content">
        <Show when={viewMode() !== VIEW_MODE.PREVIEW}><Editor /></Show>
        <Show when={viewMode() !== VIEW_MODE.EDITOR}><Preview /></Show>
      </div>
      <StatusBar />

      <Show when={showSettings()}><SettingsPanel /></Show>
      <Show when={showPalette()}>
        <CommandPalette onClose={() => setShowPalette(false)} />
      </Show>

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
    </div>
  );
}
