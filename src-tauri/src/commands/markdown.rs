use crate::markdown_parser::MarkdownParser;

#[tauri::command]
pub fn parse_markdown(content: String) -> String {
    MarkdownParser::parse(&content)
}
