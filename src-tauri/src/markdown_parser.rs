use comrak::{markdown_to_html, Options};
use once_cell::sync::Lazy;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse(input: &str) -> String {
        let html = markdown_to_html(input, &Self::default_options());
        highlight_code_blocks(&html)
    }

    fn default_options() -> Options<'static> {
        let mut options = Options::default();

        // GFM extensions
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        options.extension.footnotes = true;
        options.extension.description_lists = true;

        // GitHub-style alerts (admonitions)
        options.extension.alerts = true;

        // Math support
        options.extension.math_dollars = true;

        // Heading IDs for anchor links
        options.extension.header_ids = Some(String::new());

        // Front matter
        options.extension.front_matter_delimiter = Some("---".to_string());

        // Wiki-links
        options.extension.wikilinks_title_after_pipe = true;

        // Allow raw HTML passthrough
        options.render.r#unsafe = true;

        options
    }
}

fn highlight_code_blocks(html: &str) -> String {
    let re = regex::Regex::new(
        r#"<pre><code class="language-(\w+)">([\s\S]*?)</code></pre>"#,
    )
    .unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let lang = &caps[1];
        let code = html_escape::decode_html_entities(&caps[2]);

        // Preserve mermaid blocks for client-side rendering
        if lang == "mermaid" {
            return caps[0].to_string();
        }

        let syntax = SYNTAX_SET
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
        let theme = &THEME_SET.themes["base16-ocean.dark"];
        match highlighted_html_for_string(&code, &SYNTAX_SET, syntax, theme) {
            Ok(highlighted) => {
                format!(r#"<div class="shiki-wrapper">{}</div>"#, highlighted)
            }
            Err(_) => caps[0].to_string(),
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let html = MarkdownParser::parse("# Hello\n\nWorld");
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<p>World</p>"));
    }

    #[test]
    fn test_bold_italic() {
        let html = MarkdownParser::parse("**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_gfm_strikethrough() {
        let html = MarkdownParser::parse("~~deleted~~");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_gfm_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn test_gfm_tasklist() {
        let md = "- [x] Done\n- [ ] Todo";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("checked"));
        assert!(html.contains("type=\"checkbox\""));
    }

    #[test]
    fn test_fenced_code_block_highlighted() {
        let md = "```rust\nfn main() {}\n```";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("shiki-wrapper"));
    }

    #[test]
    fn test_mermaid_block_preserved() {
        let md = "```mermaid\ngraph LR\nA-->B\n```";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("language-mermaid"));
        assert!(!html.contains("shiki-wrapper"));
    }

    #[test]
    fn test_math_passthrough() {
        let md = "Inline $x^2$ and block:\n\n$$\nE = mc^2\n$$";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("x^2") || html.contains("math"));
    }

    #[test]
    fn test_footnotes() {
        let md = "Text[^1]\n\n[^1]: Footnote content";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("footnote"));
    }

    #[test]
    fn test_autolink() {
        let html = MarkdownParser::parse("Visit https://example.com");
        assert!(html.contains("<a href=\"https://example.com\">"));
    }

    #[test]
    fn test_unknown_language_fallback() {
        let md = "```obscurelang\nsome code\n```";
        let html = MarkdownParser::parse(md);
        // Should still produce output (falls back to plain text highlighting)
        assert!(html.contains("shiki-wrapper") || html.contains("<pre"));
    }
}
