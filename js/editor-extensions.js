// js/editor-extensions.js
//
// The TipTap extension set for the WYSIWYG editor. Kept separate from the
// bridge so the markdown round-trip can be tested in Node (see
// roundtrip.test.mjs) without a DOM.
//
// The WYSIWYG editor rewrites the whole file from its document model on save.
// Any markdown the model has no node for is therefore deleted from the user's
// file. So every construct must either have a real node (code blocks, tables)
// or be carried through verbatim by the raw nodes below.
import { Extension, Mark, Node, mergeAttributes } from '@tiptap/core';
import Paragraph from '@tiptap/extension-paragraph';
import StarterKit from '@tiptap/starter-kit';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
import { Table, TableKit } from '@tiptap/extension-table';
import { Markdown } from '@tiptap/markdown';
import { common, createLowlight } from 'lowlight';

// ---------------------------------------------------------------------------
// Raw block: markdown kept verbatim (front matter, block math, footnote
// definitions, block HTML). Shown as an editable monospace block.
// ---------------------------------------------------------------------------

// `---` fence at the very start of the document, first line non-blank so a
// leading thematic break is not swallowed.
const FRONT_MATTER = /^---[ \t]*\n(?=\S)[\s\S]*?\n---[ \t]*(?:\n|$)/;
const MATH_BLOCK = /^ {0,3}\$\$[\s\S]+?\$\$[ \t]*(?:\n|$)/;
// `[^id]: text`, plus indented continuation lines.
const FOOTNOTE_DEF = /^\[\^[^\]\s]+\]:[^\n]*(?:\n(?: {2,}|\t)[^\n]+)*(?:\n|$)/;
// Where a raw block could begin, so marked ends the preceding paragraph there.
const RAW_BLOCK_START = /^(?: {0,3}\$\$|\[\^[^\]\s]+\]:)/m;

function rawBlockToken(kind, raw) {
  return { type: 'rawBlock', raw, kind, text: raw.replace(/\n+$/, '') };
}

function nodeText(node) {
  return (node.content || []).map((child) => child.text || '').join('');
}

