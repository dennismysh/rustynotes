# Export Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an Exporter trait and working HTML export, with architecture ready for future PDF/DOCX/LaTeX formats. Frontend gets an export menu accessible from the toolbar.

**Architecture:** Rust backend defines an `Exporter` trait. Each format implements it. The HTML exporter uses comrak's `format_html` plus inlined theme CSS. A Tauri command dispatches export requests and writes the output file. Frontend shows an export dialog with format selection and save-file dialog.

**Tech Stack:** comrak (AST + format_html), Tauri save dialog, syntect (for syntax highlighting in HTML export)

**Spec:** `docs/superpowers/specs/2026-03-22-rustynotes-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/export/mod.rs` | Create | Exporter trait, ExportOptions, format registry |
| `src-tauri/src/export/html.rs` | Create | HTML exporter with theme CSS inlining |
| `src-tauri/src/commands/export.rs` | Create | Export IPC command |
| `src-tauri/src/commands/mod.rs` | Modify | Add export module |
| `src-tauri/src/lib.rs` | Modify | Register export command |
| `src/lib/ipc.ts` | Modify | Add export IPC wrapper |
| `src/components/Toolbar.tsx` | Modify | Add export button/menu |

---

## Task 1: Exporter Trait & HTML Exporter

**Files:**
- Create: `src-tauri/src/export/mod.rs`
- Create: `src-tauri/src/export/html.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create the exporter trait**

Create `src-tauri/src/export/mod.rs`:

```rust
pub mod html;

use comrak::{Arena, Options, parse_document};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExportOptions {
    pub format: String,
    pub include_theme: bool,
}

pub trait Exporter {
    fn export(&self, markdown: &str, options: &ExportOptions) -> Result<Vec<u8>, String>;
    fn file_extension(&self) -> &str;
    fn mime_type(&self) -> &str;
}

pub fn get_exporter(format: &str) -> Option<Box<dyn Exporter>> {
    match format {
        "html" => Some(Box::new(html::HtmlExporter)),
        _ => None,
    }
}
```

- [ ] **Step 2: Create HTML exporter**

Create `src-tauri/src/export/html.rs`:

```rust
use super::{ExportOptions, Exporter};
use crate::markdown_parser::MarkdownParser;

pub struct HtmlExporter;

