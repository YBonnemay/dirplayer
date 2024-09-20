use crate::app::Chord;
#[cfg(feature = "mpv")]
use crate::backend_mpv::Mpv;
use crate::backend_rodio::Rodio;
use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use crate::utils;
use crate::utils::config::{Config, Status, WorkingPath};
use chrono::{Datelike, NaiveDate};
use chrono::{NaiveDateTime, Utc};
use crossbeam_channel::unbounded;
use crossterm::event::poll;
use crossterm::event::{KeyCode, KeyModifiers};
use itertools::Itertools;
use log::debug;
use notify::{watcher, RecursiveMode, Watcher};
use rand::{thread_rng, Rng};
use ratatui::Frame;
use std::cmp;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
use sublime_fuzzy::{FuzzySearch, Scoring};
use utils::directory;
use walkdir::DirEntry;

// #[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[derive(std::clone::Clone)]
pub struct FileLine {
    pub dir_entry: DirEntry,
    pub indices: Vec<usize>,
}

pub struct Files {
    pub _sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    pub current_file: String,
    pub filter: String,
    pub line_index: i32,
    pub lines: Arc<RwLock<Vec<DirEntry>>>,
    pub lines_filtered: Vec<FileLine>,
    pub scoring: Scoring,
    #[cfg(feature = "mpv")]
    pub mpv_client: Mpv,

    pub paths: Arc<RwLock<Vec<String>>>,
    // pub path: Arc<RwLock<PathBuf>>,
    pub receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    pub rodio_client: Rodio,
    pub dir_changed: Arc<RwLock<bool>>,
    pub watcher: notify::INotifyWatcher,
    pub extensions: Vec<String>,
    extensions_archives: Vec<String>,
}

impl Files {
    pub fn new() -> Files {
        let (sender, receiver) = unbounded();
        let mut watcher = watcher(sender.clone(), Duration::from_secs(1)).unwrap();

        let config = utils::config::get_config();
        let default_paths = config.working_directories.clone();

        default_paths.iter().for_each(|working_path| {
            watcher
                .watch(&working_path.path, RecursiveMode::Recursive)
                .unwrap();
        });

        let scoring = Scoring {
            bonus_consecutive: 64,
            bonus_word_start: 0,
            ..Scoring::default()
        };

        log::debug!("new");

        let paths = default_paths
            .iter()
            .map(|WorkingPath { path, .. }| (*path).clone())
            .collect_vec();

        let current_file = config.current_file;
        Files {
            _sender: sender,
            // MAYBE WRONG
            current_file,
            filter: String::default(),
            line_index: 0,
            lines: Arc::new(RwLock::new(Vec::new())),
            lines_filtered: Vec::new(),
            scoring,
            mpv_client: Mpv::new(),
            rodio_client: Rodio::new(),
            paths: Arc::new(RwLock::new(paths)),
            receiver,
            dir_changed: Arc::new(RwLock::new(false)),
            watcher,
            extensions: config.extensions,
            extensions_archives: config.extensions_archives,
        }
    }

    pub fn update_lines(&mut self) {
        let paths: Vec<PathBuf> = self
            .paths
            .read()
            .unwrap()
            .iter()
            .map(PathBuf::from)
            .collect_vec();

        let mut new_lines = paths
            .into_iter()
            .flat_map(|path| directory::get_direntries(&path, &self.extensions))
            .collect_vec();

        new_lines.sort_by(|a, b| {
            let creation_a = a.metadata().unwrap().created().unwrap();
            let creation_b = b.metadata().unwrap().created().unwrap();
            creation_b.partial_cmp(&creation_a).unwrap()
        });

        let mut lines = self.lines.write().unwrap();
        *lines = new_lines;
    }

    pub fn update_paths(&mut self, new_working_paths: &VecDeque<WorkingPath>) {
        let new_paths = new_working_paths
            .iter()
            .filter(|new_working_path| new_working_path.status == Status::Active)
            .map(|new_working_path| new_working_path.path.clone())
            .collect_vec();
        let paths = self.paths.clone();
        let unwrapped_path = paths.read().unwrap().clone();
        unwrapped_path
            .iter()
            .for_each(|path| match self.watcher.unwatch(path) {
                Ok(_) | Err(_) => {}
            });
        unwrapped_path.iter().for_each(|path| {
            match self.watcher.watch(path, RecursiveMode::Recursive) {
                Ok(_) | Err(_) => {}
            }
        });

        let mut unwrapped_path = paths.write().unwrap();
        *unwrapped_path = new_paths;

        self.line_index = 0;
    }

