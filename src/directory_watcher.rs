use crossbeam_channel::unbounded;
use crossterm::event::{KeyCode, KeyModifiers};
use notify::{watcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tui::layout::Constraint;
use tui::text::{Span, Spans, Text};
use tui::widgets::{List, ListItem, Tabs};
use walkdir::WalkDir;

// https://users.rust-lang.org/t/how-to-use-self-while-spawning-a-thread-from-method/8282/6
// https://stackoverflow.com/questions/42043823/design-help-threading-within-a-struct

pub struct DirectoryWatcher {
    pub path: Arc<RwLock<PathBuf>>,
    pub _watcher: notify::INotifyWatcher,
    pub _sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    pub receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    pub lines: Arc<RwLock<Vec<String>>>,
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
            path,
            _sender: sender,
            _watcher: watcher,
            receiver,
            lines: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn watch(&mut self, path: &Path) {
        self._watcher
            .watch(path.to_path_buf(), RecursiveMode::Recursive)
            .unwrap();
    }

    pub fn update_lines(path: &Path, lines: &Arc<RwLock<Vec<String>>>) {
        let new_lines = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| String::from(e.file_name().to_string_lossy()))
            .collect::<Vec<String>>();

        let mut writable_lines = lines.write().unwrap();

        *writable_lines = new_lines;
    }

    pub fn listen(&self) {
        let receiver = self.receiver.clone();
        let lines = self.lines.clone();
        let path = self.path.clone();

        // Wait here for changes
        thread::spawn(move || loop {
            match receiver.recv() {
                Ok(event) => {
                    let unwrapped_path = path.read().unwrap();
                    DirectoryWatcher::update_lines(&unwrapped_path, &lines);
                }
                Err(e) => println!("watch error: {:?}", e),
            };
        });
    }

    pub fn get_displayable(&self) -> List {
        let lines = self.lines.read().unwrap().to_vec();
        let list_items: Vec<ListItem> = lines.into_iter().map(ListItem::new).collect();
        List::new(list_items)
        // Spans::from(spans)
        // Tabs::new(vec![Spans::from(spans)])
    }

    pub fn get_constraints(&self) -> Constraint {
        Constraint::Length(10)
    }
}
