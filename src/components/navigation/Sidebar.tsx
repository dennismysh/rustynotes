import { Component, For, Show, createSignal } from "solid-js";
import { appState } from "../../lib/state";
import { readFile, parseMarkdown } from "../../lib/ipc";
import type { FileEntry } from "../../lib/ipc";

const TreeNode: Component<{ entry: FileEntry; depth: number }> = (props) => {
  const [expanded, setExpanded] = createSignal(false);
  const { activeFilePath, setActiveFilePath, setActiveFileContent, setRenderedHtml, setIsDirty } = appState;

  const handleClick = async () => {
    if (props.entry.is_dir) {
      setExpanded(!expanded());
    } else {
      setActiveFilePath(props.entry.path);
      const content = await readFile(props.entry.path);
      setActiveFileContent(content);
      setIsDirty(false);
      const html = await parseMarkdown(content);
      setRenderedHtml(html);
    }
  };

  const isActive = () => activeFilePath() === props.entry.path;

  return (
    <div>
      <div
        class="tree-item"
        classList={{ active: isActive() }}
        style={{ "padding-left": `${12 + props.depth * 16}px` }}
        onClick={handleClick}
      >
        <span class="icon">
          {props.entry.is_dir ? (expanded() ? "\u25BC" : "\u25B6") : "\uD83D\uDCC4"}
        </span>
        <span class="name">{props.entry.name}</span>
      </div>
      <Show when={props.entry.is_dir && expanded() && props.entry.children}>
        <div class="tree-children">
          <For each={props.entry.children!}>
            {(child) => <TreeNode entry={child} depth={props.depth + 1} />}
          </For>
        </div>
      </Show>
    </div>
  );
};

const Sidebar: Component = () => {
  const { fileTree, currentFolder } = appState;

  return (
    <div class="sidebar">
      <Show
        when={currentFolder()}
        fallback={
          <div style="padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;">
            No folder open
          </div>
        }
      >
        <For each={fileTree()}>
          {(entry) => <TreeNode entry={entry} depth={0} />}
        </For>
      </Show>
    </div>
  );
};

export default Sidebar;
