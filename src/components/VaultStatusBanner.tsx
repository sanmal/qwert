import { createResource, For, Show } from "solid-js";
import { vaultStore } from "../stores/vault";
import * as tauri from "../lib/tauri";

export function VaultStatusBanner() {
  const [status, { refetch }] = createResource(
    () => vaultStore.vaultRoot(),
    (root) => (root ? tauri.getVaultStatus() : Promise.resolve(null)),
  );

  return (
    <Show when={status() && !status()!.healthy}>
      <div class="vault-status-banner" role="alert">
        <span class="vault-status-icon">⚠</span>
        <div class="vault-status-messages">
          <For each={status()!.warnings}>
            {(w) => <div class="vault-status-warning">{w}</div>}
          </For>
        </div>
        <button class="vault-status-dismiss" onClick={() => void refetch()} title="再確認">
          ↺
        </button>
      </div>
    </Show>
  );
}
