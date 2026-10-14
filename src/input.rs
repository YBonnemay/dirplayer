use crate::AppState;
use crate::EventEnum;
use crate::Focus;
use cocotte::sub_app::SubApp;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use std::collections::VecDeque;
use std::path::PathBuf;
use sublime_fuzzy::Scoring;

pub struct Input {
    pub completions: VecDeque<PathBuf>, // The vector of completions relative to the cwd
    pub scoring: Scoring,
    pub rotate_idx: i32,
    pub directory: Option<PathBuf>,
    pub line_index: i32,
}

impl Input {
    pub fn new() -> Input {
        let scoring = Scoring {
            bonus_consecutive: 64,
            bonus_word_start: 1,
            bonus_match_case: 8,
            penalty_distance: 16,
        };

        Input {
            completions: VecDeque::default(),
            scoring,
            rotate_idx: 0,
            directory: None,
            line_index: 0,
        }
    }
}

impl SubApp<EventEnum, AppState> for Input {
    fn constraints(&self) -> Constraint {
        Constraint::Length(3)
    }

    fn handle_input(&mut self, _event: &mut EventEnum, _app_state: &mut AppState) {}

    fn render(&self, frame: &mut Frame, area: Rect, app_state: &mut AppState) {
        let title = "Working directory".to_string();

        let cwd = app_state.current_directory.to_string_lossy().to_string();

        let filter = match app_state.focus {
            Focus::Directories => &app_state.directory_filter,
            Focus::Files => &app_state.file_filter,
        };

        let displayed = if filter.is_empty() {
            vec![Span::raw(&cwd)]
        } else {
            vec![Span::raw(&cwd), Span::raw("  |  "), Span::raw(filter)]
        };

        let block = Block::default().borders(Borders::ALL).title(title);
        let input = Paragraph::new(Line::from(displayed)).block(block);
        frame.render_widget(input, area);
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
