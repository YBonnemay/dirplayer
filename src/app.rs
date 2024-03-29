use crate::directory_selector::DirectorySelector;
use crate::directory_watcher::DirectoryWatcher;
use crate::echo_area::EchoArea;
use crate::utils;
use crate::KeyCode;
use crate::KeyModifiers;
use std::fs;
use std::io::Stdout;
use std::path::Path;
use std::path::PathBuf;
use tui::backend::CrosstermBackend;
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

pub struct App<'a> {
    current_zone: Zone,
    pub directory_selector: DirectorySelector<'a>,
    pub directory_watcher: DirectoryWatcher,
    pub echo_area: EchoArea,
    pub path: PathBuf,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let echo_area = EchoArea::new();
        let config = utils::config::get_config();
        let mut directory_watcher = DirectoryWatcher::new(echo_area.sender.clone());
        directory_watcher.listen_start();

        App {
            current_zone: Zone::Directory,
            directory_selector: DirectorySelector::new(),
            directory_watcher,
            echo_area,
            path: PathBuf::from(config.working_directories[0].clone()),
        }
    }

    pub fn set_zone(&mut self, key: &KeyCode) {
        match key {
            KeyCode::Up => {
                if self.current_zone == Zone::Content {
                    self.current_zone = Zone::Directory;
                    self.directory_selector.update_selector(&self.path.clone());
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
        if key_modifiers == KeyModifiers::CONTROL
            && (key_code == KeyCode::Up || key_code == KeyCode::Down)
        {
            self.set_zone(&key_code);
        } else {
            match self.current_zone {
                Zone::Directory => {
                    self.directory_selector
                        .process_event(&mut self.path, key_code, key_modifiers)
                }
                Zone::Content => {
                    self.directory_watcher
                        .process_event(key_code, key_modifiers, terminal)
                }
            }
        }
    }

    fn update_directory_watcher(&mut self) {
        // Update lines
        let path = &self.path;
        self.directory_watcher.update_path(path);
        self.directory_watcher.update_lines();
        self.directory_watcher.update_lines_filtered();

        // Update directories
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
        self.directory_watcher.on_tick();
    }
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self::new()
    }
}
