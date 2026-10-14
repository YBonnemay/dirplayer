use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use crossbeam_channel::unbounded;
use log::{debug, error};
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, RwLock};
use std::thread;

enum EventType {
    Start,
    Play,
    Pause,
    SilentPause,
    Tick,
}

struct Event {
    event_type: EventType,
    file_name: String,
}

pub struct Rodio {
    sender: crossbeam_channel::Sender<Event>,
    state: Arc<RwLock<SongState>>,
}

/// Opens a decoder for `file_name`, returning `None` for unreadable or
/// undecodable files. Decoding is isolated because some backend decoders panic
/// on malformed input, which would otherwise take the audio thread down.
fn open_decoder(file_name: &str) -> Option<Decoder<BufReader<File>>> {
    let file = match File::open(file_name) {
        Ok(file) => file,
        Err(e) => {
            error!("Could not open {file_name}: {e}");
            return None;
        }
    };

    let source = BufReader::new(file);
    debug! {"source: {:?}", source}

    match catch_unwind(AssertUnwindSafe(|| Decoder::new(source))) {
        Ok(Ok(decoder)) => Some(decoder),
        Ok(Err(e)) => {
            error!("Could not decode {file_name}: {e}");
            None
        }
        Err(_) => {
            error!("Decoder panicked on {file_name}");
            None
        }
    }
}

impl Rodio {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        let state = Arc::new(RwLock::new(SongState::Ended));
        let state_new = state.clone();

        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let mut sink = rodio::Sink::try_new(&stream_handle).unwrap();
            loop {
                let event: Event = match receiver.recv() {
                    Ok(e) => e,
                    Err(err) => {
                        // The sender is gone: the application is shutting down.
                        debug!("Stopping audio thread: {err}");
                        sink.stop();
                        return;
                    }
                };
                match event.event_type {
                    EventType::Start => {
                        debug! {"Start : switched to {} {}", SongState::Playing, event.file_name};
                        *state.write().unwrap() = SongState::Playing;
                        if !sink.empty() {
                            debug! {"cleaning sink"}
                            sink.stop();
                            sink = rodio::Sink::try_new(&stream_handle).unwrap();
                        }

                        match open_decoder(&event.file_name) {
                            Some(decoder) => {
                                sink.append(decoder);
                                sink.play();
                            }
                            None => {
                                *state.write().unwrap() = SongState::Ended;
                            }
                        }

                        debug! {"Decoder done"};

                        // echo_area_sender
                        //     .send(format!(
                        //         "{} {}",
                        //         SongState::Playing,
                        //         file_name.read().unwrap()
                        //     ))
                        //     .unwrap();
                    }
                    EventType::Play => {
                        sink.play();
                        debug!("switched to {} {}", SongState::Playing, event.file_name);
                        *state.write().unwrap() = SongState::Playing;

                        // echo_area_sender
                        //     .send(format!(
                        //         "{} {}",
                        //         SongState::Playing,
                        //         file_name.read().unwrap()
                        //     ))
                        //     .unwrap();
                    }
                    EventType::Pause => {
                        sink.pause();
                        debug!("switched to {} {}", SongState::Paused, event.file_name);
                        *state.write().unwrap() = SongState::Paused;

                        // echo_area_sender
                        //     .send(format!(
                        //         "{} {}",
                        //         SongState::Paused,
                        //         file_name.read().unwrap()
                        //     ))
                        //     .unwrap();
                    }
                    EventType::SilentPause => {
                        sink.pause();
                        debug!("switched to {} {}", SongState::Paused, event.file_name);
                        *state.write().unwrap() = SongState::Paused;
                    }
                    EventType::Tick => {
                        // Housekeeping
                        if sink.empty() {
                            // Should expose sink instead of doing this little dance, but doesn't work
                            debug!(
                                "(tick) switched to {} {} because sink.empty",
                                SongState::Ended,
                                event.file_name
                            );
                            *state.write().unwrap() = SongState::Ended;
                        }
                    }
                }
            }
        });

        Self {
            sender,
            state: state_new,
        }
    }

    /// Sends an event to the audio thread. A dead audio thread must not bring
    /// the interface down, so failures are logged and the song marked ended.
    fn send(&self, event_type: EventType, file_name: String) {
        if let Err(e) = self.sender.send(Event {
            event_type,
            file_name,
        }) {
            error!("Audio thread is gone: {e}");
            *self.state.write().unwrap() = SongState::Ended;
        }
    }
}

impl AudioBackend for Rodio {
    fn start(&mut self, file_name: &str) {
        debug!("starting {}", file_name);
        self.send(EventType::Start, file_name.to_string());
    }

    fn pause(&mut self) {
        log::debug!("paused ");
        self.send(EventType::Pause, String::default());
    }

    fn silent_pause(&mut self) {
        self.send(EventType::SilentPause, String::default());
    }

    fn resume(&mut self) {
        log::debug!("resumed ");
        self.send(EventType::Play, String::default());
    }

    fn busy(&mut self) -> bool {
        *self.state.read().unwrap() != SongState::Ended
    }

    fn state(&mut self) -> SongState {
        self.send(EventType::Tick, String::default());
        *self.state.read().unwrap()
    }

    fn toggle(&mut self) {
        match self.state() {
            SongState::Paused => self.resume(),
            SongState::Playing => self.pause(),
            SongState::Ended => {
                log::debug!("toggling ended song");
            }
        }
    }
}

// impl AudioBackend for Rodio {
// pub fn volume(&self) -> i32 {
//     self.volume
// }

// pub fn volume_up(&mut self) {
//     self.volume = cmp::min(self.volume + 5, 100);
//     self.player
//         .set_property("volume", i64::from(self.volume))
//         .expect("Error increase volume");
// }

// pub fn volume_down(&mut self) {
//     self.volume = cmp::max(self.volume - 5, 0);
//     self.player
//         .set_property("volume", i64::from(self.volume))
//         .expect("Error decrease volume");
// }

// pub fn set_volume(&mut self, mut volume: i32) {
//     if volume > 100 {
//         volume = 100;
//     } else if volume < 0 {
//         volume = 0;
//     }
//     self.volume = volume;
//     self.player
//         // .set_property("volume", 50_i64)
//         .set_property("volume", i64::from(self.volume))
//         .expect("Error setting volume");
// }

// pub fn resume(&mut self) {
//     self.player
//         .set_property("pause", false)
//         .expect("Toggling pause property");
// }

// pub fn is_paused(&mut self) -> bool {
//     self.player
//         .get_property("pause")
//         .expect("wrong paused state")
// }

// pub fn seek(&mut self, secs: i64) -> Result<()> {
//     match self
//         .player
//         .command("seek", &[&format!("\"{}\"", secs), "relative"])
//     {
//         Ok(r) => Ok(r),
//         Err(e) => Err(anyhow!(format!("Error in rodio: {}", e))),
//     }
// }

// pub fn get_progress(&mut self) -> Result<(f64, i64, i64)> {
//     let percent_pos = self
//         .player
//         .get_property::<f64>("percent-pos")
//         .unwrap_or(0.0);
//     // let percent = percent_pos / 100_f64;
//     let time_pos = self.player.get_property::<i64>("time-pos").unwrap_or(0);
//     let duration = self.player.get_property::<i64>("duration").unwrap_or(0);
//     Ok((percent_pos, time_pos, duration))
// }
// }
