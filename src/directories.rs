use crate::AppState;
use crate::EventEnum;
use crate::app::Chord;
use crate::utils;
use crate::utils::completion::get_path_completions;
use crate::utils::config::Config;
use crate::utils::config::Status;
use crate::utils::config::WorkingPath;
use cocotte::sub_app::SubApp;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use itertools::Itertools;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::TableState;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use sublime_fuzzy::{FuzzySearch, Scoring};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum State {
    Base,
    Editing,
}

pub struct Directories {
    pub scoring: Scoring,
    pub completions: Vec<PathBuf>,
    pub displayable_completions: Vec<Vec<Span<'static>>>,
    pub filter: String,
    pub rotate_idx: i32,
    pub working_directory: PathBuf,
    pub line_index: i32,
    pub state: State,
}

impl Directories {
    pub fn new() -> Directories {
        let config = utils::config::get_set_config();
        let scoring = Scoring {
            bonus_consecutive: 64,
            bonus_word_start: 1,
            bonus_match_case: 8,
            penalty_distance: 16,
        };
        let working_directory = config.working_directory;
        Directories {
            completions: get_path_completions(&working_directory),
            displayable_completions: vec![],
            filter: String::from(""),
            scoring,
            rotate_idx: 0,
            working_directory,
            line_index: 0,
            state: State::Base,
        }
    }

    pub fn get_displayable_completions(&self) -> Vec<Vec<Span>> {
        vec![vec![]]
        // let completions = VecDeque::from(self.completions.clone());

        // let filter = self.filter.clone();

        // let mut displayable_completions = completions
        //     .iter()
        //     .cloned()
        //     .filter_map(|completion| {
        //         if filter.is_empty() {
        //             return Some(vec![Span::from(completion.to_string_lossy().into_owned())]);
        //         }
        //         if let Some(matched) = FuzzySearch::new(&self.filter, &completion.to_string_lossy())
        //             .case_insensitive()
        //             .score_with(&self.scoring)
        //             .best_match()
        //         {
        //             let score = matched.score();
        //             if score <= 0 {
        //                 return None;
        //             }

        //             let indices = matched.matched_indices().copied().collect_vec();
        //             if let Some(completion_str) = completion.as_os_str().to_str() {
        //                 let added_spans =
        //                     utils::style::string_to_styled_spans(completion_str, &indices);
        //                 Some(added_spans)
        //             } else {
        //                 None
        //             }
        //         } else {
        //             None
        //         }
        //     })
        //     .collect_vec();

        // if !displayable_completions.is_empty() {
        //     let rotate = self
        //         .rotate_idx
        //         .rem_euclid(displayable_completions.len() as i32);
        //     displayable_completions.rotate_right(rotate as usize);
        // }

        // let pipes = vec![
        //     vec![Span::from(String::from(" | "))];
        //     displayable_completions.len().saturating_sub(1)
        // ];

        // displayable_completions = displayable_completions
        //     .into_iter()
        //     .interleave(pipes)
        //     .collect_vec();

        // displayable_completions
    }

    pub fn handle_event(&mut self, chord: Chord) {
        match self.state {
            State::Editing => match chord {
                // (_, KeyCode::Down) => {}
                (_, KeyCode::Up) => {
                    self.state = State::Base;
                    if self.line_index >= 0 {
                        self.line_index -= 1;
                    }
                }
                (KeyModifiers::NONE, KeyCode::Right) => self.rotate_idx -= 1,
                (KeyModifiers::NONE, KeyCode::Left) => {
                    self.rotate_idx += 1;
                    // let displayable_completions = self.get_displayable_completions();
                    // self.displayable_completions = displayable_completions;
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
                    let new_directory = Path::new(&self.working_directory)
                        .join(Line::from(displayable_completions[0].clone()).to_string());

                    self.working_directory = new_directory;
                    utils::config::update_working_directory(&self.working_directory);

                    // Cancel current filter
                    self.filter = String::from("");

                    log::debug!(
                        "KeyCode::Right get_path_completions {}",
                        &self.working_directory.to_string_lossy()
                    );

                    // refresh completions here
                    self.completions = get_path_completions(&self.working_directory);

                    log::debug!("KeyCode::Right end {:?}", self.completions);
                }
                (_, KeyCode::Backspace) | (_, KeyCode::Left) => {
                    if self.filter.is_empty() {
                        let mut path = PathBuf::from(&self.working_directory);
                        path.pop();
                        self.working_directory = path;
                        utils::config::update_working_directory(&self.working_directory);
                        self.completions = get_path_completions(&self.working_directory);
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
            },
            State::Base => match chord {
                (_, KeyCode::Down) => {}
                (_, KeyCode::Up) => {
                    if self.line_index >= 0 {
                        self.line_index -= 1;
                    }
                }
                (_, KeyCode::Char(c)) => {}
                _ => {}
            },
        }
    }
}

impl SubApp<EventEnum, AppState> for Directories {
    fn handle_input(&mut self, _event: &mut EventEnum, _app_state: &mut AppState) {}

    fn render(&self, _frame: &mut Frame, _area: Rect, _title: &str) {
        //
    }

    fn constraints(&self) -> Constraint {
        Constraint::Fill(1)
    }
}

impl Default for Directories {
    fn default() -> Self {
        Self::new()
    }
}
