use crate::AppState;
use crate::EventEnum;
use crate::PlayRequest;
use crate::utils::config;
use crate::utils::directory::is_playable;
use crate::utils::style::string_to_styled_spans;
use chrono::{DateTime, Datelike, Local, TimeZone};
use cocotte::sub_app::SubApp;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell as TableCell, Row, Table, TableState};
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use sublime_fuzzy::{FuzzySearch, Scoring};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum State {
    Base,
    Editing,
}

/// A ranger-style listing of the current directory: its subdirectories first,
/// then its playable files, walked by a single cursor.
pub struct Directories {
    pub scoring: Scoring,
    pub working_directory: PathBuf,
    pub state: State,
    /// How many directory levels the file walk covers, the current one included.
    pub search_depth: usize,
    pub directories: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    /// Directories left by the filter, in display order. They own the first
    /// rows, so any line index below this length points at a directory.
    pub visible_directories: Vec<PathBuf>,
    pub displayed: Vec<Line<'static>>,
    /// Creation date of each displayed entry, gradient coloured, kept parallel
    /// to `displayed`.
    pub displayed_dates: Vec<Line<'static>>,
    /// Rows the list last rendered into, which is what a page scroll moves by.
    page_size: Cell<usize>,
    /// Entry the cursor was on when the filter was opened, so editing the
    /// filter keeps the cursor on it and clearing the filter comes back to it.
    filter_anchor: Option<PathBuf>,
}

/// Width of a `%Y-%m-%d %H:%M:%S` stamp.
const DATE_WIDTH: u16 = 19;

const DIRECTORY_COLOR: Color = Color::Rgb(114, 180, 255);

/// Scroll offset keeping the cursor on the middle row. The list is only
/// scrolled once the cursor is past the middle and until the last row is
/// reached, so the cursor moves freely inside the first and last half screen,
/// and anywhere at all when the whole list fits.
fn centered_offset(selected: usize, total: usize, viewport: usize) -> usize {
    if total <= viewport {
        return 0;
    }

    selected.saturating_sub(viewport / 2).min(total - viewport)
}

/// The entry date, creation when the filesystem keeps one, last modification
/// otherwise.
fn entry_date(path: &Path) -> Option<SystemTime> {
    let metadata = fs::metadata(path).ok()?;
    metadata.created().or_else(|_| metadata.modified()).ok()
}

/// Places a date on the rainbow by its position within its own year, so that
/// neighbouring dates get neighbouring colours.
fn date_to_color(date: DateTime<Local>) -> Color {
    let year_start = Local.with_ymd_and_hms(date.year(), 1, 1, 0, 0, 0).single();
    let year_end = Local
        .with_ymd_and_hms(date.year() + 1, 1, 1, 0, 0, 0)
        .single();

    let ratio = match (year_start, year_end) {
        (Some(start), Some(end)) if end > start => {
            (date - start).num_seconds() as f64 / (end - start).num_seconds() as f64
        }
        _ => 0.0,
    };

    let (r, g, b) = colorous::RAINBOW.eval_continuous(ratio).as_tuple();
    Color::Rgb(r, g, b)
}

/// The right hand side of a row: the entry date, or nothing when the
/// filesystem does not give one.
fn date_line(path: &Path) -> Line<'static> {
    let Some(date) = entry_date(path).map(DateTime::<Local>::from) else {
        return Line::default();
    };

    Line::from(date.format("%Y-%m-%d %H:%M:%S").to_string())
        .style(Style::default().fg(date_to_color(date)))
}

/// Walks `dir`, collecting its subdirectories and every playable file found
/// within `depth` levels. `depth` counts `dir` itself, so `1` stays flat.
fn collect_entries(
    dir: &Path,
    depth: usize,
    top_level: bool,
    directories: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) {
    if depth == 0 {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        let path = entry.path();
        // `file_type` does not follow symlinks, so symlinked directories are
        // skipped and the walk cannot loop.
        if file_type.is_dir() {
            if top_level {
                directories.push(path.clone());
            }
            collect_entries(&path, depth - 1, false, directories, files);
        } else if file_type.is_file() && is_playable(&path) {
            files.push(path);
        }
    }
}

fn get_entries(dir: &Path, depth: usize) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut directories = Vec::new();
    let mut files = Vec::new();

    collect_entries(dir, depth.max(1), true, &mut directories, &mut files);

    sort_by_date_desc(&mut directories);
    sort_by_date_desc(&mut files);

    (directories, files)
}

