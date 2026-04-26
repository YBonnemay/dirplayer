use crate::app::Chord;
use crate::utils;
use crate::utils::completion::get_path_completions;
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
use ratatui::layout::Constraint;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use sublime_fuzzy::{FuzzySearch, Scoring};

pub struct Input {
    pub scoring: Scoring,
    pub completions: Vec<PathBuf>,
    pub displayable_completions: Vec<Vec<Span<'static>>>,
    pub filter: String,
    pub input: String,
    pub rotate_idx: i32,
    pub directory: PathBuf,
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

        let mut directory = config.working_directory;

        if directory.as_os_str().is_empty() {
            directory = config::get_audio_dir();
        }

        let directory_path = PathBuf::from(&directory);

        Input {
            completions: get_path_completions(&directory_path),
            displayable_completions: vec![],
            filter: String::from(""),
            input: String::from(""),
            scoring,
            rotate_idx: 0,
            directory,
            line_index: 0,
        }
    }

    pub fn get_displayable_completions(&self) -> Vec<Vec<Span<'static>>> {
        let completions = &self.completions;
        let filter = &self.filter;

        let mut displayable_completions = completions
            .iter()
            .cloned()
            .filter_map(|completion| {
                if filter.is_empty() {
                    return Some(vec![Span::from(completion.to_string_lossy().into_owned())]);
                }

                let completion_str = completion.to_str()?;
                let matched = FuzzySearch::new(&self.filter, completion_str)
                    .case_insensitive()
                    .score_with(&self.scoring)
                    .best_match()?;

                if matched.score() <= 0 {
                    return None;
                }

                let indices = matched.matched_indices().copied().collect_vec();
                Some(utils::style::string_to_styled_spans(&completion, indices))
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
                            //
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
                            self.directory = new_directory;
                            utils::config::update_working_directory(&self.directory);

                            // Cancel current filter
                            self.filter = String::from("");

                            log::debug!(
                                "KeyCode::Right get_path_completions {}",
                                &self.directory.display()
                            );

                            // refresh completions here
                            self.completions = get_path_completions(&self.directory);

                            log::debug!("KeyCode::Right end {:?}", self.completions);
                        }
                        (_, KeyCode::Backspace) | (_, KeyCode::Left) => {
                            if self.filter.is_empty() {
                                let mut path = PathBuf::from(&self.directory);
                                path.pop();
                                self.directory = path;
                                utils::config::update_working_directory(&self.directory);
                                self.completions = get_path_completions(&self.directory);
                            } else {
                                let mut chars = self.filter.chars();
                                chars.next_back();
                                self.filter = chars.as_str().to_string();
                            }
                        }
                        (_, KeyCode::Enter) => {}
                        (_, KeyCode::Char(c)) => {
                            self.filter = format!("{}{}", self.filter, c);
                        }
                        _ => {}
                    }
                }
            },
        }
    }

    // fn render(&self, frame: &mut Frame, area: Rect, title: &str) {
    fn render(&self, frame: &mut Frame, area: Rect, title: &str) {
        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(input, area);
    }

    fn constraints(&self) -> Constraint {
        Constraint::Length(3)
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
