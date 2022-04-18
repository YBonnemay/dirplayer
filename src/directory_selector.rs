use crate::app::get_path_completions;
use crate::app::App;
use crate::utils;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use tui::layout::Constraint;
use tui::layout::Rect;
use tui::text::{Span, Spans};
use tui::widgets::Tabs;
use tui::Frame;

pub struct DirectorySelector<'a> {
    pub matcher: fuzzy_matcher::skim::SkimMatcherV2,
    pub completions: Vec<String>,
    pub displayable_completions: VecDeque<Spans<'a>>,
    pub filter: String,
    pub rotate_idx: i32,
    pub rotate_history_idx: i32,
    pub constraints: Constraint,
}

impl<'a> DirectorySelector<'a> {
    pub fn new() -> DirectorySelector<'a> {
        let config = utils::config::get_config();
        DirectorySelector {
            completions: get_path_completions(&PathBuf::from(
                config.working_directories[0].clone(),
            )),
            displayable_completions: VecDeque::from(vec![Spans::from(vec![Span::raw(
                String::from(""),
            )])]),
            filter: String::from(""),
            matcher: SkimMatcherV2::default(),
            rotate_idx: 0,
            rotate_history_idx: 0,
            constraints: Constraint::Length(1),
        }
    }

    pub fn get_displayable_completions(&mut self) -> VecDeque<Spans<'a>> {
        let texts: VecDeque<Spans> = VecDeque::new();

        let mut displayable_completions: VecDeque<Spans> =
            self.completions.iter().cloned().fold(texts, |mut acc, e| {
                if self.filter.is_empty() {
                    let spans = utils::style::string_to_styled_text(&e, &[]);
                    acc.push_back(spans);
                    return acc;
                }

                if let Some((score, indices)) = self.matcher.fuzzy_indices(&e, &self.filter) {
                    if score > 0 {
                        let spans = utils::style::string_to_styled_text(&e, &indices);
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

    pub fn get_displayable(&mut self, path: &Path) -> Tabs {
        let mut displayable_completions = self.get_displayable_completions();

        displayable_completions.push_front(Spans::from(vec![Span::raw(String::from(
            path.to_string_lossy(),
        ))]));
        Tabs::new(Vec::from(displayable_completions))
    }

    pub fn process_event(&mut self, path: &mut PathBuf, key_code: KeyCode, _: KeyModifiers) {
        match key_code {
            KeyCode::Down => {
                self.rotate_history_idx += 1;
                let config = utils::config::get_config();
                let mut working_directories = config.working_directories;
                working_directories.rotate_right(
                    (self.rotate_history_idx % working_directories.len() as i32) as usize,
                );
                *path = PathBuf::from(working_directories.front().unwrap());
                self.completions = get_path_completions(path);
            }
            KeyCode::Up => {
                if self.rotate_history_idx > 0 {
                    self.rotate_history_idx -= 1;
                }
                let config = utils::config::get_config();
                let mut working_directories = config.working_directories;
                working_directories.rotate_right(
                    (self.rotate_history_idx % working_directories.len() as i32) as usize,
                );
                *path = PathBuf::from(working_directories.front().unwrap());
                self.completions = get_path_completions(path);
            }
            KeyCode::Left => {
                self.rotate_idx += 1;
                let displayable_completions = self.get_displayable_completions();
                self.displayable_completions = displayable_completions;
            }
            KeyCode::Right => {
                self.rotate_idx -= 1;
            }
            KeyCode::Backspace => {
                if self.filter.is_empty() {
                    path.pop();
                    self.completions = get_path_completions(path);
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
                path.push(String::from(current_completion));

                // Cancel current filter
                self.filter = String::from("");

                // refresh completions here
                self.completions = get_path_completions(path);
            }
            KeyCode::Char(c) => {
                self.filter = format!("{}{}", self.filter, c);
            }
            _ => {}
        }
    }

    pub fn update_selector(&mut self, path: &Path) {
        self.completions = get_path_completions(path);
    }

    pub fn draw_directory<B: tui::backend::Backend>(
        &mut self,
        f: &mut Frame<B>,
        chunk: Rect,
        path: &Path,
    ) {
        let displayable_directories = self.get_displayable(path);
        f.render_widget(displayable_directories, chunk);
    }
}
