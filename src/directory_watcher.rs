use crate::backend_mpv::Mpv;
use crate::backend_rodio::Rodio;
use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use crate::utils;
use chrono::{DateTime, Utc};
use chrono::{Datelike, NaiveDate};
use crossbeam_channel::unbounded;
use crossterm::event::{KeyCode, KeyModifiers};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use notify::{watcher, RecursiveMode, Watcher};
use std::cmp;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
use tui::backend::Backend;
use tui::backend::CrosstermBackend;
use tui::layout::Constraint;
use tui::layout::{Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Cell, Row, Table, TableState};
use tui::{Frame, Terminal};
use walkdir::{DirEntry, WalkDir};

// #[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[derive(std::clone::Clone)]
pub struct Line {
    dir_entry: DirEntry,
    spans: Vec<usize>,
}

pub struct DirectoryWatcher {
    pub _sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    pub current_file: String,
    pub filter: String,
    pub line_index: i32,
    pub lines: Arc<RwLock<Vec<DirEntry>>>,
    pub lines_filtered: Vec<Line>,
    pub matcher: fuzzy_matcher::skim::SkimMatcherV2,
    pub mpv_client: Mpv,
    pub path: Arc<RwLock<PathBuf>>,
    pub receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    pub rodio_client: Rodio,
    pub dir_changed: Arc<RwLock<bool>>,
    pub watcher: notify::INotifyWatcher,
}

impl DirectoryWatcher {
    pub fn new() -> DirectoryWatcher {
        let (sender, receiver) = unbounded();
        let mut watcher = watcher(sender.clone(), Duration::from_secs(1)).unwrap();

        let config = utils::config::get_config();
        let default_path = config.working_directories[0].clone();

        watcher
            .watch(default_path.clone(), RecursiveMode::Recursive)
            .unwrap();

        DirectoryWatcher {
            _sender: sender,
            current_file: String::default(),
            filter: String::default(),
            line_index: 0,
            lines: Arc::new(RwLock::new(Vec::new())),
            lines_filtered: Vec::new(),
            matcher: SkimMatcherV2::default(),
            mpv_client: Mpv::default(),
            rodio_client: Rodio::new(),
            path: Arc::new(RwLock::new(PathBuf::from(default_path))),
            receiver,
            dir_changed: Arc::new(RwLock::new(false)),
            watcher,
        }
    }

    pub fn update_lines(&mut self) {
        let path = self.path.read().unwrap().clone();
        let mut new_lines = WalkDir::new(path)
            .into_iter()
            .filter(|e| match e.as_ref() {
                Ok(dir) => dir.file_type().is_file(),
                Err(_e) => false,
            })
            .map(|e| e.unwrap())
            .collect::<Vec<DirEntry>>();

        new_lines.sort_by(|a, b| {
            let creation_a = a.metadata().unwrap().created().unwrap();
            let creation_b = b.metadata().unwrap().created().unwrap();
            creation_b.partial_cmp(&creation_a).unwrap()
        });

        let mut lines = self.lines.write().unwrap();
        *lines = new_lines;
    }

    pub fn update_path(&mut self, new_path: &Path) {
        let path = self.path.clone();
        let unwrapped_path = path.read().unwrap().clone();
        self.watcher.unwatch(unwrapped_path).unwrap();
        self.watcher
            .watch(new_path, RecursiveMode::Recursive)
            .unwrap();
        let mut unwrapped_path = path.write().unwrap();
        *unwrapped_path = new_path.to_path_buf();

        // Get current index from config, else 0
        let config = utils::config::get_config();
        if let Some(line_index) = config
            .working_directory_line_index
            .get(new_path.to_str().unwrap())
        {
            self.line_index = *line_index;
        } else {
            self.line_index = 0;
        }
    }

    pub fn listen_start(&mut self) {
        let receiver = self.receiver.clone();
        let dir_changed = self.dir_changed.clone();
        // Wait here for directory changes
        thread::spawn(move || loop {
            match receiver.recv() {
                Ok(_event) => {
                    let mut dir_changed = dir_changed.write().unwrap();
                    *dir_changed = true;
                }
                Err(e) => println!("watch error: {:?}", e),
            };
        });
    }

