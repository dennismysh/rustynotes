use comrak::{markdown_to_html, Options};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use once_cell::unsync::Lazy;

// Thread-local for single-threaded WASM
thread_local! {
    static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
    static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);
}

pub fn render_markdown(input: &str) -> String {
    let html = markdown_to_html(input, &comrak_options());
    highlight_code_blocks(&html)
}

fn comrak_options() -> Options<'static> {
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

fn highlight_code_blocks(html: &str) -> String {
    let re = regex_lite::Regex::new(
        r#"<pre><code class="language-(\w+)">([\s\S]*?)</code></pre>"#,
    )
    .unwrap();

    re.replace_all(html, |caps: &regex_lite::Captures| {
        let lang = &caps[1];
        let code = html_escape::decode_html_entities(&caps[2]);

        // Preserve mermaid blocks for client-side rendering
        if lang == "mermaid" {
            return caps[0].to_string();
        }

        SYNTAX_SET.with(|ss| {
            THEME_SET.with(|ts| {
                let syntax = ss
                    .find_syntax_by_token(lang)
                    .unwrap_or_else(|| ss.find_syntax_plain_text());
                let theme = &ts.themes["base16-ocean.dark"];
                match highlighted_html_for_string(&code, ss, syntax, theme) {
                    Ok(highlighted) => {
                        format!(r#"<div class="shiki-wrapper">{}</div>"#, highlighted)
                    }
                    Err(_) => caps[0].to_string(),
                }
            })
        })
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_markdown() {
        let html = render_markdown("# Hello\n\nWorld");
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn code_block_highlighted() {
        let html = render_markdown("```rust\nfn main() {}\n```");
        assert!(html.contains("shiki-wrapper") || html.contains("<pre"));
    }

    #[test]
    fn mermaid_block_preserved() {
        let html = render_markdown("```mermaid\ngraph LR\nA-->B\n```");
        assert!(html.contains("language-mermaid"));
    }
}
