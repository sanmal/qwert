// KaTeX lazy-loader.
//
// KaTeX (~280KB JS + ~30KB CSS + fonts) is imported dynamically and cached
// as a module-level promise. Notes that contain no math markers incur zero
// load cost; the first math-containing note triggers a single load.

interface KatexLike {
  render(expression: string, element: HTMLElement, options: KatexRenderOpts): void;
}

interface KatexRenderOpts {
  displayMode: boolean;
  throwOnError: boolean;
  output: "htmlAndMathml";
}

// Promise-cache: shared across all calls so KaTeX loads at most once.
let _katexPromise: Promise<KatexLike> | null = null;

function getKatex(): Promise<KatexLike> {
  if (_katexPromise === null) {
    _katexPromise = Promise.all([
      import("katex"),
      import("katex/dist/katex.min.css"),
    ]).then(([mod]) => mod.default as KatexLike);
  }
  return _katexPromise;
}

const BASE_OPTS: Omit<KatexRenderOpts, "displayMode"> = {
  throwOnError: false,    // malformed expressions → red error text, no crash
  output: "htmlAndMathml", // WCAG: hidden MathML for screen readers
};

/**
 * Render all `.math-inline` / `.math-block` markers in `container` with KaTeX.
 *
 * - Returns immediately (no import, no work) when no markers are found.
 * - KaTeX reads its expression from the `data-math` attribute set by the core (t13).
 */
export async function renderMath(container: HTMLElement): Promise<void> {
  const inlineEls = container.querySelectorAll<HTMLElement>("span.math-inline");
  const blockEls = container.querySelectorAll<HTMLElement>("div.math-block");

  if (inlineEls.length === 0 && blockEls.length === 0) return;

  const katex = await getKatex();

  for (const el of inlineEls) {
    katex.render(el.dataset.math ?? "", el, { ...BASE_OPTS, displayMode: false });
  }
  for (const el of blockEls) {
    katex.render(el.dataset.math ?? "", el, { ...BASE_OPTS, displayMode: true });
  }
}
