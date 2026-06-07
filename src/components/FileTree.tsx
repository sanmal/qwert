import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { FileEntry } from "../lib/tauri";
import type { RelativePath } from "../types/brand";
import * as tauri from "../lib/tauri";
import { vaultStore } from "../stores/vault";
import { RevisionDialog } from "./RevisionDialog";

interface ContextMenuState {
  entry: FileEntry;
  x: number;
  y: number;
}

async function handleNewFile() {
  const name = prompt("ファイル名（拡張子 .md は自動付加）:");
  if (!name) return;
  const path = (name.endsWith(".md") ? name : `${name}.md`) as RelativePath;
  await tauri.createFile(path);
  await vaultStore.refreshFileTree();
  vaultStore.setSelectedFile(path);
}

function FileTreeItem(props: {
  entry: FileEntry;
  depth: number;
  onRightClick: (e: MouseEvent, entry: FileEntry) => void;
}) {
  const [expanded, setExpanded] = createSignal(true);
  const isSelected = () => vaultStore.selectedFile() === props.entry.path;
  const hasWarning = () =>
    !props.entry.is_dir && vaultStore.filesWithWarnings().has(props.entry.path);

  return (
    <div style={{ "padding-left": `${props.depth * 16}px` }}>
      <Show when={props.entry.is_dir}>
        <div class="tree-folder" onClick={() => setExpanded(v => !v)}>
          {expanded() ? "▼" : "▶"} {props.entry.name}
        </div>
        <Show when={expanded() && props.entry.children}>
          <For each={props.entry.children}>
            {child => (
              <FileTreeItem
                entry={child}
                depth={props.depth + 1}
                onRightClick={props.onRightClick}
              />
            )}
          </For>
        </Show>
      </Show>
      <Show when={!props.entry.is_dir}>
        <div
          class="tree-file"
          data-selected={isSelected()}
          onClick={() => vaultStore.setSelectedFile(props.entry.path)}
          onContextMenu={e => {
            e.preventDefault();
            e.stopPropagation();
            props.onRightClick(e, props.entry);
          }}
        >
          <Show when={hasWarning()}>
            <span class="tree-warn-badge" title="不可視文字あり">⚠</span>
          </Show>
          {props.entry.name}
        </div>
      </Show>
    </div>
  );
}

export function FileTree() {
  const [contextMenu, setContextMenu] = createSignal<ContextMenuState | null>(null);
  const [revisionEntry, setRevisionEntry] = createSignal<FileEntry | null>(null);
  const [toast, setToast] = createSignal("");
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function closeContextMenu() {
    setContextMenu(null);
  }

  onMount(() => {
    document.addEventListener("click", closeContextMenu);
  });
  onCleanup(() => {
    document.removeEventListener("click", closeContextMenu);
    clearTimeout(toastTimer);
  });

  function showToast(msg: string) {
    setToast(msg);
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => setToast(""), 5000);
  }

  function handleRightClick(e: MouseEvent, entry: FileEntry) {
    setContextMenu({ entry, x: e.clientX, y: e.clientY });
  }

  return (
    <div class="file-tree">
      <button onClick={() => void handleNewFile()}>+ 新規ファイル</button>
      <For each={vaultStore.fileTree()}>
        {entry => (
          <FileTreeItem entry={entry} depth={0} onRightClick={handleRightClick} />
        )}
      </For>

      {/* Right-click context menu */}
      <Show when={contextMenu() !== null}>
        <div class="context-menu-overlay" onClick={closeContextMenu}>
          <ul
            class="context-menu"
            style={{ left: `${contextMenu()!.x}px`, top: `${contextMenu()!.y}px` }}
            onClick={e => e.stopPropagation()}
          >
            <li
              onClick={() => {
                setRevisionEntry(contextMenu()!.entry);
                setContextMenu(null);
              }}
            >
              Revision...
            </li>
          </ul>
        </div>
      </Show>

      {/* Revision dialog */}
      <Show when={revisionEntry() !== null}>
        <RevisionDialog
          entry={revisionEntry()!}
          onClose={() => setRevisionEntry(null)}
          onSuccess={msg => {
            setRevisionEntry(null);
            void vaultStore.refreshFileTree();
            showToast(msg);
          }}
        />
      </Show>

      {/* Toast */}
      <Show when={toast()}>
        <div class="toast" role="status" aria-live="polite">
          {toast()}
        </div>
      </Show>
    </div>
  );
}
