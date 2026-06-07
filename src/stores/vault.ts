import { createMemo, createSignal } from "solid-js";
import type { FileEntry } from "../lib/tauri";
import type { RelativePath, AbsolutePath } from "../types/brand";
import * as tauri from "../lib/tauri";

const [vaultRoot, setVaultRoot] = createSignal<AbsolutePath | null>(null);
const [fileTree, setFileTree] = createSignal<FileEntry[]>([]);
const [selectedFile, setSelectedFile] = createSignal<RelativePath | null>(null);

/** Flat list of all .md file stems in the vault (for wikilink autocomplete). */
const flatFiles = createMemo(() => {
  const stems: string[] = [];
  function walk(entries: FileEntry[]) {
    for (const e of entries) {
      if (e.is_dir && e.children) walk(e.children);
      else if (!e.is_dir) stems.push(e.name.replace(/\.[^.]+$/, ""));
    }
  }
  walk(fileTree());
  return stems;
});

/** Flat list of all non-directory FileEntry items (for command palette). */
const flatFileEntries = createMemo(() => {
  const entries: FileEntry[] = [];
  function walk(nodes: FileEntry[]) {
    for (const e of nodes) {
      if (e.is_dir && e.children) walk(e.children);
      else if (!e.is_dir) entries.push(e);
    }
  }
  walk(fileTree());
  return entries;
});

/** Vault-relative paths of files that have invisible-character findings. */
const [filesWithWarnings, setFilesWithWarnings] = createSignal<ReadonlySet<string>>(new Set());

async function refreshScanResults(): Promise<void> {
  try {
    const results = await tauri.scanVaultFiles();
    setFilesWithWarnings(new Set(results.map((r) => r.path)));
  } catch {
    // Non-fatal: clear warnings on error
    setFilesWithWarnings(new Set<string>());
  }
}

async function openVault() {
  const root = await tauri.openVaultDialog();
  if (root) {
    setVaultRoot(root);
    await refreshFileTree();
    void refreshScanResults(); // background scan on vault open
  }
}

async function refreshFileTree() {
  const entries = await tauri.listDir();
  setFileTree(entries);
}

export const vaultStore = {
  vaultRoot,
  fileTree,
  flatFiles,
  flatFileEntries,
  filesWithWarnings,
  selectedFile,
  setSelectedFile,
  openVault,
  refreshFileTree,
  refreshScanResults,
};
