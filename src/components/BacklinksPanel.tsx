import { createResource, For, Show } from "solid-js";
import { vaultStore } from "../stores/vault";
import * as tauri from "../lib/tauri";
import type { RelativePath } from "../types/brand";

export function BacklinksPanel() {
  const [backlinks] = createResource(
    () => vaultStore.selectedFile(),
    (path) => (path ? tauri.getBacklinks(path) : Promise.resolve([])),
  );

  return (
    <div class="backlinks-panel">
      <div class="backlinks-title">バックリンク</div>
      <Show when={!backlinks.loading} fallback={<div class="backlinks-empty">…</div>}>
        <Show
          when={(backlinks() ?? []).length > 0}
          fallback={<div class="backlinks-empty">なし</div>}
        >
          <For each={backlinks() ?? []}>
            {(b) => (
              <div
                class="backlinks-item"
                title={b.path}
                onClick={() =>
                  vaultStore.setSelectedFile(b.path as RelativePath)
                }
              >
                <span class="backlinks-path">{b.path}</span>
                <span class="backlinks-count">{b.wikilink_count}</span>
              </div>
            )}
          </For>
        </Show>
      </Show>
    </div>
  );
}
