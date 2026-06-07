import DOMPurify from "dompurify";
import { createEffect, onCleanup } from "solid-js";
import { editorStore } from "../stores/editor";
import { vaultStore } from "../stores/vault";
import * as tauri from "../lib/tauri";
import { renderMath } from "../lib/math";
import { renderMermaid } from "../lib/mermaid";
import { renderHighlight } from "../lib/highlight";
import type { RelativePath } from "../types/brand";

const PURIFY_CONFIG: Parameters<typeof DOMPurify.sanitize>[1] = {
  ALLOWED_TAGS: [
    "h1", "h2", "h3", "h4", "h5", "h6",
    "p", "br", "hr",
    "ul", "ol", "li",
    "a", "strong", "em", "del", "ins", "sub", "sup", "mark", "kbd",
    "code", "pre",
    "blockquote",
    "table", "thead", "tbody", "tfoot", "tr", "th", "td",
    "img",
    "details", "summary",
    "span", "div",
  ],
  ALLOWED_ATTR: [
    "href", "target", "rel",
    "src", "alt", "width", "height", "loading",
    "class", "id",
    "colspan", "rowspan", "scope", "align",
    "start", "type",
    "title",
    "open",
    // KaTeX / Mermaid markers (t13)
    "data-math", "data-diagram",
  ],
  // Allow asset: (local vault images) and data: (inline images) in addition to https?/mailto/tel.
  // javascript: and other dangerous schemes are blocked by the regex structure.
  ALLOWED_URI_REGEXP:
    /^(?:(?:asset|data|https?|mailto|tel):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i,
};

const WIKILINK_RE =
  /(!?\[\[([^\[\]|#\n]+?)(?:#([^\[\]|#\n]+?))?(?:\|([^\[\]\n]+?))?\]\])/g;

/** Walk text nodes outside <pre> / <code> and linkify [[wikilinks]]. */
function linkifyWikilinks(container: HTMLElement): void {
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  const textNodes: Text[] = [];
  let n: Node | null;
  while ((n = walker.nextNode()) !== null) {
    if (!insideCode(n)) textNodes.push(n as Text);
  }
  for (const textNode of textNodes) {
    const text = textNode.textContent ?? "";
    WIKILINK_RE.lastIndex = 0;
    if (!WIKILINK_RE.test(text)) continue;
    WIKILINK_RE.lastIndex = 0;
    const frag = document.createDocumentFragment();
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = WIKILINK_RE.exec(text)) !== null) {
      if (m.index > last) {
        frag.appendChild(document.createTextNode(text.slice(last, m.index)));
      }
      const target = (m[2] ?? "").trim();
      const display = m[4] ?? (m[3] ? `${target}#${m[3]}` : target);
      const span = document.createElement("span");
      span.className = "wikilink";
      span.dataset.target = target;
      span.textContent = display;
      frag.appendChild(span);
      last = m.index + m[0].length;
    }
    if (last < text.length) {
      frag.appendChild(document.createTextNode(text.slice(last)));
    }
    textNode.parentNode?.replaceChild(frag, textNode);
  }
}

function insideCode(node: Node): boolean {
  let el = node.parentElement;
  while (el) {
    const tag = el.tagName.toUpperCase();
    if (tag === "PRE" || tag === "CODE") return true;
    el = el.parentElement;
  }
  return false;
}

export function Preview() {
  let containerRef!: HTMLDivElement;

  createEffect(() => {
    const md = editorStore.content();
    let cancelled = false;
    const timer = setTimeout(async () => {
      const rendered = await tauri.renderMarkdown(md);
      if (cancelled) return;
      // Post-processing pipeline (order matters):
      //   ① DOMPurify       — t17: second-line XSS defence before any DOM insertion
      //   ② KaTeX math      — t14
      //   ③ Mermaid         — t15
      //   ④ highlight.js    — t15 (after mermaid so <pre> fallbacks are also highlighted)
      //   ⑤ wikilink linkify — last: avoids linkifying [[...]] inside math/diagram output
      containerRef.innerHTML = DOMPurify.sanitize(rendered, PURIFY_CONFIG) as string;
      await renderMath(containerRef);
      await renderMermaid(containerRef);
      await renderHighlight(containerRef);
      linkifyWikilinks(containerRef);
    }, 150);
    onCleanup(() => {
      cancelled = true;
      clearTimeout(timer);
    });
  });

  function handleClick(e: MouseEvent) {
    const span = (e.target as Element).closest(".wikilink");
    if (!span) return;
    const target = span.getAttribute("data-target");
    if (!target) return;
    void (async () => {
      const path = await tauri.resolveWikilink(target);
      if (path) vaultStore.setSelectedFile(path as RelativePath);
    })();
  }

  return (
    <div
      ref={containerRef}
      class="preview-container"
      onClick={handleClick}
    />
  );
}
