import { EditorView } from "@codemirror/view";

export const qwertTheme = EditorView.theme(
  {
    "&": {
      backgroundColor: "var(--qw-bg)",
      color: "var(--qw-fg)",
      fontFamily: "var(--qw-font-family)",
      fontSize: "var(--qw-font-size)",
      lineHeight: "var(--qw-line-height)",
      maxWidth: "var(--qw-editor-max-width)",
    },
    ".cm-content": {
      caretColor: "var(--qw-cursor)",
      padding: "16px",
    },
    ".cm-cursor": { borderLeftColor: "var(--qw-cursor)" },
    ".cm-selectionBackground, ::selection": {
      backgroundColor: "var(--qw-selection-bg) !important",
    },
    ".cm-gutters": {
      backgroundColor: "var(--qw-surface)",
      color: "var(--qw-fg-muted)",
      border: "none",
    },
    "&.cm-focused": { outline: "none" },
  },
  { dark: false },
);
