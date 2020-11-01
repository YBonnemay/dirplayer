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

// real time updating of the data

// cd /home/bonnemay/github/dirplayer/ ; cargo run --verbose
// cd /home/bonnemay/github/dirplayer/ ; RUST_BACKTRACE=1 cargo run --verbose
// println!("write_page{:#?}", lock.len());

use crossbeam_channel::unbounded;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::thread;
use std::time::{Duration, Instant};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Tabs, Text};
use tui::{backend::CrosstermBackend, buffer::Buffer, Terminal};
use tui::{widgets::Widget, Frame};
use walkdir::{DirEntry, WalkDir};

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

use std::path::PathBuf;

mod directory;
use directory::Directory;

mod zone;
use zone::Zone;

struct Label<'a> {
    text: &'a str,
}

impl<'a> Default for Label<'a> {
    fn default() -> Label<'a> {
        Label { text: "" }
    }
}

impl<'a> Widget for Label<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.left(), area.top(), self.text, Style::default());
    }
}

impl<'a> Label<'a> {
    fn text(mut self, text: &'a str) -> Label<'a> {
        self.text = text;
        self
    }
}

pub struct DirectoryPath {
    pub path: PathBuf,
    pub completions: String,
}

impl DirectoryPath {
    pub fn new(path: PathBuf) -> DirectoryPath {
        let completions = String::from(" dummy ");
        return DirectoryPath { path, completions };
    }

    pub fn set_path(&mut self, path: String) {
        self.path = PathBuf::from(path);
    }

    // pub fn get_path_string(self) -> &String {
    //     String::from(self.path.to_string_lossy())
    // }

    pub fn get_path_completions(&self) -> std::vec::Vec<std::string::String> {
        WalkDir::new(&self.path)
            .into_iter()
            .filter(|e| e.as_ref().unwrap().metadata().unwrap().is_dir())
            .map(|e| String::from(e.unwrap().file_name().to_string_lossy()))
            .collect::<Vec<String>>()
    }
}

impl Zone for DirectoryPath {
    fn get_displayable(&self) -> Vec<String> {
        vec![format!(
            "{} | {}",
            self.path.clone().to_string_lossy().to_string(),
            self.completions
        )]
    }

    fn get_constraints(&self) -> Constraint {
        Constraint::Length(1)
    }

    fn process_event(&mut self, key_code: KeyCode, key_modifiers: KeyModifiers) {
        match key_code {
            KeyCode::Tab => {
                let completions = self.get_path_completions().join(" | ");
                self.completions = completions
            }
            _ => println!("unknown key"),
        }
    }
}

struct Displayer {
    // pub directory_path: DirectoryPath,
    // pub directory: Directory,
    zone_index: i8,
    zone_number: i8,
    pub zones: Vec<Box<dyn Zone>>,
}

impl Displayer {
    pub fn new() -> Displayer {
        return Displayer {
            zones: Vec::new(),
            zone_index: 0,
            zone_number: 0,
        };
    }

    pub fn push_zone(&mut self, zone: Box<dyn Zone>) {
        self.zones.push(zone);
        self.zone_number = self.zone_number + 1;
    }

    pub fn move_zone(&mut self, zone_index_increment: i8) {
        self.zone_index = (self.zone_index + zone_index_increment).rem_euclid(self.zone_number);
    }

    pub fn process_event(&mut self, key_code: KeyCode, key_modifiers: KeyModifiers) {
        self.zones[self.zone_index as usize].process_event(key_code, key_modifiers);
    }
}

fn draw<B: tui::backend::Backend>(f: &mut Frame<B>, displayer: &mut Displayer) {
    // let chunks = Layout::default().constraints(constraints).split(f.size());
    let zones = &displayer.zones;

    let constraints = zones
        .iter()
        .map(|e| e.get_constraints())
        .collect::<Vec<Constraint>>();

    let chunks = Layout::default().constraints(constraints).split(f.size());

    for (idx, zone) in zones.iter().enumerate() {
        let displayable = zone.get_displayable();
        let displayable = displayable.iter().map(|e| Text::raw(e));
        let displayable = List::new(displayable);
        f.render_widget(displayable, chunks[idx]);
    }

    // let displayable_path = &displayer.directory_path.path.to_string_lossy();
    // let displayable_path = Label::default().text(displayable_path);

    // let displayable_lines = displayer.directory.lines.clone();

    // let displayable_lines = displayable_lines.read().unwrap().to_vec();
    // let text_lines = displayable_lines.iter().map(|e| Text::raw(e));
    // let text_lines = List::new(text_lines);

    // f.render_widget(displayable_path, chunks[0]);
    // f.render_widget(text_lines, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start
    let starting_directory = "/home/bonnemay/downloads/aa_inbox";
    let directory_path = DirectoryPath::new(PathBuf::from(&starting_directory));
    let directory = Directory::new(PathBuf::from(&starting_directory));
    // let directory = Directory::new(PathBuf::from("/home/bonnemay/tests/src"));
    directory.refresh_lines();
    directory.listen();
    let mut displayer = Displayer::new();
    displayer.push_zone(Box::new(directory_path));
    displayer.push_zone(Box::new(directory));

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
        terminal.draw(|mut f| draw(&mut f, &mut displayer))?;
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

                // Displayyer wide
                KeyCode::Up => {
                    if event.modifiers == KeyModifiers::CONTROL {
                        displayer.move_zone(-1);
                    }
                }

                KeyCode::Down => {
                    if event.modifiers == KeyModifiers::CONTROL {
                        displayer.move_zone(1);
                    }
                }

                // Zone wide
                _ => displayer.process_event(event.code, event.modifiers),
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
