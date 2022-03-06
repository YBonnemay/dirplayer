use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use libmpv::Mpv as MpvBackend;

// http://mpv.io/manual/master/#options
// http://mpv.io/manual/master/#list-of-input-commands
// https://mpv.io/manual/master/#properties

pub struct Mpv {
    player: MpvBackend,
}

impl Default for Mpv {
    fn default() -> Self {
        let mpv = MpvBackend::new().expect("Couldn't initialize MpvHandlerBuilder");
        mpv.set_property("vo", "null")
            .expect("Couldn't set vo=null in libmpv");
        Self { player: mpv }
    }
}

impl AudioBackend for Mpv {
    fn stop(&mut self) {
        // TODO
    }

    fn start(&mut self, new: &str) {
        self.player
            .command("loadfile", &[&format!("\"{}\"", new), "replace"])
            .expect("Error loading file");
        self.resume();
    }

    fn pause(&mut self) {
        self.player
            .set_property("pause", true)
            .expect("Toggling pause property");
    }

    fn resume(&mut self) {
        self.player
            .set_property("pause", false)
            .expect("Toggling pause property");
    }

    fn busy(&self) -> bool {
        !self
            .player
            .get_property::<bool>("core-idle")
            // .get_property::<bool>("eof-reached")
            .unwrap_or(false)
    }

    fn state(&self) -> SongState {
        if self
            .player
            .get_property("pause")
            .expect("wrong paused state")
        {
            SongState::Paused
        } else if self.busy() {
            SongState::Playing
        } else {
            SongState::Ended
        }
    }

    fn toggle(&mut self) {
        if self.is_paused() {
            self.resume();
        } else {
            self.pause();
        }
    }
}

impl Mpv {
    fn is_paused(&mut self) -> bool {
        self.player
            .get_property("pause")
            .expect("wrong paused state")
    }
}