    pub fn listen_start(&mut self) {
        let receiver = self.receiver.clone();
        let dir_changed = self.dir_changed.clone();
        // Wait here for directory changes
        thread::spawn(move || loop {
            match receiver.recv() {
                Ok(_event) => {
                    // Send event resize instead
                    let mut dir_changed = dir_changed.write().unwrap();
                    *dir_changed = true;
                }
                Err(e) => println!("watch error: {:?}", e),
            };
        });
    }

    pub fn date_to_color(created: SystemTime) -> (u8, u8, u8) {
        let current_date = chrono::DateTime::<Utc>::from(created);
        let gradient = colorous::RAINBOW;

        let start_of_year = NaiveDate::from_ymd_opt(current_date.year(), 1, 1)
            .unwrap_or(NaiveDate::MIN)
            .and_hms_opt(0, 0, 0)
            .unwrap_or(NaiveDateTime::MIN);

        let start_of_year: chrono::DateTime<Utc> =
            chrono::DateTime::from_naive_utc_and_offset(start_of_year, Utc);

        let end_of_year = NaiveDate::from_ymd_opt(current_date.year() + 1, 1, 1)
            .unwrap_or(NaiveDate::MIN)
            .and_hms_opt(0, 0, 0)
            .unwrap_or(NaiveDateTime::MIN)
            - chrono::Duration::microseconds(1);

        let end_of_year: chrono::DateTime<Utc> =
            chrono::DateTime::from_naive_utc_and_offset(end_of_year, Utc);

        let elapsed = current_date - start_of_year;
        let total = end_of_year - start_of_year;
        let ratio = elapsed.num_seconds() as f64 / total.num_seconds() as f64;

        gradient.eval_continuous(ratio).as_tuple()
    }

    pub fn get_backend(&mut self, file_name: &str) -> &mut dyn AudioBackend {
        if file_name.ends_with("opus") && cfg!(feature = "mpv") {
            &mut self.mpv_client
        } else {
            &mut self.rodio_client
        }
    }

