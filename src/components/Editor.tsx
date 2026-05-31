import { onMount, onCleanup, createEffect } from "solid-js";
import { EditorView } from "@codemirror/view";
import { EditorState, type Extension } from "@codemirror/state";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { basicSetup } from "codemirror";
import { qwertTheme } from "../lib/codemirror/theme";
import {
  highlightCompartment,
  highlightOn,
  highlightOff,
} from "../lib/codemirror/highlight";
import { editorStore } from "../stores/editor";
import { settingsStore } from "../stores/settings";

export function Editor() {
  let containerRef!: HTMLDivElement;
  // strict での未代入参照を避けるため undefined 許容にする。
  let view: EditorView | undefined;
  // プログラム的なコンテンツ差し替え（ファイル切替）中は onChange を発火させない。
  // これをしないと、読み込み直後に docChanged → UNSAVED → 自動保存が走り、
  // 開いた瞬間に「未保存」扱い＆自動上書きが発生する。
  let applyingRemote = false;

  onMount(async () => {
    // vim() は basicSetup より「前」に置く（後ろに push するとキーバインド衝突の元）。
    const extensions: Extension[] = [];
    if (settingsStore.vimMode()) {
      const { vim } = await import("@replit/codemirror-vim");
      extensions.push(vim());
    }
    extensions.push(
      basicSetup,
      markdown({ base: markdownLanguage, codeLanguages: languages }),
      qwertTheme,
      settingsStore.syntaxHighlight() ? highlightOn : highlightOff,
      EditorView.updateListener.of(update => {
        if (update.docChanged && !applyingRemote) {
          editorStore.onChange(update.state.doc.toString());
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

  onCleanup(() => view?.destroy());

  return <div ref={containerRef} class="editor-container" />;
}
