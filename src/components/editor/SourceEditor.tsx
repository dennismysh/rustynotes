import { Component, onMount, onCleanup, createEffect } from "solid-js";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { markdown } from "@codemirror/lang-markdown";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { appState } from "../../lib/state";
import { writeFile, parseMarkdown } from "../../lib/ipc";

const SourceEditor: Component = () => {
  let containerRef: HTMLDivElement | undefined;
  let view: EditorView | undefined;
  let skipUpdate = false;

  const {
    activeFileContent,
    setActiveFileContent,
    activeFilePath,
    setIsDirty,
    setRenderedHtml,
  } = appState;

  const theme = EditorView.theme({
    "&": {
      height: "100%",
      backgroundColor: "var(--bg-primary)",
      color: "var(--text-primary)",
    },
    ".cm-content": {
      fontFamily: "var(--font-mono)",
      fontSize: "14px",
      padding: "16px 24px",
      caretColor: "var(--accent)",
    },
    ".cm-cursor": { borderLeftColor: "var(--accent)" },
    ".cm-gutters": {
      backgroundColor: "var(--bg-secondary)",
      borderRight: "1px solid var(--border)",
      color: "var(--text-muted)",
    },
    ".cm-activeLineGutter": { backgroundColor: "var(--bg-tertiary)" },
    ".cm-activeLine": { backgroundColor: "var(--bg-secondary)" },
    ".cm-selectionMatch": { backgroundColor: "var(--bg-tertiary)" },
    "&.cm-focused .cm-selectionBackground, ::selection": {
      backgroundColor: "var(--accent)",
      opacity: "0.3",
    },
  });

  const saveFile = async () => {
    const path = activeFilePath();
    if (path && view) {
      const content = view.state.doc.toString();
      await writeFile(path, content);
      setIsDirty(false);
    }
  };

  const saveKeymap = keymap.of([
    { key: "Mod-s", run: () => { saveFile(); return true; } },
  ]);

  onMount(() => {
    if (!containerRef) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && !skipUpdate) {
        const content = update.state.doc.toString();
        setActiveFileContent(content);
        setIsDirty(true);
        parseMarkdown(content).then((html) => { setRenderedHtml(html); });
      }
    });

    const state = EditorState.create({
      doc: activeFileContent(),
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        history(),
        markdown(),
        theme,
        keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
        saveKeymap,
        updateListener,
      ],
    });

    view = new EditorView({ state, parent: containerRef });
  });

  createEffect(() => {
    const content = activeFileContent();
    if (view && view.state.doc.toString() !== content) {
      skipUpdate = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
      });
      skipUpdate = false;
    }
  });

  onCleanup(() => { view?.destroy(); });

  return <div ref={containerRef} style={{ height: "100%", overflow: "hidden" }} />;
};

export default SourceEditor;
