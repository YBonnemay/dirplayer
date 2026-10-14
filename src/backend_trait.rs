use crate::constants::SongState;

pub trait AudioBackend {
    fn start(&mut self, file_name: &str);
    fn pause(&mut self);
    fn silent_pause(&mut self);
    fn resume(&mut self);
    fn busy(&mut self) -> bool;
    fn state(&mut self) -> SongState;
    fn toggle(&mut self);
}
