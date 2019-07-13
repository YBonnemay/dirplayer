
// cd /home/bonnemay/git/github/dirplayer/ ; cargo run --verbose
// ./target/debug/dirplayer
// println!("write_page{:#?}", lock.len());
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

#[derive(Deserialize, Debug, Clone)]
struct DPConfig {
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
    config: DPConfig,
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

fn write_page<W: Write>(starting_line: u16, timer: &Timer, screen: &mut W)  {
    let (_, height) = termion::terminal_size().unwrap();
    let ending_line = starting_line + height - (CONST_BOX.output_zone + CONST_BOX.input_zone);
    let lock = timer.files.read().unwrap();
    let files_len = lock.len() as u16;
    let slice = &lock[((starting_line) as usize)..(cmp::min(files_len, ending_line) as usize)];
    for (i, file_name) in slice.iter().enumerate() {
        let root_dir = timer.root_dir.clone();
        write!(screen, "{}{}", termion::cursor::Goto(1, i as u16 + CONST_BOX.input_zone + 1), file_name.path().strip_prefix(root_dir).unwrap().to_str().unwrap()).unwrap();
    }
}

fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
    write!(screen, "{}", msg).unwrap();
}

fn write_alt_screen_msg<W: Write>(screen: &mut W, timer: &Timer, current_index :u16) {
    println!("write_alt_screen_msg");
    write!(screen, "{}", termion::clear::All).unwrap();
    write_page(current_index, &timer, screen);
}

fn get_config() -> DPConfig {
    let json = include_str!("/home/bonnemay/git/github/dirplayer/src/config/config.json");
    println!("{}", json);
    serde_json::from_str::<DPConfig>(&json).unwrap()
}

pub struct Display {
    screen: std::sync::Arc<std::sync::RwLock<termion::screen::AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>>>,
}


use std::{sync, time};
use std::sync::atomic::{AtomicBool, Ordering};





pub struct Timer {
    handle: Option<thread::JoinHandle<()>>,
    alive: sync::Arc<AtomicBool>,

    files: std::sync::Arc<std::sync::RwLock<Vec<DirEntry>>>,
    files_number: u16,
    config: DPConfig,
    root_dir: PathBuf,
}

impl Timer {

    fn get_files(current_dir: &PathBuf, config: &DPConfig, files: &mut Vec<DirEntry> )
                 -> Result<(), Box<Error>> {
        // println!("current_dir{:#?}", current_dir);
        // println!("config{:#?}", config);
        // println!("files{:#?}", files);
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = &entry.path();
            if path.is_dir() {
                Timer::get_files(path, config, files).unwrap();
            } else {
                let os_extension = path.extension().unwrap();
                let extension = os_extension.to_str().unwrap();
                let extension_string = extension.to_string();
                if config.extensions.contains(&extension_string) {
                    files.push(entry);
                }
            }
        }
        Ok(())
    }

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
                    let files = self.files.clone();
                    let mut lock = files.write().unwrap();
                    lock.push(entry);
                }
            }
        }
        Ok(())
    }

    pub fn update_files(&mut self) -> Result<(), Box<Error>> {
        let root_dir = self.root_dir.clone();

        let files = self.files.clone();
        let mut lock = files.write().unwrap();
        lock.truncate(0);

        self.set_files(root_dir).unwrap();

        let files = self.files.clone();
        let mut lock = files.read().unwrap();

        self.files_number = lock.len() as u16;
        Ok(())
    }

    // https://stackoverflow.com/questions/42043823/design-help-threading-within-a-struct
    pub fn start_refresh(&mut self)
    {
        // let mut lockedFiles = self.files.read().unwrap();
        // let files = &mut Vec::<DirEntry>::new();
        let root_dir = self.root_dir.clone();
        // Timer::get_files(&self.root_dir, &self.config, files).unwrap();

        // Get a reference old files
        let old_files = self.files.clone();
        let mut files_number = self.files_number.clone();
        let config = self.config.clone();

        self.handle = Some(thread::spawn(move || {
            // Get updated file list

            loop {
                thread::sleep(time::Duration::from_millis(1000));
                let new_files = &mut Vec::<DirEntry>::new();
                // println!("BBBB{:#?}", root_dir);
                Timer::get_files(&root_dir, &config, new_files).unwrap();

                // Update Mutexed
                let mut lock = old_files.write().unwrap();
                lock.truncate(0);
                lock.append(new_files);
                files_number = lock.len() as u16;
            }
        }));
    }

    fn new(config: DPConfig, path: PathBuf) -> Timer {
        Timer {
            handle: None,
            alive: sync::Arc::new(AtomicBool::new(false)),
            files: sync::Arc::new(sync::RwLock::new(Vec::<DirEntry>::new())),
            files_number: 0u16,
            config: config,
            root_dir: path,
        }
    }
    // https://stackoverflow.com/questions/42043823/design-help-threading-within-a-struct
    // https://stackoverflow.com/questions/48017290/what-does-boxfn-send-static-mean-in-rust
// https://doc.rust-lang.org/book/ch10-02-traits.html
    pub fn start<F: Send + 'static + FnMut()>(&mut self, fun: F)
    {
        self.alive.store(true, Ordering::SeqCst);

        let alive = self.alive.clone();

        self.handle = Some(thread::spawn(move || {
            let mut fun = fun;
            while alive.load(Ordering::SeqCst) {
                fun();
                thread::sleep(time::Duration::from_millis(10));
            }
        }));
    }

    pub fn stop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        self.handle
            .take().expect("Called stop on non-running thread")
            .join().expect("Could not join spawned thread");
    }
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

    // let file_list_watcher = Arc::new(RwLock::new(
    //     FileList {
    //         files: Vec::<DirEntry>::new(),
    //         files_number: 0u16,
    //         config: config,
    //         root_dir: path,
    //     }
    // ));

    // entries.update_files().unwrap();
    // let file_list_watcher_thread = file_list_watcher.clone();

    // thread::sleep(Duration::new(1, 0));
    let mut current_index = 0u16;

    let mut timer = Timer::new(config, path);
    timer.start_refresh();
    // timer.handle.take().expect("Called stop on non-running thread")
    //     .join().expect("Could not join spawned thread");

    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();
    write_screen_msg(&mut screen, "qsdfv");
    write_alt_screen_msg(&mut screen, &timer, current_index);
    screen.flush().unwrap();

    let stdin = stdin();
    for c in stdin.keys() {
        let key = c.unwrap();

        match key {
            Key::Char('q') => break,
            Key::Char('1') => {
                write!(screen, "{}", ToMainScreen).unwrap();
            }
            Key::Char('2') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen, &timer, current_index);
            }
            Key::Char('i') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen, &timer, current_index);
            }
            Key::Down => {
                // let timer_reader = timer.clone();

                if current_index < timer.files_number {
                    current_index += 1;
                }

                write_alt_screen_msg(&mut screen, &timer, current_index);
            }
            Key::Up => {
                if current_index > 0 {
                    current_index -= 1;
                }
                write_alt_screen_msg(&mut screen, &timer, current_index);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
}
