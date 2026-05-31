import { createSignal, createEffect } from "solid-js";
import { editorStore } from "../stores/editor";
import * as tauri from "../lib/tauri";

export function Preview() {
  const [html, setHtml] = createSignal("");
  let renderTimer: ReturnType<typeof setTimeout>;

  createEffect(() => {
    const md = editorStore.content();
    clearTimeout(renderTimer);
    renderTimer = setTimeout(async () => {
      const rendered = await tauri.renderMarkdown(md);
      setHtml(rendered);
    }, 150);
  });

  return <div class="preview-container" innerHTML={html()} />;
}
