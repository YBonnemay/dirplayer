use crossterm::event::{KeyCode, KeyModifiers};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use tui::layout::Constraint;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::Tabs;

pub struct DirectorySelector<'a> {
    pub matcher: fuzzy_matcher::skim::SkimMatcherV2,
    pub completions: Vec<String>,
    pub displayable_completions: VecDeque<Spans<'a>>,
    pub filter: String,
    pub rotate_idx: i32,
    pub path: PathBuf,
}

// pub fn get_path_completions_recursive(path: &Path) -> Vec<String> {
//     WalkDir::new(path)
//         .into_iter()
//         .filter(|e| e.as_ref().unwrap().metadata().unwrap().is_dir())
//         .map(|e| String::from(e.unwrap().file_name().to_string_lossy()))
//         .collect::<Vec<String>>()
// }

pub fn get_path_completions(path: &Path) -> Vec<String> {
    fs::read_dir(path)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().metadata().unwrap().is_dir())
        .map(|res| String::from(res.unwrap().file_name().to_string_lossy()))
        .collect::<Vec<String>>()
}

fn string_to_styled_text(raw_string: String, mut indices: Vec<usize>) -> Spans<'static> {
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
impl<'a> DirectorySelector<'a> {
    pub fn new(path: PathBuf) -> DirectorySelector<'a> {
        let completions = get_path_completions(&path);
        DirectorySelector {
            completions,
            displayable_completions: VecDeque::from(vec![Spans::from(vec![Span::raw(
                String::from(""),
            )])]),
            filter: String::from(""),
            matcher: SkimMatcherV2::default(),
            rotate_idx: 0,
            path,
        }
    }
}

impl<'a> DirectorySelector<'a> {
    fn get_displayable_completions(&self) -> VecDeque<Spans<'a>> {
        let texts: VecDeque<Spans> = VecDeque::new();

        let mut displayable_completions: VecDeque<Spans> =
            self.completions.iter().cloned().fold(texts, |mut acc, e| {
                if self.filter.is_empty() {
                    let spans = string_to_styled_text(e, vec![]);
                    acc.push_back(spans);
                    return acc;
                }

                if let Some((score, indices)) = self.matcher.fuzzy_indices(&e, &self.filter) {
                    if score > 0 {
                        let spans = string_to_styled_text(e, indices);
                        acc.push_back(spans);
                    }
                }

                acc
            });

        if !displayable_completions.is_empty() {
            let rotate = self
                .rotate_idx
                .rem_euclid(displayable_completions.len() as i32);

            displayable_completions.rotate_right(rotate as usize);
        }

        displayable_completions
    }

    pub fn get_displayable(&self) -> Tabs {
        let mut displayable_completions = self.get_displayable_completions();

        // Add path
        displayable_completions
            .push_front(Spans::from(vec![Span::raw(self.path.to_string_lossy())]));

        Tabs::new(Vec::from(displayable_completions))
    }

    pub fn get_constraints(&self) -> Constraint {
        Constraint::Length(1)
    }

    pub fn process_event(&mut self, key_code: KeyCode, _: KeyModifiers) {
        match key_code {
            KeyCode::Left => {
                self.rotate_idx += 1;
                let _displayable_completions = self.get_displayable_completions();
                self.displayable_completions = _displayable_completions;
                // TODO HERE UPDATE CONPLETION CANDIDATE
            }
            KeyCode::Right => {
                self.rotate_idx -= 1;
            }
            KeyCode::Backspace => {
                if self.filter.is_empty() {
                    self.path.pop();
                    self.completions = get_path_completions(&self.path);
                } else {
                    let mut chars = self.filter.chars();
                    chars.next_back();
                    self.filter = chars.as_str().to_string();
                }
            }
            KeyCode::Enter => {
                let displayable_completions = self.get_displayable_completions();

                if displayable_completions.is_empty() {
                    return;
                }

                let current_completion = displayable_completions[0].clone();

                // Update path
                self.path.push(String::from(current_completion));

                // Cancel current filter
                self.filter = String::from("");

                // refresh completions here
                self.completions = get_path_completions(&self.path);
            }
            KeyCode::Char(c) => {
                self.filter = format!("{}{}", self.filter, c);
            }
            _ => {}
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
        self.completions = get_path_completions(&self.path);
    }
}
