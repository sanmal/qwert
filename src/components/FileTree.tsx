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

interface DndHandlers {
  /** Vault-relative path of the file currently being dragged, or null. */
  dragSrc: () => string | null;
  setDragSrc: (v: string | null) => void;
  /** Vault-relative path of the folder currently under the cursor, or null. */
  dragOverFolder: () => string | null;
  setDragOverFolder: (v: string | null) => void;
  onDrop: (folderPath: string) => void;
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
  dnd: DndHandlers;
}) {
  const [expanded, setExpanded] = createSignal(true);
  const isSelected = () => vaultStore.selectedFile() === props.entry.path;
  const hasWarning = () =>
    !props.entry.is_dir && vaultStore.filesWithWarnings().has(props.entry.path);
  const isDragOver = () => props.dnd.dragOverFolder() === props.entry.path;

  return (
    <div style={{ "padding-left": `${props.depth * 16}px` }}>
      <Show when={props.entry.is_dir}>
        <div
          class="tree-folder"
          classList={{ "tree-folder--drag-over": isDragOver() }}
          onClick={() => setExpanded(v => !v)}
          onDragOver={e => {
            if (props.dnd.dragSrc() === null) return;
            e.preventDefault();
            e.stopPropagation();
            props.dnd.setDragOverFolder(props.entry.path);
          }}
          onDragLeave={() => {
            if (props.dnd.dragOverFolder() === props.entry.path) {
              props.dnd.setDragOverFolder(null);
            }
          }}
          onDrop={e => {
            e.preventDefault();
            e.stopPropagation();
            props.dnd.onDrop(props.entry.path);
          }}
        >
          {expanded() ? "▼" : "▶"} {props.entry.name}
        </div>
        <Show when={expanded() && props.entry.children}>
          <For each={props.entry.children}>
            {child => (
              <FileTreeItem
                entry={child}
                depth={props.depth + 1}
                onRightClick={props.onRightClick}
                dnd={props.dnd}
              />
            )}
          </For>
        </Show>
      </Show>
      <Show when={!props.entry.is_dir}>
        <div
          class="tree-file"
          data-selected={isSelected()}
          draggable="true"
          onDragStart={e => {
            props.dnd.setDragSrc(props.entry.path);
            e.dataTransfer?.setData("text/plain", props.entry.path);
          }}
          onDragEnd={() => props.dnd.setDragSrc(null)}
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

  // ── DnD state ──────────────────────────────────────────────────────────────
  const [dragSrc, setDragSrc] = createSignal<string | null>(null);
  const [dragOverFolder, setDragOverFolder] = createSignal<string | null>(null);

  /** Called when the user drops a file onto a folder (or the vault root ""). */
  async function handleDrop(folderPath: string) {
    const src = dragSrc();
    setDragSrc(null);
    setDragOverFolder(null);
    if (!src) return;
    const fileName = src.split("/").at(-1) ?? "";
    const dst = (folderPath ? `${folderPath}/${fileName}` : fileName) as RelativePath;
    if ((dst as string) === src) return; // dropped onto own parent folder
    try {
      await tauri.moveFile(src as RelativePath, dst);
      await vaultStore.refreshFileTree();
      // keep the moved file selected
      vaultStore.setSelectedFile(dst);
      showToast(`移動: ${src} → ${dst}`);
    } catch (e) {
      showToast(`移動エラー: ${String(e)}`);
    }
  }

  const dnd: DndHandlers = {
    dragSrc,
    setDragSrc,
    dragOverFolder,
    setDragOverFolder,
    onDrop: (folderPath) => void handleDrop(folderPath),
  };

  // ── other handlers ─────────────────────────────────────────────────────────

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
    <div
      class="file-tree"
      onDragOver={e => { if (dragSrc() !== null) e.preventDefault(); }}
      onDrop={e => { e.preventDefault(); void handleDrop(""); }}
    >
      <button onClick={() => void handleNewFile()}>+ 新規ファイル</button>
      <For each={vaultStore.fileTree()}>
        {entry => (
          <FileTreeItem entry={entry} depth={0} onRightClick={handleRightClick} dnd={dnd} />
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
