<!-- markdownlint-disable MD024 MD040 MD013 -->

# Playing a directory

`dirplayer` will play the music files in a directory, one after the other, in the order of the files creation.

This emulates the behavior of <https://mpesch3.de/1by1.html>.

Control key + up/down move focus, characters filter the current list (directories or files).

`dirplayer` wants to be pure Rust, but will use  [mpv](https://github.com/Cobrand/mpv-rs) for Opus files. Plan is to switch to a pure Rust solution asap.

Some other similar projects:

- [The apps using tui-rs](https://github.com/fdehau/tui-rs#apps-using-tui) for nice tuis
- [Songbird](https://github.com/serenity-rs/songbird) for sound stuff
