use crate::AppState;
use crate::EventEnum;
use crate::PlayRequest;
use crate::backend_rodio::Rodio;
use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use crate::utils::config::{self, Config, PlayMode};
use crate::utils::directory::extension_file;
use cocotte::sub_app::SubApp;
use rand::{Rng, rng};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use std::path::Path;

/// The backend a file should be played with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendKind {
    Rodio,
    #[cfg(feature = "mpv")]
    Mpv,
}

impl BackendKind {
    fn for_path(path: &Path) -> Self {
        #[cfg(feature = "mpv")]
        {
            let pure_rust = extension_file(path).is_some_and(|extension| extension.is_pure_rust());
            if !pure_rust {
                return Self::Mpv;
            }
        }

        #[cfg(not(feature = "mpv"))]
        let _ = extension_file(path);

        Self::Rodio
    }
}

/// Owns the audio backend and turns [`PlayRequest`]s into playback.
pub struct Player {
    backend: Box<dyn AudioBackend>,
    backend_kind: BackendKind,
    play_mode: PlayMode,
    /// Whether a file was started at least once, so an `Ended` state means
    /// "the song is over" rather than "nothing ever played".
    started: bool,
    /// Files autoplay skipped over without managing to play any of them.
    autoplay_failures: usize,
}

impl Player {
    pub fn new() -> Self {
        Self {
            backend: Box::new(Rodio::new()),
            backend_kind: BackendKind::Rodio,
            play_mode: PlayMode::Queue,
            started: false,
            autoplay_failures: 0,
        }
    }

    /// Makes `kind` the current backend, reporting whether it is usable.
    ///
    /// The previous backend keeps its audio device until it is dropped, which
    /// happens as soon as the new one takes its place.
    fn use_backend(&mut self, kind: BackendKind) -> bool {
        if kind == self.backend_kind {
            return true;
        }

        match kind {
            BackendKind::Rodio => {
                self.backend = Box::new(Rodio::new());
            }
            #[cfg(feature = "mpv")]
            BackendKind::Mpv => match crate::backend_mpv::Mpv::new() {
                Some(mpv) => self.backend = Box::new(mpv),
                None => return false,
            },
        }

        self.backend_kind = kind;
        true
    }

    fn play(&mut self, path: &Path, app_state: &mut AppState) {
        let file_name = path.to_string_lossy().to_string();

        if self.backend.busy() {
            self.backend.silent_pause();
        }

        if let Some(index) = app_state.visible_files.iter().position(|p| p == path) {
            app_state.line_index = app_state.first_file_line + index;
        }
        app_state.now_playing = Some(path.to_path_buf());

        if !self.use_backend(BackendKind::for_path(path)) {
            log::error!("No backend available for {file_name}");
            // Autoplay must not walk the whole directory looking for a file
            // the missing backend could have played.
            self.started = false;
            app_state.song_state = SongState::Ended;
            return;
        }

        self.backend.start(&file_name);
        self.started = true;
        app_state.song_state = SongState::Playing;

        persist_current_file(&file_name);
    }

    /// Moves `offset` entries away from the file currently playing and plays it.
    ///
    /// A cursor sitting somewhere else than on the file playing wins over that
    /// order: moving forward then plays the highlighted file.
    fn step(&mut self, offset: isize, app_state: &mut AppState) {
        let length = app_state.visible_files.len();
        if length == 0 {
            return;
        }

        if offset > 0
            && let Some(path) = focused_file(app_state)
            && app_state.now_playing.as_deref() != Some(path.as_path())
        {
            self.play(&path, app_state);
            return;
        }

        let current = app_state.now_playing.as_ref().and_then(|playing| {
            app_state
                .visible_files
                .iter()
                .position(|path| path == playing)
        });

        let next = match (&self.play_mode, current) {
            (PlayMode::Random, _) if offset > 0 => rng().random_range(0..length),
            (_, Some(index)) => (index as isize + offset).rem_euclid(length as isize) as usize,
            (_, None) => 0,
        };

        let path = app_state.visible_files[next].clone();
        self.play(&path, app_state);
    }

