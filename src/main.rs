extern crate termion;
extern crate crossbeam_channel;
extern crate walkdir;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

// cd /home/bonnemay/git/github/dirplayer/ ; cargo run --verbose
// ./target/debug/dirplayer
// println!("write_page{:#?}", lock.len());


use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode};
use termion::screen::{AlternateScreen, ToAlternateScreen, ToMainScreen};
use std::io::{Write, stdout, stdin};
use walkdir::WalkDir;

use std::thread;
use std::sync::{Arc, RwLock};

use std::time;

#[derive(Serialize, Deserialize)]
struct Config {
    working_directory: String,
    extensions: Vec< String >,
}

fn get_config() -> Config {
    let json = include_str!("/home/bonnemay/git/github/dirplayer/src/config/config.json");
    println!("{}", json);
    serde_json::from_str::<Config>(&json).unwrap()
}

pub trait Zone {
    fn get_length(&self) -> i16;
    fn set_length(&mut self, length: i16);
    fn get_start(&self) -> i16;
    fn set_start(&mut self, length: i16);
    fn get_lines(&self) -> Vec<String>;
}

struct Zones {
    pub zones: Vec<Box<dyn Zone>>,
}

impl Zones {
    fn add_zone(&mut self, zone: Box<dyn Zone>) {
        self.zones.push(zone);
    }

    fn redisplay() {

        // thread::spawn(move || {
        //     loop {
        //         thread::sleep(Duration::from_millis(1000));
        //         let config = get_config();
        //         let working_directory = config.working_directory.clone();
        //         // let root_dir = PathBuf::from(working_directory);
        //         let walker = WalkDir::new(working_directory).into_iter();
        //         let new_files = &mut Vec::<DirEntry>::new();
        //         // get_files(&root_dir, &config, new_files).unwrap();
        //     }
        // });
    }
}

struct Display {
    length: i16,
    start: i16,
    lines: Arc<RwLock<Vec<String>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Display {
    fn refresh_start(&self) {
        let lines = self.lines.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(time::Duration::from_millis(1000));
                let config = get_config();
                let working_directory = config.working_directory.clone();

                let new_lines = WalkDir::new(working_directory)
                    .into_iter()
                    .filter_map(Result::ok)
                    .map(|e| String::from(e.file_name().to_string_lossy()))
                    .collect::<Vec<String>>();

                let mut writable_lines = lines.write().unwrap();
                *writable_lines = new_lines;
            }
        });
    }
}


impl Zone for Display {

    fn get_length(&self) -> i16 {
        self.length
    }

    fn set_length(&mut self, length: i16) {
        self.length = length;
    }

    fn get_start(&self) -> i16 {
        self.start
    }

    fn set_start(&mut self, start: i16) {
        self.start = start;
    }

    fn get_lines(&self) -> Vec<String> {
        let lines = self.lines.read().unwrap();
        lines.to_vec()
    }
}

// pub fn write_lines_to_screen<W: Write>(lines: std::vec::Vec<std::string::String>, screen: &mut W) {
//     for (i, line) in lines.into_iter().enumerate() {
//         write!(screen, "{}{}", termion::cursor::Goto(1, i as u16 + CONST_BOX.input_zone as u16 + 1), line).unwrap();
//     }
// }

// pub fn write_zone_to_screen<W: Write>(zone: & Zone, screen: &mut W) {
//     let lines = zone.get_lines();
//     for (i, line) in lines.into_iter().enumerate() {
//         write!(screen, "{}{}", termion::cursor::Goto(1, i as u16 + CONST_BOX.input_zone as u16 + 1), line).unwrap();
//     }
// }

fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
    write!(screen, "{}", msg).unwrap();
}

fn main() {
    let mut zones = Zones {
        zones: Vec::new(),
    };

    // let mut zone = DirectoryZone::new();
    // zone.set_start(0);
    // zone.start_refresh();
    // zones.add_zone(zone);

    // let path: PathBuf = [r"/home", "bonnemay", "downloads", "aa_inbox", "Adrien"].iter().collect();
    // let config = get_config();
    // let mut current_index = 0i16;
    // let mut timer = FileList::new(config, path);
    // timer.start_refresh();

    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();
    write_screen_msg(&mut screen, "qsdfv");
    // write_alt_screen_msg(&mut screen, &mut timer, 0i16);
    // display_zones(&zones, &mut screen);
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
                // write_alt_screen_msg(&mut screen, &mut timer, 0i16);
                                // display_zones(&zones, &mut screen);
            }
            Key::Char('i') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                // write_alt_screen_msg(&mut screen, &mut timer, 0i16);
                                // display_zones(&zones, &mut screen);
            }
            Key::Down => {
                // display_zones(&zones, &mut screen);
                // write_alt_screen_msg(&mut screen, &mut timer, 1i16);
            }
            Key::Up => {
                // write_alt_screen_msg(&mut screen, &mut timer, -1i16);
                                // display_zones(&zones, &mut screen);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
}
