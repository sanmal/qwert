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
import { vaultStore } from "./stores/vault";
import { editorStore } from "./stores/editor";
import { appearanceStore } from "./stores/appearance";
import * as tauri from "./lib/tauri";
import "./App.css";

export default function App() {
  const [viewMode, setViewMode] = createSignal<ViewMode>(VIEW_MODE.SPLIT);
  const [showSettings, setShowSettings] = createSignal(false);
  const [showSidebar, setShowSidebar] = createSignal(true);
  const [showExternalChangeDialog, setShowExternalChangeDialog] = createSignal(false);
  const [externalChangeFile, setExternalChangeFile] = createSignal<string>("");

  onMount(() => {
    appearanceStore.applyAppearance();

    editorStore.registerSaveCallback(async () => {
      const file = vaultStore.selectedFile();
      if (file) await tauri.writeFile(file, editorStore.content());
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
    const onKey = (e: KeyboardEvent) => {
      if (!e.ctrlKey) return;
      switch (e.key) {
        case "s":
          e.preventDefault();
          void editorStore.saveCurrentFile();
          break;
        case ",":
          e.preventDefault();
          setShowSettings(v => !v);
          break;
        case "b":
          e.preventDefault();
          setShowSidebar(v => !v);
          break;
        case "e":
          e.preventDefault();
          setViewMode(current => {
            const modes = [VIEW_MODE.EDITOR, VIEW_MODE.SPLIT, VIEW_MODE.PREVIEW] as const;
            const idx = modes.indexOf(current);
            return modes[(idx + 1) % modes.length] ?? VIEW_MODE.SPLIT;
          });
          break;
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

  return (
    <div class="app-layout">
      <Show when={showSidebar()}>
        <div class="sidebar">
          <button onClick={vaultStore.openVault}>Vault を開く</button>
          <FileTree />
        </div>
      </Show>
      <div class="main-content">
        <Show when={viewMode() !== VIEW_MODE.PREVIEW}><Editor /></Show>
        <Show when={viewMode() !== VIEW_MODE.EDITOR}><Preview /></Show>
      </div>
      <StatusBar />

      <Show when={showSettings()}><SettingsPanel /></Show>

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