/// Most recent first. Entries without a usable date sink to the bottom, and
/// ties fall back to the path so the order stays stable between refreshes.
fn sort_by_date_desc(paths: &mut [PathBuf]) {
    paths.sort_by(|a, b| entry_date(b).cmp(&entry_date(a)).then_with(|| a.cmp(b)));
}

fn directory_label(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    format!("{name}/")
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
            search_depth: config::get_set_config().search_depth,
            directories: vec![],
            files: vec![],
            visible_directories: vec![],
            displayed: vec![],
            displayed_dates: vec![],
            page_size: Cell::new(1),
            filter_anchor: None,
        }
    }

    fn refresh_entries(&mut self, dir: &Path) {
        self.working_directory = dir.to_path_buf();
        let (directories, files) = get_entries(dir, self.search_depth);
        self.directories = directories;
        self.files = files;
    }

    /// Files found below the current directory keep their directories as a
    /// prefix, so the listing stays unambiguous when the walk goes deep.
    fn file_label(&self, path: &Path) -> String {
        path.strip_prefix(&self.working_directory)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    fn match_indices(&self, label: &str, filter: &str) -> Option<Vec<usize>> {
        if filter.is_empty() {
            return Some(Vec::new());
        }

        FuzzySearch::new(filter, label)
            .case_insensitive()
            .score_with(&self.scoring)
            .best_match()
            .filter(|matched| matched.score() > 0)
            .map(|matched| matched.matched_indices().copied().collect())
    }

    /// Rebuilds the single list. Directories keep the top rows, and only the
    /// files reach `visible_files`, which is what plays and gets picked at
    /// random.
    fn rebuild_view(&mut self, app_state: &mut AppState) {
        let filter = &app_state.filter;
        let mut visible_directories = Vec::new();
        let mut visible_files = Vec::new();
        let mut displayed = Vec::new();
        let mut displayed_dates = Vec::new();

        for path in self.directories.iter() {
            let label = directory_label(path);
            if let Some(indices) = self.match_indices(&label, filter) {
                visible_directories.push(path.clone());
                displayed.push(
                    Line::from(string_to_styled_spans(&label, &indices)).style(
                        Style::default()
                            .fg(DIRECTORY_COLOR)
                            .add_modifier(Modifier::BOLD),
                    ),
                );
                displayed_dates.push(date_line(path));
            }
        }

        for path in self.files.iter() {
            let label = self.file_label(path);
            if let Some(indices) = self.match_indices(&label, filter) {
                visible_files.push(path.clone());
                displayed.push(Line::from(string_to_styled_spans(&label, &indices)));
                displayed_dates.push(date_line(path));
            }
        }

        self.visible_directories = visible_directories;
        self.displayed = displayed;
        self.displayed_dates = displayed_dates;
        app_state.first_file_line = self.visible_directories.len();
        app_state.visible_files = visible_files;
    }

    fn selected_directory(&self, app_state: &AppState) -> Option<PathBuf> {
        self.visible_directories.get(app_state.line_index).cloned()
    }

    fn selected_file(&self, app_state: &AppState) -> Option<PathBuf> {
        let index = app_state
            .line_index
            .checked_sub(self.visible_directories.len())?;
        app_state.visible_files.get(index).cloned()
    }

    /// The entry under the cursor, directory or file alike.
    fn selected_entry(&self, app_state: &AppState) -> Option<PathBuf> {
        self.selected_directory(app_state)
            .or_else(|| self.selected_file(app_state))
    }

    /// Rebuilds the list and puts the cursor back on the anchored entry, or on
    /// the first row when the filter leaves that entry out.
    fn rebuild_filtered_view(&mut self, app_state: &mut AppState) {
        self.rebuild_view(app_state);

        app_state.line_index = self
            .filter_anchor
            .as_ref()
            .and_then(|anchor| self.line_of(anchor, app_state))
            .unwrap_or(0);

        if app_state.filter.is_empty() {
            self.filter_anchor = None;
        }
    }

    /// Row `path` is displayed on, directories owning the rows above the files.
    fn line_of(&self, path: &Path, app_state: &AppState) -> Option<usize> {
        if let Some(index) = self
            .visible_directories
            .iter()
            .position(|entry| entry == path)
        {
            return Some(index);
        }

        app_state
            .visible_files
            .iter()
            .position(|entry| entry == path)
            .map(|index| index + self.visible_directories.len())
    }

    fn move_up(&mut self, app_state: &mut AppState) {
        if app_state.line_index > 0 {
            app_state.line_index -= 1;
        }
        self.reanchor(app_state);
    }

    fn move_down(&mut self, app_state: &mut AppState) {
        if app_state.line_index + 1 < self.displayed.len() {
            app_state.line_index += 1;
        }
        self.reanchor(app_state);
    }

    fn move_page_up(&mut self, app_state: &mut AppState) {
        app_state.line_index = app_state.line_index.saturating_sub(self.page_size.get());
        self.reanchor(app_state);
    }

    fn move_page_down(&mut self, app_state: &mut AppState) {
        let last = self.displayed.len().saturating_sub(1);
        app_state.line_index = (app_state.line_index + self.page_size.get()).min(last);
        self.reanchor(app_state);
    }

    /// Moving the cursor while filtering picks a new entry to come back to.
    fn reanchor(&mut self, app_state: &AppState) {
        if !app_state.filter.is_empty() {
            self.filter_anchor = self.selected_entry(app_state);
        }
    }

    /// Starts the cursor on the file played last time, when that file is still
    /// part of the listing.
    fn focus_last_played(&self, app_state: &mut AppState) {
        let last_played = config::get_config().current_file;
        if last_played.is_empty() {
            return;
        }

        if let Some(line) = self.line_of(Path::new(&last_played), app_state) {
            app_state.line_index = line;
        }
    }

    /// Enters `path`, persists it as the working directory and rebuilds the list.
    fn set_directory(&mut self, path: PathBuf, app_state: &mut AppState) {
        app_state.current_directory = path;
        app_state.filter.clear();
        app_state.line_index = 0;
        self.filter_anchor = None;

        config::update_working_directory(&app_state.current_directory);

        self.refresh_entries(&app_state.current_directory);
        self.rebuild_view(app_state);
    }

    /// Goes up one level, leaving the cursor on the directory just left.
    fn go_to_parent(&mut self, app_state: &mut AppState) {
        let left = app_state.current_directory.clone();
        let Some(parent) = left.parent().map(Path::to_path_buf) else {
            return;
        };

        self.set_directory(parent, app_state);

        if let Some(line) = self.line_of(&left, app_state) {
            app_state.line_index = line;
        }
    }
}

