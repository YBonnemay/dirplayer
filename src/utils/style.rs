use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
};

pub fn string_to_styled_text(raw_string: &str, indices: &[usize]) -> Spans<'static> {
    let bold_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![];
    let mut indices = indices.to_owned();

    for (i, c) in raw_string.chars().enumerate() {
        if !indices.is_empty() && i == indices[0] {
            indices.drain(0..1);
            spans.push(Span::styled(String::from(c), bold_style));
        } else {
            spans.push(Span::raw(String::from(c)));
        }
    }

    Spans::from(spans)
}
