
// cd /home/bonnemay/git/github/dirplayer/ ; cargo run --verbose
// ./target/debug/dirplayer
extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, ToAlternateScreen, ToMainScreen};
use std::io::{Write, stdout, stdin};
use std::cmp;

use std::thread;
use std::time::Duration;
use std::sync::{Arc, RwLock};

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

enum Modes {
    Display,
    Insert,
}

struct States {
    mode: Modes,
    screen: termion::screen::AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
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
    root_dir: PathBuf,
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

    pub fn update_files(&mut self) -> Result<(), Box<Error>> {
        let root_dir = self.root_dir.clone();
        self.files.truncate(0);
        self.set_files(root_dir).unwrap();
        self.files_number = self.files.len() as u16;
        // println!("{}", self.files_number);
        Ok(())
    }

}

fn write_page<W: Write>(starting_line: u16, file_list: &std::sync::Arc<std::sync::RwLock<FileList>>, screen: &mut W)  {
    let (_, height) = termion::terminal_size().unwrap();
    let ending_line = starting_line + height - (CONST_BOX.output_zone + CONST_BOX.input_zone);
    let lock = file_list.read().unwrap();
    let slice = &lock.files[((starting_line) as usize)..(cmp::min(lock.files_number, ending_line) as usize)];
    for (i, file_name) in slice.iter().enumerate() {
        write!(screen, "{}{}", termion::cursor::Goto(1, i as u16 + CONST_BOX.input_zone + 1), file_name.path().strip_prefix(&lock.root_dir).unwrap().to_str().unwrap()).unwrap();
    }
}

fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
    write!(screen, "{}", msg).unwrap();
}

fn write_alt_screen_msg<W: Write>(screen: &mut W, file_list: &std::sync::Arc<std::sync::RwLock<FileList>>, current_index :u16) {
    write!(screen, "{}", termion::clear::All).unwrap();
    write_page(current_index, &file_list, screen);
}

fn get_config() -> Config {
    let json = include_str!("/home/bonnemay/git/github/dirplayer/src/config/config.json");
    println!("{}", json);
    serde_json::from_str::<Config>(&json).unwrap()
}

pub struct Display {
    screen: std::sync::Arc<std::sync::RwLock<termion::screen::AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>>>,
}

fn main() {

    let mut _mode = Modes::Display;

    let config = get_config();

    let path: PathBuf = [r"/home", "bonnemay", "downloads", "aa_inbox", "Adrien"].iter().collect();
    // let mut entries = FileList {
    //     files: Vec::<DirEntry>::new(),
    //     files_number: 0u16,
    //     config: config,
    //     root_dir: path,
    // };

    let file_list_watcher = Arc::new(RwLock::new(
        FileList {
            files: Vec::<DirEntry>::new(),
            files_number: 0u16,
            config: config,
            root_dir: path,
        }
    ));

    // entries.update_files().unwrap();
    let file_list_watcher_thread = file_list_watcher.clone();

    thread::sleep(Duration::new(1, 0));
    let mut current_index = 0u16;

    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());

    write!(screen, "{}", termion::cursor::Hide).unwrap();
    write_screen_msg(&mut screen, "qsdfv");
    write_alt_screen_msg(&mut screen, &file_list_watcher, current_index);
    screen.flush().unwrap();

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(1000));
        let mut lock = file_list_watcher_thread.write().unwrap();
        lock.update_files().unwrap();
    });


    // let mut _states = States {
    //     mode: _mode,
    //     screen: screen,
    // };

    let stdin = stdin();
    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char('q') => break,
            Key::Char('1') => {
                write!(screen, "{}", ToMainScreen).unwrap();
            }
            Key::Char('2') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen, &file_list_watcher, current_index);
            }
            Key::Char('i') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen, &file_list_watcher, current_index);
            }
            Key::Down => {
                // let file_list_watcher_reader = file_list_watcher.clone();
                let mut lock = file_list_watcher.read().unwrap();

                if current_index < lock.files_number {
                    current_index += 1;
                }

                write_alt_screen_msg(&mut screen, &file_list_watcher, current_index);
            }
            Key::Up => {
                if current_index > 0 {
                    current_index -= 1;
                }
                write_alt_screen_msg(&mut screen, &file_list_watcher, current_index);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
}
