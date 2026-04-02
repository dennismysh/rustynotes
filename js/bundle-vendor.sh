#!/bin/bash
set -e
cd "$(dirname "$0")/.."

pnpm install

npx esbuild js/bridge-src.js \
  --bundle --splitting --format=esm --outdir=js/dist/

# Vendor KaTeX CSS locally
cp node_modules/katex/dist/katex.min.css styles/katex.min.css

echo "Done: js/dist/ (chunked) + styles/katex.min.css"
