import { settingsStore } from "../stores/settings";

export function SettingsPanel() {
  return (
    <div class="settings-panel">
      <h2>設定</h2>
      <label>
        <input
          type="checkbox"
          checked={settingsStore.vimMode()}
          onChange={(e) => settingsStore.setVimMode(e.currentTarget.checked)}
        />
        Vim バインド（変更後は再起動が必要）
      </label>
      <label>
        <input
          type="checkbox"
          checked={settingsStore.syntaxHighlight()}
          onChange={(e) => settingsStore.setSyntaxHighlight(e.currentTarget.checked)}
        />
        構文ハイライト
      </label>
    </div>
  );
}
