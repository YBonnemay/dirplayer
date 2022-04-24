use crate::constants::SongState;

pub trait AudioBackend {
    fn stop(&mut self);
    fn start(&mut self, file_name: &str);
    fn pause(&mut self);
    fn silent_pause(&mut self);
    fn resume(&mut self);
    fn busy(&self) -> bool;
    fn state(&self) -> SongState;
    fn toggle(&mut self);
    fn file_name(&self) -> String;
}
