import { Component, Show, For } from "solid-js";
import { appState } from "../../lib/state";
import { isFirstRun, markWelcomed } from "../../lib/onboarding";

interface Props {
  onOpenFolder: () => void;
  onOpenRecent: (folder: string) => void;
}

const WelcomeEmptyState: Component<Props> = (props) => {
  const { appConfig } = appState;
  const isMac = navigator.platform.includes("Mac");
  const mod = isMac ? "\u2318" : "Ctrl+";

  const recentFolders = () => {
    const config = appConfig();
    return config ? config.recent_folders : [];
  };

  const folderName = (path: string) => path.split("/").pop() ?? path;

  const handleOpenFolder = () => {
    if (isFirstRun()) markWelcomed();
    props.onOpenFolder();
  };

  const handleOpenRecent = (folder: string) => {
    if (isFirstRun()) markWelcomed();
    props.onOpenRecent(folder);
  };

  return (
    <>
      <Show when={isFirstRun()} fallback={<h1 class="empty-state-title">RustyNotes</h1>}>
        <h1 class="empty-state-title">Welcome to RustyNotes</h1>
        <p class="empty-state-welcome">
          A local-first markdown editor. Your files stay on your machine.
        </p>
      </Show>
      <p class="hint">
        WYSIWYG editing, LaTeX math, Mermaid diagrams, and syntax-highlighted code.
      </p>
      <button class="empty-state-cta" onClick={handleOpenFolder}>
        Open Folder
      </button>
      <Show when={recentFolders().length > 0}>
        <div class="recent-folders">
          <h2 class="recent-folders-heading">Recent</h2>
          <ul class="recent-folders-list">
            <For each={recentFolders().slice(0, 5)}>
              {(folder) => (
                <li>
                  <button
                    class="recent-folder-item"
                    onClick={() => handleOpenRecent(folder)}
                    title={folder}
                  >
                    <span class="recent-folder-icon" aria-hidden="true">{"\u2013"}</span>
                    <span class="recent-folder-name">{folderName(folder)}</span>
                    <span class="recent-folder-path">{folder}</span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </div>
      </Show>
      <div class="empty-state-shortcuts">
        <div class="shortcut-row"><kbd>{mod}K</kbd> <span>Search files</span></div>
        <div class="shortcut-row"><kbd>{mod}E</kbd> <span>Cycle editor mode</span></div>
        <div class="shortcut-row"><kbd>{mod}1/2/3</kbd> <span>Switch navigation</span></div>
        <Show when={isFirstRun()}>
          <div class="shortcut-row"><kbd>{mod},</kbd> <span>Open settings</span></div>
        </Show>
      </div>
    </>
  );
};

export default WelcomeEmptyState;
