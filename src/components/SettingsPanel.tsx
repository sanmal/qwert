import { createSignal, Show } from "solid-js";
import { settingsStore } from "../stores/settings";
import type { Keybindings } from "../lib/tauri";
import * as tauri from "../lib/tauri";

type KeyAction = keyof Keybindings;

const KB_LABELS: Record<KeyAction, string> = {
  save: "保存",
  new_note: "新規ノート",
  command_palette: "コマンドパレット",
  full_search: "全文検索",
  view_mode_toggle: "表示モード切替",
  sidebar_toggle: "サイドバー切替",
  settings: "設定画面",
};

const KB_ACTIONS: KeyAction[] = [
  "save",
  "new_note",
  "command_palette",
  "full_search",
  "view_mode_toggle",
  "sidebar_toggle",
  "settings",
];

export function SettingsPanel() {
  const [kbDraft, setKbDraft] = createSignal<Keybindings>({ ...settingsStore.keybindings() });
  const [kbError, setKbError] = createSignal("");
  const [kbSaved, setKbSaved] = createSignal(false);

  function updateDraft(action: KeyAction, value: string) {
    setKbDraft(prev => ({ ...prev, [action]: value } as Keybindings));
  }

  async function handleSaveKeybindings() {
    setKbError("");
    setKbSaved(false);
    try {
      const kb = kbDraft();
      await tauri.saveKeybindings(kb);
      settingsStore.setKeybindings(kb);
      setKbSaved(true);
      setTimeout(() => setKbSaved(false), 2000);
    } catch (e) {
      setKbError(String(e));
    }
  }

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

      <h3>キーボードショートカット</h3>
      <div class="keybindings-editor">
        {KB_ACTIONS.map(action => (
          <label class="keybinding-row">
            <span class="keybinding-label">{KB_LABELS[action]}</span>
            <input
              type="text"
              class="keybinding-input"
              value={kbDraft()[action]}
              onInput={(e) => updateDraft(action, e.currentTarget.value)}
              placeholder="例: Ctrl+S"
            />
          </label>
        ))}
        <Show when={kbError()}>
          <p class="keybinding-error" role="alert">{kbError()}</p>
        </Show>
        <Show when={kbSaved()}>
          <p class="keybinding-saved" role="status">保存しました</p>
        </Show>
        <button onClick={() => void handleSaveKeybindings()}>保存</button>
      </div>
    </div>
  );
}
