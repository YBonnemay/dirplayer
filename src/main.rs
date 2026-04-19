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
mod input;
mod ui;
mod utils;
use cocotte::app::App;
use cocotte::sub_app::SubApp;
use cocotte::sub_app_view::SubAppViewTrait;
use crossterm::event::{Event, EventStream, KeyEvent};
use crossterm::{
    event::{self, poll, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use eyre::Result;
use futures::StreamExt;
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
use tokio::time::Interval;

#[derive(Debug, Clone)]
pub enum EventEnum {
    Tick,
    Key(KeyEvent),
}

#[derive(Default)]
pub enum Focus {
    Input,
}

#[derive(Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Active,
    #[default]
    Inactive,
    Cache,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct WorkingPath {
    pub path: String,
    pub status: Status,
}

#[derive(Default)]
pub struct AppState {
    input_string: String,
    should_quit: bool,
    focus: Focus,
}

struct EventPump {
    reader: EventStream,
    interval: tokio::time::Interval,
}

impl EventPump {
    fn new(fps: f32) -> Self {
        let period = Duration::from_secs_f32(1.0 / fps);
        Self {
            reader: EventStream::new(),
            interval: tokio::time::interval(period),
        }
    }

    async fn next(&mut self) -> Option<EventEnum> {
        tokio::select! {
            _ = self.interval.tick() => {
                // tick -> currently no app event
                Some(EventEnum::Tick)
            }
            Some(Ok(ev)) = self.reader.next() => {
                if let Event::Key(key) = ev {
                    Some(EventEnum::Key(key))
                } else {
                    None
                }
            }
        }
    }
}

cocotte::define_sub_apps! {
    event = EventEnum;
    state = AppState;
    Input(crate::input_sub_app::InputSubApp) => crate::input_sub_app::InputSubApp::new(),
    Display(crate::display_sub_app::DisplaySubApp) => crate::display_sub_app::DisplaySubApp::new(),
}

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

#[tokio::main]
pub async fn start() -> Result<()> {
    utils::config::get_home_dir();

    // events
    let mut event_pump = EventPump::new(2.0);
    let sub_apps = make_sub_apps();
    let mut app = App::new(sub_apps);
    let mut app_state = AppState::default();

    while let Some(mut app_event) = event_pump.next().await {
        // TODO implement key comparison for Ctrl+C here
        if let EventEnum::Key(key_event) = app_event {
            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && key_event.code == KeyCode::Char('c')
            {
                break;
            }
        }
        app.handle_input(&mut app_event, &mut app_state);
        app.draw()?;
        // terminal.draw(|frame| app.render(frame))?;
    }

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

fn main() -> Result<()> {
    let res = start();

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

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
