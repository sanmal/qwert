import { For, Show, createSignal } from "solid-js";
import type { FileEntry } from "../lib/tauri";
import type { RelativePath } from "../types/brand";
import * as tauri from "../lib/tauri";
import { vaultStore } from "../stores/vault";

async function handleNewFile() {
  const name = prompt("ファイル名（拡張子 .md は自動付加）:");
  if (!name) return;
  const path = (name.endsWith(".md") ? name : `${name}.md`) as RelativePath;
  await tauri.createFile(path);
  await vaultStore.refreshFileTree();
  vaultStore.setSelectedFile(path);
}

// depth を伝播させてネストの字下げを効かせる。
function FileTreeItem(props: { entry: FileEntry; depth: number }) {
  const [expanded, setExpanded] = createSignal(true);
  // entry.path は Rust 側が返す RelativePath（branded 済み）なので再キャストは不要。
  const isSelected = () => vaultStore.selectedFile() === props.entry.path;

  return (
    <div style={{ "padding-left": `${props.depth * 16}px` }}>
      <Show when={props.entry.is_dir}>
        <div class="tree-folder" onClick={() => setExpanded(v => !v)}>
          {expanded() ? "▼" : "▶"} {props.entry.name}
        </div>
        <Show when={expanded() && props.entry.children}>
          <For each={props.entry.children}>
            {child => <FileTreeItem entry={child} depth={props.depth + 1} />}
          </For>
        </Show>
      </Show>
      <Show when={!props.entry.is_dir}>
        <div
          class="tree-file"
          data-selected={isSelected()}
          onClick={() => vaultStore.setSelectedFile(props.entry.path)}
        >
          {props.entry.name}
        </div>
      </Show>
    </div>
  );
}

export function FileTree() {
  return (
    <div class="file-tree">
      <button onClick={() => void handleNewFile()}>+ 新規ファイル</button>
      <For each={vaultStore.fileTree()}>
        {entry => <FileTreeItem entry={entry} depth={0} />}
      </For>
    </div>
  );
}
