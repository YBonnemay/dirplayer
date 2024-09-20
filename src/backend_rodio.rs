use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use crossbeam_channel::unbounded;
use log::{debug, error};
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, RwLock};
use std::thread;

enum EventType {
    Start,
    Play,
    Pause,
    SilentPause,
    Stop,
    Tick,
}

struct Event {
    event_type: EventType,
    file_name: String,
}

pub struct Rodio {
    sender: crossbeam_channel::Sender<Event>,
    state: Arc<RwLock<SongState>>,
    file_name: Arc<RwLock<String>>,
}

impl Rodio {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        let state = Arc::new(RwLock::new(SongState::Ended));
        let state_new = state.clone();

        let file_name = Arc::new(RwLock::new(String::default()));
        let file_name_new = file_name.clone();

        thread::spawn(move || -> ! {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let mut sink = rodio::Sink::try_new(&stream_handle).unwrap();
            loop {
                let event: Event = match receiver.recv() {
                    Ok(e) => e,
                    Err(err) => {
                        debug!("Error receiving {}", err);
                        panic!("Error event: {err:?}");
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
                        let source = BufReader::new(File::open(event.file_name.clone()).unwrap());
                        debug! {"source: {:?}", source}
                        match Decoder::new(source) {
                            Ok(decoder) => {
                                sink.append(decoder);
                                sink.play();
                            }
                            Err(e) => {
                                error! {"{e}"};
                            }
                        }

                        debug! {"Decoder done"};

                        *file_name.write().unwrap() = event.file_name;

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
                    EventType::Stop => {
                        sink.stop();
                        debug!("switched to {} {}", SongState::Ended, event.file_name);
                        *state.write().unwrap() = SongState::Ended;
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
            file_name: file_name_new,
        }
    }
}

impl AudioBackend for Rodio {
    fn stop(&mut self) {
        self.sender
            .send(Event {
                event_type: EventType::Stop,
                file_name: String::default(),
            })
            .unwrap();
    }

    fn start(&mut self, file_name: &str) {
        debug!("starting {}", file_name);

        self.sender
            .send(Event {
                event_type: EventType::Start,
                file_name: file_name.to_string(),
            })
            .unwrap();
    }

    fn pause(&mut self) {
        log::debug!("paused ");
        self.sender
            .send(Event {
                event_type: EventType::Pause,
                file_name: String::default(),
            })
            .unwrap();
    }

    fn silent_pause(&mut self) {
        self.sender
            .send(Event {
                event_type: EventType::SilentPause,
                file_name: String::default(),
            })
            .unwrap();
    }

    fn resume(&mut self) {
        log::debug!("resumed ");
        self.sender
            .send(Event {
                event_type: EventType::Play,
                file_name: String::default(),
            })
            .unwrap();
    }

    fn busy(&self) -> bool {
        *self.state.read().unwrap() != SongState::Ended
    }

    fn state(&self) -> SongState {
        self.sender
            .send(Event {
                event_type: EventType::Tick,
                file_name: String::default(),
            })
            .unwrap();
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

    fn file_name(&self) -> String {
        let test = self.file_name.read().unwrap();
        (*test).to_string()
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
