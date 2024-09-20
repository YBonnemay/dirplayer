use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::cmp::Ordering;

pub fn string_to_styled_text(raw_string: &str, indices: &[usize]) -> Line<'static> {
    let bold_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let mut line = vec![];
    let mut indices = indices.to_owned();

    for (i, c) in raw_string.chars().enumerate() {
        if !indices.is_empty() && i == indices[0] {
            indices.drain(0..1);
            line.push(Span::styled(String::from(c), bold_style));
        } else {
            line.push(Span::raw(String::from(c)));
        }
    }

    Line::from(line)
}

/// Return the string in argument styled at indices.
/// Return empty string when out of bound indices found.
/// Return a word a spans
pub fn string_to_styled_spans(raw_string: String, mut indices: Vec<usize>) -> Vec<Span<'static>> {
    let bold_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    let mut styled_string: Vec<Span> = vec![];
    let mut start_index = 0_usize;
    let raw_string_len = raw_string.len();
    let max_index = raw_string_len - 1;

    indices.sort();

    // Add termination if not present
    // Return if out of bound
    match indices.last() {
        Some(last) => match (*last).cmp(&max_index) {
            Ordering::Greater => {
                return vec![Span::from("".to_string())];
            }
            Ordering::Less => {
                indices.push(max_index + 1);
            }
            _ => {}
        },
        None => {
            indices.push(max_index + 1);
        }
    }

    for indice in indices.iter() {
        styled_string.push(Span::from(String::from(&raw_string[start_index..*indice])));
        if indice < &raw_string_len {
            styled_string.push(Span::styled(
                String::from(&raw_string[*indice..*indice + 1]),
                bold_style,
            ));
        }
        start_index = indice + 1;
    }

    styled_string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_to_styled_spans_test() {
        assert_eq!(
            string_to_styled_string(&"aaaaaa".to_string(), &vec![1, 3, 4]),
            "a\u{1b}[33m\u{1b}[0ma\u{1b}[33m\u{1b}[0m\u{1b}[33m\u{1b}[0ma"
        );

        assert_eq!(
            string_to_styled_string(&"bb".to_string(), &vec![1, 3, 4]),
            ""
        );

        assert_eq!(
            string_to_styled_string(&"cccccc".to_string(), &vec![]),
            "cccccc"
        );

        assert_eq!(
            string_to_styled_string(&"dddddd".to_string(), &vec![4, 1, 3]),
            "d\u{1b}[33m\u{1b}[0md\u{1b}[33m\u{1b}[0m\u{1b}[33m\u{1b}[0md"
        );
    }
}
