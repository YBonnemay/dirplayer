// cd /home/bonnemay/git/github/dirplayer/ ; cargo run --verbose
// ./target/debug/dirplayer
extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, ToAlternateScreen, ToMainScreen};
use std::io::{Write, stdout, stdin};
use std::cmp;

// use std::{env, fs};
use std::{fs};
use std::path::PathBuf;
use std::fs::DirEntry;

// use std::process;
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
    input_zone: u16,
    output_zone: u16,
}

static CONST_BOX: ConstBox = ConstBox {
    input_zone: 1,
    output_zone: 1,
};

pub struct FileList {
    files: Vec<DirEntry>,
    files_number: u16,
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
        self.files_number = self.files.len() as u16;
    }
}


fn write_page<W: Write>(starting_line: u16, file_list: &FileList, screen: &mut W)  {
    println!("{:?} starting_line", starting_line);
    let (_, height) = termion::terminal_size().unwrap();
    let ending_line = starting_line + height - (CONST_BOX.output_zone + CONST_BOX.input_zone);

    let slice = &file_list.files[((starting_line) as usize)..(cmp::min(file_list.files_number, ending_line) as usize)];
    for (i, file_name) in slice.iter().enumerate() {
        write!(screen, "{}{:?}", termion::cursor::Goto(1, i as u16 + CONST_BOX.input_zone + 1), file_name.path().as_os_str()).unwrap();
    }
}

// fn get_dirs(current_dir: &PathBuf) -> Vec<DirEntry> {
//     let mut entries: Vec<DirEntry> = vec![];
//     for entry in fs::read_dir(current_dir).expect("Unable to list") {
//         match entry {
//             Ok(v) => {
//                 entries.push(v);
//             }
//             Err(_) => {}
//         }
//     }
//     (entries)
// }

// fn entries_to_filenames(entries: Vec<DirEntry>) -> Vec<PathBuf> {
//     let filenames: Vec<PathBuf> = entries.into_iter().map(|i| i.path()).collect();
//     (filenames)
// }

fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
    write!(screen, "{}", msg).unwrap();
}

fn write_alt_screen_msg<W: Write>(screen: &mut W, file_list: &FileList, current_index :u16) {
    write!(screen, "{}",
           termion::clear::All)
        .unwrap();

    write_page(current_index, &file_list, screen);
}

fn main() {
    let mut path: PathBuf = [r"/home", "bonnemay"].iter().collect();
    let mut entries = FileList{
        files: Vec::<DirEntry>::new(),
        files_number: 0u16,
    };

    entries.set_files(path);
    let mut current_index = 0u16;

    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();
    write_screen_msg(&mut screen, "qsdfv");
    write_alt_screen_msg(&mut screen, &entries, current_index);
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
                write_alt_screen_msg(&mut screen, &entries, current_index);
            }
            Key::Char('j') => {
                if current_index < entries.files_number {
                    current_index += 1;
                }

                write_alt_screen_msg(&mut screen, &entries, current_index);
            }
            Key::Char('k') => {
                if current_index > 0 {
                    current_index -= 1;
                }
                write_alt_screen_msg(&mut screen, &entries, current_index);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
}
