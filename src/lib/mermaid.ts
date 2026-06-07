// Mermaid lazy-loader for div.mermaid-block markers emitted by core (t13).
//
// Mermaid is imported dynamically; notes without mermaid blocks pay no load cost.
// initialize() is called once at load time with securityLevel:"strict" to sandbox
// clickable elements inside generated SVGs.

interface MermaidAPI {
  initialize(config: { startOnLoad: boolean; securityLevel: string }): void;
  render(id: string, text: string): Promise<{ svg: string }>;
}

// Promise-cache: Mermaid initializes exactly once per session.
let _mermaidPromise: Promise<MermaidAPI> | null = null;
// Monotonic counter so each render call gets a globally unique element id.
let _idSeq = 0;

function getMermaid(): Promise<MermaidAPI> {
  if (_mermaidPromise === null) {
    _mermaidPromise = import("mermaid").then((mod) => {
      const m = mod.default as unknown as MermaidAPI;
      m.initialize({ startOnLoad: false, securityLevel: "strict" });
      return m;
    });
  }
  return _mermaidPromise;
}

/**
 * Render all `div.mermaid-block` markers in `container` using Mermaid.
 *
 * - Returns immediately when no markers exist (no Mermaid import triggered).
 * - On parse error: replaces the marker with a `<pre>` showing raw diagram source.
 * - The diagram source is read from `data-diagram` (HTML-decoded by the browser).
 */
export async function renderMermaid(container: HTMLElement): Promise<void> {
  const els = container.querySelectorAll<HTMLElement>("div.mermaid-block");
  if (els.length === 0) return;

  const mermaid = await getMermaid();

  for (const el of els) {
    const code = el.dataset.diagram ?? "";
    if (!code.trim()) {
      el.remove();
      continue;
    }
    const id = `qwert-mermaid-${++_idSeq}`;
    try {
      const { svg } = await mermaid.render(id, code);
      el.innerHTML = svg;
      // Mermaid may leave a hidden staging element; remove it if present.
      document.getElementById(id)?.remove();
    } catch {
      // Fallback: show original diagram source as preformatted text (壊さない)
      const pre = document.createElement("pre");
      pre.textContent = code;
      el.replaceWith(pre);
    }
  }
}
