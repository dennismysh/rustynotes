# RustyNotes Bloat Reduction & Cleanup

**Date:** 2026-04-01
**Scope:** Dead code removal, WASM binary diet, JS bundle optimization

## Goal

Reduce the RustyNotes build footprint by removing dead code from the Solid.js migration, consolidating duplicated dependencies (comrak, syntect) into the backend, and splitting the monolithic JS bridge bundle. Target: ~2-3MB off the 6.2MB WASM binary and a significantly smaller initial JS payload.

## Section 1: Dead Code Removal

### Rust — Unused Functions

Remove the following functions that are defined but never called:

| Function | File | Reason |
|---|---|---|
| `parse_markdown` | `src-tauri/src/commands/markdown.rs` | Defined but never registered in Tauri handler (will be repurposed in Section 2) |
| `listen_file_changed` | `crates/rustynotes-frontend/src/tauri_ipc.rs` | Never called |
| `focus_code_mirror` | `crates/rustynotes-frontend/src/bridge.rs` | Never called |
| `focus_tiptap` | `crates/rustynotes-frontend/src/bridge.rs` | Never called |
| `get_tiptap_markdown` | `crates/rustynotes-frontend/src/bridge.rs` | Never called |
| `render_katex` | `crates/rustynotes-frontend/src/bridge.rs` | Never called |
| `render_mermaid` | `crates/rustynotes-frontend/src/bridge.rs` | Never called |

### npm — Solid.js Leftovers

Remove from `package.json`:
- **dependencies:** `solid-js`, `@solidjs/router`
- **devDependencies:** `vite-plugin-solid`
- **scripts:** `start`, `dev`, `build`, `serve` (Vite-based, replaced by Trunk)
- Evaluate whether `vite` itself can be removed (Trunk is the build tool)
- Run `pnpm install` to clean `node_modules` and regenerate `pnpm-lock.yaml`

### CSS — Dead Properties

- **Remove** `--toolbar-height` and `--overlay-bg` from `:root` (defined, never referenced)
- **Add** `--surface`, `--text`, `--surface-hover` to theme JSON files and the CSS fallback `:root` block (referenced in modal CSS but currently undefined)

## Section 2: Comrak Consolidation

**Problem:** comrak is compiled into both the Rust backend and the WASM frontend. The frontend already communicates with the backend via IPC for file operations.

**Solution:** Remove comrak from the frontend. All markdown parsing goes through the backend's `parse_markdown` IPC command.

### Changes

1. **Backend** (`src-tauri/src/commands/markdown.rs`):
   - Keep the existing `parse_markdown` command
   - Register it in `lib.rs`'s `generate_handler!` macro

2. **Frontend** (`crates/rustynotes-frontend/`):
   - Remove `comrak` from `Cargo.toml`
   - Add `parse_markdown(content: String) -> String` to `tauri_ipc.rs`
   - Update preview component to call IPC instead of parsing locally

3. **IPC capability**: Add `parse_markdown` to Tauri capability permissions if needed

### Trade-off

Adds an IPC round-trip per render. In Tauri this is same-process, sub-millisecond — negligible compared to the rendering itself.

## Section 3: Syntect Optimization

**Problem:** The frontend loads all ~400 syntax definitions and all themes via `SyntaxSet::load_defaults_newlines()` and `ThemeSet::load_defaults()`. Only one theme (`base16-ocean.dark`) is used.

**Solution:** Move syntax highlighting to the backend (alongside comrak), using a minimal configuration.

### Changes

1. **Backend** (`src-tauri/src/markdown_parser.rs`):
   - Add syntect as a dependency to `src-tauri/Cargo.toml`
   - Post-process comrak HTML output to add syntax highlighting to code blocks
   - For the backend, use `SyntaxSet::load_defaults_newlines()` and `ThemeSet::load_defaults()` — asset size doesn't matter here since it's native code, not WASM. The whole point is removing these from the WASM binary. Cache with `once_cell::Lazy` for one-time initialization.
   - Use the `base16-ocean.dark` theme for highlighting (same as current frontend behavior)

2. **Frontend** (`crates/rustynotes-frontend/`):
   - Remove `syntect`, `regex-lite`, `html-escape`, `once_cell` from `Cargo.toml`
   - Delete `crates/rustynotes-frontend/src/components/preview/markdown.rs`
   - Preview component becomes a thin shell: call `parse_markdown` IPC, set `innerHTML`, then run JS post-processing (KaTeX, Mermaid) via the bridge

3. **Backend returns fully-highlighted HTML** — the frontend only handles JS-dependent post-processing (KaTeX math, Mermaid diagrams)

### Estimated WASM Savings

- comrak removal: ~200-400KB
- syntect + assets removal: ~1-2MB
- Supporting crates (regex-lite, html-escape, once_cell): ~50KB
- **Total: ~1.5-2.5MB** off the 6.2MB WASM binary

## Section 4: JS Bundle Optimization

**Problem:** `bridge.bundle.js` is 8.2MB — bundles CodeMirror, TipTap, KaTeX, and Mermaid into a single file loaded at startup.

**Solution:** Use dynamic `import()` for KaTeX and Mermaid so they load on-demand, and enable esbuild code splitting.

### Changes

1. **`js/bridge-src.js`:**
   - Keep CodeMirror and TipTap as eager imports (needed immediately for editor)
   - Convert KaTeX and Mermaid to dynamic `import()` — loaded on first use when preview encounters math/mermaid blocks
   - Bridge functions that use these libraries become async, loading the library on first call and caching the module reference

2. **`js/bundle-vendor.sh`:**
   - Change esbuild config from single `--outfile` to code splitting:
     ```bash
     npx esbuild js/bridge-src.js --bundle --splitting --format=esm --outdir=js/dist/
     ```
   - Main chunk contains CodeMirror + TipTap (~2-3MB)
   - KaTeX and Mermaid become separate chunks loaded on demand

3. **`index.html`:**
   - Update Trunk directives: `<link data-trunk rel="copy-dir" href="js/dist" />` instead of single file copy
   - Update `<script type="module">` to point to main chunk in `js/dist/`

### Expected Result

- Initial JS payload: ~2-3MB (CodeMirror + TipTap only)
- KaTeX chunk: loaded on-demand when math blocks are detected
- Mermaid chunk: loaded on-demand when mermaid diagrams are detected
- Total bytes shipped stays the same, but startup is significantly faster

## Out of Scope

- Making `RenderingToggles` functional (render_math, render_diagrams, etc.) — this is feature work for a separate design
- Theme system deduplication (CSS `:root` vs JSON files)
- Conditional KaTeX CSS loading
- Consolidating inline styles in navigation components

## Testing Strategy

1. **After dead code removal:** `cargo build` succeeds, `trunk build` succeeds, app launches normally
2. **After comrak consolidation:** Preview mode renders markdown identically to before (compare same document)
3. **After syntect move:** Code blocks render with syntax highlighting in preview (test multiple languages)
4. **After JS bundle split:** Editor modes work, math renders in preview, mermaid diagrams render in preview, no console errors about missing modules
5. **Size verification:** Compare WASM binary size before/after, JS bundle size before/after
