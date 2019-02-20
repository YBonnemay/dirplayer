// cd /home/bonnemay/tests/rust/projects/dirplayer ; cargo run --verbose
// cd /home/bonnemay/tests/rust/projects/dirplayer ; cargo build
// ./target/debug/dirplayer
extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, ToAlternateScreen, ToMainScreen};
use std::io::{Write, stdout, stdin};

// use std::{env, fs};
use std::{fs};
use std::path::PathBuf;
use std::fs::DirEntry;

// use std::fs::ReadDir;

// fn list_dir() -> Result<ReadDir> {
//     let current_dir = env::current_dir()?;
//     println!(
//         "Entries modified in the last 24 hours in {:?}:",
//         current_dir
//     );
//     fs::read_dir(current_dir)?

//     for entry in fs::read_dir(current_dir)? {
//         let entry = entry?;
//         let path = entry.path();

//         let metadata = fs::metadata(&path)?;
//         let last_modified = metadata.modified()?.elapsed()?.as_secs();

//         if last_modified < 24 * 3600 && metadata.is_file() {
//             println!(
//                 "Last modified: {:?} seconds, is read only: {:?}, size: {:?} bytes, filename: {:?}",
//                 last_modified,
//                 metadata.permissions().readonly(),
//                 metadata.len(),
//                 path.file_name().ok_or("No filename")?
//             );
//         }
//     }
// }

struct ConstBox {
    inputZone: u16,
    outputZone: u16,
}

static CONST_BOX: ConstBox = ConstBox {
    inputZone: 1,
    outputZone: 1,
};

pub struct FileList {
    files: Vec<DirEntry>,
}

impl FileList {
    pub fn set_files(&mut self, current_dir: PathBuf) {
        for entry in fs::read_dir(current_dir).expect("Unable to list") {
            match entry {
                Ok(v) => {
                    self.files.push(v);
                }
                Err(_) => {}
            }
        }
    }
}

fn write_page<W: Write>(starting_line: u16, file_list: FileList, screen: &mut W)  {
    let (_, height) = termion::terminal_size().unwrap();
    let slice = &file_list.files[(starting_line as usize)..((height - (CONST_BOX.outputZone + CONST_BOX.inputZone + starting_line)) as usize)];
    for (i, file_name) in slice.iter().enumerate() {
        write!(screen, "{}{:?}", termion::cursor::Goto(1, i as u16 + CONST_BOX.inputZone + 1), file_name.path().as_os_str());
    }
}

fn get_dirs(current_dir: &PathBuf) -> Vec<DirEntry> {
    let mut entries: Vec<DirEntry> = vec![];
    for entry in fs::read_dir(current_dir).expect("Unable to list") {
        match entry {
            Ok(v) => {
                entries.push(v);
            }
            Err(_) => {}
        }
    }
    (entries)
}

fn entries_to_filenames(entries: Vec<DirEntry>) -> Vec<PathBuf> {
    let filenames: Vec<PathBuf> = entries.into_iter().map(|i| i.path()).collect();
    (filenames)
}

fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
    write!(screen, "{}", msg).unwrap();
}

fn write_alt_screen_msg<W: Write>(screen: &mut W) {
    let path: PathBuf = [r"/home", "bonnemay"].iter().collect();
    // let entries: Vec<DirEntry> = get_dirs(&path);
    // let filenames: Vec<PathBuf> = entries_to_filenames(entries);

    write!(screen, "{}",
           termion::clear::All)
        .unwrap();

    let mut entries = FileList{
        files: Vec::<DirEntry>::new(),
    };

    entries.set_files(path);
    write_page(0u16, entries, screen);

    // write!(screen, "{}{}Welcome to the alternate screen.",
    //        termion::clear::All,
    //        termion::cursor::Goto(1, 1))
    //     .unwrap();

    // for (i, filename) in filenames.iter().enumerate() {
    //     write!(screen, "{}{:?}", termion::cursor::Goto(1, i as u16), filename.as_os_str());
    // }

    // write!(screen, "{}Press '1' to switch to the main screen or '2' to switch to the alternate screen.{}Press 'q' to exit (and switch back to the main screen).",
    //        termion::cursor::Goto(1, 3),
    //        termion::cursor::Goto(1, 4))
    //     .unwrap();
}

fn main() {
    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();
    write_screen_msg(&mut screen, "qsdfv");
    write_alt_screen_msg(&mut screen);
    screen.flush().unwrap();

    let stdin = stdin();
    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char('q') => break,
            Key::Char('1') => {
                write!(screen, "{}", ToMainScreen).unwrap();
            }
            Key::Char('2') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
}
