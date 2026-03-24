import { Component, createEffect } from "solid-js";
import { appState } from "../../lib/state";
import { postProcessPreview } from "../../lib/postprocess";

const Preview: Component = () => {
  let containerRef: HTMLDivElement | undefined;
  const { renderedHtml } = appState;

  createEffect(async () => {
    const html = renderedHtml();
    if (containerRef) {
      containerRef.innerHTML = html;
      await postProcessPreview(containerRef);
    }
  });

  return <div ref={containerRef} class="preview-container markdown-content" />;
};

export default Preview;
