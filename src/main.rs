#![feature(backtrace_frames)]
mod app;
#[cfg(feature = "mpv")]
mod backend_mpv;
mod backend_rodio;
pub mod backend_trait;
mod constants;
mod directories;
mod echo_area;
mod files;
mod ui;
mod utils;
use app::App;
use crossterm::{
    event::{self, poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use log4rs::config::{Appender, Root};
use log4rs::Config;
use log4rs::{append::file::FileAppender, encode::pattern::PatternEncoder};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io::{self, stdout};
use std::panic::set_hook;
use std::panic::take_hook;
use std::{backtrace::Backtrace, time::Duration};

fn switch_play_mode() {
    let mut config = utils::config::get_config();
    // TODO : variant_count, enumerate PlayModes
    let play_mode = match config.play_mode {
        utils::config::PlayMode::Queue => utils::config::PlayMode::Random,
        utils::config::PlayMode::Random => utils::config::PlayMode::Queue,
    };

    config.play_mode = play_mode;
    utils::config::update_config(&config);
}

pub fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    execute!(stdout(), DisableMouseCapture)?;

    let trace = Backtrace::capture();

    trace
        .frames()
        .iter()
        .map(|frame| {
            log::error! {"panic occurred: {:?}", frame};
        })
        .collect_vec();
    Ok(())
}

fn main_app() -> Result<(), Box<dyn std::error::Error>> {
    utils::config::get_home_dir();

    enable_raw_mode()?; // crossterm terminal setup
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?; // crossterm event setup
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    let mut app = App::new();

    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();
        original_hook(panic_info);
    }));

    terminal.clear()?;

    loop {
        if poll(Duration::from_millis(1000))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                // Only key presses.
                if key.kind != event::KeyEventKind::Press && key.kind != event::KeyEventKind::Repeat
                {
                    // Skip events that are not KeyEventKind::Press
                    continue;
                }

                let frame = terminal.get_frame();
                match app.handle_event(&frame, &event) {
                    Ok(_) => {
                        terminal.draw(|f| ui::ui(f, &mut app))?;
                    }
                    Err(_) => {
                        break;
                        // Manage handling err
                    }
                }

                log::debug!("Event done");
            }
        } else {
            app.handle_tick();
            terminal.draw(|f| ui::ui(f, &mut app))?;
        }
    }

    // Restore the terminal and close application
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.clear()?;
    crossterm::terminal::disable_raw_mode()?;
    terminal.show_cursor()?;

    Ok(())
}

fn log_setup() {
    let logfile = FileAppender::builder()
        .encoder(Box::<PatternEncoder>::default())
        .build("log/dirplayer.log")
        .unwrap();

    let config = utils::config::get_set_config();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(config.log_level))
        .unwrap();

    log4rs::init_config(config).unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log_setup();
    match main_app() {
        Ok(_) => {}
        Err(_err) => {
            let stdout = stdout();
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.clear()?;
            crossterm::terminal::disable_raw_mode()?;
            terminal.show_cursor()?;
        }
    };

    Ok(())

    // TODO: show next
    // TODO: remove epiubs from files
    // TODO: unarchive and read
    // TODO: read mpc
    // TODO: more info in echo area. Maybe refresh not on tick but on event
    // TODO: fix filtering of songs (should be no rar, etc.)
    // TODO: display filterg
    // TODO: display help
    // TODO: movement to echo area
    // TODO: home / end movements
    // TODO: better event matrix
    // TODO: better shortcut management
    // TODO: volume control?
}