    pub fn date_to_color(created: SystemTime) -> (u8, u8, u8) {
        let current_date = chrono::DateTime::<Utc>::from(created);

        let start_of_year: chrono::DateTime<Utc> = chrono::DateTime::from_utc(
            NaiveDate::from_ymd(current_date.year(), 1, 1).and_hms(0, 0, 0),
            Utc,
        );

        let end_of_year: chrono::DateTime<Utc> = chrono::DateTime::from_utc(
            NaiveDate::from_ymd(current_date.year() + 1, 1, 1).and_hms(0, 0, 0),
            Utc,
        ) - chrono::Duration::microseconds(1);

        let elapsed = current_date - start_of_year;
        let total = end_of_year - start_of_year;
        let ratio = elapsed.num_seconds() as f64 / total.num_seconds() as f64;

        let gradient = colorous::RAINBOW;
        gradient.eval_continuous(ratio as f64).as_tuple()
    }

    pub fn draw_directory<B: tui::backend::Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let lines = &self.lines_filtered;

        // We only display a term-sized slice of the songs, centered on the current index.
        // This silly dance to ensure table scales with many songs.
        let lines_size = lines.len() as i32;
        let slice_size = f.size().height;

        let slice_size_half = slice_size / 2;
        let mut slice_index = slice_size_half as i32;
        let mut slice_free_index_high_border;
        let mut slice_free_index_low_border;

        let sliding_slice_mode = lines_size > slice_size as i32;

        if sliding_slice_mode {
            slice_free_index_high_border = self.line_index - slice_size_half as i32;
            slice_free_index_low_border = self.line_index + slice_size_half as i32;

            let overlap_high = cmp::max(-slice_free_index_high_border, 0);
            let overlap_low = slice_free_index_low_border - lines_size;

            if overlap_high > 0 {
                // Move slice down
                slice_free_index_high_border += overlap_high;
                slice_free_index_low_border += overlap_high;

                // Move index up
                slice_index -= overlap_high;
            }

            if overlap_low > 0 {
                // Move slice up
                slice_free_index_high_border -= overlap_low;
                slice_free_index_low_border -= overlap_low;

                // Move index up
                slice_index += overlap_low;
            }
        } else {
            slice_free_index_high_border = 0;
            slice_free_index_low_border = lines_size as i32;
            slice_index = self.line_index;
        }

        let list_items =
            &lines[slice_free_index_high_border as usize..slice_free_index_low_border as usize];

        let path = self.path.read().unwrap();

        // #![feature]` may not be used on the stable release channel
        // TODO No such file or directory here on file remove
        let list_items: Vec<Row> = list_items
            .iter()
            .filter_map(|e| {
                let Line { dir_entry, spans } = e;
                let metadata = dir_entry.metadata();
                if metadata.is_err() {
                    return None;
                }

                let created = dir_entry.metadata().unwrap().created().unwrap();
                let date_time = DateTime::<Utc>::from(created);
                let dir_entry_path = dir_entry.path();

                let path: &str = dir_entry_path
                    .strip_prefix(&*path)
                    .unwrap()
                    .to_str()
                    .unwrap();

                let spans = crate::selector::string_to_styled_text(path, spans);
                let (r, g, b) = DirectoryWatcher::date_to_color(created);

                let data = vec![
                    Cell::from(spans),
                    Cell::from(date_time.format("%Y-%m-%d %H-%M-%S").to_string())
                        .style(Style::default().fg(tui::style::Color::Rgb(r, g, b))),
                ];
                Some(Row::new(data))
            })
            .collect();

        let displayables = Table::new(list_items)
            .highlight_style(Style::default().bg(Color::Rgb(51, 51, 51)))
            .widths(&[Constraint::Percentage(80), Constraint::Length(30)]);

        let mut state = TableState::default();
        state.select(Some(slice_index as usize));

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(chunk);

