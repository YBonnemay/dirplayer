use core::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SongState {
    Playing,
    Paused,
    Ended,
}

impl fmt::Display for SongState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub const ECHO_SIZE: i32 = 5;
