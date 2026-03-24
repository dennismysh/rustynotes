import katex from "katex";
import "katex/dist/katex.min.css";
import mermaid from "mermaid";
import { codeToHtml } from "shiki";

let mermaidInitialized = false;
let mermaidIdCounter = 0;

function initMermaid(isDark: boolean) {
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? "dark" : "default",
    securityLevel: "loose",
  });
  mermaidInitialized = true;
}

export async function postProcessPreview(container: HTMLElement): Promise<void> {
  const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;

  await Promise.all([
    renderMath(container),
    renderDiagrams(container, isDark),
    highlightCode(container, isDark),
  ]);
}

function renderMath(container: HTMLElement): void {
  // comrak with math_dollars wraps inline math in <code class="math-inline">
  // and block math in <code class="math-display">
  container.querySelectorAll('code.math-inline, [data-math-style="inline"]').forEach((el) => {
    try {
      const tex = el.textContent || "";
      const rendered = katex.renderToString(tex, { throwOnError: false, displayMode: false });
      const span = document.createElement("span");
      span.innerHTML = rendered;
      span.className = "math-rendered";
      el.replaceWith(span);
    } catch (_e) {
      // Leave raw on error
    }
  });

  container.querySelectorAll('code.math-display, [data-math-style="display"]').forEach((el) => {
    try {
      const tex = el.textContent || "";
      const rendered = katex.renderToString(tex, { throwOnError: false, displayMode: true });
      const div = document.createElement("div");
      div.innerHTML = rendered;
      div.className = "math-rendered math-block";
      const parent = el.closest("pre") || el;
      parent.replaceWith(div);
    } catch (_e) {
      // Leave raw on error
    }
  });
}

async function renderDiagrams(container: HTMLElement, isDark: boolean): Promise<void> {
  if (!mermaidInitialized) initMermaid(isDark);

  const mermaidBlocks = container.querySelectorAll('code.language-mermaid');
  for (let i = 0; i < mermaidBlocks.length; i++) {
    const el = mermaidBlocks[i];
    const code = el.textContent || "";
    const pre = el.closest("pre");
    if (!pre) continue;
    try {
      const id = `mermaid-${mermaidIdCounter++}`;
      const { svg } = await mermaid.render(id, code);
      const div = document.createElement("div");
      div.className = "mermaid-diagram";
      div.innerHTML = svg;
      pre.replaceWith(div);
    } catch (_e) {
      // Leave as code block on error
    }
  }
}

async function highlightCode(container: HTMLElement, isDark: boolean): Promise<void> {
  const codeBlocks = Array.from(
    container.querySelectorAll("pre > code[class*='language-']")
  );

  const highlights = codeBlocks.map(async (el) => {
    const langMatch = el.className.match(/language-(\w+)/);
    if (!langMatch) return;
    const lang = langMatch[1];
    if (lang === "mermaid") return;
    const code = el.textContent || "";
    const pre = el.closest("pre");
    if (!pre) return;
    try {
      const html = await codeToHtml(code, {
        lang,
        theme: isDark ? "github-dark" : "github-light",
      });
      const wrapper = document.createElement("div");
      wrapper.className = "shiki-wrapper";
      wrapper.innerHTML = html;
      pre.replaceWith(wrapper);
    } catch (_e) {
      // Leave unhighlighted on error
    }
  });

  await Promise.all(highlights);
}
