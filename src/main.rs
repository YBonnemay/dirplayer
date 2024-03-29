mod app;
#[cfg(feature = "mpv")]
mod backend_mpv;
mod backend_rodio;
pub mod backend_trait;
mod constants;
mod directory_selector;
mod directory_watcher;
mod echo_area;
mod utils;

use app::App;
use crossbeam_channel::unbounded;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CrossTermEvent, KeyCode,
        KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use tui::layout::{Constraint, Layout};
use tui::Frame;
use tui::{backend::CrosstermBackend, Terminal};

enum Event<I> {
    Input(I),
    Tick,
}

// fn draw<'a, B: tui::backend::Backend>(f: &mut Frame<B>, app: &'a mut App<'a>) {
fn draw<B: tui::backend::Backend>(f: &mut Frame<B>, app: &mut App) {
    // let constraints = app.directory_selector.constraints;
    // let test = f.size();
    let chunks = Layout::default()
        .constraints(vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(f.size());

    app.directory_selector
        .draw_directory(f, chunks[0], &app.path);
    app.directory_watcher.draw_directory(f, chunks[1]);
    app.echo_area.draw(f, chunks[2]);
}

fn switch_play_mode() {
    let mut config = utils::config::get_config();
    // TODO : variant_count, enumerate PlayModes
    let play_mode = match config.play_mode {
        utils::config::PlayMode::Queue => utils::config::PlayMode::Random,
        utils::config::PlayMode::Random => utils::config::PlayMode::Queue,
    };

    config.play_mode = play_mode;
    utils::config::update_config(config);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: graceful stop
    // TODO: better filter diacritics. Better filter algo too.
    // TODO: more info in echo area. Maybe refresh not on tick but on event
    // TODO: fix filtering of songs (should be no rar, etc.)
    // TODO: display filterg
    // TODO: display help
    // TODO: movement to echo area
    // TODO: home / end movements
    // TODO: better event matrix
    // TODO: better shortcut management
    // TODO: volume control?
    // TODO: use scoped threads

    let config = utils::config::get_set_config();
    let mut app = App::new();
    let path = PathBuf::from(config.working_directories.front().unwrap());
    app.directory_selector.update_selector(&path);
    enable_raw_mode()?; // crossterm terminal setup
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?; // crossterm event setup
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

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
                // CrossTermEvent happened (keyboard, mouse, resize)
                match event::read().unwrap() {
                    CrossTermEvent::Key(key) => {
                        tx.send(Event::Input(key)).unwrap();
                    }
                    CrossTermEvent::Resize(_, _) => {
                        tx.send(Event::Tick).unwrap();
                    }
                    _ => (),
                }
            }

            // If here because event, we do nothing.
            // If here because timeout, we send tick.
            if last_tick.elapsed() >= time_before_tick {
                tx.send(Event::Tick).unwrap();
            }
            last_tick = Instant::now();
        }
    });

    terminal.clear()?;

    loop {
        terminal.draw(|f| draw(f, &mut app))?;
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
                } else if event.modifiers == KeyModifiers::CONTROL
                    && (event.code == KeyCode::Char('t'))
                {
                    switch_play_mode();
                } else {
                    app.process_event(event.code, event.modifiers, &terminal);
                }
            }
            Event::Tick => {
                app.process_tick();
            }
        }
    }

    Ok(())
}
