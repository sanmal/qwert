import { createResource, Show } from "solid-js";
import { SAVE_STATE } from "../types/constants";
import { editorStore } from "../stores/editor";
import { vaultStore } from "../stores/vault";
import { appearanceStore } from "../stores/appearance";
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

  // C9: contrast ratio/level come from Rust IPC; refetch on every appearance change.
  const [statusResource] = createResource(
    () => appearanceStore.appearanceVersion(),
    () => tauri.getAppearanceStatus(),
  );

  const contrastData = () => {
    const s = statusResource();
    if (!s || s.contrast_ratio == null) return null;
    return { ratio: s.contrast_ratio, level: s.level ?? "fail" };
  };

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
      <Show when={contrastData()} keyed>
        {(cd) => (
          <span
            class={cd.level !== "fail" ? "status-contrast-ok" : "status-contrast-warn"}
            title={`コントラスト比 ${cd.ratio.toFixed(2)}:1 (${cd.level === "fail" ? "AA未満" : cd.level})`}
          >
            CR {cd.ratio.toFixed(1)}
          </span>
        )}
      </Show>
      <Show when={appearanceStore.currentWarning()}>
        {(msg) => (
          <span class="status-warn" title={msg()}>
            ⚠ appearance
          </span>
        )}
      </Show>
      <span>{saveLabel()}</span>
    </div>
  );
}
