#[cfg(feature = "mpv")]
mod backend_mpv;
mod backend_rodio;
mod backend_trait;
mod constants;
mod directories;
mod player;
pub mod utils;
use constants::SongState;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use futures::StreamExt;
use log4rs::Config;
use log4rs::config::{Appender, Root};
use log4rs::{append::file::FileAppender, encode::pattern::PatternEncoder};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use utils::config;

#[derive(Debug, Clone)]
pub enum EventEnum {
    Tick,
    Setup,
    Key(KeyEvent),
}

/// A playback action emitted by a sub-app and consumed by [`player::Player`].
#[derive(Debug, Clone)]
pub enum PlayRequest {
    Play(PathBuf),
    Toggle,
    Next,
    Previous,
    CyclePlayMode,
}

#[derive(Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Active,
    #[default]
    Inactive,
    Cache,
}

pub struct AppState {
    /// Fuzzy filter applied to directories and files alike.
    pub filter: String,
    pub current_directory: PathBuf,
    /// Index of the highlighted line, directories first then files.
    pub line_index: usize,
    /// Playable files of the current directory, in display order.
    pub visible_files: Vec<PathBuf>,
    /// Line index of the first file, since directories own the rows above it.
    pub first_file_line: usize,
    pub play_request: Option<PlayRequest>,
    pub now_playing: Option<PathBuf>,
    pub song_state: SongState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            filter: String::default(),
            current_directory: config::get_set_config().working_directory,
            line_index: 0,
            visible_files: Vec::new(),
            first_file_line: 0,
            play_request: None,
            now_playing: None,
            song_state: SongState::Ended,
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
            event = self.reader.next() => match event {
                Some(Ok(Event::Key(key))) => Some(EventEnum::Key(key)),
                // Resizes and the like carry nothing for the sub-apps, but the
                // frame still has to be drawn against the new area. Only an
                // exhausted reader ends the loop.
                Some(Ok(_)) => Some(EventEnum::Tick),
                Some(Err(error)) => {
                    log::error!("Terminal event error: {error}");
                    Some(EventEnum::Tick)
                }
                None => None,
            },
        }
    }
}

cocotte::define_sub_apps! {
    event = EventEnum;
    state = AppState;
    Directories(directories::Directories) => directories::Directories::new(),
    Player(player::Player) => player::Player::new(),
}

#[tokio::main]
pub async fn start() -> Result<()> {
    log_setup();
    config::get_home_dir();

    // events
    let mut event_pump = EventPump::new(2.0);
    let mut app = make_app()?;
    let mut app_state = AppState::default();

    // App setup
    app.handle_input(&mut EventEnum::Setup, &mut app_state);

    while let Some(mut app_event) = event_pump.next().await {
        if let EventEnum::Key(key_event) = app_event
            && key_event.modifiers.contains(KeyModifiers::CONTROL)
            && (key_event.code == KeyCode::Char('c') || key_event.code == KeyCode::Char('d'))
        {
            break;
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
// TODO: unarchive and read
// TODO: read mpc
// TODO: display help
// TODO: home / end movements
// TODO: better event matrix
// TODO: better shortcut management
// TODO: volume control?
