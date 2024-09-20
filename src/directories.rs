use crate::app::get_path_completions;
use crate::app::Chord;
use crate::utils;
use crate::utils::config;
use crate::utils::config::Config;
use crate::utils::config::Status;
use crate::utils::config::WorkingPath;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use itertools::Itertools;
use ratatui::text::{Line, Span};
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use sublime_fuzzy::{FuzzySearch, Scoring};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum State {
    Base,
    Editing,
}

pub struct Directories<'a> {
    pub scoring: Scoring,
    pub completions: Vec<String>,
    pub displayable_completions: Vec<Vec<Span<'a>>>,
    pub filter: String,
    pub rotate_idx: i32,
    pub working_directory: String,
    pub working_directories: VecDeque<WorkingPath>,
    pub line_index: i32,
    pub state: State,
}

impl<'a> Directories<'a> {
    pub fn new() -> Directories<'a> {
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

        Directories {
            completions: get_path_completions(&working_directory),
            displayable_completions: vec![],
            filter: String::from(""),
            scoring,
            rotate_idx: 0,
            working_directory,
            working_directories,
            line_index: 0,
            state: State::Base,
        }
    }

    pub fn get_displayable_completions(&self) -> Vec<Vec<Span<'a>>> {
        let completions = VecDeque::from(self.completions.clone());

        let filter = self.filter.clone();

        let mut displayable_completions = completions
            .iter()
            .cloned()
            .filter_map(|completion| {
                if filter.is_empty() {
                    return Some(vec![Span::from(completion)]);
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
        let candidate = String::from(&self.working_directory);
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

    pub fn handle_event(&mut self, chord: Chord) {
        let edit_index = self.working_directories.len() - 1;
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
                    let new_directory = Path::new(&self.working_directory)
                        .join(Line::from(displayable_completions[0].clone()).to_string());
                    self.working_directory = String::from(new_directory.to_string_lossy());
                    utils::config::update_working_directory(&self.working_directory);

                    // Cancel current filter
                    self.filter = String::from("");

                    log::debug!(
                        "KeyCode::Right get_path_completions {}",
                        &self.working_directory
                    );

                    // refresh completions here
                    self.completions = get_path_completions(&self.working_directory);

                    log::debug!("KeyCode::Right end {:?}", self.completions);
                }
                (_, KeyCode::Backspace) | (_, KeyCode::Left) => {
                    if self.filter.is_empty() {
                        let mut path = PathBuf::from(&self.working_directory);
                        path.pop();
                        self.working_directory = String::from(path.to_string_lossy());
                        utils::config::update_working_directory(&self.working_directory);
                        self.completions = get_path_completions(&self.working_directory);
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
            },
            State::Base => match chord {
                (_, KeyCode::Down) => {
                    if self.line_index as usize >= edit_index {
                        self.state = State::Editing;
                    }

                    if (self.line_index as usize) < edit_index + 1 {
                        self.line_index += 1;
                    }
                }
                (_, KeyCode::Up) => {
                    if self.line_index >= 0 {
                        self.line_index -= 1;
                    }
                }
                (_, KeyCode::Char(c)) => {
                    if c == 'k' {
                        self.working_directories.remove(self.line_index as usize);
                    }
                }
                _ => {}
            },
        }
    }
}

impl<'a> Default for Directories<'a> {
    fn default() -> Self {
        Self::new()
    }
}
