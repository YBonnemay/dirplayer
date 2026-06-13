use crate::AppState;
use crate::EventEnum;
use crate::Focus;
use crate::app::Chord;
use crate::utils::style::string_to_styled_spans;
use cocotte::sub_app::SubApp;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListState};
use std::fs;
use std::path::{Path, PathBuf};
use sublime_fuzzy::{FuzzySearch, Scoring};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum State {
    Base,
    Editing,
}

pub struct Directories {
    pub scoring: Scoring,
    pub working_directory: PathBuf,
    pub line_index: usize,
    pub state: State,
    pub directories: Vec<PathBuf>,
    pub filtered_directory_indices: Vec<usize>,
    pub displayed_directories: Vec<Line<'static>>,
    pub files: Vec<PathBuf>,
    pub filtered_file_indices: Vec<usize>,
    pub displayed_files: Vec<Line<'static>>,
}

fn get_entries(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut directories = Vec::new();
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            let path = entry.path();
            if file_type.is_dir() {
                directories.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }

    (directories, files)
}

impl Directories {
    pub fn new() -> Directories {
        let scoring = Scoring {
            bonus_consecutive: 64,
            bonus_word_start: 1,
            bonus_match_case: 8,
            penalty_distance: 16,
        };

        Directories {
            scoring,
            working_directory: PathBuf::default(),
            line_index: 0,
            state: State::Base,
            directories: vec![],
            filtered_directory_indices: vec![],
            displayed_directories: vec![],
            files: vec![],
            filtered_file_indices: vec![],
            displayed_files: vec![],
        }
    }

    fn refresh_entries(&mut self, dir: &Path) {
        self.working_directory = dir.to_path_buf();
        let (directories, files) = get_entries(dir);
        self.directories = directories;
        self.files = files;
    }

    fn rebuild_directory_view(&mut self, filter: &str) {
        let mut filtered_directory_indices = Vec::new();
        let mut displayed_directories = Vec::new();

        for (index, path) in self.directories.iter().enumerate() {
            if let Some((label, indices)) = self.directory_match_indices(path, filter) {
                filtered_directory_indices.push(index);
                displayed_directories.push(Line::from(string_to_styled_spans(&label, &indices)));
            }
        }

        self.filtered_directory_indices = filtered_directory_indices;
        self.displayed_directories = displayed_directories;
    }

    fn directory_label(path: &Path) -> String {
        path.display().to_string()
    }

    fn file_label(path: &Path) -> String {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string())
    }

    fn directory_match_indices(&self, path: &Path, filter: &str) -> Option<(String, Vec<usize>)> {
        let label = Self::directory_label(path);
        if filter.is_empty() {
            return Some((label, Vec::new()));
        }
        let indices = FuzzySearch::new(filter, &label)
            .case_insensitive()
            .score_with(&self.scoring)
            .best_match()
            .filter(|matched| matched.score() > 0)
            .map(|matched| matched.matched_indices().copied().collect())?;
        Some((label, indices))
    }

    fn file_match_indices(&self, path: &Path, filter: &str) -> Option<(String, Vec<usize>)> {
        let label = Self::file_label(path);
        if filter.is_empty() {
            return Some((label, Vec::new()));
        }
        let indices = FuzzySearch::new(filter, &label)
            .case_insensitive()
            .score_with(&self.scoring)
            .best_match()
            .filter(|matched| matched.score() > 0)
            .map(|matched| matched.matched_indices().copied().collect())?;
        Some((label, indices))
    }

    fn rebuild_file_view(&mut self, filter: &str) {
        let mut filtered_file_indices = Vec::new();
        let mut displayed_files = Vec::new();

        for (index, path) in self.files.iter().enumerate() {
            if let Some((label, indices)) = self.file_match_indices(path, filter) {
                filtered_file_indices.push(index);
                displayed_files.push(Line::from(string_to_styled_spans(&label, &indices)));
            }
        }

        self.filtered_file_indices = filtered_file_indices;
        self.displayed_files = displayed_files;
    }

    fn visible_len(&self, focus: &Focus) -> usize {
        match focus {
            Focus::Directories => self.filtered_directory_indices.len(),
            Focus::Files => self.filtered_file_indices.len(),
        }
    }

    fn move_up(&mut self) {
        if self.line_index > 0 {
            self.line_index -= 1;
        }
    }

    fn move_down(&mut self, focus: &Focus) {
        if self.line_index + 1 < self.visible_len(focus) {
            self.line_index += 1;
        }
    }

    pub fn handle_event(&mut self, _chord: Chord) {}
}

