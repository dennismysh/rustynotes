# RustyNotes

Local-first markdown editor built with Tauri 2 (Rust backend) + Solid.js (TypeScript frontend). Rich rendering via KaTeX, Mermaid, Shiki. Editing via CodeMirror 6 and TipTap 3.

## Design Context

### Users
Knowledge workers, researchers, and developers who want a clean, local-first markdown editor with rich rendering (math, diagrams, code) and full control over appearance. They value speed, keyboard-driven workflows, and owning their files — no cloud lock-in, no proprietary formats.

### Brand Personality
**Crafted, thoughtful, quiet.** RustyNotes feels hand-made and intentional — like a precision tool built by someone who cares deeply about the craft. The rust-toned identity gives it warmth without being loud. It should feel engineered with care, not manufactured.

### Design Principles
1. **Content is king** — UI serves the writing. Chrome recedes; content dominates.
2. **Fast and honest** — No jank, no fake loading. 0.2s max transitions.
3. **Precision over decoration** — Systematic spacing, semantic color tokens, traceable design decisions.
4. **Keyboard-first, mouse-welcome** — Power users on keyboard, everything discoverable by mouse.
5. **Earn complexity** — Start simple, reveal depth progressively.

### Anti-references
No Electron bloat, no Google Docs generic feel, no plain-text-editor bareness. Think VS Code / Zed — keyboard-driven, developer-oriented, fast — but with the warmth of a writing tool.


<!-- VAULT-KNOWLEDGE-START -->
## Vault Knowledge
<!-- Auto-generated from Obsidian vault. Do not edit manually. -->
<!-- Project: RustyNotes -->

### Rules
- **Triple Reinforcement for Critical Workflows**

### Patterns
- **Comrak HTML Post-Processing Pattern**: Parse markdown with comrak in Rust, return HTML to frontend, then post-process the DOM with JS libraries (KaTeX, Mermaid, Shiki) for rich rendering.
- **WFF Alpha-Gated Groups for Conditional Rendering**: Use alpha-gated Groups to conditionally render WFF element variants by config value.
- **Multi-Level Discipline Enforcement**: Enforce critical workflows at three layers: docs, skill/protocol, and automation hook.

### Project-Specific Knowledge (RustyNotes)
- **TipTap v3 Markdown Extension API: getMarkdown and setContent** [major] (lesson)
- **Tauri v2 Multi-Window Settings Pattern** (insight)
- **Theme Override Key Mapping Mismatch** [major] (lesson)
- **WebKit Native Select Elements Look Skeuomorphic in Dark UIs** [minor] (lesson)
- **Tauri v2 Capability Window Scope** [major] (lesson)
- **CSS Custom Properties as Theme Engine: accent-fg for Contrast** (insight)
- **Saved Config Overrides Rust Default Changes** [minor] (lesson)
- **Tauri Dev Mode Doesn't Use Custom Icons** [minor] (lesson)
- **Tauri v2 Window Visibility for Flash Prevention** [major] (lesson)
- **TipTap v3 Code Block Lowlight for WYSIWYG Syntax Highlighting** [major] (lesson)
- **Lazy-Load Heavy JS Libraries for Tauri Bundle Size** [major] (lesson)
- **CSS Custom Properties as Theme Engine** (insight)
- **Tauri Rust+TS Split for Markdown Editors** (insight)
- **Tauri v2 + Solid.js Project Setup** [major] (lesson)
- **TipTap v3 Framework-Agnostic Usage with Solid.js** [major] (lesson)
- **Tauri v2 IPC Command Patterns** [minor] (lesson)

### Critical Items (Other Projects)
- **Kotlin import Reserved Keyword as Package Name** (Android-Launcher)
- **VLOOKUP FALSE Argument Handling** (Spreadsheet-App)
- **TRUE/FALSE Bare Identifiers Parsing** (Spreadsheet-App)

<!-- VAULT-KNOWLEDGE-END -->