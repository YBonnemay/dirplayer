use crate::backend_mpv::Mpv;
use crate::backend_rodio::Rodio;
use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use crate::utils;
use chrono::{DateTime, Utc};
use crossbeam_channel::unbounded;
use crossterm::event::{KeyCode, KeyModifiers};
use notify::{watcher, RecursiveMode, Watcher};
use std::cmp;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tui::backend::Backend;
use tui::backend::CrosstermBackend;
use tui::layout::Constraint;
use tui::layout::{Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Cell, Row, Table, TableState};
use tui::{Frame, Terminal};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Line {
    file_name: String,
    date_time: DateTime<Utc>,
}

pub struct DirectoryWatcher {
    pub _sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    pub current_file: String,
    pub line_index: i32,
    pub lines: Arc<RwLock<Vec<DirEntry>>>,
    pub mpv_client: Mpv,
    pub paused: bool,
    pub rodio_client: Rodio,
    pub path: Arc<RwLock<PathBuf>>,
    pub receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    pub watcher: notify::INotifyWatcher,
}

impl DirectoryWatcher {
    pub fn new() -> DirectoryWatcher {
        let (sender, receiver) = unbounded();
        let mut watcher = watcher(sender.clone(), Duration::from_secs(1)).unwrap();
        let path = Arc::new(RwLock::new(PathBuf::default()));
        // Watching directory.

        watcher
            .watch(PathBuf::default(), RecursiveMode::Recursive)
            .unwrap();

        DirectoryWatcher {
            _sender: sender,
            current_file: String::default(),
            line_index: 0,
            lines: Arc::new(RwLock::new(Vec::new())),
            mpv_client: Mpv::default(),
            rodio_client: Rodio::new(),
            paused: false,
            path,
            receiver,
            watcher,
        }
    }

    pub fn update_lines(path: &Path, lines: &Arc<RwLock<Vec<DirEntry>>>) {
        // Update index
        let mut new_lines = WalkDir::new(path)
            .into_iter()
            .filter(|e| {
                let dir = e.as_ref().unwrap();
                dir.file_type().is_file()
            })
            .map(|e| e.unwrap())
            .collect::<Vec<DirEntry>>();

        new_lines.sort_by(|a, b| {
            let creation_a = a.metadata().unwrap().created().unwrap();
            let creation_b = b.metadata().unwrap().created().unwrap();
            creation_b.partial_cmp(&creation_a).unwrap()
        });

        let mut writable_lines = lines.write().unwrap();
        *writable_lines = new_lines;
    }

    pub fn update_path(&mut self, new_path: PathBuf) {
        let path = self.path.clone();
        let unwrapped_path = path.read().unwrap().clone();
        self.watcher.unwatch(unwrapped_path).unwrap();
        self.watcher
            .watch(new_path.clone(), RecursiveMode::Recursive)
            .unwrap();
        let path = self.path.clone();
        let mut unwrapped_path = path.write().unwrap();
        *unwrapped_path = new_path.clone();

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

    pub fn listen(&mut self, path: PathBuf) {
        self.update_path(path);

        let receiver = self.receiver.clone();
        let lines = self.lines.clone();
        let path = self.path.clone();

        // Wait here for changes directory changes
        thread::spawn(move || loop {
            match receiver.recv() {
                Ok(_event) => {
                    let unwrapped_path = path.read().unwrap();
                    DirectoryWatcher::update_lines(&unwrapped_path, &lines);
                }
                Err(e) => println!("watch error: {:?}", e),
            };
        });
    }

    pub fn draw_directory<B: tui::backend::Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        let lines = self.lines.read().unwrap().to_vec();
        let path = self.path.read().unwrap().clone();

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

        let list_items: Vec<Row> = list_items
            .iter()
            .map(|e| {
                let created = e.metadata().unwrap().created().unwrap();
                let date_time = DateTime::<Utc>::from(created);
                let path: String = e
                    .clone()
                    .into_path()
                    .strip_prefix(&path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into();

                let data = vec![
                    Cell::from(path),
                    Cell::from(date_time.format("%Y-%m-%d %H-%M-%S").to_string()),
                ];
                Row::new(data)
            })
            .collect();

        let displayables = Table::new(list_items)
            .highlight_style(Style::default().bg(Color::Rgb(51, 51, 51)))
            .widths(&[Constraint::Percentage(50), Constraint::Length(30)]);

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
                crate::deprintln!("Enter event");
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
            _ => (),
        }
    }

    fn play_file(&mut self) {
        let lines = self.lines.clone();
        let lines = lines.read().unwrap();
        let lines_length = lines.len();
        if self.line_index + 1 > lines_length as i32 {
            return;
        }
        let file_name = &lines[self.line_index as usize];
        let new_file = String::from(file_name.path().to_str().unwrap());
        let current_file = self.current_file.clone();
        let current_backend = self.get_backend(&current_file);
        if new_file == current_file {
            crate::deprintln!("Toggle file {} ", new_file);
            current_backend.toggle();
            self.paused = current_backend.state() == SongState::Paused;
        } else {
            if current_backend.busy() {
                crate::deprintln!("Stop backend");
                current_backend.pause();
            }
            let new_backend = self.get_backend(&new_file);
            crate::deprintln!("Start file {} ", new_file);
            new_backend.start(&new_file);
            self.current_file = new_file;
        }

        let mut config = utils::config::get_config();
        let path = self.path.read().unwrap().clone();
        crate::deprintln!("self.line_index {}", self.line_index);
        let entry = config
            .working_directory_line_index
            .entry(String::from(path.to_str().unwrap()))
            .or_insert(self.line_index);
        *entry = self.line_index;
        utils::config::update_config(config);
    }

    fn lines_down(&mut self, line_number: i32) {
        let line_length: i32 = self.lines.read().unwrap().to_vec().len() as i32;
        self.line_index = cmp::min(self.line_index + line_number, line_length);
    }

    fn lines_up(&mut self, line_number: i32) {
        self.line_index = cmp::max(self.line_index - line_number, 0);
    }

    fn play_next(&mut self) {
        self.lines_down(1);
        crate::deprintln!("play_next");
        self.play_file()
    }

    pub fn autoplay(&mut self) {
        {
            let writable_lines = self.lines.read().unwrap();
            if (*writable_lines).is_empty() {
                return;
            }
        }

        // If dirplayer is not paused, bu no song is playing, play next song.
        let player_is_paused = self.paused;
        let current_file = self.current_file.clone();
        let current_backend = self.get_backend(&current_file);
        let song_is_ended = current_backend.state() == SongState::Ended;

        // We are at the end of a song, play next one
        if !player_is_paused && song_is_ended {
            crate::deprintln!("autoplaying next");
            self.play_next();
        }
    }
}
