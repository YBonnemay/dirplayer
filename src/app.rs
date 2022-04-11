use crate::directory_watcher::DirectoryWatcher;
use crate::handlers::selector;
use crate::utils;
use crate::KeyCode;
use crate::KeyModifiers;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::VecDeque;
use std::fs;
use std::io::Stdout;
use std::path::Path;
use std::path::PathBuf;
use tui::backend::CrosstermBackend;
use tui::layout::Constraint;
use tui::text::{Span, Spans};
use tui::Terminal;

pub fn get_path_completions(path: &Path) -> Vec<String> {
    fs::read_dir(path)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().metadata().unwrap().is_dir())
        .map(|res| String::from(res.unwrap().file_name().to_string_lossy()))
        .collect::<Vec<String>>()
}

#[derive(PartialEq, Eq)]
pub enum Zone {
    Directory,
    Content,
}

pub struct DirectorySelector<'a> {
    pub matcher: fuzzy_matcher::skim::SkimMatcherV2,
    pub completions: Vec<String>,
    pub displayable_completions: VecDeque<Spans<'a>>,
    pub filter: String,
    pub rotate_idx: i32,
    pub rotate_history_idx: i32,
    pub constraints: Constraint,
}

impl<'a> DirectorySelector<'a> {
    pub fn new() -> DirectorySelector<'a> {
        let config = utils::config::get_config();
        DirectorySelector {
            completions: get_path_completions(&PathBuf::from(
                config.working_directories[0].clone(),
            )),
            displayable_completions: VecDeque::from(vec![Spans::from(vec![Span::raw(
                String::from(""),
            )])]),
            filter: String::from(""),
            matcher: SkimMatcherV2::default(),
            rotate_idx: 0,
            rotate_history_idx: 0,
            constraints: Constraint::Length(1),
        }
    }
}

pub struct App<'a> {
    current_zone: Zone,
    pub directory_selector: DirectorySelector<'a>,
    pub directory_watcher: DirectoryWatcher,
    pub path: PathBuf,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let mut app = App::default();
        app.directory_watcher.listen_start();
        app
    }

    pub fn default() -> App<'a> {
        let config = utils::config::get_config();
        App {
            current_zone: Zone::Directory,
            directory_selector: DirectorySelector::new(),
            directory_watcher: DirectoryWatcher::new(),
            path: PathBuf::from(config.working_directories[0].clone()),
        }
    }

    pub fn set_zone(&mut self, key: &KeyCode) {
        match key {
            KeyCode::Up => {
                if self.current_zone == Zone::Content {
                    self.current_zone = Zone::Directory;
                    selector::update_selector(self, &self.path.clone());
                }
            }

            KeyCode::Down => {
                if self.current_zone == Zone::Directory {
                    self.update_directory_watcher();
                    self.current_zone = Zone::Content;
                    self.directory_selector.completions = Vec::new();
                }
            }

            _ => (),
        }
    }

    pub fn process_event(
        &mut self,
        key_code: KeyCode,
        key_modifiers: KeyModifiers,
        terminal: &Terminal<CrosstermBackend<Stdout>>,
    ) {
        if key_modifiers == KeyModifiers::CONTROL {
            self.set_zone(&key_code);
        } else {
            match self.current_zone {
                Zone::Directory => selector::process_event(self, key_code, key_modifiers),
                Zone::Content => {
                    self.directory_watcher
                        .process_event(key_code, key_modifiers, terminal)
                }
            }
        }
    }

    fn update_directory_watcher(&mut self) {
        let path = &self.path;
        self.directory_watcher.update_path(path);
        self.directory_watcher.update_lines();
        self.directory_watcher.update_lines_filtered();

        let mut config = utils::config::get_config();
        let new_path = String::from(path.to_str().unwrap());

        if !config.working_directories.contains(&new_path) {
            config.working_directories.push_front(new_path);
        } else {
            config.working_directories.retain(|e| e != &new_path);
            config.working_directories.push_front(new_path);
        }
        utils::config::update_config(config);
    }

    pub fn process_tick(&mut self) {
        // move that to directory_watcher
        let dir_changed = self.directory_watcher.dir_changed.clone();
        let mut dir_changed = dir_changed.write().unwrap();
        if *dir_changed {
            *dir_changed = false;
            self.directory_watcher.update_lines();
            self.directory_watcher.update_lines_filtered();
        }
        self.directory_watcher.autoplay();
    }
}