    fn toggle(&mut self, app_state: &mut AppState) {
        self.backend.toggle();
        app_state.song_state = self.backend.state();
    }

    fn cycle_play_mode(&mut self) {
        self.play_mode = match self.play_mode {
            PlayMode::Queue => PlayMode::Random,
            PlayMode::Random => PlayMode::Queue,
        };

        let config = config::get_config();
        config::update_config(&Config {
            play_mode: self.play_mode,
            ..config
        });
    }

    fn handle_request(&mut self, request: PlayRequest, app_state: &mut AppState) {
        match request {
            PlayRequest::Play(path) => {
                if app_state.now_playing.as_deref() == Some(path.as_path()) {
                    self.toggle(app_state);
                } else {
                    self.play(&path, app_state);
                }
            }
            PlayRequest::Toggle => self.toggle(app_state),
            PlayRequest::Next => self.step(1, app_state),
            PlayRequest::Previous => self.step(-1, app_state),
            PlayRequest::CyclePlayMode => self.cycle_play_mode(),
        }
    }

    /// Plays the next file once the current one has ended on its own.
    fn autoplay(&mut self, app_state: &mut AppState) {
        let state = self.backend.state();
        app_state.song_state = state;

        if state != SongState::Ended {
            // Something is playing, so the files skipped to reach it no longer
            // count against the search for a playable one.
            self.autoplay_failures = 0;
            return;
        }

        if !self.started {
            return;
        }

        // A file that ends without ever playing could not be decoded. Walking
        // the whole list without finding a playable one means stopping, rather
        // than skipping through the directory several times a second.
        self.autoplay_failures += 1;
        if self.autoplay_failures > app_state.visible_files.len() {
            log::error!("No playable file found, stopping playback");
            self.started = false;
            self.autoplay_failures = 0;
            return;
        }

        self.step(1, app_state);
    }
}

/// The file under the cursor, if the cursor is on a file row at all.
fn focused_file(app_state: &AppState) -> Option<std::path::PathBuf> {
    let index = app_state
        .line_index
        .checked_sub(app_state.first_file_line)?;
    app_state.visible_files.get(index).cloned()
}

fn persist_current_file(file_name: &str) {
    let config = config::get_config();
    config::update_config(&Config {
        current_file: file_name.to_string(),
        ..config
    });
}

impl SubApp<EventEnum, AppState> for Player {
    fn handle_input(&mut self, event: &mut EventEnum, app_state: &mut AppState) {
        if let Some(request) = app_state.play_request.take() {
            self.handle_request(request, app_state);
        }

        match event {
            EventEnum::Setup => {
                self.play_mode = config::get_config().play_mode;
            }
            EventEnum::Tick => self.autoplay(app_state),
            EventEnum::Key(_) => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, app_state: &mut AppState) {
        let (symbol, color) = match app_state.song_state {
            SongState::Playing => ("▶", Color::Green),
            SongState::Paused => ("⏸", Color::Yellow),
            SongState::Ended => ("⏹", Color::DarkGray),
        };

        let playing = app_state
            .now_playing
            .as_deref()
            .map(|path| file_label(path, &app_state.current_directory))
            .unwrap_or_else(|| String::from("nothing playing"));

        let mode = match self.play_mode {
            PlayMode::Queue => "queue",
            PlayMode::Random => "random",
        };

        let line = Line::from(vec![
            Span::styled(
                format!("{symbol} "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(playing),
            Span::raw("  |  "),
            Span::styled(mode, Style::default().fg(Color::Cyan)),
        ]);

        // Only the top border, which doubles as the line closing the listing
        // above, and a padding lining the text up with the rows of that listing.
        let block = Block::default()
            .borders(Borders::TOP)
            .padding(Padding::horizontal(1));
        frame.render_widget(Paragraph::new(line).block(block), area);
    }

    fn constraints(&self) -> Constraint {
        Constraint::Length(2)
    }
}

/// Names the song the way the listing does, so a file found in a
/// subdirectory keeps that directory as a prefix.
fn file_label(path: &Path, current_directory: &Path) -> String {
    path.strip_prefix(current_directory)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
