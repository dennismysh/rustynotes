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
