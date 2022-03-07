use crate::app::get_path_completions;
use crate::app::App;
use crossterm::event::{KeyCode, KeyModifiers};
use fuzzy_matcher::FuzzyMatcher;
use std::collections::VecDeque;
use std::path::Path;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::Tabs;

pub fn string_to_styled_text(raw_string: String, mut indices: Vec<usize>) -> Spans<'static> {
    let bold_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![];

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

pub fn get_displayable_completions<'a>(app: &App) -> VecDeque<Spans<'a>> {
    let texts: VecDeque<Spans> = VecDeque::new();

    let mut displayable_completions: VecDeque<Spans> = app
        .directory_selector
        .completions
        .iter()
        .cloned()
        .fold(texts, |mut acc, e| {
            if app.directory_selector.filter.is_empty() {
                let spans = string_to_styled_text(e, vec![]);
                acc.push_back(spans);
                return acc;
            }

            if let Some((score, indices)) = app
                .directory_selector
                .matcher
                .fuzzy_indices(&e, &app.directory_selector.filter)
            {
                if score > 0 {
                    let spans = string_to_styled_text(e, indices);
                    acc.push_back(spans);
                }
            }

            acc
        });

    if !displayable_completions.is_empty() {
        let rotate = app
            .directory_selector
            .rotate_idx
            .rem_euclid(displayable_completions.len() as i32);

        displayable_completions.rotate_right(rotate as usize);
    }

    displayable_completions
}

pub fn get_displayable<'a>(app: &'a App) -> Tabs<'a> {
    let mut displayable_completions = get_displayable_completions(app);

    // Add path
    displayable_completions.push_front(Spans::from(vec![Span::raw(app.path.to_string_lossy())]));
    Tabs::new(Vec::from(displayable_completions))
}

pub fn process_event(app: &mut App, key_code: KeyCode, _: KeyModifiers) {
    match key_code {
        KeyCode::Left => {
            app.directory_selector.rotate_idx += 1;
            let _displayable_completions = get_displayable_completions(app);
            app.directory_selector.displayable_completions = _displayable_completions;
            // TODO HERE UPDATE CONPLETION CANDIDATE
        }
        KeyCode::Right => {
            app.directory_selector.rotate_idx -= 1;
        }
        KeyCode::Backspace => {
            if app.directory_selector.filter.is_empty() {
                app.path.pop();
                app.directory_selector.completions = get_path_completions(&app.path);
            } else {
                let mut chars = app.directory_selector.filter.chars();
                chars.next_back();
                app.directory_selector.filter = chars.as_str().to_string();
            }
        }
        KeyCode::Enter => {
            let displayable_completions = get_displayable_completions(app);

            if displayable_completions.is_empty() {
                return;
            }

            let current_completion = displayable_completions[0].clone();

            // Update path
            app.path.push(String::from(current_completion));

            // Cancel current filter
            app.directory_selector.filter = String::from("");

            // refresh completions here
            app.directory_selector.completions = get_path_completions(&app.path);
        }
        KeyCode::Char(c) => {
            app.directory_selector.filter = format!("{}{}", app.directory_selector.filter, c);
        }
        _ => {}
    }
}

pub fn update_selector<'a>(app: &'a mut App, path: &Path) {
    app.directory_selector.completions = get_path_completions(path);
}
