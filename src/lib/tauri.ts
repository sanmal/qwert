import { invoke } from "@tauri-apps/api/core";
import type { RelativePath, AbsolutePath } from "../types/brand";

export interface FileEntry {
  name: string;
  path: RelativePath;
  is_dir: boolean;
  children?: FileEntry[];
}

export async function listDir(): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("list_dir");
}

export async function readFile(path: RelativePath): Promise<string> {
  return invoke<string>("read_file", { path });
}

export async function writeFile(path: RelativePath, content: string): Promise<void> {
  return invoke<void>("write_file", { path, content });
}

export async function createFile(path: RelativePath): Promise<void> {
  return invoke<void>("create_file", { path });
}

export async function getVaultRoot(): Promise<AbsolutePath | null> {
  const result = await invoke<string | null>("get_vault_root");
  return result as AbsolutePath | null;
}

export async function openVaultDialog(): Promise<AbsolutePath | null> {
  const result = await invoke<string | null>("open_vault_dialog");
  return result as AbsolutePath | null;
}

export async function renderMarkdown(content: string): Promise<string> {
  return invoke<string>("render_markdown", { content });
}

export async function loadAppearance(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("load_appearance");
}
