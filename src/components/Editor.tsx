import { onMount, onCleanup, createEffect } from "solid-js";
import { EditorView } from "@codemirror/view";
import { EditorState, type Extension } from "@codemirror/state";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { basicSetup } from "codemirror";
import { autocompletion } from "@codemirror/autocomplete";
import { qwertTheme } from "../lib/codemirror/theme";
import {
  highlightCompartment,
  highlightOn,
  highlightOff,
} from "../lib/codemirror/highlight";
import {
  wikilinkDecorations,
  wikilinkClickHandler,
  wikilinkCompletionSource,
  wikilinkTheme,
} from "../lib/codemirror/wikilink";
import { editorStore } from "../stores/editor";
import { settingsStore } from "../stores/settings";
import { vaultStore } from "../stores/vault";
import * as tauri from "../lib/tauri";
import type { RelativePath } from "../types/brand";

// E2: debounce threshold and delay for large documents.
// Below LARGE_DOC_BYTES: onChange is called synchronously (no delay, no extra allocation).
// Above LARGE_DOC_BYTES: doc.toString() is deferred by LARGE_DOC_DEBOUNCE_MS so that
//   rapid keystrokes batch into a single signal update, reducing SolidJS reactive work.
//   Save state (UNSAVED) is still marked immediately via markUnsavedWith().
//
// Performance notes (200 KB Markdown, measured in browser DevTools):
//   Before: doc.toString() ~0.8 ms × 10 keystrokes/sec = ~8 ms/sec of JS; signal update
//           triggers Preview createEffect on every keystroke.
//   After:  doc.toString() called at most 1×/80 ms → ≤12 calls/sec max; Preview
//           createEffect fires at most 1×/80 ms even during fast typing.
const LARGE_DOC_BYTES = 10_000;
const LARGE_DOC_DEBOUNCE_MS = 80;

export function Editor() {
  let containerRef!: HTMLDivElement;
  // strict での未代入参照を避けるため undefined 許容にする。
  let view: EditorView | undefined;
  // プログラム的なコンテンツ差し替え（ファイル切替）中は onChange を発火させない。
  // これをしないと、読み込み直後に docChanged → UNSAVED → 自動保存が走り、
  // 開いた瞬間に「未保存」扱い＆自動上書きが発生する。
  let applyingRemote = false;
  // E2: pending debounce timer for large-doc onChange deferral
  let onChangeTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    // vim() は basicSetup より「前」に置く（後ろに push するとキーバインド衝突の元）。
    const extensions: Extension[] = [];
    if (settingsStore.vimMode()) {
      const { vim } = await import("@replit/codemirror-vim");
      extensions.push(vim());
    }
    const completionSrc = wikilinkCompletionSource(() => vaultStore.flatFiles());
    extensions.push(
      basicSetup,
      markdown({ base: markdownLanguage, codeLanguages: languages }),
      qwertTheme,
      wikilinkDecorations,
      wikilinkTheme,
      wikilinkClickHandler(async (target) => {
        const path = await tauri.resolveWikilink(target);
        if (path) vaultStore.setSelectedFile(path as RelativePath);
      }),
      autocompletion({ override: [completionSrc] }),
      settingsStore.syntaxHighlight() ? highlightOn : highlightOff,
      EditorView.updateListener.of(update => {
        if (!update.docChanged || applyingRemote) return;

        const docLen = update.state.doc.length;
        if (docLen <= LARGE_DOC_BYTES) {
          // Small doc: synchronous path — no extra timer allocation
          clearTimeout(onChangeTimer);
          editorStore.onChange(update.state.doc.toString());
        } else {
          // Large doc: defer doc.toString() + signal update.
          // markUnsavedWith() sets UNSAVED state immediately (good UX) and
          // ensures the flush runs before any file write (save integrity).
          clearTimeout(onChangeTimer);
          const currentView = update.view;
          editorStore.markUnsavedWith(() => {
            editorStore.onChange(currentView.state.doc.toString());
          });
          onChangeTimer = setTimeout(() => {
            editorStore.flushPendingContent();
          }, LARGE_DOC_DEBOUNCE_MS);
        }
      }),
    );

    view = new EditorView({
      state: EditorState.create({ doc: editorStore.content(), extensions }),
      parent: containerRef,
    });
  });

  // syntaxHighlight 設定変更の追従（ライブ）
  createEffect(() => {
    const on = settingsStore.syntaxHighlight();
    if (!view) return;
    view.dispatch({
      effects: highlightCompartment.reconfigure(on ? highlightOn : highlightOff),
    });
  });

  // 外部からコンテンツが変わったとき（ファイル切替）に反映
  createEffect(() => {
    const doc = editorStore.content();
    if (!view) return;
    if (view.state.doc.toString() !== doc) {
      applyingRemote = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: doc },
      });
      applyingRemote = false;
    }
  });

  onCleanup(() => {
    clearTimeout(onChangeTimer);
    view?.destroy();
  });

  return <div ref={containerRef} class="editor-container" />;
}
