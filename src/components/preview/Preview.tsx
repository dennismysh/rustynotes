import { Component } from "solid-js";
import { appState } from "../../lib/state";

const Preview: Component = () => {
  const { renderedHtml } = appState;

  return (
    <div class="preview-container" innerHTML={renderedHtml()} />
  );
};

export default Preview;
