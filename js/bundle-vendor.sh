#!/bin/bash
set -e
cd "$(dirname "$0")/.."

pnpm install

npx esbuild js/bridge-src.js \
  --bundle --format=esm --outfile=js/bridge.bundle.js

# Vendor KaTeX CSS locally
cp node_modules/katex/dist/katex.min.css styles/katex.min.css

echo "Done: js/bridge.bundle.js + styles/katex.min.css"
