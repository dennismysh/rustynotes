use comrak::{markdown_to_html, Options};

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse(input: &str) -> String {
        let options = Self::default_options();
        markdown_to_html(input, &options)
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
    fn test_fenced_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("<code class=\"language-rust\">"));
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
}
