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
mod utils;
use cocotte::app::App;
use cocotte::sub_app::SubApp;
use cocotte::sub_app_view::SubAppViewTrait;
use crossterm::event::{Event, EventStream, KeyEvent};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use eyre::Result;
use futures::StreamExt;
use itertools::Itertools;
use log4rs::Config;
use log4rs::config::{Appender, Root};
use log4rs::{append::file::FileAppender, encode::pattern::PatternEncoder};
use ratatui::{Terminal, backend::CrosstermBackend};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, stdout};
use std::panic::set_hook;
use std::panic::take_hook;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::Interval;
use utils::config::{self, get_set_config};

#[derive(Debug, Clone)]
pub enum EventEnum {
    Tick,
    Setup,
    Key(KeyEvent),
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum Focus {
    #[default]
    Directories,
    Files,
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
    pub directory_filter: String,
    pub file_filter: String,
    pub focus: Focus,
    pub current_directory: PathBuf,
}

impl AppState {
    pub fn default() -> Self {
        Self {
            directory_filter: String::default(),
            file_filter: String::default(),
            focus: Focus::Directories,
            current_directory: config::get_audio_dir(),
        }
    }
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
    Input(input::Input) => input::Input::new(),
    Directories(directories::Directories) => directories::Directories::new(),
}

fn switch_play_mode() {
    let mut config = get_set_config();
    // TODO : variant_count, enumerate PlayModes
    let play_mode = match config.play_mode {
        config::PlayMode::Queue => config::PlayMode::Random,
        config::PlayMode::Random => config::PlayMode::Queue,
    };

    config.play_mode = play_mode;
    config::update_config(&config);
}

#[tokio::main]
pub async fn start() -> Result<()> {
    log_setup();
    config::get_home_dir();

    // events
    let mut event_pump = EventPump::new(2.0);
    let sub_apps = make_sub_apps();
    let mut app = App::new(sub_apps);
    let mut app_state = AppState::default();

    // App setup
    app.handle_input(&mut EventEnum::Setup, &mut app_state);

    while let Some(mut app_event) = event_pump.next().await {
        if let EventEnum::Key(key_event) = app_event {
            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && (key_event.code == KeyCode::Char('c') || key_event.code == KeyCode::Char('d'))
            {
                break;
            }
        }
        app.handle_input(&mut app_event, &mut app_state);
        app.draw(&mut app_state)?;
    }

    Ok(())
}

fn log_setup() {
    let logfile = FileAppender::builder()
        .encoder(Box::<PatternEncoder>::default())
        .build("log/dirplayer.log")
        .unwrap();

    let config = config::get_set_config();

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
