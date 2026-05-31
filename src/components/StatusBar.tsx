import { SAVE_STATE } from "../types/constants";
import { editorStore } from "../stores/editor";
import { vaultStore } from "../stores/vault";

export function StatusBar() {
  const saveLabel = () => {
    switch (editorStore.saveState()) {
      case SAVE_STATE.SAVING: return "保存中...";
      case SAVE_STATE.UNSAVED: return "未保存";
      default: return "保存済み";
    }
  };

  return (
    <div class="status-bar">
      <span>{vaultStore.selectedFile() ?? ""}</span>
      <span>{saveLabel()}</span>
    </div>
  );
}
