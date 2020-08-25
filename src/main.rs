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
use std::cmp;
// use std::io::stdout;
use walkdir::WalkDir;

use std::sync::{Arc, RwLock};
use std::thread;
use std::{
    error::Error,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::time;
use tui::{backend::CrosstermBackend, Terminal};

// use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Tabs, Text};
use tui::Frame;

// use crate::demo::{ui, App};
mod Data;
mod util;

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

pub trait Zone {
    fn get_zone_offset_start(&self) -> i16;
    fn get_zone_offset_end(&self) -> i16;
    fn set_zone_offset_start(&mut self, weight: i16);
    fn set_zone_offset_end(&mut self, weight: i16);

    fn get_length(&self) -> i16;
    fn get_lines(&self) -> Vec<String>;
    // fn use_keystroke(&mut self, _: CEvent);
}

struct Zones {
    pub zones: Vec<Box<dyn Zone>>,
    // pub screen: AlternateScreen<RawTerminal<Stdout>>,
    pub current_zone: i16,
}

impl Zones {
    fn new() -> Zones {
        Zones {
            zones: Vec::new(),
            // screen: screen,
            current_zone: 0,
        }
    }

    fn get_lengths(&self) -> i16 {
        let zones = &self.zones;
        let mut lengths = 0;
        for existing_zone in zones {
            lengths += existing_zone.get_length();
        }
        lengths
    }

    fn add_zone(&mut self, mut zone: Box<dyn Zone>) {
        self.zones.push(zone)
    }

    fn display(&mut self) {}

    // fn flush(&mut self) {
    //     // self.screen.flush().unwrap();
    // }

    // fn set_current_zone(&mut self, current_zone: i16) {
    //     self.current_zone = current_zone
    // }

    // fn send_keystroke(&mut self, key: CEvent) {
    //     self.zones[self.current_zone as usize].use_keystroke(key);
    // }
}

struct MiddleZone {
    // https://users.rust-lang.org/t/heartbeat-in-a-thread-done-right/13596
    x_index: i16,
    zone_offset_start: i16,
    zone_offset_end: i16,
    start: i16,
    lines: Arc<RwLock<Vec<String>>>,
    lines_index: i16,
    // handle: Option<thread::JoinHandle<()>>,
}

impl MiddleZone {
    fn new() -> MiddleZone {
        MiddleZone {
            x_index: 0,
            zone_offset_start: 0,
            zone_offset_end: 0,
            start: 0,
            lines: Arc::new(RwLock::new(Vec::new())),
            lines_index: 0,
            // handle: None,
        }
    }

    // fn refresh_start(&self) {
    //     let lines = self.lines.clone();
    //     thread::spawn(move || loop {
    //         thread::sleep(time::Duration::from_millis(1000));
    //         let config = get_config();
    //         let working_directory = config.working_directory.clone();

    //         let new_lines = WalkDir::new(working_directory)
    //             .into_iter()
    //             .filter_map(Result::ok)
    //             .map(|e| String::from(e.file_name().to_string_lossy()))
    //             .collect::<Vec<String>>();

    //         let mut writable_lines = lines.write().unwrap();
    //         *writable_lines = new_lines;
    //     });
    // }
}

impl Zone for MiddleZone {
    fn get_zone_offset_start(&self) -> i16 {
        self.zone_offset_start
    }

    fn set_zone_offset_start(&mut self, zone_offset_start: i16) {
        self.zone_offset_start = zone_offset_start + 1;
    }

    fn get_zone_offset_end(&self) -> i16 {
        self.zone_offset_end
    }

    fn set_zone_offset_end(&mut self, zone_offset_end: i16) {
        self.zone_offset_end = zone_offset_end;
    }

    fn get_length(&self) -> i16 {
        // let terminal_size = termion::terminal_size().unwrap();
        // terminal_size.1 as i16 - self.get_zone_offset_start() + self.get_zone_offset_end()
        return 1;
    }

    fn get_lines(&self) -> Vec<String> {
        // let lines = self.lines.read().unwrap();
        // return lines.to_vec();
        let lines = self.lines.read().unwrap().to_vec();
        let lines_len = lines.len();
        let lines_index = self.lines_index as usize;
        let lines_start = cmp::min(lines_len, lines_index);
        let lines_end = cmp::min(lines_len, lines_index + self.get_length() as usize);

        // println!("lines_start {:#?}", lines_start);
        // println!("lines_end {:#?}", lines_end);
        // println!("self.get_length() {:#?}", self.get_length());

        return (lines[lines_start as usize..lines_end as usize]).to_vec();
    }

    // fn use_keystroke(&mut self, key: CEvent) {
    //     match key {
    //         CEvent::Down => {
    //             self.lines_index = self.lines_index + 1;
    //         }
    //         CEvent::Up => {
    //             if self.lines_index > 0 {
    //                 self.lines_index = self.lines_index - 1;
    //             }
    //         }
    //         _ => {}
    //     }
    // }
}

// use std::io::Write::flush as Wflush;

use std::io::{stdout, Write};

use crossbeam_channel::unbounded;
use notify::{watcher, RecursiveMode, Watcher};
use std::path::Path;
use std::path::PathBuf;
// use std::sync::mpsc::channel;

// A DataSource represents data that will de displayed. A DirectoryDataSource is a watched directory
// A Displayer

pub trait DataSource<T> {
    fn get_lines(&self) -> Vec<T>;
}

// THis will represent a Directory for us

struct Directory {
    path: Arc<RwLock<PathBuf>>,
    watcher: notify::INotifyWatcher,
    walk_dir: WalkDir,
    sender: crossbeam_channel::Sender<std::result::Result<notify::Event, notify::Error>>,
    receiver: crossbeam_channel::Receiver<std::result::Result<notify::Event, notify::Error>>,
    lines: Arc<RwLock<Vec<String>>>,
}

impl Directory {
    fn new(pathbuf: PathBuf) -> Directory {
        let (sender, receiver) = unbounded();
        // Automatically select the best implementation for your platform.
        let mut watcher = watcher(sender.clone(), Duration::from_secs(1)).unwrap();

        let path = Arc::new(RwLock::new(pathbuf.clone()));
        // Watching directory.
        watcher
            .watch(pathbuf.clone(), RecursiveMode::Recursive)
            .unwrap();

        Directory {
            path,
            watcher,
            walk_dir: WalkDir::new(pathbuf),
            sender: sender,
            receiver,
            lines: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // fn listen(self) {}

    fn refresh_lines(&self) {
        let lines = self.lines.clone();
        let path = self.path.clone();

        let path_read = &*(path.read().unwrap());
        // let new_path = &*path_read;
        let new_lines = WalkDir::new(path_read)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| String::from(e.file_name().to_string_lossy()))
            .collect::<Vec<String>>();

        let mut writable_lines = lines.write().unwrap();
        *writable_lines = new_lines;
    }

    fn listen(&self) {
        // Listening to changes.
        let receiver = self.receiver.clone();
        let lines = self.lines.clone();
        let path = self.path.clone();

        thread::spawn(move || loop {
            match receiver.recv() {
                Ok(event) => {
                    let path_read = &*(path.read().unwrap());
                    let new_lines = WalkDir::new(path_read)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .map(|e| String::from(e.file_name().to_string_lossy()))
                        .collect::<Vec<String>>();
                    let mut writable_lines = lines.write().unwrap();
                    *writable_lines = new_lines;

                    // println!("{:?}", event);
                }
                Err(e) => println!("watch error: {:?}", e),
            };
            // println!("thread loopng");
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

    // let displayable_lines = display_data.directory.lines.read().unwrap();
    // let text_lines = displayable_lines.iter().map(|e| Text::raw(e));
    // let text_lines = List::new(text_lines);

    f.render_widget(text_lines, chunks[0])
    // lines.iter().map(|e| Text::raw(e)).collect()

    // let new_lines = WalkDir::new(working_directory)
    //     .into_iter()
    //     .filter_map(Result::ok)
    //     .map(|e| String::from(e.file_name().to_string_lossy()))
    //     .collect::<Vec<String>>();

    // .map(|e| tui::widgets::Text::raw(e.file_name().to_string_lossy()));

    // let tasks = List::new(lines);
    // f.render_widget(tasks, chunks[0]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start
    let mut directory = Directory::new(PathBuf::from("/home/bonnemay/tests/src"));
    directory.refresh_lines();
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
