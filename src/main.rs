mod app;
mod backend_mpv;
mod backend_rodio;
pub mod backend_trait;
mod constants;
mod directory_watcher;
mod handlers;

use app::App;
use crossbeam_channel::unbounded;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use tui::layout::{Constraint, Layout};
use tui::Frame;
use tui::{backend::CrosstermBackend, Terminal};

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
    serde_json::from_str::<Config>(json).unwrap()
}

fn draw<B: tui::backend::Backend>(f: &mut Frame<B>, app: &App) {
    // let constraints = app.directory_selector.constraints;
    // let test = f.size();
    let chunks = Layout::default()
        .constraints(vec![Constraint::Length(1), Constraint::Percentage(100)])
        .split(f.size());

    let displayable_directories = handlers::selector::get_displayable(app);
    f.render_widget(displayable_directories, chunks[0]);
    app.directory_watcher.draw_directory(f, chunks[1]);
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
    let mut app = App::new();
    let path = PathBuf::from(String::from("/home/bonnemay/data/music_test"));
    app.set_path(path);

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
        terminal.draw(|f| draw(f, &app))?;
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
                    let path = app.path.clone();
                    app.set_path(path);
                    app.process_event(event.code, event.modifiers);
                }
            }
            Event::Tick => {
                app.process_tick();
            }
        }
    }

    Ok(())
}
