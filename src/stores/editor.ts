import { createSignal } from "solid-js";
import type { RelativePath } from "../types/brand";
import type { SaveState } from "../types/constants";
import { SAVE_STATE } from "../types/constants";
import * as tauri from "../lib/tauri";

const [content, setContent] = createSignal("");
const [saveState, setSaveState] = createSignal<SaveState>(SAVE_STATE.SAVED);
let autosaveTimer: ReturnType<typeof setTimeout> | undefined;
let saveCallback: (() => Promise<void>) | undefined;

async function loadFile(path: RelativePath) {
  const text = await tauri.readFile(path);
  setContent(text);
  setSaveState(SAVE_STATE.SAVED);
}

function onChange(newContent: string) {
  setContent(newContent);
  setSaveState(SAVE_STATE.UNSAVED);
  scheduleAutosave();
}

// 実際の書き込み処理は App.tsx 側で登録する（t09 で配線）。
function registerSaveCallback(cb: () => Promise<void>) {
  saveCallback = cb;
}

function scheduleAutosave(delayMs = 3000) {
  clearTimeout(autosaveTimer);
  autosaveTimer = setTimeout(async () => {
    if (!saveCallback) return;
    setSaveState(SAVE_STATE.SAVING);
    await saveCallback();
    setSaveState(SAVE_STATE.SAVED);
  }, delayMs);
}

// 即時保存（Ctrl+S 用）。タイマーをクリアして即座に書き込む。
async function saveCurrentFile() {
  clearTimeout(autosaveTimer);
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
  registerSaveCallback,
  scheduleAutosave,
  saveCurrentFile,
};
