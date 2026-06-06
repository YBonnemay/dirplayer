use crate::AppState;
use crate::EventEnum;
use crate::Focus;
use crate::utils;
use crate::utils::completion::get_path_completions;
use crate::utils::config;
use crate::utils::config::update_working_directory;
use crate::utils::style::string_to_styled_spans;
use cocotte::sub_app::SubApp;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::event::{Event, EventStream, KeyEvent};
use itertools::Itertools;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use std::path::Path;
use std::path::PathBuf;
use sublime_fuzzy::{FuzzySearch, Scoring};

pub struct Input {
    pub completions: Vec<PathBuf>, // The vector of completions
    pub completions_displayable_idx: Vec<(usize, Vec<usize>)>, // The vector of indices for displayable completions, and matched letters
    pub completions_displayable: Vec<Vec<Span<'static>>>,
    pub scoring: Scoring,
    pub filter: String,
    pub input: String,
    pub rotate_idx: i32,
    pub directory: Option<PathBuf>,
    pub line_index: i32,
}

pub fn get_search_idx(completion: &PathBuf, filter: &String) -> Option<Vec<usize>> {
    let completion_str = completion.to_str()?;
    let matched = FuzzySearch::new(filter, completion_str)
        .case_insensitive()
        .best_match()?;
    let indices = matched.matched_indices().copied().collect_vec();
    (matched.score() > 0).then_some(indices)
}

pub fn get_filtered_completions<'a>(
    completions: &'a Vec<PathBuf>,
    filter: &String,
) -> Vec<Vec<Span<'a>>> {
    completions
        .iter()
        .filter_map(|completion| {
            get_search_idx(completion, filter).map(|search| (completion, search))
        })
        .filter_map(|(completion, search_idx)| {
            completion
                .to_str()
                .map(|completion| string_to_styled_spans(completion, &search_idx))
        })
        .collect_vec()
}

// if filter.is_empty() {
//     (0..completions.len()).map(|i| (i, vec![])).collect()
// } else {
//     completions
//         .iter()
//         .enumerate()
//         .filter_map(|(idx, completion)| {
//             let completion_str = completion.to_str()?;
//             let matched = FuzzySearch::new(filter, completion_str)
//                 .case_insensitive()
//                 .best_match()?;
//             let indices = matched.matched_indices().copied().collect_vec();
//             (matched.score() > 0).then_some((idx, indices))
//         })
//         .collect_vec()
// }

impl Input {
    pub fn new() -> Input {
        let scoring = Scoring {
            bonus_consecutive: 64,
            bonus_word_start: 1,
            bonus_match_case: 8,
            penalty_distance: 16,
        };

        Input {
            completions: vec![],
            completions_displayable_idx: vec![],
            completions_displayable: vec![],
            filter: String::from(""),
            input: String::from(""),
            scoring,
            rotate_idx: 0,
            directory: None,
            line_index: 0,
        }
    }

    pub fn get_completions(&self) -> Vec<Vec<Span<'_>>> {
        let completions = &self.completions;
        let filter = &self.filter;

        let filtered_completions = get_filtered_completions(&completions, filter);

        // if !displayable_completions.is_empty() {
        //     let rotate = self
        //         .rotate_idx
        //         .rem_euclid(displayable_completions.len() as i32);
        //     displayable_completions.rotate_right(rotate as usize);
        // }

        let pipes = vec![
            vec![Span::from(String::from(" | "))];
            filtered_completions.len().saturating_sub(1)
        ];

        filtered_completions
            .into_iter()
            .interleave(pipes)
            .collect_vec()
    }

    // pub fn get_filtered_completions(&self) -> Vec<Vec<Span<'static>>> {
    //     let completions = &self.completions;
    //     let filter = &self.filter;

    //     let mut displayable_completions = completions
    //         .iter()
    //         .filter_map(|completion| {
    //             if filter.is_empty() {
    //                 return Some(vec![Span::from(completion.to_string_lossy().into_owned())]);
    //             }

    //             let completion_str = completion.to_str()?;
    //             let matched = FuzzySearch::new(&self.filter, completion_str)
    //                 .case_insensitive()
    //                 .score_with(&self.scoring)
    //                 .best_match()?;

    //             if matched.score() <= 0 {
    //                 return None;
    //             }

    //             let indices = matched.matched_indices().copied().collect_vec();
    //             Some(utils::style::string_to_styled_spans(&completion, indices))
    //         })
    //         .collect_vec();

    //     if !displayable_completions.is_empty() {
    //         let rotate = self
    //             .rotate_idx
    //             .rem_euclid(displayable_completions.len() as i32);
    //         displayable_completions.rotate_right(rotate as usize);
    //     }

    //     let pipes = vec![
    //         vec![Span::from(String::from(" | "))];
    //         displayable_completions.len().saturating_sub(1)
    //     ];

    //     displayable_completions = displayable_completions
    //         .into_iter()
    //         .interleave(pipes)
    //         .collect_vec();

    //     displayable_completions
    // }
}

impl SubApp<EventEnum, AppState> for Input {
    fn handle_input(&mut self, event: &mut EventEnum, app_state: &mut AppState) {
        log::error!("handle_input");

        match event {
            EventEnum::Setup => {
                self.completions = get_path_completions(&app_state.current_directory);
                log::error!("{:?}", self.completions);
                // self.completions_displayable_idx =
                // get_filtered_completions_idx(&self.completions, &self.filter);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }) => {
                self.input.push(*c);
                let filtered_completions = get_filtered_completions(&self.completions, &self.input);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => {
                self.input.pop();
            }

            _ => {}
        }
    }

    // fn render(&self, frame: &mut Frame, area: Rect, title: &str) {
    fn render(&self, frame: &mut Frame, area: Rect, title: &str) {
        // let display_string = app.current_directory;
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