impl SubApp<EventEnum, AppState> for Directories {
    fn handle_input(&mut self, event: &mut EventEnum, app_state: &mut AppState) {
        match event {
            EventEnum::Setup => {
                self.refresh_entries(&app_state.current_directory);
                self.rebuild_view(app_state);
                self.focus_last_played(app_state);
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
                app_state.filter.clear();
                self.rebuild_filtered_view(app_state);
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
                code: KeyCode::PageUp,
                ..
            }) => {
                self.move_page_up(app_state);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::PageDown,
                ..
            }) => {
                self.move_page_down(app_state);
            }
            // Directories are navigation only, files are what plays.
            EventEnum::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) => {
                if let Some(path) = self.selected_file(app_state) {
                    app_state.play_request = Some(PlayRequest::Play(path));
                }
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Right,
                ..
            }) => {
                if let Some(path) = self.selected_directory(app_state) {
                    self.set_directory(path, app_state);
                }
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Left,
                ..
            }) => {
                self.go_to_parent(app_state);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => {
                app_state.filter.pop();
                self.rebuild_filtered_view(app_state);
            }
            EventEnum::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }) => {
                if app_state.filter.is_empty() {
                    self.filter_anchor = self.selected_entry(app_state);
                }
                app_state.filter.push(*c);
                self.rebuild_filtered_view(app_state);
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, app_state: &mut AppState) {
        // The title row eats one row, the player owning the line closing the
        // list, and a page always has to move by at least one.
        self.page_size
            .set(usize::from(area.height.saturating_sub(1)).max(1));

        let mut title = format!(
            "Current Directory {}",
            app_state.current_directory.to_string_lossy()
        );
        if !app_state.filter.is_empty() {
            title.push_str("  |  ");
            title.push_str(&app_state.filter);
        }

        let block = Block::default()
            // The player below draws the line closing this one.
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(title)
            .border_style(Style::new().bold());

        let rows = self
            .displayed
            .iter()
            .zip(self.displayed_dates.iter())
            .map(|(label, date)| {
                Row::new(vec![
                    TableCell::from(label.clone()),
                    TableCell::from(date.clone()),
                ])
            });

        let table = Table::new(rows, [Constraint::Fill(1), Constraint::Length(DATE_WIDTH)])
            .block(block)
            .row_highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = TableState::default();
        if !self.displayed.is_empty() {
            let selected = app_state.line_index.min(self.displayed.len() - 1);
            state.select(Some(selected));
            *state.offset_mut() = centered_offset(
                selected,
                self.displayed.len(),
                usize::from(area.height.saturating_sub(1)).max(1),
            );
        }

        frame.render_stateful_widget(table, area, &mut state);
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
