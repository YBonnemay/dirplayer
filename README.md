<!-- markdownlint-disable MD024 MD040 MD013 -->

# Playing a directory

`dirplayer` will play the music files in a directory, one after the other.

This emulates the behavior of <https://mpesch3.de/1by1.html>.

## Architecture

`dirplayer` is built with [cocotte](https://github.com/YBonnemay/cocotte), which composes a
[ratatui](https://github.com/ratatui/ratatui) application out of independent sub-apps. Each sub-app
handles events and renders itself, while shared data lives in a single `AppState`:

- `Input` : shows the working directory and the active filter
- `Directories` : browses directories and playable files, and emits `PlayRequest`s
- `Player` : owns the audio backend, plays requests and auto-advances when a song ends

## Audio backends

Playback stays pure Rust where it can. [rodio](https://github.com/RustAudio/rodio) handles `mp3` and
`flac`, and everything else goes to [mpv](https://mpv.io), which is used through `libmpv` as an
audio-only decoder. This matters for `opus` and `webm`, the formats `yt-dlp` downloads by default:
symphonia, the decoder behind rodio, has neither an Opus decoder nor a Matroska demuxer.

mpv is optional. Building without it keeps `dirplayer` free of any C dependency, at the cost of
those formats:

```sh
cargo build --no-default-features
```

With the default `mpv` feature, `libmpv` is needed at build and run time:

```sh
brew install mpv        # macOS
apt install libmpv-dev  # Debian and derivatives
```

`libmpv2-sys` asks the linker for `mpv` without saying where to look, so `.cargo/config.toml` adds
Homebrew's library directory to the search path on macOS.

## Keys

| Key | Action |
| --- | --- |
| `Up` / `Down` | Move in the current list |
| `Tab` | Enter the highlighted directory |
| `Enter` | Switch to the file list, then play (or pause/resume) the highlighted file |
| `Backspace` | Delete a filter character, or go to the parent directory |
| `Esc` | Go back to the directory list |
| any character | Filter the current list |
| `Ctrl-n` / `Ctrl-p` | Play the next / previous file |
| `Ctrl-t` | Pause or resume |
| `Ctrl-r` | Switch between queue and random play modes |
| `Ctrl-c` / `Ctrl-d` | Quit |

`dirplayer` wants to be pure Rust, and uses mpv only for the formats rodio cannot decode. Plan is to
switch to a pure Rust solution asap.

Some other similar projects:

- [The apps using tui-rs](https://github.com/fdehau/tui-rs#apps-using-tui) for nice tuis
- [Songbird](https://github.com/serenity-rs/songbird) for sound stuff