export const RawBlock = Node.create({
  name: 'rawBlock',
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,
  defining: true,
  // Match before the code block's generic `pre` rule.
  priority: 200,

  addAttributes() {
    return {
      kind: {
        default: 'raw',
        parseHTML: (el) => el.getAttribute('data-raw-block') || 'raw',
        renderHTML: (attrs) => ({ 'data-raw-block': attrs.kind }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'pre[data-raw-block]', preserveWhitespace: 'full' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['pre', mergeAttributes(HTMLAttributes, { class: 'raw-block' }), 0];
  },

  markdownTokenName: 'rawBlock',

  markdownTokenizer: {
    name: 'rawBlock',
    level: 'block',
    start(src) {
      const match = RAW_BLOCK_START.exec(src);
      return match ? match.index : -1;
    },
    tokenize(src, tokens) {
      if (tokens.length === 0) {
        const fm = FRONT_MATTER.exec(src);
        if (fm) return rawBlockToken('frontmatter', fm[0]);
      }
      const math = MATH_BLOCK.exec(src);
      if (math) return rawBlockToken('math', math[0]);
      const footnote = FOOTNOTE_DEF.exec(src);
      if (footnote) return rawBlockToken('footnote', footnote[0]);
      return undefined;
    },
  },

  parseMarkdown(token, helpers) {
    return helpers.createNode(
      'rawBlock',
      { kind: token.kind || 'raw' },
      token.text ? [helpers.createTextNode(token.text)] : [],
    );
  },

  renderMarkdown(node) {
    return nodeText(node);
  },
});

// Block-level HTML arrives as marked's own `html` token. Without this, TipTap
// parses it against the schema and drops every tag it has no node for.
export const RawHtmlBlock = Extension.create({
  name: 'rawHtmlBlock',

  markdownTokenName: 'html',

  parseMarkdown(token, helpers) {
    if (!token.block) return [];
    const text = (token.raw || '').replace(/\n+$/, '');
    if (!text.trim()) return [];
    return helpers.createNode('rawBlock', { kind: 'html' }, [helpers.createTextNode(text)]);
  },
});

// ---------------------------------------------------------------------------
// Raw inline: inline markdown kept verbatim (inline math, wikilinks, inline
// HTML tags, images, backslash escapes). Without it, marked strips backslashes
// inside `$...$` and reads underscores as emphasis, and TipTap drops unknown
// tags, images and the escaped character of every `\x` escape.
//
// This is a mark on ordinary text rather than an atom node, so bold/italic
// can run across it (the serializer closes and reopens marks around inline
// nodes, which corrupts emphasis) and the raw text stays editable in place.
// ---------------------------------------------------------------------------

const RAW_INLINE_RULES = [
  ['html', /^<!--[\s\S]*?-->/],
  // A real tag, not an autolink like <https://example.com> or <a@b.c>.
  ['html', /^<\/?[A-Za-z][A-Za-z0-9-]*(?:\s+[^<>]*)?\s*\/?>/],
  ['math', /^\$\$[^$\n]+\$\$/],
  // Pandoc's rule: no space just inside the dollars, no digit right after,
  // so "$5 or $10" is left alone.
  ['math', /^\$(?![\s$])(?:\\.|[^$\\\n])+?(?<![\s\\])\$(?!\d)/],
  ['wikilink', /^\[\[[^[\]\n]+\]\]/],
];

function rawInlineResult(helpers, raw, kind) {
  return helpers.applyMark('rawInline', [helpers.createTextNode(raw)], { kind });
}

export const RawInline = Mark.create({
  name: 'rawInline',
  // Literal text: no input rules inside, no entity encoding on serialize.
  code: true,
  // Unlike inline code, allow bold/italic/links to span across it.
  excludes: '',
  inclusive: false,

  addAttributes() {
    return {
      kind: {
        default: 'raw',
        parseHTML: (el) => el.getAttribute('data-raw-inline') || 'raw',
        renderHTML: (attrs) => ({ 'data-raw-inline': attrs.kind }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'span[data-raw-inline]' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', mergeAttributes(HTMLAttributes, { class: 'raw-inline' }), 0];
  },

  markdownTokenName: 'rawInline',

  markdownTokenizer: {
    name: 'rawInline',
    level: 'inline',
    start(src) {
      return src.search(/<|\$|\[\[/);
    },
    tokenize(src) {
      for (const [kind, rule] of RAW_INLINE_RULES) {
        const match = rule.exec(src);
        if (match) return { type: 'rawInline', raw: match[0], kind };
      }
      return undefined;
    },
  },

  parseMarkdown(token, helpers) {
    return rawInlineResult(helpers, token.raw, token.kind || 'raw');
  },

  // The marked text is already the markdown source.
  renderMarkdown(node, helpers) {
    return helpers.renderChildren(node.content || []);
  },
});

// marked's `escape` token (`\*`, `\$`, ...). TipTap has no handler for it and
// drops the token, deleting the escaped character itself.
export const RawEscape = Extension.create({
  name: 'rawEscape',
  markdownTokenName: 'escape',
  parseMarkdown(token, helpers) {
    return rawInlineResult(helpers, token.raw, 'escape');
  },
});

// Images have no node here (local paths cannot be displayed in the webview
// yet), so keep the source. Reference-style images are rewritten inline
// because link reference definitions do not survive the round-trip.
export const RawImage = Extension.create({
  name: 'rawImage',
  markdownTokenName: 'image',
  parseMarkdown(token, helpers) {
    let raw = token.raw || '';
    if (!/^!\[[\s\S]*\]\([\s\S]*\)$/.test(raw)) {
      const title = token.title ? ` "${token.title.replace(/"/g, '\\"')}"` : '';
      raw = `![${token.text || ''}](${token.href || ''}${title})`;
    }
    return rawInlineResult(helpers, raw, 'image');
  },
});

// The stock paragraph unwraps a paragraph holding a single image so the image
// can be a block node. Here images are marked text, which must stay inside
// the paragraph or the document is invalid.
const ImageSafeParagraph = Paragraph.extend({
  parseMarkdown(token, helpers) {
    const tokens = token.tokens || [];
    if (tokens.length === 1 && tokens[0].type === 'image') {
      return helpers.createNode('paragraph', undefined, helpers.parseInline(tokens));
    }
    return this.parent?.(token, helpers);
  },
});

// ---------------------------------------------------------------------------
// Serializer fixes
// ---------------------------------------------------------------------------

// marked unescapes `\|` inside table cells before inline parsing, and the
// stock table renderer writes the bare pipe back, which adds a column on the
// next load.
const PipeSafeTable = Table.extend({
  renderMarkdown(node, helpers, context) {
    const escapePipes = (text) => text.replace(/(?<!\\)\|/g, '\\|');
    return this.parent?.(
      node,
      { ...helpers, renderChildren: (nodes, separator) => escapePipes(helpers.renderChildren(nodes, separator)) },
      context,
    ) ?? '';
  },
});

// Stock TipTap entity-encodes every `<`, `>` and `&` in text, so touching a
// file turns `a -> b` into `a -&gt; b` and `R&D` into `R&amp;D`. Encode only
// where the character would otherwise change meaning.
function encodeText(text) {
  return text
    .replace(/&(?=(?:#\d+|#[xX][0-9a-fA-F]+|[A-Za-z][A-Za-z0-9]*);)/g, '&amp;')
    .replace(/<(?=[A-Za-z/!?])/g, '&lt;')
    .replace(/^>/gm, '&gt;');
}

/**
 * Apply the gentler text encoding to a MarkdownManager. Call on
 * `editor.markdown` after creating the editor (and on any manager built
 * directly, as the tests do).
 */
export function tuneMarkdownManager(manager) {
  const stock = manager.encodeTextForMarkdown.bind(manager);
  manager.encodeTextForMarkdown = (text, node, parentNode) => {
    // The stock method returns text untouched inside code contexts.
    return stock(text, node, parentNode) === text ? text : encodeText(text);
  };
  return manager;
}

// ---------------------------------------------------------------------------

export function buildExtensions(options = {}) {
  const exts = [
    // The lowlight code block and image-safe paragraph below replace
    // StarterKit's own.
    StarterKit.configure({ codeBlock: false, paragraph: false }),
    ImageSafeParagraph,
    CodeBlockLowlight.configure({ lowlight: createLowlight(common) }),
    TableKit.configure({ table: false }),
    PipeSafeTable.configure({ resizable: false }),
    RawBlock,
    RawHtmlBlock,
    RawInline,
    RawEscape,
    RawImage,
    Markdown,
  ];
  if (options.taskLists !== false) {
    exts.push(TaskList, TaskItem.configure({ nested: true }));
  }
  return exts;
}
