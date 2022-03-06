# Playing a directory

This is meant for a rather specific use: mine.

`dirplayer` allows me to play a directory's sound files, sorted by creation date - choose directory, CTRL+down to start autoplay, select files.

Backend tries to be [Rodio](https://github.com/RustAudio/rodio), because Rust only.

For Opus files, however, [mpv](https://github.com/Cobrand/mpv-rs) is used - plan is to switch to a Rust-only solution as soon as convenient.

This has the features I need, but is unfinished and meant as an exercise in Rust, so you are probably rather looking for:

- [The apps using tui-rs](https://github.com/fdehau/tui-rs#apps-using-tui) for nice tuis
- [Songbird](https://github.com/serenity-rs/songbird) for sound stuff
