use crate::app::get_path_completions;
use crate::app::Chord;
use crate::utils;
use crate::utils::config;
use crate::utils::config::Config;
use crate::utils::config::Status;
use crate::utils::config::WorkingPath;
use crate::AppState;
use crate::EventEnum;
use crate::Focus;
use cocotte::sub_app::SubApp;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use itertools::Itertools;
use ratatui::text::{Line, Span};
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use sublime_fuzzy::{FuzzySearch, Scoring};

pub struct Input {
    pub scoring: Scoring,
    pub completions: Vec<String>,
    pub displayable_completions: Vec<Vec<String>>,
    pub filter: String,
    pub input: String,
    pub rotate_idx: i32,
    pub directory: String,
    pub line_index: i32,
}

impl Input {
    pub fn new() -> Input {
        let config = utils::config::get_config();
        let scoring = Scoring {
            bonus_consecutive: 64,
            bonus_word_start: 1,
            bonus_match_case: 8,
            penalty_distance: 16,
        };
        let mut working_directory = config.working_directory.clone();
        if working_directory.is_empty() {
            working_directory = String::from(config::get_audio_dir().to_string_lossy());
        }

        let working_directories = config.working_directories;

        Input {
            completions: get_path_completions(&working_directory),
            displayable_completions: vec![],
            filter: String::from(""),
            input: String::from(""),
            scoring,
            rotate_idx: 0,
            directory: working_directory,
            working_directories,
            line_index: 0,
        }
    }

    pub fn get_displayable_completions(&self) -> Vec<Vec<string>> {
        let completions = VecDeque::from(self.completions.clone());

        let filter = self.filter.clone();

        let mut displayable_completions = completions
            .iter()
            .cloned()
            .filter_map(|completion| {
                if filter.is_empty() {
                    return Some(vec![completion]);
                }
                if let Some(matched) = FuzzySearch::new(&self.filter, &completion)
                    .case_insensitive()
                    .score_with(&self.scoring)
                    .best_match()
                {
                    let score = matched.score();
                    if score <= 0 {
                        return None;
                    }

                    let indices = matched.matched_indices().copied().collect_vec();
                    let added_spans = utils::style::string_to_styled_spans(completion, indices);
                    Some(added_spans)
                } else {
                    None
                }
            })
            .collect_vec();

        if !displayable_completions.is_empty() {
            let rotate = self
                .rotate_idx
                .rem_euclid(displayable_completions.len() as i32);
            displayable_completions.rotate_right(rotate as usize);
        }

        let pipes = vec![
            vec![Span::from(String::from(" | "))];
            displayable_completions.len().saturating_sub(1)
        ];

        displayable_completions = displayable_completions
            .into_iter()
            .interleave(pipes)
            .collect_vec();

        displayable_completions
    }

    fn update_working_directories(&mut self) {
        let candidate = String::from(&self.directory);
        log::debug!("update_working_directories");
        if self
            .working_directories
            .iter()
            .any(|WorkingPath { path, .. }| path.eq(&candidate) || candidate.starts_with(path))
        {
            return;
        }
        log::debug!("update_working_directories after");

        self.working_directories.push_back(WorkingPath {
            path: candidate,
            status: Status::Active,
        });

        //
        let config = utils::config::get_config();
        utils::config::update_config(&Config {
            working_directories: self.working_directories.clone(),
            ..config
        });
        self.line_index += 1;
    }
}

impl SubApp<EventEnum, AppState> for Input {
    fn handle_input(&mut self, event: &mut EventEnum, app_state: &mut AppState) {
        match app_state.focus {
            Focus::Input => match event {
                EventEnum::Tick => return,
                EventEnum::Key(key_event) => {
                    match (key_event.modifiers, key_event.code) {
                        // (_, KeyCode::Down) => {}
                        (_, KeyCode::Up) => {
                            app_state.focus = Focus::Input;
                        }
                        (KeyModifiers::NONE, KeyCode::Right) => self.rotate_idx -= 1,
                        (KeyModifiers::NONE, KeyCode::Left) => {
                            self.rotate_idx += 1;
                            let displayable_completions = self.get_displayable_completions();
                            self.displayable_completions = displayable_completions;
                            // self.displayable_completions = VecDeque::from(vec![displayable_completions]);
                        }
                        (_, KeyCode::Tab) => {
                            log::debug!("KeyCode::Tab start {:?}", self.completions);

                            // Go into directory
                            let displayable_completions = self.get_displayable_completions();

                            log::debug!(
                                "KeyCode::Tab displayable_completions {:?}",
                                displayable_completions
                            );

                            // if displayable_completions.spans.len() == 0 {
                            if displayable_completions.is_empty() {
                                return;
                            }

                            // Update path
                            let new_directory = Path::new(&self.directory)
                                .join(Line::from(displayable_completions[0].clone()).to_string());
                            self.directory = String::from(new_directory.to_string_lossy());
                            utils::config::update_working_directory(&self.directory);

                            // Cancel current filter
                            self.filter = String::from("");

                            log::debug!("KeyCode::Right get_path_completions {}", &self.directory);

                            // refresh completions here
                            self.completions = get_path_completions(&self.directory);

                            log::debug!("KeyCode::Right end {:?}", self.completions);
                        }
                        (_, KeyCode::Backspace) | (_, KeyCode::Left) => {
                            if self.filter.is_empty() {
                                let mut path = PathBuf::from(&self.directory);
                                path.pop();
                                self.directory = String::from(path.to_string_lossy());
                                utils::config::update_working_directory(&self.directory);
                                self.completions = get_path_completions(&self.directory);
                            } else {
                                let mut chars = self.filter.chars();
                                chars.next_back();
                                self.filter = chars.as_str().to_string();
                            }
                        }
                        (_, KeyCode::Enter) => {
                            self.update_working_directories();
                        }
                        (_, KeyCode::Char(c)) => {
                            self.filter = format!("{}{}", self.filter, c);
                        }
                        _ => {}
                    }
                }
            },
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, title: &str) {
        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(input, area);
    }

    fn constraints(&self) -> ratatui::layout::Constraint {
        ratatui::layout::Constraint::Length(3)
    }
}

impl<'a> Default for Input<'a> {
    fn default() -> Self {
        Self::new()
    }
}
