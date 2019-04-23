
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
use std::error::Error;


extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

#[derive(Deserialize, Debug)]
struct Config {
    extensions: Vec< String >,
}

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
    config: Config,
}

impl FileList {
    pub fn set_files(&mut self, current_dir: PathBuf) -> Result<(), Box<Error>> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.set_files(path).unwrap();
            } else {
                let os_extension = path.extension().unwrap();
                let extension = os_extension.to_str().unwrap();
                let extension_string = extension.to_string();
                if self.config.extensions.contains(&extension_string) {
                    self.files.push(entry);
                }
            }
        }
        Ok(())
    }

    pub fn update_files(&mut self, current_dir: PathBuf) -> Result<(), Box<Error>> {
        self.set_files(current_dir).unwrap();
        self.files_number = self.files.len() as u16;
        Ok(())
    }
}

fn write_page<W: Write>(starting_line: u16, file_list: &FileList, screen: &mut W)  {
    let (_, height) = termion::terminal_size().unwrap();
    let ending_line = starting_line + height - (CONST_BOX.output_zone + CONST_BOX.input_zone);

    let slice = &file_list.files[((starting_line) as usize)..(cmp::min(file_list.files_number, ending_line) as usize)];
    for (i, file_name) in slice.iter().enumerate() {
        write!(screen, "{}{:?}", termion::cursor::Goto(1, i as u16 + CONST_BOX.input_zone + 1), file_name.path().as_os_str()).unwrap();
    }
}

fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
    write!(screen, "{}", msg).unwrap();
}

fn write_alt_screen_msg<W: Write>(screen: &mut W, file_list: &FileList, current_index :u16) {
    write!(screen, "{}",
           termion::clear::All)
        .unwrap();

    write_page(current_index, &file_list, screen);
}

fn get_config() -> Config {
    let json = include_str!("/home/bonnemay/git/github/dirplayer/src/config/config.json");
    println!("{}", json);
    serde_json::from_str::<Config>(&json).unwrap()
}

fn main() {
    let config = get_config();
    let path: PathBuf = [r"/home", "bonnemay", "downloads", "aa_inbox", "Adrien"].iter().collect();
    let mut entries = FileList{
        files: Vec::<DirEntry>::new(),
        files_number: 0u16,
        config: config,
    };

    entries.update_files(path).unwrap();
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
