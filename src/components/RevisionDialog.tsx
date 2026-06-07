import { createSignal, For, onMount, Show } from "solid-js";
import type { FileEntry, RevisionPlan } from "../lib/tauri";
import * as tauri from "../lib/tauri";

interface Props {
  entry: FileEntry;
  onClose: () => void;
  onSuccess: (message: string) => void;
}

const DIFF_PREVIEW_LIMIT = 2000;

export function RevisionDialog(props: Props) {
  const [plan, setPlan] = createSignal<RevisionPlan | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [executing, setExecuting] = createSignal(false);
  const [newName, setNewName] = createSignal("");

  onMount(() => {
    void loadPlan();
  });

  async function loadPlan() {
    setLoading(true);
    setError(null);
    try {
      const result = await tauri.planRevisionNote(props.entry.path, "increment");
      setPlan(result);
      setNewName(result.new_name);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleExecute() {
    const p = plan();
    if (!p) return;
    const trimmedName = newName().trim();
    if (!trimmedName) return;

    setExecuting(true);
    setError(null);

    try {
      const result = await tauri.executeRevisionNote(props.entry.path, "manual", trimmedName);
      props.onSuccess(
        `「${result.old_path} → ${result.new_path}」完了: ${result.total_wikilinks}件のリンクを更新しました`,
      );
    } catch (e) {
      setError(String(e));
      setExecuting(false);
    }
  }

  const diffPreview = () => {
    const p = plan();
    if (!p?.diff) return "";
    return p.diff.length > DIFF_PREVIEW_LIMIT
      ? p.diff.slice(0, DIFF_PREVIEW_LIMIT) + "\n... (省略)"
      : p.diff;
  };

  return (
    <div class="dialog-overlay" onClick={props.onClose}>
      <div
        class="dialog revision-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Revision"
        onClick={e => e.stopPropagation()}
      >
        <h2 class="revision-title">Revision: {props.entry.name}</h2>

        <Show when={loading()}>
          <p class="revision-loading">プレビューを計算中...</p>
        </Show>

        <Show when={error()}>
          <p class="revision-error">{error()}</p>
        </Show>

        <Show when={plan() !== null && !loading()}>
          <div class="revision-field">
            <label for="revision-name">新しい名前:</label>
            <input
              id="revision-name"
              type="text"
              class="revision-name-input"
              value={newName()}
              onInput={e => setNewName(e.currentTarget.value)}
            />
          </div>

          <p class="revision-summary">
            影響ファイル: {plan()!.affected_files.length}ファイル、
            {plan()!.total_wikilinks}リンク
          </p>

          <Show when={plan()!.affected_files.length > 0}>
            <ul class="revision-affected">
              <For each={plan()!.affected_files}>
                {f => (
                  <li>
                    {f.path} ({f.wikilink_count}件)
                  </li>
                )}
              </For>
            </ul>
          </Show>

          <Show when={diffPreview()}>
            <pre class="revision-diff">{diffPreview()}</pre>
          </Show>
        </Show>

        <div class="revision-actions">
          <button onClick={props.onClose}>キャンセル</button>
          <button
            class="revision-execute-btn"
            onClick={() => void handleExecute()}
            disabled={loading() || executing() || plan() === null || !newName().trim()}
          >
            {executing() ? "実行中..." : "実行"}
          </button>
        </div>
      </div>
    </div>
  );
}