        f.render_stateful_widget(displayables, chunks[0], &mut state);
    }

    pub fn get_backend(&mut self, file_name: &str) -> &mut dyn AudioBackend {
        if file_name.ends_with("opus") {
            &mut self.mpv_client
        } else {
            &mut self.rodio_client
        }
    }

    pub fn process_event(
        &mut self,
        key_code: KeyCode,
        _: KeyModifiers,
        terminal: &Terminal<CrosstermBackend<Stdout>>,
    ) {
        match key_code {
            KeyCode::Down => self.lines_down(1),
            KeyCode::Up => self.lines_up(1),
            KeyCode::Enter => {
                self.play_file();
            }
            KeyCode::PageDown => {
                let slice_size_half = terminal.backend().size().unwrap().height / 2;
                self.lines_down(slice_size_half as i32)
            }
            KeyCode::PageUp => {
                let slice_size_half = terminal.backend().size().unwrap().height / 2;
                self.lines_up(slice_size_half as i32)
            }
            KeyCode::Char(c) => {
                self.filter = format!("{}{}", self.filter, c);
                self.update_lines_filtered();
            }
            KeyCode::Backspace => {
                let mut chars = self.filter.chars();
                chars.next_back();
                self.filter = chars.as_str().to_string();
                self.update_lines_filtered();
            }
            _ => (),
        }
    }

    fn play_file(&mut self) {
        let lines = &self.lines_filtered;
        let lines_length = lines.len();
        if self.line_index + 1 > lines_length as i32 {
            return;
        }
        let file_name = &lines[self.line_index as usize];
        let new_file = String::from(file_name.dir_entry.path().to_str().unwrap());
        let current_file = self.current_file.clone();
        let current_backend = self.get_backend(&current_file);
        if new_file == *current_file {
            current_backend.toggle();
        } else {
            if current_backend.busy() {
                current_backend.pause();
            }
            let new_backend = self.get_backend(&new_file);
            new_backend.start(&new_file);
            self.current_file = new_file;
        }

        let mut config = utils::config::get_config();
        let path = self.path.read().unwrap();
        let entry = config
            .working_directory_line_index
            .entry(String::from(path.to_str().unwrap()))
            .or_insert(self.line_index);
        *entry = self.line_index;
        utils::config::update_config(config);
    }

    fn lines_down(&mut self, line_number: i32) {
        let line_length: i32 = self.lines_filtered.len() as i32;
        self.line_index = cmp::min(self.line_index + line_number, line_length);
    }

    fn lines_up(&mut self, line_number: i32) {
        self.line_index = cmp::max(self.line_index - line_number, 0);
    }

    fn play_next(&mut self) {
        let index_moved;
        {
            let lines = &self.lines_filtered;
            let file_name = &lines[self.line_index as usize];
            let index_file = String::from(file_name.dir_entry.path().to_str().unwrap());
            index_moved = index_file != self.current_file;
        }

        if !index_moved {
            self.lines_down(1);
        }

        self.play_file()
    }

    pub fn autoplay(&mut self) {
        // Do nothing if nothing in directory watcher.
        let readable_lines = &self.lines_filtered;
        if (*readable_lines).is_empty() {
            return;
        }

        // If dirplayer is not paused, bu no song is playing, play next song.
        let current_file = self.current_file.clone();
        let current_backend = self.get_backend(&current_file);
        let player_is_paused = current_backend.state() == SongState::Paused;
        let song_is_ended = current_backend.state() == SongState::Ended;

        // We are at the end of a song, play next one
        if !player_is_paused && song_is_ended {
            self.play_next();
        }
    }

    pub fn update_lines_filtered(&mut self) {
        {
            let readable_lines = self.lines.read().unwrap();
            let lines: Vec<Line> = Vec::default();

            self.lines_filtered = vec![];
            self.lines_filtered = (*readable_lines).iter().fold(lines, |mut acc, e| {
                if self.filter.is_empty() {
                    acc.push(Line {
                        dir_entry: e.to_owned(),
                        spans: vec![],
                    });
                    return acc;
                }

                let path = self.path.read().unwrap();
                let test = e.path().strip_prefix(&*path).unwrap().to_str().unwrap();

                if let Some((score, indices)) = self.matcher.fuzzy_indices(test, &self.filter) {
                    if score > 32 {
                        acc.push(Line {
                            dir_entry: e.to_owned(),
                            spans: indices,
                        });
                    }
                }

                acc
            });
        }

        let current_file = self.current_file.clone();
        let current_backend = self.get_backend(&current_file);
        let file_name = current_backend.file_name();

        if *current_file != String::default() {
            // Get index in filtered list
            if let Some(current_index) = self
                .lines_filtered
                .iter()
                .position(|line| line.dir_entry.path().to_str().unwrap() == file_name)
            {
                self.line_index = current_index as i32;
            } else {
                self.line_index = 0;
            }
        }
    }
}
