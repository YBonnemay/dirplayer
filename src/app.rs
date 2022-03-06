use crate::directory_watcher::DirectoryWatcher;
use crate::handlers::selector;
use crate::KeyCode;
use crate::KeyModifiers;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tui::layout::Constraint;
use tui::text::{Span, Spans};

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
    pub constraints: Constraint,
}

impl<'a> DirectorySelector<'a> {
    pub fn new() -> DirectorySelector<'a> {
        DirectorySelector {
            completions: get_path_completions(&PathBuf::from(String::from("/"))),
            displayable_completions: VecDeque::from(vec![Spans::from(vec![Span::raw(
                String::from(""),
            )])]),
            filter: String::from(""),
            matcher: SkimMatcherV2::default(),
            rotate_idx: 0,
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
        app.directory_watcher.listen(PathBuf::default());
        app
    }

    pub fn default() -> App<'a> {
        App {
            current_zone: Zone::Directory,
            directory_selector: DirectorySelector::new(),
            directory_watcher: DirectoryWatcher::new(),
            path: PathBuf::from(String::from("/")),
        }
    }

    pub fn set_zone(&mut self, key: &KeyCode) {
        match key {
            KeyCode::Up => {
                if self.current_zone == Zone::Content {
                    self.current_zone = Zone::Directory
                }
            }

            KeyCode::Down => {
                let lines = self.directory_watcher.lines.clone();
                let path = self.path.clone();
                self.set_path(path);
                let path = self.path.clone();
                DirectoryWatcher::update_lines(&path, &lines);
                if self.current_zone == Zone::Directory {
                    self.current_zone = Zone::Content
                }
            }

            _ => (),
        }
    }

    pub fn process_event(&mut self, key_code: KeyCode, key_modifiers: KeyModifiers) {
        if key_modifiers == KeyModifiers::CONTROL {
            self.set_zone(&key_code);
        } else {
            match self.current_zone {
                Zone::Directory => selector::process_event(self, key_code, key_modifiers),
                Zone::Content => self
                    .directory_watcher
                    .process_event(key_code, key_modifiers),
            }
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        selector::update_completions(self, &path);
        self.directory_watcher.set_path(path.clone());
        self.path = path;
    }

    pub fn process_tick(&mut self) {
        self.directory_watcher.autoplay();
    }
}
