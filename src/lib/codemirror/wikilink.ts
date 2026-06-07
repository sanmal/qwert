import {
  Decoration,
  type DecorationSet,
  EditorView,
  MatchDecorator,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";
import type { Extension } from "@codemirror/state";
import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete";

// Same pattern as qwert-core link_index.rs
const WIKILINK_PATTERN =
  /(!?\[\[([^\[\]|#\n]+?)(?:#([^\[\]|#\n]+?))?(?:\|([^\[\]\n]+?))?\]\])/g;

// ── Decoration plugin ─────────────────────────────────────────────────────────

const decorator = new MatchDecorator({
  regexp: new RegExp(WIKILINK_PATTERN.source, "g"),
  decoration(match) {
    const target = (match[2] ?? "").trim();
    return Decoration.mark({
      class: "cm-wikilink",
      attributes: { "data-target": target, title: `[[${target}]]` },
    });
  },
});

class WikilinkPlugin {
  decorations: DecorationSet;
  constructor(view: EditorView) {
    this.decorations = decorator.createDeco(view);
  }
  update(update: ViewUpdate) {
    this.decorations = decorator.updateDeco(update, this.decorations);
  }
}

export const wikilinkDecorations: Extension = ViewPlugin.fromClass(
  WikilinkPlugin,
  { decorations: (v) => v.decorations },
);

// ── Ctrl+Click handler ────────────────────────────────────────────────────────

export function wikilinkClickHandler(
  onCtrlClick: (target: string) => void,
): Extension {
  return EditorView.domEventHandlers({
    click(event: MouseEvent) {
      if (!event.ctrlKey) return false;
      const el = (event.target as HTMLElement).closest(".cm-wikilink");
      if (!el) return false;
      const target = el.getAttribute("data-target");
      if (target) onCtrlClick(target);
      return true;
    },
  });
}

// ── Completion source ─────────────────────────────────────────────────────────

export function wikilinkCompletionSource(getStems: () => string[]) {
  return (context: CompletionContext): CompletionResult | null => {
    const before = context.matchBefore(/\[\[[^\[\]\n]*$/);
    if (!before) return null;
    if (!context.explicit && before.text.length <= 2) return null;
    const typed = before.text.slice(2).toLowerCase();
    const options = getStems()
      .filter((s) => s.toLowerCase().includes(typed))
      .map((s) => ({ label: s, apply: `${s}]]` }));
    if (options.length === 0) return null;
    return { from: before.from + 2, options };
  };
}

// ── EditorView theme for wikilinks ────────────────────────────────────────────

export const wikilinkTheme: Extension = EditorView.theme({
  ".cm-wikilink": {
    color: "var(--qw-accent)",
    borderBottom: "1px solid var(--qw-accent)",
    cursor: "pointer",
  },
  ".cm-wikilink:hover": {
    opacity: "0.8",
  },
});
