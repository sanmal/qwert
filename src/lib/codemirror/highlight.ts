import { Compartment } from "@codemirror/state";
import { syntaxHighlighting, HighlightStyle } from "@codemirror/language";
import { tags } from "@lezer/highlight";

export const highlightCompartment = new Compartment();

export const qwertHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, color: "var(--qw-cm-keyword)" },
  { tag: tags.string, color: "var(--qw-cm-string)" },
  { tag: tags.comment, color: "var(--qw-cm-comment)", fontStyle: "italic" },
  { tag: tags.heading, color: "var(--qw-cm-heading)", fontWeight: "bold" },
  { tag: tags.link, color: "var(--qw-cm-link)" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "bold" },
]);

export const highlightOn = highlightCompartment.of(
  syntaxHighlighting(qwertHighlightStyle),
);

export const highlightOff = highlightCompartment.of([]);
