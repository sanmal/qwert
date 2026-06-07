// highlight.js lazy-loader for pre>code blocks.
//
// Only the core + 11 targeted languages are imported (not the full bundle)
// to minimise the lazy chunk size. The CSS theme is loaded alongside the JS.
// Notes with no code blocks pay no load cost.

interface HljsCore {
  registerLanguage(name: string, language: object): void;
  highlightElement(element: HTMLElement): void;
}

// Promise-cache: registered languages persist across renders.
let _hljsPromise: Promise<HljsCore> | null = null;

function getHljs(): Promise<HljsCore> {
  if (_hljsPromise === null) {
    _hljsPromise = (async () => {
      const [
        coreModule,
        jsLang,
        tsLang,
        pyLang,
        bashLang,
        rustLang,
        jsonLang,
        cssLang,
        sqlLang,
        xmlLang,
        mdLang,
        diffLang,
        // CSS theme — side-effect import; return value intentionally unused
      ] = await Promise.all([
        import("highlight.js/lib/core"),
        import("highlight.js/lib/languages/javascript"),
        import("highlight.js/lib/languages/typescript"),
        import("highlight.js/lib/languages/python"),
        import("highlight.js/lib/languages/bash"),
        import("highlight.js/lib/languages/rust"),
        import("highlight.js/lib/languages/json"),
        import("highlight.js/lib/languages/css"),
        import("highlight.js/lib/languages/sql"),
        import("highlight.js/lib/languages/xml"),
        import("highlight.js/lib/languages/markdown"),
        import("highlight.js/lib/languages/diff"),
        import("highlight.js/styles/github-dark-dimmed.css"),
      ]);

      const hljs = coreModule.default as unknown as HljsCore;
      hljs.registerLanguage("javascript", jsLang.default);
      hljs.registerLanguage("typescript", tsLang.default);
      hljs.registerLanguage("python", pyLang.default);
      hljs.registerLanguage("bash", bashLang.default);
      hljs.registerLanguage("rust", rustLang.default);
      hljs.registerLanguage("json", jsonLang.default);
      hljs.registerLanguage("css", cssLang.default);
      hljs.registerLanguage("sql", sqlLang.default);
      hljs.registerLanguage("xml", xmlLang.default);
      hljs.registerLanguage("markdown", mdLang.default);
      hljs.registerLanguage("diff", diffLang.default);

      return hljs;
    })();
  }
  return _hljsPromise;
}

/**
 * Apply syntax highlighting to all `pre > code` blocks in `container`.
 *
 * - Returns immediately when no code blocks exist (no highlight.js import triggered).
 * - Language is detected from the `language-*` class set by pulldown-cmark.
 * - Mermaid blocks are `<div>` (not `<pre><code>`), so they are never affected.
 */
export async function renderHighlight(container: HTMLElement): Promise<void> {
  const els = container.querySelectorAll<HTMLElement>("pre code");
  if (els.length === 0) return;

  const hljs = await getHljs();
  for (const el of els) {
    hljs.highlightElement(el);
  }
}
