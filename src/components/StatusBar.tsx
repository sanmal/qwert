import { createMemo, createResource, Show } from "solid-js";
import { SAVE_STATE } from "../types/constants";
import { editorStore } from "../stores/editor";
import { vaultStore } from "../stores/vault";
import { appearanceStore } from "../stores/appearance";
import { calcContrastRatio } from "../lib/contrast";
import * as tauri from "../lib/tauri";

export function StatusBar() {
  const saveLabel = () => {
    switch (editorStore.saveState()) {
      case SAVE_STATE.SAVING: return "保存中...";
      case SAVE_STATE.UNSAVED: return "未保存";
      default: return "保存済み";
    }
  };

  const [scanResult] = createResource(
    () => vaultStore.selectedFile(),
    (path) => (path ? tauri.scanNote(path) : Promise.resolve([])),
  );

  const warnCount = () => scanResult()?.length ?? 0;

  const contrast = createMemo(() => {
    const fg = appearanceStore.currentFg();
    const bg = appearanceStore.currentBg();
    if (!fg || !bg) return null;
    return calcContrastRatio(fg, bg);
  });

  return (
    <div class="status-bar">
      <span>{vaultStore.selectedFile() ?? ""}</span>
      <Show when={warnCount() > 0}>
        <span
          class="status-warn"
          title={`不可視文字が ${warnCount()} 件検出されました（note scan で詳細を確認）`}
        >
          ⚠ 不可視文字 {warnCount()}
        </span>
      </Show>
      <Show when={contrast()} keyed>
        {(cr) => (
          <span
            class={cr >= 4.5 ? "status-contrast-ok" : "status-contrast-warn"}
            title={`コントラスト比 ${cr.toFixed(2)}:1 ${cr >= 7.0 ? "(AAA)" : cr >= 4.5 ? "(AA)" : "(AA未満)"}`}
          >
            CR {cr.toFixed(1)}
          </span>
        )}
      </Show>
      <span>{saveLabel()}</span>
    </div>
  );
}
