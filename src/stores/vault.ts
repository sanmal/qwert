import { createSignal } from "solid-js";
import type { FileEntry } from "../lib/tauri";
import type { RelativePath, AbsolutePath } from "../types/brand";
import * as tauri from "../lib/tauri";

const [vaultRoot, setVaultRoot] = createSignal<AbsolutePath | null>(null);
const [fileTree, setFileTree] = createSignal<FileEntry[]>([]);
const [selectedFile, setSelectedFile] = createSignal<RelativePath | null>(null);

async function openVault() {
  const root = await tauri.openVaultDialog();
  if (root) {
    setVaultRoot(root);
    await refreshFileTree();
  }
}

async function refreshFileTree() {
  const entries = await tauri.listDir();
  setFileTree(entries);
}

export const vaultStore = {
  vaultRoot,
  fileTree,
  selectedFile,
  setSelectedFile,
  openVault,
  refreshFileTree,
};
