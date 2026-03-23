import { Component } from "solid-js";
import SourceEditor from "./SourceEditor";
import Preview from "../preview/Preview";

const SplitPane: Component = () => {
  return (
    <div class="split-pane">
      <div class="split-pane-left">
        <SourceEditor />
      </div>
      <div class="split-pane-divider" />
      <div class="split-pane-right">
        <Preview />
      </div>
    </div>
  );
};

export default SplitPane;
