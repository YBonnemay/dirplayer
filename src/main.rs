extern crate argh;
extern crate crossbeam_channel;
extern crate crossterm;
extern crate notify;
extern crate serde;
extern crate serde_json;
extern crate tui;
extern crate walkdir;

#[macro_use]
extern crate serde_derive;

// reale time updating of the data

// cd /home/bonnemay/github/tui-rs/example/ ; cargo run --verbose
// cd /home/bonnemay/github/dirplayer/ ; cargo run --verbose
// cd /home/bonnemay/github/dirplayer/ ; RUST_BACKTRACE=1 cargo run --verbose
// ./target/debug/dirplayer
// println!("write_page{:#?}", lock.len());
// use termion::screen::{AlternateScreen, ToAlternateScreen, ToMainScreen};
// use std::io::stdout;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Tabs, Text};
use tui::Frame;
use tui::{backend::CrosstermBackend, Terminal};
use walkdir::WalkDir;

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Serialize, Deserialize)]
struct Config {
    enhanced_graphics: bool,
    extensions: Vec<String>,
    tick_rate: String,
    working_directory: String,
}

fn get_config() -> Config {
    let json = include_str!("/home/bonnemay/github/dirplayer/src/config/config.json");
    serde_json::from_str::<Config>(&json).unwrap()
}

use std::io::{stdout, Write};

use crossbeam_channel::unbounded;
use notify::{watcher, RecursiveMode, Watcher};
use std::path::PathBuf;

pub trait DataSource<T> {
    fn get_lines(&self) -> Vec<T>;
}

struct Directory {
    path: Arc<RwLock<PathBuf>>,
    _watcher: notify::INotifyWatcher,
    _sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    lines: Arc<RwLock<Vec<String>>>,
}

impl Directory {
    fn new(pathbuf: PathBuf) -> Directory {
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

    fn get_new_lines(
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

    fn refresh_lines(&self) {
        let path = self.path.clone();
        let lines = self.lines.clone();
        Directory::get_new_lines(&path, &lines);
    }

    fn listen(&self) {
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

struct DisplayData {
    directory: Directory,
}

impl DisplayData {
    fn new(directory: Directory) -> DisplayData {
        return DisplayData { directory };
    }
}

fn draw<B: tui::backend::Backend>(f: &mut Frame<B>, display_data: &mut DisplayData) {
    let constraints = vec![Constraint::Percentage(100)];
    let chunks = Layout::default().constraints(constraints).split(f.size());

    let displayable_lines = display_data.directory.lines.clone();
    let displayable_lines = displayable_lines.read().unwrap().to_vec();

    // println!("{:#?}", displayable_lines);

    let text_lines = displayable_lines.iter().map(|e| Text::raw(e));
    let text_lines = List::new(text_lines);

    f.render_widget(text_lines, chunks[0])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start
    let directory = Directory::new(PathBuf::from("/home/bonnemay/tests/src"));
    directory.refresh_lines();
    println!("LISTEN CALL");
    directory.listen();
    let mut display_data = DisplayData::new(directory);

    enable_raw_mode()?; // crossterm terminal setup
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?; // crossterm event setup
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let config = get_config();

    // Setup input handling
    let (tx, rx) = unbounded();
    let tick_rate_u64 = config.tick_rate.parse().unwrap();
    let tick_rate = Duration::from_millis(tick_rate_u64);

    // Heartbeat for the  display. Will send each second, of each key
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let time_before_tick = tick_rate - last_tick.elapsed();

            if event::poll(time_before_tick).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }

            if time_before_tick > Duration::new(0, 0) {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;

    loop {
        terminal.draw(|mut f| draw(&mut f, &mut display_data))?;
        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                // KeyCode::Char(c) => display_data.on_key(c),
                // KeyCode::Left => display_data.on_left(),
                // KeyCode::Up => display_data.on_up(),
                // KeyCode::Right => display_data.on_right(),
                // KeyCode::Down => display_data.on_down(),
                _ => {}
            },
            Event::Tick => {
                // data.on_tick();
            }
        }
        // if data.should_quit {
        //     break;
        // }
    }

    Ok(())
}
