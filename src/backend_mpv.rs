use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use libmpv2::events::{Event, PropertyData};
use libmpv2::{Format, Mpv as MpvBackend};
use log::{debug, error};

// http://mpv.io/manual/master/#options
// http://mpv.io/manual/master/#list-of-input-commands
// https://mpv.io/manual/master/#properties

pub struct Mpv {
    player: MpvBackend,
    /// Playback state, kept from mpv's own events rather than polled.
    ///
    /// `loadfile` is asynchronous, so right after a file is requested mpv is
    /// still idle. Polling `idle-active` would report the song as ended and
    /// make the player skip to the next one.
    state: SongState,
    /// Number of files handed to mpv, against [`Self::ends_seen`]. Replacing a
    /// playing file makes mpv end it, and that end must not be mistaken for
    /// the new file finishing.
    starts_requested: u64,
    ends_seen: u64,
}

impl Mpv {
    /// Builds a backend, or `None` when libmpv is unavailable or rejects the
    /// initial configuration. Playback is optional: the interface must keep
    /// running without it.
    pub fn new() -> Option<Self> {
        let player = match MpvBackend::new() {
            Ok(player) => player,
            Err(e) => {
                error!("Could not initialize mpv: {e}");
                return None;
            }
        };

        let mut mpv = Self {
            player,
            state: SongState::Ended,
            starts_requested: 0,
            ends_seen: 0,
        };

        // Audio only, and let files end on their own so an end event is sent.
        mpv.set_property("vo", "null");
        mpv.set_property("keep-open", "no");

        if let Err(e) = mpv
            .player
            .event_context_mut()
            .observe_property("pause", Format::Flag, 0)
        {
            error!("Could not observe the mpv pause property: {e}");
        }

        Some(mpv)
    }

    fn set_property<T: libmpv2::SetData>(&self, name: &str, value: T) {
        if let Err(e) = self.player.set_property(name, value) {
            error!("Could not set the mpv {name} property: {e}");
        }
    }

    /// Consumes pending mpv events to keep [`Self::state`] current.
    fn drain_events(&mut self) {
        loop {
            // A zero timeout polls: the interface thread must not block here.
            match self.player.event_context_mut().wait_event(0.0) {
                Some(Ok(Event::EndFile(reason))) => {
                    self.ends_seen += 1;
                    debug!("mpv ended a file: {reason:?}");
                    if self.ends_seen >= self.starts_requested {
                        self.state = SongState::Ended;
                    }
                }
                Some(Ok(Event::StartFile)) => {
                    self.state = SongState::Playing;
                }
                Some(Ok(Event::PropertyChange {
                    name: "pause",
                    change: PropertyData::Flag(paused),
                    ..
                })) => {
                    // Ignore pause events once the file is over, so a paused
                    // state cannot mask the end of the queue.
                    if self.state != SongState::Ended {
                        self.state = if paused {
                            SongState::Paused
                        } else {
                            SongState::Playing
                        };
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    error!("mpv event error: {e}");
                    break;
                }
                None => break,
            }
        }
    }
}

/// Quotes a path for mpv's command parser.
///
/// [`libmpv2::Mpv::command`] joins its arguments with spaces and hands the
/// result to `mpv_command_string`, which parses it again, so paths have to
/// survive that round trip.
fn quote(argument: &str) -> String {
    let escaped = argument.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

impl AudioBackend for Mpv {
    fn start(&mut self, file_name: &str) {
        debug!("starting {file_name}");

        // Count the file before loading it: replacing the current one makes
        // mpv end it, and that end belongs to the previous file.
        self.starts_requested += 1;

        if let Err(e) = self
            .player
            .command("loadfile", &[&quote(file_name), "replace"])
        {
            error!("Could not load {file_name}: {e}");
            self.ends_seen = self.starts_requested;
            self.state = SongState::Ended;
            return;
        }

        self.state = SongState::Playing;
        self.resume();
    }

    fn pause(&mut self) {
        debug!("paused");
        self.set_property("pause", true);
        self.state = SongState::Paused;
    }

    fn silent_pause(&mut self) {
        self.set_property("pause", true);
        self.state = SongState::Paused;
    }

    fn resume(&mut self) {
        debug!("resumed");
        self.set_property("pause", false);
        self.state = SongState::Playing;
    }

    fn busy(&mut self) -> bool {
        self.state() != SongState::Ended
    }

    fn state(&mut self) -> SongState {
        self.drain_events();
        self.state
    }

    fn toggle(&mut self) {
        match self.state() {
            SongState::Paused => self.resume(),
            SongState::Playing => self.pause(),
            SongState::Ended => debug!("toggling ended song"),
        }
    }
}
