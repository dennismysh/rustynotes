import { Component, onMount, onCleanup, createEffect } from "solid-js";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "@tiptap/markdown";
import TaskList from "@tiptap/extension-task-list";
import TaskItem from "@tiptap/extension-task-item";
import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
import { common, createLowlight } from "lowlight";
import { appState } from "../../lib/state";
import { writeFile, parseMarkdown } from "../../lib/ipc";

const lowlight = createLowlight(common);

const WysiwygEditor: Component = () => {
  let editorElement: HTMLDivElement | undefined;
  let editor: Editor | undefined;
  let skipUpdate = false;

  const {
    activeFileContent,
    setActiveFileContent,
    activeFilePath,
    setIsDirty,
    setRenderedHtml,
  } = appState;

  onMount(() => {
    if (!editorElement) return;

    editor = new Editor({
      element: editorElement,
      extensions: [
        StarterKit.configure({
          codeBlock: false,
        }),
        CodeBlockLowlight.configure({
          lowlight,
        }),
        Markdown.configure({
          markedOptions: { gfm: true },
        }),
        TaskList,
        TaskItem.configure({ nested: true }),
      ],
      content: activeFileContent(),
      contentType: "markdown",
      onUpdate({ editor: ed }) {
        if (skipUpdate) return;
        const md = ed.getMarkdown();
        setActiveFileContent(md);
        setIsDirty(true);
        parseMarkdown(md).then((html) => setRenderedHtml(html));
      },
    });
  });

  createEffect(() => {
    const content = activeFileContent();
    if (editor && !editor.isDestroyed) {
      const currentMd = editor.getMarkdown();
      if (currentMd !== content) {
        skipUpdate = true;
        editor.commands.setContent(content, { contentType: "markdown" });
        skipUpdate = false;
      }
    }
  });

  const handleKeyDown = async (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      const path = activeFilePath();
      if (path && editor) {
        const md = editor.getMarkdown();
        await writeFile(path, md);
        setIsDirty(false);
      }
    }
  };

  onMount(() => document.addEventListener("keydown", handleKeyDown));
  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
    editor?.destroy();
  });

  return <div ref={editorElement} class="wysiwyg-editor markdown-content" />;
};

export default WysiwygEditor;