impl SubApp<EventEnum, AppState> for Directories {
    fn handle_input(&mut self, event: &mut EventEnum, app_state: &mut AppState) {
        match event {
            EventEnum::Setup => {
                self.refresh_entries(&app_state.current_directory);
                self.rebuild_directory_view(&app_state.directory_filter);
                self.rebuild_file_view(&app_state.file_filter);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) => {
                app_state.focus = Focus::Directories;
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Up, ..
            }) => {
                self.move_up();
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Down,
                ..
            }) => {
                self.move_down(&app_state.focus);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) if app_state.focus == Focus::Directories => {
                self.rebuild_file_view(&app_state.file_filter);
                app_state.focus = Focus::Files;
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Tab, ..
            }) if app_state.focus == Focus::Directories => {
                let Some(&index) = self.filtered_directory_indices.get(self.line_index) else {
                    return;
                };

                app_state.current_directory = self.directories[index].clone();
                self.refresh_entries(&app_state.current_directory);
                self.rebuild_directory_view(&app_state.directory_filter);
            }

            EventEnum::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => match app_state.focus {
                Focus::Directories => {
                    if app_state.directory_filter.is_empty() {
                        if let Some(parent) = app_state.current_directory.parent() {
                            app_state.current_directory = parent.to_path_buf();
                        }
                    } else {
                        app_state.directory_filter.pop();
                    }
                    self.refresh_entries(&app_state.current_directory);
                    self.rebuild_directory_view(&app_state.directory_filter);
                }
                Focus::Files => {
                    if !app_state.file_filter.is_empty() {
                        app_state.file_filter.pop();
                        self.rebuild_file_view(&app_state.file_filter);
                    } else {
                        if let Some(parent) = app_state.current_directory.parent() {
                            app_state.current_directory = parent.to_path_buf();
                        }
                        self.refresh_entries(&app_state.current_directory);
                        self.rebuild_directory_view(&app_state.directory_filter);
                        self.rebuild_directory_view(&app_state.directory_filter);
                        app_state.focus = Focus::Directories;
                    }
                }
            },
            EventEnum::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }) => match app_state.focus {
                Focus::Directories => {
                    app_state.directory_filter.push(*c);
                    self.rebuild_directory_view(&app_state.directory_filter);
                    self.line_index = 0;
                }
                Focus::Files => {
                    app_state.file_filter.push(*c);
                    self.rebuild_file_view(&app_state.file_filter);
                    self.line_index = 0;
                }
            },
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, _title: &str, app_state: &mut AppState) {
        let focused = app_state.focus == Focus::Files;

        let title = match app_state.focus {
            Focus::Directories => "Directories",
            Focus::Files => "Files",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if focused {
                Style::new().bold()
            } else {
                Style::new()
            });

        let list = if app_state.focus == Focus::Directories {
            List::new(self.displayed_directories.iter().cloned())
        } else {
            List::new(self.displayed_files.iter().cloned())
        }
        .block(block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        );

        let mut state = ListState::default();
        let visible_len = self.visible_len(&app_state.focus);
        if visible_len != 0 {
            state.select(Some(self.line_index.min(visible_len - 1)));
        }

        frame.render_stateful_widget(list, area, &mut state);
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
