// States a backend can be in

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SongState {
    Playing,
    Paused,
    Ended,
}
