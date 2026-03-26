#!/bin/bash
set -e
cd "$(dirname "$0")/.."

npm install --save-dev \
  codemirror @codemirror/state @codemirror/view @codemirror/lang-markdown \
  @codemirror/commands @codemirror/search @codemirror/theme-one-dark \
  @tiptap/core @tiptap/starter-kit @tiptap/extension-task-list \
  @tiptap/extension-task-item @tiptap/markdown \
  katex mermaid esbuild

npx esbuild js/bridge-src.js \
  --bundle --format=esm --outfile=js/bridge.bundle.js

# Vendor KaTeX CSS locally
cp node_modules/katex/dist/katex.min.css styles/katex.min.css

echo "Done: js/bridge.bundle.js + styles/katex.min.css"
