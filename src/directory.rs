use crossbeam_channel::unbounded;
use notify::{watcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tui::layout::Constraint;
use walkdir::WalkDir;
use zone::Zone;

// This represents an updated view on a directory.
// It makes the recursive contents of a directory available and updated.
// For that is waits for notify::Watcher events

pub struct Directory {
    pub path: Arc<RwLock<PathBuf>>,
    pub _watcher: notify::INotifyWatcher,
    pub _sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    pub receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    pub lines: Arc<RwLock<Vec<String>>>,
}

impl Directory {
    pub fn new(pathbuf: PathBuf) -> Directory {
        let (sender, receiver) = unbounded();
        let mut watcher = watcher(sender.clone(), Duration::from_secs(1)).unwrap();

        let path = Arc::new(RwLock::new(pathbuf.clone()));
        // Watching directory.
        watcher
            .watch(pathbuf.clone(), RecursiveMode::Recursive)
            .unwrap();

        Directory {
            path,
            _sender: sender,
            _watcher: watcher,
            receiver,
            lines: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_new_lines(
        path: &std::sync::Arc<std::sync::RwLock<std::path::PathBuf>>,
        lines: &std::sync::Arc<std::sync::RwLock<std::vec::Vec<std::string::String>>>,
    ) {
        let path_read = &*(path.read().unwrap());
        let new_lines = WalkDir::new(path_read)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| String::from(e.file_name().to_string_lossy()))
            .collect::<Vec<String>>();

        let mut writable_lines = lines.write().unwrap();
        *writable_lines = new_lines;
    }

    pub fn refresh_lines(&self) {
        let path = self.path.clone();
        let lines = self.lines.clone();
        Directory::get_new_lines(&path, &lines);
    }

    pub fn listen(&self) {
        let receiver = self.receiver.clone();
        let lines = self.lines.clone();
        let path = self.path.clone();

        // Wait here for changes
        thread::spawn(move || loop {
            match receiver.recv() {
                Ok(_) => {
                    Directory::get_new_lines(&path, &lines);
                }
                Err(e) => println!("watch error: {:?}", e),
            };
        });
    }
}

impl Zone for Directory {
    fn get_displayable(&self) -> Vec<String> {
        self.lines.clone().read().unwrap().to_vec()
    }

    fn get_constraints(&self) -> Constraint {
        Constraint::Length(1)
    }
}
