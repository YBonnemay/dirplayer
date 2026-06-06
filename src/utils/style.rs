use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

pub fn string_to_styled_spans<'a>(raw_string: &'a str, indices: &[usize]) -> Vec<Span<'a>> {
    let bold_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![];
    let mut next_match = 0;

    for (i, c) in raw_string.chars().enumerate() {
        if next_match < indices.len() && i == indices[next_match] {
            next_match += 1;
            spans.push(Span::styled(c.to_string(), bold_style));
        } else {
            spans.push(Span::raw(c.to_string()));
        }
    }

    spans
}

// /// Return the string in argument styled at indices.
// /// Return empty string when out of bound indices found.
// /// Return a word a spans
// pub fn string_to_styled_spans(raw_string: &Path, mut indices: Vec<usize>) -> Vec<Span<'static>> {
//     let bold_style = Style::default()
//         .fg(Color::Green)
//         .add_modifier(Modifier::BOLD);

//     let raw_string = raw_string.to_string_lossy();
//     let raw_string = raw_string.as_ref();
//     let mut styled_string: Vec<Span> = vec![];

//     if raw_string.is_empty() {
//         return styled_string;
//     }

//     let mut start_index = 0_usize;
//     let raw_string_len = raw_string.len();
//     let max_index = raw_string_len - 1;

//     indices.sort();

//     // Add termination if not present
//     // Return if out of bound
//     match indices.last() {
//         Some(last) => match (*last).cmp(&max_index) {
//             Ordering::Greater => {
//                 return vec![Span::from("".to_string())];
//             }
//             Ordering::Less => {
//                 indices.push(max_index + 1);
//             }
//             _ => {}
//         },
//         None => {
//             indices.push(max_index + 1);
//         }
//     }

//     for indice in indices.iter() {
//         styled_string.push(Span::from(String::from(&raw_string[start_index..*indice])));
//         if indice < &raw_string_len {
//             styled_string.push(Span::styled(
//                 String::from(&raw_string[*indice..*indice + 1]),
//                 bold_style,
//             ));
//         }
//         start_index = indice + 1;
//     }

//     styled_string
// }
