mod directory_selector;
mod directory_watcher;

use crossbeam_channel::unbounded;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use directory_selector::DirectorySelector;
use directory_watcher::DirectoryWatcher;
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use tui::layout::Layout;
use tui::Frame;
use tui::{backend::CrosstermBackend, Terminal};

enum Event<I> {
    Input(I),
    Tick,
}

// https://blog.logrocket.com/rust-and-tui-building-a-command-line-interface-in-rust/
// cd /home/bonnemay/github/dirplayer/ ; cargo run --verbose
// cd /home/bonnemay/github/dirplayer/ ; RUST_BACKTRACE=1 cargo run --verbose
// println!("write_page{:#?}", lock.len());

#[derive(Serialize, Deserialize)]
struct Config {
    enhanced_graphics: bool,
    extensions: Vec<String>,
    tick_rate: String,
    working_directory: String,
}

pub struct Displayer<'a> {
    zone_index: i8,
    zone_number: i8,
    pub directory_path: DirectorySelector<'a>,
    // pub directory_watcher: DirectoryWatcher<'a>,
}

impl<'a> Displayer<'a> {
    pub fn new() -> Displayer<'a> {
        Displayer {
            zone_index: 0,
            zone_number: 0,
            directory_path: DirectorySelector::new(PathBuf::from(String::from("/"))),
            // directory_watcher: DirectoryWatcher::new(PathBuf::from(String::from("/"))),
        }
    }
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Displayer<'a> {
    pub fn move_zone(&mut self, zone_index_increment: i8) {
        self.zone_index = (self.zone_index + zone_index_increment).rem_euclid(self.zone_number);
    }

    pub fn process_event(&mut self, key_code: KeyCode, key_modifiers: KeyModifiers) {
        // self.zones[self.zone_index as usize].process_event(key_code, key_modifiers);

        if key_modifiers == KeyModifiers::CONTROL {
            match key_code {
                KeyCode::Up => {
                    self.move_zone(-1);
                }

                KeyCode::Down => {
                    self.move_zone(1);
                }

                _ => (),
            }
        } else {
            self.directory_path.process_event(key_code, key_modifiers);
        }
    }
}

fn get_config() -> Config {
    let json = include_str!("/home/bonnemay/github/dirplayer/src/config/config.json");
    serde_json::from_str::<Config>(json).unwrap()
}

fn draw<B: tui::backend::Backend>(f: &mut Frame<B>, displayer: &Displayer) {
    let constraints = displayer.directory_path.get_constraints();
    let chunks = Layout::default()
        .constraints(vec![constraints])
        .split(f.size());

    let displayable = displayer.directory_path.get_displayable();
    f.render_widget(displayable, chunks[0]);
}

pub fn get_path_lines(path: &Path, acc: &mut Vec<String>) {
    let paths = fs::read_dir(path).unwrap();
    for content in paths {
        let unwrapped_content = content.unwrap();
        if !unwrapped_content.path().is_dir() {
            acc.push(String::from(
                unwrapped_content.file_name().to_string_lossy(),
            ));
        } else {
            get_path_lines(&unwrapped_content.path(), acc);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start
    // let starting_directory = "/home/bonnemay/downloads/aa_inbox";
    let mut displayer = Displayer::new();
    displayer
        .directory_path
        .set_path(PathBuf::from(String::from(
            "/home/bonnemay/github/dirplayer",
        )));

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

    // Heartbeat for the  display. Will send each second or each key
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
        terminal.draw(|mut f| draw(&mut f, &displayer))?;
        match rx.recv()? {
            Event::Input(event) => {
                if event.modifiers == KeyModifiers::CONTROL
                    && (event.code == KeyCode::Char('q') || event.code == KeyCode::Char('d'))
                {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    // end
                    break;
                } else {
                    displayer.process_event(event.code, event.modifiers)
                }
            }
            Event::Tick => {
                // data.on_tick();
            }
        }
    }

    Ok(())
}
