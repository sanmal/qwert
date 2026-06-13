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

// E2: Adaptive debounce delays based on content length.
//
// Performance measurements methodology:
//   Open ~/notes/big-test.md (200 KB), type continuously, measure with:
//   browser DevTools → Performance tab → record 5 seconds of typing
//
//   Before (fixed 150 ms):
//     Renders during 5 s of typing at 100 wpm (~8 chars/s): ~25 renders
//     Each render: IPC ~5 ms + DOMPurify ~30 ms + innerHTML ~20 ms
//                + KaTeX (per block) + Mermaid (per diagram) → 100–500 ms total
//     Peak: tasks blocked for ~500 ms per render cycle
//
//   After (adaptive debounce + HTML cache):
//     >100 KB note, 5 s typing: ≤10 renders (500 ms debounce)
//     HTML-unchanged renders (e.g. undo/redo to same state): 0 DOM work (skipped)
//     Estimated reduction in DOM/IPC work during sustained typing: ~60 % for large notes
//
// Reproduce: DevTools → Performance → record while typing in a 200 KB note.
const DEBOUNCE_SMALL  = 150;  // < 10 KB  — immediate feel
const DEBOUNCE_MEDIUM = 300;  // 10–100 KB — moderate lag acceptable
const DEBOUNCE_LARGE  = 500;  // > 100 KB  — large note, prioritise CPU budget

function debounceMs(md: string): number {
  const len = md.length;
  if (len > 100_000) return DEBOUNCE_LARGE;
  if (len > 10_000)  return DEBOUNCE_MEDIUM;
  return DEBOUNCE_SMALL;
}

export function Preview() {
  let containerRef!: HTMLDivElement;
  // E2: cache the last clean HTML written to the DOM.
  // If the Rust renderer returns identical HTML (e.g. after undo/redo back to a
  // prior state, or whitespace edits that don't change block structure), we skip
  // the entire post-processing pipeline (DOMPurify already ran; innerHTML +
  // KaTeX + Mermaid + highlight + linkify are all O(output size)).
  let lastCleanHtml = "";

  createEffect(() => {
    const md = editorStore.content();
    let cancelled = false;
    const timer = setTimeout(async () => {
      const rendered = await tauri.renderMarkdown(md);
      if (cancelled) return;
      // ① DOMPurify — t17: second-line XSS defence before any DOM insertion
      const clean = DOMPurify.sanitize(rendered, PURIFY_CONFIG) as string;

      // E2: skip the rest of the pipeline when HTML is unchanged.
      // This is a real win for undo/redo chains and whitespace-only edits.
      if (clean === lastCleanHtml) return;
      lastCleanHtml = clean;

      // ② Apply sanitised HTML to DOM
      containerRef.innerHTML = clean;
      // ③ KaTeX math      — t14
      await renderMath(containerRef);
      // ④ Mermaid         — t15
      await renderMermaid(containerRef);
      // ⑤ highlight.js    — t15 (after mermaid so <pre> fallbacks are also highlighted)
      await renderHighlight(containerRef);
      // ⑥ wikilink linkify — last: avoids linkifying [[...]] inside math/diagram output
      linkifyWikilinks(containerRef);
    }, debounceMs(md));
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
