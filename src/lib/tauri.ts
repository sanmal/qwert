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

/** Move a file within the vault. Pure file-system rename; wikilinks are unaffected
 *  because they resolve by stem, not by path. Distinct from Revision. */
export async function moveFile(src: RelativePath, dst: RelativePath): Promise<void> {
  return invoke<void>("move_file", { src, dst });
}

/** Level 3 editing hint: notify the backend that `path` is unsaved (isEditing=true) or saved (false). */
export async function setEditingState(path: RelativePath, isEditing: boolean): Promise<void> {
  return invoke<void>("set_editing_state", { path, isEditing });
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

// ── Appearance status (C9) ─────────────────────────────────────────────────────

export interface AppearanceStatus {
  schema_version: string;
  kind: string;
  scope: "global" | "vault";
  preset: string | null;
  fg: string | null;
  bg: string | null;
  /** WCAG 2.x contrast ratio rounded to 2 decimal places, or null when no colors. */
  contrast_ratio: number | null;
  /** "AAA" | "AA" | "fail" for normal text, or null when no colors. */
  level: string | null;
  text: {
    font_size: number;
    font_family: string;
    line_height: number;
    letter_spacing: number;
    word_spacing: number;
    editor_max_width: number;
  };
  highlight: { enabled: boolean };
}

export async function getAppearanceStatus(): Promise<AppearanceStatus> {
  return invoke<AppearanceStatus>("get_appearance_status");
}

// ── Vault status ───────────────────────────────────────────────────────────────

export interface SyncConflict {
  base: string;
  conflict_file: string;
}

export interface PendingRevisionInfo {
  source_old_path: string;
  source_new_path: string;
}

export interface VaultStatus {
  vault: string;
  sync_conflicts: SyncConflict[];
  pending_revision: PendingRevisionInfo | null;
  healthy: boolean;
  warnings: string[];
}

export async function getVaultStatus(): Promise<VaultStatus> {
  return invoke<VaultStatus>("get_vault_status");
}

// ── Invisible char scan ────────────────────────────────────────────────────────

export interface ScanFinding {
  line: number;
  column: number;
  char_code: number;
  char_hex: string;
  category: string;
}

export interface FileScanResult {
  path: string;
  findings: ScanFinding[];
}

export async function scanNote(path: string): Promise<ScanFinding[]> {
  return invoke<ScanFinding[]>("scan_note", { path });
}

export async function scanVaultFiles(): Promise<FileScanResult[]> {
  return invoke<FileScanResult[]>("scan_vault_files");
}

export interface BacklinkEntry {
  path: string;
  wikilink_count: number;
}

export async function getBacklinks(path: string): Promise<BacklinkEntry[]> {
  return invoke<BacklinkEntry[]>("get_backlinks", { path });
}

export async function resolveWikilink(target: string): Promise<string | null> {
  return invoke<string | null>("resolve_wikilink", { target });
}

// ── Revision ──────────────────────────────────────────────────────────────────

export interface RevisionAffectedFile {
  path: string;
  wikilink_count: number;
}

export interface RevisionPlan {
  old_name: string;
  new_name: string;
  old_path: string;
  new_path: string;
  affected_files: RevisionAffectedFile[];
  total_wikilinks: number;
  diff: string;
}

export interface RevisionResult {
  old_path: string;
  new_path: string;
  total_wikilinks: number;
}

export async function planRevisionNote(
  path: string,
  naming: string,
  name?: string,
): Promise<RevisionPlan> {
  return invoke<RevisionPlan>("plan_revision_note", { path, naming, name });
}

export async function executeRevisionNote(
  path: string,
  naming: string,
  name?: string,
): Promise<RevisionResult> {
  return invoke<RevisionResult>("execute_revision_note", { path, naming, name });
}
