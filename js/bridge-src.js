// js/bridge-src.js
import { EditorView, basicSetup } from 'codemirror';
import { EditorState } from '@codemirror/state';
import { markdown } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';
import { keymap } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { searchKeymap } from '@codemirror/search';
import { Editor } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { Markdown } from '@tiptap/markdown';

let katexModule = null;
let mermaidModule = null;

window.RustyNotesBridge = {
  mountCodeMirror(element, content, options, onChange) {
    const extensions = [
      basicSetup,
      markdown(),
      keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
      history(),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) onChange(update.state.doc.toString());
      }),
    ];
    if (options.theme === 'dark') extensions.push(oneDark);
    const state = EditorState.create({ doc: content, extensions });
    return { view: new EditorView({ state, parent: element }) };
  },

  updateCodeMirror(handle, content) {
    const cur = handle.view.state.doc.toString();
    if (cur !== content) {
      handle.view.dispatch({
        changes: { from: 0, to: cur.length, insert: content },
      });
    }
  },

  focusCodeMirror(handle) {
    handle.view.focus();
  },

  destroyCodeMirror(handle) {
    handle.view.destroy();
  },

  mountTipTap(element, content, options, onChange) {
    const exts = [
      StarterKit.configure({ codeBlock: false }),
      Markdown,
    ];
    if (options.taskLists !== false) {
      exts.push(TaskList, TaskItem.configure({ nested: true }));
    }
    const editor = new Editor({
      element,
      extensions: exts,
      content,
      contentType: 'markdown',
      onUpdate: ({ editor }) => onChange(editor.getMarkdown()),
    });
    return { editor };
  },

  updateTipTap(handle, content) {
    if (handle.editor.getMarkdown() !== content) {
      handle.editor.commands.setContent(content, { contentType: 'markdown' });
    }
  },

  getTipTapMarkdown(handle) {
    return handle.editor.getMarkdown();
  },

  focusTipTap(handle) {
    handle.editor.commands.focus();
  },

  destroyTipTap(handle) {
    handle.editor.destroy();
  },

  async renderKatex(element, latex, displayMode) {
    if (!katexModule) katexModule = await import('katex');
    element.innerHTML = katexModule.default.renderToString(latex, {
      throwOnError: false,
      displayMode,
    });
  },

  async renderMermaid(element, code, theme) {
    if (!mermaidModule) {
      mermaidModule = await import('mermaid');
      mermaidModule.default.initialize({
        startOnLoad: false,
        theme: theme === 'dark' ? 'dark' : 'default',
        securityLevel: 'loose',
      });
    }
    const id = `mermaid-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const { svg } = await mermaidModule.default.render(id, code);
    element.innerHTML = svg;
  },
};
