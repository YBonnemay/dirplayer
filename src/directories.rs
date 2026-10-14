use crate::AppState;
use crate::EventEnum;
use crate::Focus;
use crate::PlayRequest;
use crate::utils::config;
use crate::utils::directory::is_playable;
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
    pub state: State,
    pub directories: Vec<PathBuf>,
    pub filtered_directory_indices: Vec<usize>,
    pub displayed_directories: Vec<Line<'static>>,
    pub files: Vec<PathBuf>,
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
            } else if file_type.is_file() && is_playable(&path) {
                files.push(path);
            }
        }
    }

    directories.sort();
    files.sort();

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
            state: State::Base,
            directories: vec![],
            filtered_directory_indices: vec![],
            displayed_directories: vec![],
            files: vec![],
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

    fn rebuild_file_view(&mut self, app_state: &mut AppState) {
        let mut visible_files = Vec::new();
        let mut displayed_files = Vec::new();

        for path in self.files.iter() {
            if let Some((label, indices)) = self.file_match_indices(path, &app_state.file_filter) {
                visible_files.push(path.clone());
                displayed_files.push(Line::from(string_to_styled_spans(&label, &indices)));
            }
        }

        self.displayed_files = displayed_files;
        app_state.visible_files = visible_files;
    }

    fn visible_len(&self, focus: &Focus) -> usize {
        match focus {
            Focus::Directories => self.filtered_directory_indices.len(),
            Focus::Files => self.displayed_files.len(),
        }
    }

    fn move_up(&mut self, app_state: &mut AppState) {
        if app_state.line_index > 0 {
            app_state.line_index -= 1;
        }
    }

    fn move_down(&mut self, app_state: &mut AppState) {
        if app_state.line_index + 1 < self.visible_len(&app_state.focus) {
            app_state.line_index += 1;
        }
    }

    fn selected_file(&self, app_state: &AppState) -> Option<PathBuf> {
        app_state.visible_files.get(app_state.line_index).cloned()
    }

    /// Enters `path`, persists it as the working directory and rebuilds both views.
    fn set_directory(&mut self, path: PathBuf, app_state: &mut AppState) {
        app_state.current_directory = path;
        app_state.directory_filter.clear();
        app_state.file_filter.clear();
        app_state.line_index = 0;

        config::update_working_directory(&app_state.current_directory);

        self.refresh_entries(&app_state.current_directory);
        self.rebuild_directory_view(&app_state.directory_filter);
        self.rebuild_file_view(app_state);
    }
}

impl SubApp<EventEnum, AppState> for Directories {
    fn handle_input(&mut self, event: &mut EventEnum, app_state: &mut AppState) {
        match event {
            EventEnum::Setup => {
                self.refresh_entries(&app_state.current_directory);
                self.rebuild_directory_view(&app_state.directory_filter);
                self.rebuild_file_view(app_state);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                app_state.play_request = match c {
                    'n' => Some(PlayRequest::Next),
                    'p' => Some(PlayRequest::Previous),
                    't' => Some(PlayRequest::Toggle),
                    'r' => Some(PlayRequest::CyclePlayMode),
                    _ => None,
                };
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) => {
                app_state.focus = Focus::Directories;
                app_state.line_index = 0;
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Up, ..
            }) => {
                self.move_up(app_state);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Down,
                ..
            }) => {
                self.move_down(app_state);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) => match app_state.focus {
                Focus::Directories => {
                    self.rebuild_file_view(app_state);
                    app_state.focus = Focus::Files;
                    app_state.line_index = 0;
                }
                Focus::Files => {
                    if let Some(path) = self.selected_file(app_state) {
                        app_state.play_request = Some(PlayRequest::Play(path));
                    }
                }
            },
            EventEnum::Key(KeyEvent {
                code: KeyCode::Tab, ..
            }) if app_state.focus == Focus::Directories => {
                let Some(&index) = self.filtered_directory_indices.get(app_state.line_index) else {
                    return;
                };

                let path = self.directories[index].clone();
                self.set_directory(path, app_state);
            }

            EventEnum::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => match app_state.focus {
                Focus::Directories => {
                    if app_state.directory_filter.is_empty() {
                        let parent = app_state.current_directory.parent().map(Path::to_path_buf);
                        if let Some(parent) = parent {
                            self.set_directory(parent, app_state);
                        }
                    } else {
                        app_state.directory_filter.pop();
                        app_state.line_index = 0;
                        self.rebuild_directory_view(&app_state.directory_filter);
                    }
                }
                Focus::Files => {
                    if !app_state.file_filter.is_empty() {
                        app_state.file_filter.pop();
                        app_state.line_index = 0;
                        self.rebuild_file_view(app_state);
                    } else {
                        let parent = app_state.current_directory.parent().map(Path::to_path_buf);
                        if let Some(parent) = parent {
                            self.set_directory(parent, app_state);
                        }
                        app_state.focus = Focus::Directories;
                        app_state.line_index = 0;
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
                    app_state.line_index = 0;
                }
                Focus::Files => {
                    app_state.file_filter.push(*c);
                    self.rebuild_file_view(app_state);
                    app_state.line_index = 0;
                }
            },
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, app_state: &mut AppState) {
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
            state.select(Some(app_state.line_index.min(visible_len - 1)));
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
