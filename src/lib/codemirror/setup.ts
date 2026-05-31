import { basicSetup } from "codemirror";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { qwertTheme } from "./theme";
import { highlightOn } from "./highlight";

export function createBaseExtensions(_vimMode: boolean) {
  const extensions = [
    basicSetup,
    markdown({ base: markdownLanguage, codeLanguages: languages }),
    qwertTheme,
    highlightOn,
  ];

  // vim() は動的インポートで遅延ロード（t08 Editor コンポーネントで実装）

  return extensions;
}
