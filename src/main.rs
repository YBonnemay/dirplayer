// extern crate argh;
// extern crate crossbeam_channel;
// extern crate crossterm;
// extern crate fuzzy_matcher;
// extern crate notify;
// extern crate serde;
// extern crate serde_json;
// extern crate tui;
// extern crate walkdir;

// https://blog.logrocket.com/rust-and-tui-building-a-command-line-interface-in-rust/
#[macro_use]
extern crate serde_derive;

// real time updating of the data

// cd /home/bonnemay/github/dirplayer/ ; cargo run --verbose
// cd /home/bonnemay/github/dirplayer/ ; RUST_BACKTRACE=1 cargo run --verbose
// println!("write_page{:#?}", lock.len());

use crate::zone::Zone;
use crossbeam_channel::unbounded;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::thread;
use std::time::{Duration, Instant};
use tui::layout::{Constraint, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{List, Paragraph, Tabs};
use tui::{backend::CrosstermBackend, buffer::Buffer, Terminal};
use tui::{widgets::Widget, Frame};
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

use std::path::PathBuf;

mod directory;
use directory::Directory;

mod zone;

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

pub enum DirectoryPathStates {
    Default,
    Tabbing,
}

pub struct DirectoryPath {
    pub matcher: fuzzy_matcher::skim::SkimMatcherV2,
    pub path: PathBuf,
    pub completions: Vec<String>,
    pub state: DirectoryPathStates,
    pub filter: String,
    pub rotate_idx: i32,
}

pub fn get_path_completions(path: &PathBuf) -> Vec<String> {
    WalkDir::new(path)
        .into_iter()
        .filter(|e| e.as_ref().unwrap().metadata().unwrap().is_dir())
        .map(|e| String::from(e.unwrap().file_name().to_string_lossy()))
        .collect::<Vec<String>>()
}

impl<'a> DirectoryPath {
    pub fn new(path: PathBuf) -> DirectoryPath {
        let completions = get_path_completions(&path);
        DirectoryPath {
            path,
            completions,
            state: DirectoryPathStates::Default,
            // filter: String::from(""),
            filter: String::from(""),
            matcher: SkimMatcherV2::default(),
            rotate_idx: 0,
        }
    }
}

pub enum StyleMode {
    Bold,
    Raw,
}

fn get_style(is_bold: bool) -> Style {
    if is_bold {
        return Style::default().add_modifier(Modifier::BOLD);
    }
    Style::default()
}

// fn string_to_styled_text(raw_string: String, mut indices: Vec<usize>) -> Vec<Span<'static>> {
fn string_to_styled_text(raw_string: String, mut indices: Vec<usize>) -> Spans<'static> {
    let bold_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![];

    for (i, c) in raw_string.chars().enumerate() {
        if !indices.is_empty() && i == indices[0] {
            indices.drain(0..1);
            spans.push(Span::styled(String::from(c), bold_style));
        } else {
            spans.push(Span::raw(String::from(c)));
        }
    }

    Spans::from(spans)
}

impl<'a> Zone for DirectoryPath {
    fn get_displayable(&self) -> Tabs {
        let texts: Vec<Spans> = vec![];

        let mut filtered_completions: Vec<Spans> =
            self.completions.iter().cloned().fold(texts, |mut acc, e| {
                if self.filter.is_empty() {
                    let spans = string_to_styled_text(e, vec![]);
                    acc.push(spans);
                    return acc;
                }

                if let Some((score, indices)) = self.matcher.fuzzy_indices(&e, &self.filter) {
                    if score > 0 {
                        let spans = string_to_styled_text(e, indices);
                        acc.push(spans);
                    }
                }

                acc
            });

        if !filtered_completions.is_empty() {
            // println!("\nfiltered_completions{:#?}", self.rotate_idx);
            let rotate = (self.rotate_idx * 2).rem_euclid(filtered_completions.len() as i32);
            filtered_completions.rotate_right(rotate as usize);
        }

        Tabs::new(filtered_completions)
    }

    fn get_constraints(&self) -> Constraint {
        Constraint::Length(1)
    }

    fn process_event(&mut self, key_code: KeyCode, key_modifiers: KeyModifiers) {
        match key_code {
            // KeyCode::Tab => match self.state {
            //     DirectoryPathStates::Default => {}
            //     _ => println!("unknown state"),
            // },
            KeyCode::Tab => {
                self.rotate_idx += 1;
            }
            KeyCode::Backspace => {
                let mut chars = self.filter.chars();
                chars.next_back();
                chars.next_back();
                self.filter = chars.as_str().to_string();
            }
            KeyCode::Char(c) => {
                self.filter = format!("{}{}", self.filter, c);
            }
            _ => {}
        }
    }
}

struct Displayer<'a> {
    // pub directory_path: DirectoryPath,
    // pub directory: Directory,
    zone_index: i8,
    zone_number: i8,
    pub zones: Vec<&'a mut dyn Zone>,
}

impl<'a> Displayer<'a> {
    pub fn new() -> Displayer<'a> {
        return Displayer {
            zones: Vec::new(),
            zone_index: 0,
            zone_number: 0,
        };
    }

    pub fn push_zone(&mut self, zone: &'a mut dyn Zone) {
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
        f.render_widget(displayable, chunks[idx]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start
    // let starting_directory = "/home/bonnemay/downloads/aa_inbox";
    let starting_directory = "/home/bonnemay/github/dirplayer";
    let mut directory_path = DirectoryPath::new(PathBuf::from(&starting_directory));
    let directory = Directory::new(PathBuf::from(&starting_directory));
    // let directory = Directory::new(PathBuf::from("/home/bonnemay/tests/src"));
    directory.refresh_lines();
    directory.listen();
    let mut displayer = Displayer::new();
    displayer.push_zone(&mut directory_path);
    // displayer.push_zone(Box::new(directory));

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
            Event::Input(event) => {
                match event.code {
                    KeyCode::Char('q') => {
                        if event.modifiers == KeyModifiers::CONTROL {
                            disable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                LeaveAlternateScreen,
                                DisableMouseCapture
                            )?;
                            terminal.show_cursor()?;
                            break;
                        } else {
                            displayer.process_event(event.code, event.modifiers)
                        }
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
                }
            }
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
