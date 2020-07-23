extern crate argh;
extern crate crossbeam_channel;
extern crate crossterm;
extern crate serde;
extern crate serde_json;
extern crate tui;
extern crate walkdir;

#[macro_use]
extern crate serde_derive;

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
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::time;

use tui::{backend::CrosstermBackend, Terminal};
// use tui::{backend::CrosstermBackend, Terminal};

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

    fn refresh_start(&self) {
        let lines = self.lines.clone();
        thread::spawn(move || loop {
            thread::sleep(time::Duration::from_millis(1000));
            let config = get_config();
            let working_directory = config.working_directory.clone();

            let new_lines = WalkDir::new(working_directory)
                .into_iter()
                .filter_map(Result::ok)
                .map(|e| String::from(e.file_name().to_string_lossy()))
                .collect::<Vec<String>>();

            let mut writable_lines = lines.write().unwrap();
            *writable_lines = new_lines;
        });
    }
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

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let config = get_config();

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup input handling
    let (tx, rx) = mpsc::channel();
    let tick_rate_u64 = config.tick_rate.parse().unwrap();
    let tick_rate = Duration::from_millis(tick_rate_u64);

    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeBeforeTick = tick_rate - last_tick.elapsed();

            if event::poll(timeBeforeTick).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }

            if timeBeforeTick <= Duration::new(0, 0) {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    let mut data = Data::Data::new();
    terminal.clear()?;

    loop {
        terminal.draw(|mut f| draw(&mut f, &mut data))?;
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
                KeyCode::Char(c) => data.on_key(c),
                KeyCode::Left => data.on_left(),
                KeyCode::Up => data.on_up(),
                KeyCode::Right => data.on_right(),
                KeyCode::Down => data.on_down(),
                _ => {}
            },
            Event::Tick => {
                data.on_tick();
            }
        }
        if data.should_quit {
            break;
        }
    }
    Ok(())
}

pub fn draw<B: tui::backend::Backend>(f: &mut Frame<B>, data: &mut Data::Data) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());
    let tabs = Tabs::default()
        .block(Block::default().borders(Borders::ALL).title("title"))
        .titles(&data.tabs.titles)
        .style(Style::default().fg(Color::Green))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(data.tabs.index);
    f.render_widget(tabs, chunks[0]);
    match data.tabs.index {
        0 => draw_first_tab(f, data, chunks[1]),
        _ => {}
    };
}

fn draw_first_tab<B>(f: &mut Frame<B>, data: &mut Data::Data, area: Rect)
where
    B: tui::backend::Backend,
{
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(7),
                Constraint::Min(7),
                Constraint::Length(7),
            ]
            .as_ref(),
        )
        .split(area);
    draw_charts(f, data, chunks[1]);
}

fn draw_charts<B>(f: &mut Frame<B>, data: &mut Data::Data, area: Rect)
where
    B: tui::backend::Backend,
{
    let constraints = if data.show_chart {
        vec![Constraint::Percentage(50), Constraint::Percentage(50)]
    } else {
        vec![Constraint::Percentage(100)]
    };
    let chunks = Layout::default()
        .constraints(constraints)
        .direction(Direction::Horizontal)
        .split(area);
    {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);
        {
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .direction(Direction::Horizontal)
                .split(chunks[0]);

            // Draw tasks
            let tasks = data.tasks.items.iter().map(|i| Text::raw(*i));
            let tasks = List::new(tasks)
                .block(Block::default().borders(Borders::ALL).title("List"))
                .highlight_style(Style::default().fg(Color::Yellow).modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            f.render_stateful_widget(tasks, chunks[0], &mut data.tasks.state);
        }
    }
}
