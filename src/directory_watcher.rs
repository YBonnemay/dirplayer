use crossbeam_channel::unbounded;
use crossterm::event::{KeyCode, KeyModifiers};
use notify::{watcher, RecursiveMode, Watcher};
use rodio::{source::Source, Decoder, OutputStream};
use std::cmp;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tui::layout::Constraint;
use tui::style::{Color, Style};
use tui::widgets::{List, ListItem, ListState};
use walkdir::{DirEntry, WalkDir};

// https://users.rust-lang.org/t/how-to-use-self-while-spawning-a-thread-from-method/8282/6
// https://stackoverflow.com/questions/42043823/design-help-threading-within-a-struct

pub struct DirectoryWatcher {
    pub path: Arc<RwLock<PathBuf>>,
    pub _watcher: notify::INotifyWatcher,
    pub _sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    pub receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    pub lines: Arc<RwLock<Vec<DirEntry>>>,
    pub line_index: i32,
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
            line_index: 0,
        }
    }

    pub fn watch(&mut self, path: &Path) {
        self._watcher
            .watch(path.to_path_buf(), RecursiveMode::Recursive)
            .unwrap();
    }

    pub fn update_lines(path: &Path, lines: &Arc<RwLock<Vec<DirEntry>>>) {
        let new_lines = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .collect::<Vec<DirEntry>>();

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
                Ok(_event) => {
                    let unwrapped_path = path.read().unwrap();
                    DirectoryWatcher::update_lines(&unwrapped_path, &lines);
                }
                Err(e) => println!("watch error: {:?}", e),
            };
        });
    }

    pub fn get_displayable(&self) -> (List, ListState) {
        let lines = self.lines.read().unwrap().to_vec();
        let list_items: Vec<ListItem> = lines
            .into_iter()
            .map(|e| ListItem::new(String::from(e.file_name().to_string_lossy())))
            .collect();

        let displayables =
            List::new(list_items).highlight_style(Style::default().bg(Color::Rgb(51, 51, 51)));
        let mut state = ListState::default();
        state.select(Some(self.line_index as usize));
        (displayables, state)
    }

    pub fn get_constraints(&self) -> Constraint {
        Constraint::Length(10)
    }

    pub fn process_event(&mut self, key_code: KeyCode, _: KeyModifiers) {
        match key_code {
            KeyCode::Down => self.line_down(),
            KeyCode::Up => self.line_up(),
            KeyCode::Enter => self.play(),
            _ => (),
        }
    }

    fn line_down(&mut self) {
        let line_length: i32 = self.lines.read().unwrap().to_vec().len() as i32;
        self.line_index = cmp::min(self.line_index + 1, line_length);
    }

    fn line_up(&mut self) {
        self.line_index = cmp::max(self.line_index - 1, 0);
    }

    fn play(&self) {
        let lines = self.lines.read().unwrap().to_vec();
        let _filename = &lines[self.line_index as usize];

        // Get a output stream handle to the default physical sound device
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        // Load a sound from a file, using a path relative to Cargo.toml
        let file = BufReader::new(File::open(_filename.path()).unwrap());
        // Decode that sound file into a source
        let source = Decoder::new(file).unwrap();
        // Play the sound directly on the device
        stream_handle.play_raw(source.convert_samples());
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