impl HtmlExporter {
    fn default_css() -> &'static str {
        r#"
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
            font-size: 16px;
            line-height: 1.6;
            color: #1d1d1f;
            max-width: 800px;
            margin: 0 auto;
            padding: 40px 20px;
            background: #ffffff;
        }
        h1 { font-size: 2em; margin: 0.67em 0; }
        h2 { font-size: 1.5em; margin: 0.75em 0; }
        h3 { font-size: 1.17em; margin: 0.83em 0; }
        code {
            background: #f5f5f7;
            padding: 2px 6px;
            border-radius: 4px;
            font-family: 'SF Mono', 'JetBrains Mono', monospace;
            font-size: 0.9em;
        }
        pre {
            background: #f5f5f7;
            border: 1px solid #d2d2d7;
            border-radius: 8px;
            padding: 16px;
            overflow-x: auto;
        }
        pre code { background: none; padding: 0; }
        blockquote {
            border-left: 3px solid #007aff;
            padding-left: 16px;
            color: #6e6e73;
            margin: 1em 0;
        }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #d2d2d7; padding: 8px 12px; text-align: left; }
        th { background: #f5f5f7; font-weight: 600; }
        a { color: #007aff; text-decoration: none; }
        a:hover { text-decoration: underline; }
        img { max-width: 100%; }
        .footnotes { margin-top: 2em; padding-top: 1em; border-top: 1px solid #d2d2d7; font-size: 0.9em; }
        "#
    }
}

impl Exporter for HtmlExporter {
    fn export(&self, markdown: &str, options: &ExportOptions) -> Result<Vec<u8>, String> {
        let body_html = MarkdownParser::parse(markdown);

        let html = if options.include_theme {
            format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Exported Document</title>
<style>{}</style>
</head>
<body>
{}
</body>
</html>"#,
                Self::default_css(),
                body_html
            )
        } else {
            format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Exported Document</title>
</head>
<body>
{}
</body>
</html>"#,
                body_html
            )
        };

        Ok(html.into_bytes())
    }

    fn file_extension(&self) -> &str { "html" }
    fn mime_type(&self) -> &str { "text/html" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_export_basic() {
        let exporter = HtmlExporter;
        let options = ExportOptions { format: "html".to_string(), include_theme: true };
        let result = exporter.export("# Hello\n\nWorld", &options).unwrap();
        let html = String::from_utf8(result).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<style>"));
    }

    #[test]
    fn test_html_export_no_theme() {
        let exporter = HtmlExporter;
        let options = ExportOptions { format: "html".to_string(), include_theme: false };
        let result = exporter.export("# Hello", &options).unwrap();
        let html = String::from_utf8(result).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(!html.contains("<style>"));
    }

    #[test]
    fn test_html_export_gfm() {
        let exporter = HtmlExporter;
        let options = ExportOptions { format: "html".to_string(), include_theme: true };
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n\n- [x] Done";
        let result = exporter.export(md, &options).unwrap();
        let html = String::from_utf8(result).unwrap();
        assert!(html.contains("<table>"));
        assert!(html.contains("checkbox"));
    }

    #[test]
    fn test_get_exporter_html() {
        let exporter = super::super::get_exporter("html");
        assert!(exporter.is_some());
        assert_eq!(exporter.unwrap().file_extension(), "html");
    }

    #[test]
    fn test_get_exporter_unknown() {
        let exporter = super::super::get_exporter("pdf");
        assert!(exporter.is_none());
    }
}
```

Add `mod export;` to `src-tauri/src/lib.rs`.

Run: `cargo test export`

Commit: `git commit -m "feat: add Exporter trait and HTML exporter with theme CSS"`

---

## Task 2: Export IPC Command

**Files:**
- Create: `src-tauri/src/commands/export.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create export command**

Create `src-tauri/src/commands/export.rs`:

```rust
use crate::export::{self, ExportOptions};

#[tauri::command]
pub fn export_file(
    markdown: String,
    output_path: String,
    format: String,
    include_theme: bool,
) -> Result<(), String> {
    let exporter = export::get_exporter(&format)
        .ok_or_else(|| format!("Unsupported export format: {}", format))?;

    let options = ExportOptions { format, include_theme };
    let output = exporter.export(&markdown, &options)?;

    std::fs::write(&output_path, &output)
        .map_err(|e| format!("Failed to write export file: {}", e))?;

    Ok(())
}
```

Add `pub mod export;` to `commands/mod.rs`.
Register `commands::export::export_file` in `generate_handler!` in `lib.rs`.

Run: `cargo check`

Commit: `git commit -m "feat: add export IPC command with save-to-file"`

---

## Task 3: Frontend Export UI

**Files:**
- Modify: `src/lib/ipc.ts`
- Modify: `src/components/Toolbar.tsx`

- [ ] **Step 1: Add export IPC and save dialog**

Add to `src/lib/ipc.ts`:

```typescript
import { save } from "@tauri-apps/plugin-dialog";

export async function exportFile(
  markdown: string,
  outputPath: string,
  format: string,
  includeTheme: boolean,
): Promise<void> {
  return invoke<void>("export_file", { markdown, outputPath, format, includeTheme });
}

export async function showSaveDialog(defaultName: string, extension: string): Promise<string | null> {
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
  });
  return path as string | null;
}
```

Also add `"dialog:allow-save"` to `src-tauri/capabilities/default.json` permissions array.

- [ ] **Step 2: Add export button to Toolbar**

Add an "Export" button to the Toolbar. When clicked:
1. Get active file content from state
2. Show save dialog with `.html` extension
3. If path selected, call `exportFile()` with the markdown content

Read the current Toolbar.tsx first, then add the export handler and button.

- [ ] **Step 3: Verify**

```bash
cargo test
npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add export button with HTML save dialog"
```
