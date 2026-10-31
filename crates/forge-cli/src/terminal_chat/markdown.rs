use super::display;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(super) const ORANGE: Color = Color::Rgb(255, 154, 82);
pub(super) const TEAL: Color = Color::Rgb(126, 204, 181);
fn flush(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}
pub(super) fn render(text: &str) -> Vec<Line<'static>> {
    let clean = display(text);
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut style = Style::default();
    let mut styles = Vec::new();
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut code = false;
    let mut destinations = Vec::<Option<String>>::new();
    for event in Parser::new_ext(
        &clean,
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    ) {
        match event {
            Event::Start(tag) => {
                styles.push(style);
                match tag {
                    Tag::Heading { .. } => {
                        flush(&mut lines, &mut spans);
                        style = style.fg(ORANGE).add_modifier(Modifier::BOLD);
                    }
                    Tag::Strong => style = style.add_modifier(Modifier::BOLD),
                    Tag::Emphasis => style = style.add_modifier(Modifier::ITALIC),
                    Tag::Strikethrough => style = style.add_modifier(Modifier::CROSSED_OUT),
                    Tag::CodeBlock(kind) => {
                        flush(&mut lines, &mut spans);
                        code = true;
                        style = style.fg(TEAL).bg(Color::Rgb(19, 29, 30));
                        let language = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(v) => v.to_string(),
                            _ => String::new(),
                        };
                        lines.push(Line::styled(
                            format!(
                                "  Código{}",
                                if language.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · {language}")
                                }
                            ),
                            Style::default().fg(TEAL),
                        ));
                    }
                    Tag::List(start) => lists.push(start),
                    Tag::Item => {
                        flush(&mut lines, &mut spans);
                        let prefix = match lists.last_mut() {
                            Some(Some(n)) => {
                                let value = format!("{n}. ");
                                *n = n.saturating_add(1);
                                value
                            }
                            _ => "• ".into(),
                        };
                        spans.push(Span::raw(format!(
                            "{}{}",
                            "  ".repeat(lists.len().saturating_sub(1)),
                            prefix
                        )));
                    }
                    Tag::BlockQuote(_) => spans.push(Span::styled("│ ", Style::default().fg(TEAL))),
                    Tag::Link { dest_url, .. } => {
                        style = style.fg(TEAL).add_modifier(Modifier::UNDERLINED);
                        destinations.push(
                            (dest_url.starts_with("https://") || dest_url.starts_with("http://"))
                                .then(|| dest_url.to_string()),
                        );
                    }
                    Tag::Image { .. } => {
                        spans.push(Span::styled("Imagem: ", Style::default().fg(TEAL)))
                    }
                    Tag::TableHead | Tag::TableRow => flush(&mut lines, &mut spans),
                    _ => {}
                }
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Paragraph | TagEnd::Heading(_) => {
                        flush(&mut lines, &mut spans);
                        if lists.is_empty() {
                            lines.push(Line::default());
                        }
                    }
                    TagEnd::Item => flush(&mut lines, &mut spans),
                    TagEnd::List(_) => {
                        lists.pop();
                        if lists.is_empty() {
                            lines.push(Line::default());
                        }
                    }
                    TagEnd::CodeBlock => {
                        flush(&mut lines, &mut spans);
                        code = false;
                        lines.push(Line::default());
                    }
                    TagEnd::Link => {
                        if let Some(Some(url)) = destinations.pop() {
                            spans
                                .push(Span::styled(format!(" ({url})"), Style::default().fg(TEAL)));
                        }
                    }
                    TagEnd::TableCell => spans.push(Span::raw(" │ ")),
                    TagEnd::TableHead | TagEnd::TableRow | TagEnd::Table => {
                        flush(&mut lines, &mut spans)
                    }
                    _ => {}
                }
                style = styles.pop().unwrap_or_default();
            }
            Event::Text(text) => {
                let parts = text.split('\n').collect::<Vec<_>>();
                for (i, part) in parts.iter().enumerate() {
                    if code && spans.is_empty() {
                        spans.push(Span::raw("  "));
                    }
                    spans.push(Span::styled((*part).to_owned(), style));
                    if i + 1 < parts.len() {
                        flush(&mut lines, &mut spans);
                    }
                }
            }
            Event::Code(text) => spans.push(Span::styled(
                text.into_string(),
                style.fg(TEAL).bg(Color::Rgb(19, 29, 30)),
            )),
            Event::SoftBreak if !code => spans.push(Span::raw(" ")),
            Event::SoftBreak | Event::HardBreak => flush(&mut lines, &mut spans),
            Event::Rule => {
                flush(&mut lines, &mut spans);
                lines.push(Line::styled(
                    "────────────────────────",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Event::TaskListMarker(checked) => {
                spans.push(Span::raw(if checked { "[x] " } else { "[ ] " }))
            }
            // HTML is never a terminal renderer instruction. Images never make network requests.
            Event::Html(_) | Event::InlineHtml(_) => {}
            _ => {}
        }
    }
    flush(&mut lines, &mut spans);
    while lines.last().is_some_and(|line| line.spans.is_empty()) {
        lines.pop();
    }
    lines
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_markdown_keeps_code_lists_and_sanitizes_control_sequences() {
        let lines = render(
            "## Título\n\nUma **resposta** com `código`.\n\n- passo\n\n```rust\nfn main() {}\n```\n\n<script>bad()</script>\n\n![alt](https://track.invalid/x)\n\n\x1b]52;copiar\x07",
        );
        let text = lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Título"));
        assert!(!text.contains("##"));
        assert!(!text.contains("**"));
        assert!(text.contains("fn main() {}"));
        assert!(text.contains("• passo"));
        assert!(!text.contains("script"));
        assert!(!text.contains("track.invalid"));
        assert!(!text.contains('\x1b'));
        assert!(!text.contains('\x07'));
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD))
        );
    }
}
