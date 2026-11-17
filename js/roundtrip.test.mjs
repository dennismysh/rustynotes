// Markdown round-trip tests for the WYSIWYG editor's extension set.
//
// The WYSIWYG editor rewrites the whole file from its own document model on
// save, so anything the model cannot represent is deleted from the user's
// file. These tests pin down that parse -> serialize keeps every construct.
//
// Run with: pnpm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { getSchema } from '@tiptap/core';
import { MarkdownManager } from '@tiptap/markdown';
import { buildExtensions, tuneMarkdownManager } from './editor-extensions.js';

const extensions = buildExtensions();
const schema = getSchema(extensions);
const manager = tuneMarkdownManager(new MarkdownManager({ extensions }));

function roundTrip(md) {
  const json = manager.parse(md);
  // The parser builds plain JSON; the editor rejects (or silently repairs)
  // anything that does not fit its schema, so check that here.
  schema.nodeFromJSON(json).check();
  return manager.serialize(json);
}

/** Assert `fragment` survives a round-trip of `md` verbatim. */
function keeps(md, fragment = md.trim()) {
  const out = roundTrip(md);
  assert.ok(
    out.includes(fragment),
    `expected output to contain:\n${fragment}\n--- got ---\n${out}`,
  );
  return out;
}

test('fenced code block with language', () => {
  keeps('```rust\nfn main() {\n    println!("hi");\n}\n```\n');
});

test('fenced code block without language', () => {
  keeps('```\nplain *not italic*\n```\n');
});

test('mermaid block', () => {
  keeps('```mermaid\ngraph LR\nA-->B\n```\n');
});

test('table', () => {
  const out = roundTrip('| A | B |\n|---|---|\n| 1 | 2 |\n');
  assert.match(out, /\|\s*A\s*\|\s*B\s*\|/);
  assert.match(out, /\|\s*-+\s*\|\s*-+\s*\|/);
  assert.match(out, /\|\s*1\s*\|\s*2\s*\|/);
});

test('front matter stays front matter', () => {
  const out = keeps('---\ntitle: Test\ntags: [a, b]\n---\n\n# Heading\n', '---\ntitle: Test\ntags: [a, b]\n---');
  assert.ok(out.startsWith('---\ntitle: Test'), out);
  assert.ok(out.includes('# Heading'), out);
});

test('thematic break is not mistaken for front matter', () => {
  const out = roundTrip('Intro\n\n---\n\nAfter\n');
  assert.equal(out, 'Intro\n\n---\n\nAfter');
});

test('footnotes', () => {
  // A one-word definition is the dangerous case: marked reads it as a link
  // reference definition and turns `[^1]` into a link.
  const out = keeps('Text[^1] and more.\n\n[^1]: Footnote\n', '[^1]: Footnote');
  assert.ok(out.includes('Text[^1] and more.'), out);
});

test('multi-line footnote definition', () => {
  keeps('Text[^note]\n\n[^note]: First line\n    continued here\n', '[^note]: First line\n    continued here');
});

test('block HTML', () => {
  keeps('<details>\n<summary>More</summary>\n\nHidden *text*\n\n</details>\n', '<details>\n<summary>More</summary>');
  keeps('<details>\n<summary>More</summary>\n\nHidden *text*\n\n</details>\n', '</details>');
});

test('HTML comment', () => {
  keeps('<!-- keep me -->\n\nText\n', '<!-- keep me -->');
});

test('inline HTML', () => {
  keeps('Press <kbd>Ctrl</kbd>+<kbd>C</kbd> to copy.<br>Done.\n', 'Press <kbd>Ctrl</kbd>+<kbd>C</kbd> to copy.<br>Done.');
});

test('autolinks are not treated as HTML', () => {
  const out = roundTrip('See <https://example.com> now.\n');
  assert.ok(out.includes('https://example.com'), out);
});

test('inline math keeps underscores and asterisks', () => {
  const md = 'Let $\\{a_1\\}$ and $_x_ \\_ y$ and $a \\\\ b$ hold.\n';
  keeps(md, '$\\{a_1\\}$');
  keeps(md, '$_x_ \\_ y$');
  keeps(md, '$a \\\\ b$');
});

test('block math', () => {
  keeps('$$\n\\begin{aligned}\na_1 &= \\{b\\} \\\\\nc_2 &= *d*\n\\end{aligned}\n$$\n');
});

test('prices are left alone', () => {
  keeps('It costs $5 or $10 today.\n', 'It costs $5 or $10 today.');
});

test('wikilinks', () => {
  keeps('See [[My_Note_Name]] and [[other_note|the *label*]].\n', '[[My_Note_Name]]');
  keeps('See [[My_Note_Name]] and [[other_note|the *label*]].\n', '[[other_note|the *label*]]');
});

test('images', () => {
  keeps('Before ![alt text](img/a_b.png "Title") after.\n', '![alt text](img/a_b.png "Title")');
});

test('image alone in a paragraph', () => {
  keeps('Intro\n\n![alt](img/a.png)\n\nOutro\n', '![alt](img/a.png)');
});

test('reference-style image keeps its target', () => {
  const out = roundTrip('![logo][l]\n\n[l]: img/logo.png\n');
  assert.ok(out.includes('img/logo.png'), out);
});

test('backslash escapes keep the character and the backslash', () => {
  keeps('Literal \\*stars\\*, \\$7 and \\<kbd>.\n', 'Literal \\*stars\\*, \\$7 and \\<kbd>.');
});

test('emphasis spans raw inline content', () => {
  keeps('*with $a_b$ math* and **bold <kbd>K</kbd> \\* end**\n', '*with $a_b$ math*');
  keeps('*with $a_b$ math* and **bold <kbd>K</kbd> \\* end**\n', '**bold <kbd>K</kbd> \\* end**');
});

test('code spans and code blocks are not touched by raw rules', () => {
  keeps('Use `$x$ <b> [[y]] \\*` here.\n', '`$x$ <b> [[y]] \\*`');
  keeps('```html\n<div>$x$ &amp; [[y]]</div>\n```\n');
});

test('table cells keep inline content', () => {
  const out = roundTrip('| A | B |\n|---|---|\n| `c` and $x_1$ | a \\| b |\n');
  assert.ok(out.includes('`c` and $x_1$'), out);
  assert.ok(out.includes('a \\| b'), out);
});

test('plain text is not entity-encoded', () => {
  keeps('If a > b & c < d then a -> d, R&D => done.\n', 'If a > b & c < d then a -> d, R&D => done.');
});

test('entities that would change meaning stay encoded', () => {
  const md = 'Write &lt;div&gt; and &amp;amp; literally.\n';
  const once = roundTrip(md);
  assert.deepEqual(manager.parse(once), manager.parse(md));
});

test('GitHub alert', () => {
  keeps('> [!NOTE]\n> An alert\n');
});

test('task list', () => {
  keeps('- [x] done\n- [ ] todo\n');
});

test('raw blocks inside a blockquote keep their prefix', () => {
  keeps('> $$\n> a_1\n> $$\n');
});

test('round-trip is stable', () => {
  const md = [
    '---\ntitle: T\n---',
    '# H',
    'Text $a_1$ with [[link_a]] and <kbd>K</kbd>[^1].',
    '```js\nlet a = 1;\n```',
    '| A | B |\n|---|---|\n| 1 | 2 |',
    '$$\nE = mc^2\n$$',
    '<details>\n<summary>s</summary>',
    'body',
    '</details>',
    '[^1]: Note',
  ].join('\n\n') + '\n';
  const once = roundTrip(md);
  assert.equal(roundTrip(once), once);
});
