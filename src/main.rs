
// cd /home/bonnemay/git/github/dirplayer/ ; cargo run --verbose
// ./target/debug/dirplayer
// println!("write_page{:#?}", lock.len());
extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, ToAlternateScreen, ToMainScreen};
use std::io::{Write, stdout, stdin};
use std::{cmp, time};

use std::thread;
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

struct ConstBox {
    input_zone: i16,
    output_zone: i16,
}

static CONST_BOX: ConstBox = ConstBox {
    input_zone: 1,
    output_zone: 1,
};

fn write_page<W: Write>(timer: &FileList, screen: &mut W)  {
    let starting_line = timer.current_index;
    let (_, height) = termion::terminal_size().unwrap();
    let ending_line = starting_line + height as i16 - (CONST_BOX.output_zone + CONST_BOX.input_zone);
    let lock = timer.files.read().unwrap();
    let files_len = lock.len() as i16;
    let slice = &lock[((starting_line) as usize)..(cmp::min(files_len, ending_line) as usize)];
    for (i, file_name) in slice.iter().enumerate() {
        let root_dir = timer.root_dir.clone();
        write!(screen, "{}{}", termion::cursor::Goto(1, i as u16 + CONST_BOX.input_zone as u16 + 1), file_name.path().strip_prefix(root_dir).unwrap().to_str().unwrap()).unwrap();
    }
}

fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
    write!(screen, "{}", msg).unwrap();
}

fn write_alt_screen_msg<W: Write>(screen: &mut W, timer: &mut FileList, increment :i16) {

    let incremented = timer.current_index + increment;
    if incremented > timer.files_len()
        || incremented < 0
    {
        return
    }

    timer.current_index = incremented;
    println!("write_alt_screen_msg normal{:#?}", timer.current_index);
    write!(screen, "{}", termion::clear::All).unwrap();
    write_page(&timer, screen);
}

fn get_config() -> DPConfig {
    let json = include_str!("/home/bonnemay/git/github/dirplayer/src/config/config.json");
    println!("{}", json);
    serde_json::from_str::<DPConfig>(&json).unwrap()
}


pub struct FileList {
    handle: Option<thread::JoinHandle<()>>,
    files: Arc<RwLock<Vec<DirEntry>>>,
    config: DPConfig,
    root_dir: PathBuf,
    current_index: i16,
}

impl FileList {

    fn get_files(current_dir: &PathBuf, config: &DPConfig, files: &mut Vec<DirEntry> )
                 -> Result<(), Box<Error>> {
        // println!("current_dir{:#?}", current_dir);
        // println!("config{:#?}", config);
        // println!("files{:#?}", files);
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = &entry.path();
            if path.is_dir() {
                FileList::get_files(path, config, files).unwrap();
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

    pub fn files_len(&self) -> i16 {
        let files = &self.files.read().unwrap();
        files.len() as i16
    }

    // https://stackoverflow.com/questions/42043823/design-help-threading-within-a-struct
    pub fn start_refresh(&mut self)
    {
        // let mut lockedFiles = self.files.read().unwrap();
        // let files = &mut Vec::<DirEntry>::new();
        let root_dir = self.root_dir.clone();
        // FileList::get_files(&self.root_dir, &self.config, files).unwrap();

        // Get a reference old files
        let old_files = self.files.clone();
        let config = self.config.clone();

        self.handle = Some(thread::spawn(move || {
            // Get updated file list

            loop {
                thread::sleep(time::Duration::from_millis(1000));
                let new_files = &mut Vec::<DirEntry>::new();
                // println!("BBBB{:#?}", root_dir);
                FileList::get_files(&root_dir, &config, new_files).unwrap();

                // Update Mutexed
                let mut lock = old_files.write().unwrap();
                lock.truncate(0);
                lock.append(new_files);
            }
        }));
    }

    fn new(config: DPConfig, path: PathBuf) -> FileList {
        FileList {
            handle: None,
            files: Arc::new(RwLock::new(Vec::<DirEntry>::new())),
            config: config,
            root_dir: path,
            current_index: 0i16,
        }
    }
}

fn main() {

    let config = get_config();

    let path: PathBuf = [r"/home", "bonnemay", "downloads", "aa_inbox", "Adrien"].iter().collect();

    // let mut current_index = 0i16;
    let mut timer = FileList::new(config, path);
    timer.start_refresh();

    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();
    write_screen_msg(&mut screen, "qsdfv");
    write_alt_screen_msg(&mut screen, &mut timer, 0i16);
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
                write_alt_screen_msg(&mut screen, &mut timer, 0i16);
            }
            Key::Char('i') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen, &mut timer, 0i16);
            }
            Key::Down => {
                write_alt_screen_msg(&mut screen, &mut timer, 1i16);
            }
            Key::Up => {
                write_alt_screen_msg(&mut screen, &mut timer, -1i16);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
}
