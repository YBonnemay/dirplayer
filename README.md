<!-- markdownlint-disable MD024 MD040 MD013 -->

# Playing a directory

`dirplayer` will play the music files in a directory, one after the other.

This emulates the behavior of <https://mpesch3.de/1by1.html>.

## Architecture

`dirplayer` is built with [cocotte](https://github.com/YBonnemay/cocotte), which composes a
[ratatui](https://github.com/ratatui/ratatui) application out of independent sub-apps. Each sub-app
handles events and renders itself, while shared data lives in a single `AppState`:

- `Directories` : lists the current directory, ranger style, titled with that directory and the
  active filter, and emits `PlayRequest`s
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

On Linux, rodio builds against ALSA, so its headers are needed too:

```sh
apt install libasound2-dev pkg-config
```

`libmpv2-sys` asks the linker for `mpv` without saying where to look, so `.cargo/config.toml` adds
Homebrew's library directory to the search path on macOS. Linux keeps `libmpv` on the default
search path, and the file is target scoped, so it stays out of the way there.

The listing is ranger style: one list holding the current directory's subdirectories, shown in blue
with a trailing `/`, followed by its playable files. A single cursor walks the whole list, and
typing filters it as a whole. Only files are played, queued and picked at random; directories are
navigation only.

Each row carries its date on the right, the creation date where the filesystem keeps one and the
last modification otherwise. The date is coloured by its position within its own year on a rainbow
gradient, so entries of the same period share a colour. Directories and files are each sorted by
that date, most recent first, with directories still listed above files.

The file walk is recursive. `search_depth` in the config counts directory levels from the current
one, so `1` lists it alone and the default `2` also brings up the files of its subdirectories.
Files found below the current directory are listed with their directories as a prefix, such as
`live/encore.opus`, and are played, ordered, queued and picked at random like any other file.

## Keys

| Key | Action |
| --- | --- |
| `Up` / `Down` | Move the cursor through directories then files |
| `PageUp` / `PageDown` | Move the cursor by a screenful |
| `Right` | Enter the highlighted directory |
| `Left` | Go to the parent directory |
| `Enter` | Play the highlighted file |
| `Backspace` | Delete a filter character |
| `Esc` | Clear the filter |
| any character | Filter the list |
| `Ctrl-n` / `Ctrl-p` | Play the next / previous file |
| `Ctrl-t` | Pause or resume |
| `Ctrl-r` | Switch between queue and random play modes |
| `Ctrl-c` / `Ctrl-d` | Quit |

`dirplayer` wants to be pure Rust, and uses mpv only for the formats rodio cannot decode. Plan is to
switch to a pure Rust solution asap.

Some other similar projects:

- [The apps using tui-rs](https://github.com/fdehau/tui-rs#apps-using-tui) for nice tuis
- [Songbird](https://github.com/serenity-rs/songbird) for sound stuff