    pub fn handle_event(&mut self, frame: &Frame, chord: Chord) {
        let chunks = crate::ui::get_chunks(frame);
        let height = chunks[1].height;
        match chord.1 {
            KeyCode::Down => self.lines_down(1),
            KeyCode::Up => self.lines_up(1),
            KeyCode::Enter => {
                self.play_file();
            }
            KeyCode::PageDown => {
                let slice_size_half = height / 2;
                self.lines_down(slice_size_half as i32)
            }
            KeyCode::PageUp => {
                let slice_size_half = height / 2;
                self.lines_up(slice_size_half as i32)
            }
            KeyCode::Char(c) => {
                log::debug!("EVENT {:?}", c);
                if c == 'n' && chord.0 == KeyModifiers::CONTROL {
                    self.play_next();
                } else if c == 'p' && chord.0 == KeyModifiers::CONTROL {
                    self.lines_up(1);
                    self.play_next();
                } else {
                    self.filter = format!("{}{}", self.filter, c);
                    self.update_lines_filtered();
                }
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

    pub fn play_file(&mut self) {
        let lines = &self.lines_filtered;
        let lines_length = lines.len();
        if self.line_index + 1 > lines_length as i32 {
            return;
        }
        let file_name = &lines[self.line_index as usize];
        let new_file = String::from(file_name.dir_entry.path().to_str().unwrap());
        let current_file = self.current_file.clone();

        log::debug!("current_file {:?}", current_file);
        log::debug!("new_file {:?}", new_file);

        let current_backend = self.get_backend(&current_file);
        if new_file == *current_file {
            log::debug!("current_backend.toggle");
            current_backend.toggle();
            //
            //
            //
            //
        } else {
            if current_backend.busy() {
                current_backend.silent_pause();
            }
            let new_backend = self.get_backend(&new_file);
            new_backend.start(&new_file);
            self.current_file = new_file.clone();
        }

        // Update currently playing file in Config
        let config = utils::config::get_config();
        utils::config::update_config(&Config {
            current_file: self.current_file.clone(),
            ..config
        });

        log::debug!("play_file {:?}", self.line_index);
    }

    fn lines_down(&mut self, line_number: i32) {
        let line_length: i32 = self.lines_filtered.len() as i32;
        log::debug!("play_file A {:?}", self.line_index);
        self.line_index = cmp::min(self.line_index + line_number, line_length - 1);
        log::debug!("play_file B {:?}", self.line_index);
    }

    fn lines_up(&mut self, line_number: i32) {
        self.line_index = cmp::max(self.line_index - line_number, 0);
    }

    fn play_next(&mut self) {
        let index_moved;
        {
            let lines = &self.lines_filtered;
            match lines.get(self.line_index as usize) {
                Some(line) => {
                    let index_file = String::from(line.dir_entry.path().to_str().unwrap());
                    //
                    log::debug!("index_file {index_file}");
                    log::debug!("self.current_file {:?}", self.current_file);
                    index_moved = index_file != self.current_file;
                }
                None => {
                    return;
                }
            }
        }

        if !index_moved {
            log::debug!("!index_moved");
            let config = utils::config::get_config();
            match config.play_mode {
                utils::config::PlayMode::Queue => {
                    self.lines_down(1); //
                    log::debug!("lines_down Queue");
                }
                utils::config::PlayMode::Random => {
                    let line_length: i32 = self.lines_filtered.len() as i32;
                    // let r = 0..(line_length - 1);
                    let mut rng = thread_rng();
                    let n: u32 = rng.gen_range(0..(line_length - 1) as u32);
                    // let n: u32 = rng.gen_range(0..(line_length - 1));
                    self.line_index = n as i32;
                    log::debug!("play_next 1 {:?}", self.line_index);
                    self.lines_down(1);
                    log::debug!("play_next 2 {:?}", self.line_index);
                }
            }
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
        log::debug!(
            "song_is_ended {:?} for file {:?}",
            song_is_ended,
            current_file
        );

        if !player_is_paused && song_is_ended {
            self.play_next();
        }
    }

    pub fn update_lines_filtered(&mut self) {
        {
            let readable_lines = self.lines.read().unwrap();
            let lines: Vec<FileLine> = Vec::default();

            self.lines_filtered = vec![];
            self.lines_filtered = (*readable_lines).iter().fold(lines, |mut acc, e| {
                if self.filter.is_empty() {
                    acc.push(FileLine {
                        dir_entry: e.to_owned(),
                        indices: vec![],
                    });
                    return acc;
                }

                let haystack = e.file_name().to_str().unwrap();

                if let Some(matched) = FuzzySearch::new(&self.filter, haystack)
                    .case_insensitive()
                    .score_with(&self.scoring)
                    .best_match()
                {
                    let score = matched.score();
                    let indices = matched.matched_indices().copied().collect_vec();
                    if score > 0 {
                        acc.push(FileLine {
                            dir_entry: e.to_owned(),
                            indices,
                        });
                    }
                }

                acc
            });
        }

        let current_file = self.current_file.clone();
        let current_backend = self.get_backend(&current_file);
        let file_name = current_backend.file_name();
        log::info!("update_lines_filtered");
        log::info!("file_name {file_name}");

        let line_index = self.get_line_index(&current_file);
        log::info!("line_index {line_index}");
        log::info!("current_file {current_file}");
        self.line_index = line_index;
    }

    fn get_line_index(&mut self, current_file: &String) -> i32 {
        if *current_file == String::default() {
            return 0;
        }
        if let Some(current_index) = self
            .lines_filtered
            .iter()
            .position(|line| line.dir_entry.path().to_str().unwrap() == current_file)
        {
            current_index as i32
        } else {
            0
        }
    }

    pub fn startup(&mut self) {}

    pub fn watch_archives(&mut self) {
        let paths = self.paths.read().unwrap().clone();
        let extensions_archives = self.extensions_archives.clone();
        let cache_dir = utils::config::get_cache_dir();

        thread::spawn(move || loop {
            if poll(Duration::from_millis(1000)).unwrap() {
                utils::directory::process_archives(&paths, &extensions_archives, &cache_dir);
            }
        });
    }

    pub fn on_tick(&mut self) {
        // Check for updated directory
        let dir_changed = self.dir_changed.clone();
        let mut dir_changed = dir_changed.write().unwrap();
        if *dir_changed {
            *dir_changed = false;
            self.update_lines();
            self.update_lines_filtered();
        }
        // Maybe autoplay next
        self.autoplay();
        log::debug!("on_tick");
        // let paths = self.paths.read().unwrap();
    }
}
