use crate::backend_trait::AudioBackend;
use crate::constants::SongState;
use libmpv::Mpv as MpvBackend;

// http://mpv.io/manual/master/#options
// http://mpv.io/manual/master/#list-of-input-commands
// https://mpv.io/manual/master/#properties

pub struct Mpv {
    player: MpvBackend,
    file_name: String,
}

impl Default for Mpv {
    fn default() -> Self {
        let mpv = MpvBackend::new().expect("Couldn't initialize MpvHandlerBuilder");
        mpv.set_property("vo", "null")
            .expect("Couldn't set vo=null in libmpv");
        Self {
            player: mpv,
            file_name: String::default(),
        }
    }
}

impl AudioBackend for Mpv {
    fn stop(&mut self) {
        // TODO, maybe?
    }

    fn start(&mut self, new: &str) {
        self.file_name = String::from(new);
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
            .get_property::<bool>("idle-active")
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

    fn file_name(&self) -> String {
        self.file_name.clone()
    }
}

impl Mpv {
    fn is_paused(&mut self) -> bool {
        self.player
            .get_property("pause")
            .expect("wrong paused state")
    }
}
