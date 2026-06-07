import { createEffect, createMemo, createSignal, For, onMount } from "solid-js";
import { vaultStore } from "../stores/vault";
import type { FileEntry } from "../lib/tauri";
import type { RelativePath } from "../types/brand";

interface Props {
  onClose: () => void;
}

/** Returns a score > 0 if all query chars appear in order within text, 0 otherwise. */
function fuzzyScore(text: string, query: string): number {
  if (!query) return 1;
  const t = text.toLowerCase();
  const q = query.toLowerCase();
  let score = 0;
  let ti = 0;
  let qi = 0;
  let consecutive = 0;
  while (ti < t.length && qi < q.length) {
    if (t[ti] === q[qi]) {
      score += 1 + consecutive;
      consecutive++;
      qi++;
    } else {
      consecutive = 0;
    }
    ti++;
  }
  return qi < q.length ? 0 : score;
}

export function CommandPalette(props: Props) {
  const [query, setQuery] = createSignal("");
  const [activeIndex, setActiveIndex] = createSignal(0);
  let inputRef!: HTMLInputElement;
  let listRef!: HTMLUListElement;

  const results = createMemo((): FileEntry[] => {
    const q = query();
    const entries = vaultStore.flatFileEntries();
    if (!q) return entries.slice(0, 50);
    return entries
      .map(e => ({ entry: e, score: fuzzyScore(e.path, q) }))
      .filter(x => x.score > 0)
      .sort((a, b) => b.score - a.score)
      .map(x => x.entry)
      .slice(0, 50);
  });

  createEffect(() => {
    results();
    setActiveIndex(0);
  });

  createEffect(() => {
    const i = activeIndex();
    const item = listRef?.children[i] as HTMLElement | undefined;
    item?.scrollIntoView({ block: "nearest" });
  });

  onMount(() => inputRef.focus());

  function openEntry(entry: FileEntry) {
    vaultStore.setSelectedFile(entry.path as RelativePath);
    props.onClose();
  }

  function onKeyDown(e: KeyboardEvent) {
    switch (e.key) {
      case "Escape":
        e.preventDefault();
        props.onClose();
        break;
      case "ArrowDown":
        e.preventDefault();
        setActiveIndex(i => Math.min(i + 1, results().length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setActiveIndex(i => Math.max(i - 1, 0));
        break;
      case "Enter": {
        e.preventDefault();
        const entry = results()[activeIndex()];
        if (entry) openEntry(entry);
        break;
      }
    }
  }

  return (
    <div
      class="palette-overlay"
      role="dialog"
      aria-modal="true"
      aria-label="コマンドパレット"
      onClick={props.onClose}
    >
      <div class="palette" onClick={e => e.stopPropagation()} onKeyDown={onKeyDown}>
        <input
          ref={inputRef}
          class="palette-input"
          type="text"
          placeholder="ファイルを検索..."
          aria-label="ファイル名を入力"
          value={query()}
          onInput={e => setQuery(e.currentTarget.value)}
        />
        <ul
          ref={listRef}
          class="palette-list"
          role="listbox"
          aria-label="検索結果"
        >
          <For each={results()}>
            {(entry, i) => (
              <li
                class="palette-item"
                classList={{ "palette-item--active": i() === activeIndex() }}
                role="option"
                aria-selected={i() === activeIndex()}
                onClick={() => openEntry(entry)}
                onMouseMove={() => setActiveIndex(i())}
              >
                {entry.path}
              </li>
            )}
          </For>
        </ul>
      </div>
    </div>
  );
}
