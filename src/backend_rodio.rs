use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use crossbeam_channel::unbounded;
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, RwLock};
use std::thread;

enum EventType {
    Start,
    Play,
    Pause,
    Stop,
    Tick,
}

struct Event {
    event_type: EventType,
    file_name: String,
}

pub struct Rodio {
    sender: crossbeam_channel::Sender<Event>,
    busy: Arc<RwLock<bool>>,
    state: Arc<RwLock<SongState>>,
}

impl Rodio {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        let busy = Arc::new(RwLock::new(false));
        let busy_new = busy.clone();
        let state = Arc::new(RwLock::new(SongState::Ended));
        let state_new = state.clone();
        thread::spawn(move || -> ! {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let mut sink = rodio::Sink::try_new(&stream_handle).unwrap();

            loop {
                let event: Event = match receiver.recv() {
                    Ok(e) => e,
                    Err(err) => panic!("Error event: {:?}", err),
                };
                match event.event_type {
                    EventType::Start => {
                        *busy.write().unwrap() = true;
                        *state.write().unwrap() = SongState::Playing;

                        if !sink.empty() {
                            sink.stop();
                            sink = rodio::Sink::try_new(&stream_handle).unwrap();
                        }
                        let source = BufReader::new(File::open(event.file_name).unwrap());
                        match Decoder::new(source) {
                            Ok(decoder) => {
                                sink.append(decoder);
                                sink.play();
                            }
                            Err(e) => print!("{}", e),
                        }
                    }
                    EventType::Play => {
                        sink.play();
                        *state.write().unwrap() = SongState::Playing;
                    }
                    EventType::Pause => {
                        sink.pause();
                        *state.write().unwrap() = SongState::Paused;
                    }
                    EventType::Stop => {
                        sink.stop();
                        *busy.write().unwrap() = false;
                    }
                    EventType::Tick => {
                        // Housekeeping
                        if sink.empty() {
                            // Should expose sink instead of doing this little dance, but doesn't work
                            *state.write().unwrap() = SongState::Ended;
                        }
                    }
                }
            }
        });

        Self {
            sender,
            busy: busy_new,
            state: state_new,
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
        self.sender
            .send(Event {
                event_type: EventType::Start,
                file_name: file_name.to_string(),
            })
            .unwrap();
    }

    fn pause(&mut self) {
        self.sender
            .send(Event {
                event_type: EventType::Pause,
                file_name: String::default(),
            })
            .unwrap();
    }

    fn resume(&mut self) {
        self.sender
            .send(Event {
                event_type: EventType::Play,
                file_name: String::default(),
            })
            .unwrap();
    }

    fn busy(&self) -> bool {
        return *self.busy.read().unwrap();
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
        let state = self.state();
        match state {
            SongState::Paused => self.resume(),
            SongState::Playing => self.pause(),
            SongState::Ended => {}
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