import { createSignal } from "solid-js";
import type { RelativePath } from "../types/brand";
import type { SaveState } from "../types/constants";
import { SAVE_STATE } from "../types/constants";
import * as tauri from "../lib/tauri";

const [content, setContent] = createSignal("");
const [saveState, setSaveState] = createSignal<SaveState>(SAVE_STATE.SAVED);
let autosaveTimer: ReturnType<typeof setTimeout> | undefined;
let saveCallback: (() => Promise<void>) | undefined;

// E2: Editor側 debounce で pending な flush コールバックを保持する。
// Ctrl+S / 自動保存はこれをフラッシュしてから書き込む（保存整合性維持）。
let pendingFlush: (() => void) | null = null;

async function loadFile(path: RelativePath) {
  const text = await tauri.readFile(path);
  pendingFlush = null;
  setContent(text);
  setSaveState(SAVE_STATE.SAVED);
}

function onChange(newContent: string) {
  pendingFlush = null;
  setContent(newContent);
  setSaveState(SAVE_STATE.UNSAVED);
  scheduleAutosave();
}

/** E2: Called by Editor debounce path (large docs). Marks unsaved immediately but
 *  defers the signal update + preview trigger until flush fires. */
function markUnsavedWith(flushFn: () => void) {
  pendingFlush = flushFn;
  setSaveState(SAVE_STATE.UNSAVED);
  scheduleAutosave();
}

/** Flush any pending debounced content immediately (called before Ctrl+S / autosave write). */
function flushPendingContent() {
  if (pendingFlush) {
    pendingFlush();
    pendingFlush = null;
  }
}

// 実際の書き込み処理は App.tsx 側で登録する（t09 で配線）。
function registerSaveCallback(cb: () => Promise<void>) {
  saveCallback = cb;
}

function scheduleAutosave(delayMs = 3000) {
  clearTimeout(autosaveTimer);
  autosaveTimer = setTimeout(async () => {
    if (!saveCallback) return;
    flushPendingContent(); // E2: ensure latest content is flushed before write
    setSaveState(SAVE_STATE.SAVING);
    await saveCallback();
    setSaveState(SAVE_STATE.SAVED);
  }, delayMs);
}

// 即時保存（Ctrl+S 用）。タイマーをクリアして即座に書き込む。
async function saveCurrentFile() {
  clearTimeout(autosaveTimer);
  flushPendingContent(); // E2: flush before read
  if (!saveCallback) return;
  setSaveState(SAVE_STATE.SAVING);
  await saveCallback();
  setSaveState(SAVE_STATE.SAVED);
}

export const editorStore = {
  content,
  saveState,
  loadFile,
  onChange,
  markUnsavedWith,
  flushPendingContent,
  registerSaveCallback,
  scheduleAutosave,
  saveCurrentFile,
};
