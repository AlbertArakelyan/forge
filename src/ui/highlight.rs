use std::sync::LazyLock;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

pub fn highlight_text(text: &str, lang: &str) -> Text<'static> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension(lang)
        .or_else(|| SYNTAX_SET.find_syntax_by_name(lang))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = match THEME_SET.themes.get("Solarized (dark)") {
        Some(t) => t,
        None => return Text::raw(text.to_string()),
    };

    let mut h = HighlightLines::new(syntax, theme);
    let mut lines: Vec<Line<'static>> = Vec::new();

    for line in LinesWithEndings::from(text) {
        match h.highlight_line(line, &SYNTAX_SET) {
            Ok(ranges) => {
                let spans: Vec<Span<'static>> = ranges
                    .into_iter()
                    .map(|(style, content)| {
                        let fg = style.foreground;
                        Span::styled(
                            content.to_string(),
                            Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b)),
                        )
                    })
                    .collect();
                lines.push(Line::from(spans));
            }
            Err(_) => lines.push(Line::raw(line.to_string())),
        }
    }

    Text::from(lines)
}
