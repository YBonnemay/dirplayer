extern crate termion;
extern crate crossbeam_channel;
extern crate walkdir;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

// cd /home/bonnemay/git/github/dirplayer/ ; cargo run --verbose
// cd /home/bonnemay/git/github/dirplayer/ ; RUST_BACKTRACE=1 cargo run --verbose
// ./target/debug/dirplayer
// println!("write_page{:#?}", lock.len());

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode};
// use termion::screen::{AlternateScreen, ToAlternateScreen, ToMainScreen};
use termion::screen::{AlternateScreen};
use termion::raw::RawTerminal;
use std::io::{Write, Stdout, stdout, stdin};
use std::cmp;
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
    serde_json::from_str::<Config>(&json).unwrap()
}

pub trait Zone {
    fn get_zone_offset_start(&self) -> i16;
    fn get_zone_offset_end(&self) -> i16;
    fn set_zone_offset_start(&mut self, weight: i16);
    fn set_zone_offset_end(&mut self, weight: i16);

    fn get_length(&self) -> i16;
    fn get_lines(&self) -> Vec<String>;
    fn use_keystroke(&mut self, Key);
}

struct Zones {
    pub zones: Vec<Box<dyn Zone>>,
    pub screen: AlternateScreen<RawTerminal<Stdout>>,
    pub current_zone: i16,
}

impl Zones {
    fn new(screen: AlternateScreen<RawTerminal<Stdout>>) -> Zones {
        Zones {
            zones: Vec::new(),
            screen: screen,
            current_zone: 0,
        }
    }

    fn get_lengths(&self) -> i16 {
        let zones = &self.zones;
        let mut lengths = 0;
        for existing_zone in zones {
            lengths+= existing_zone.get_length();
        }
        lengths
    }

    fn add_zone(&mut self, mut zone:  Box<dyn Zone>) {
        self.zones.push(zone)
    }

    fn display(&mut self) {
        self.flush();
        let zones = &mut self.zones;
        let terminal_size = termion::terminal_size().unwrap();

        for zone in zones {
            let lines = zone.get_lines();
            let zone_offset_start = zone.get_zone_offset_start() as usize;

            for (i, line) in lines.into_iter().enumerate() {
                let mut line = line;
                write!(
                    self.screen,
                    "{}{}",
                    termion::cursor::Goto(1, (i + zone_offset_start) as u16),
                    termion::clear::CurrentLine
                ).unwrap();
                line.truncate(terminal_size.0 as usize);
                write!(
                    self.screen,
                    "{}{}",
                    termion::cursor::Goto(1, (i + zone_offset_start) as u16),
                    line
                ).unwrap();

            }
        }
    }

    fn flush(&mut self) {
        self.screen.flush().unwrap();
    }

    // fn set_current_zone(&mut self, current_zone: i16) {
    //     self.current_zone = current_zone
    // }

    fn send_keystroke(&mut self, key: Key) {
        self.zones[self.current_zone as usize].use_keystroke(key);
    }
}

struct MiddleZone {
    // https://users.rust-lang.org/t/heartbeat-in-a-thread-done-right/13596
    x_index: i16,
    zone_offset_start: i16,
    zone_offset_end: i16,
    start: i16,
    lines: Arc<RwLock<Vec<String>>>,
    lines_index: i16,
    // handle: Option<thread::JoinHandle<()>>,
}

impl MiddleZone {
    fn new() -> MiddleZone {
        MiddleZone {
            x_index: 0,
            zone_offset_start: 0,
            zone_offset_end: 0,
            start: 0,
            lines: Arc::new(RwLock::new(Vec::new())),
            lines_index: 0,
            // handle: None,
        }
    }

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

impl Zone for MiddleZone {

    fn get_zone_offset_start(&self) -> i16 {
        self.zone_offset_start
    }

    fn set_zone_offset_start(&mut self, zone_offset_start: i16) {
        self.zone_offset_start = zone_offset_start + 1;
    }

    fn get_zone_offset_end(&self) -> i16 {
        self.zone_offset_end
    }

    fn set_zone_offset_end(&mut self, zone_offset_end: i16) {
        self.zone_offset_end = zone_offset_end;
    }

    fn get_length(&self) -> i16 {
        let terminal_size = termion::terminal_size().unwrap();
        terminal_size.1 as i16 - self.get_zone_offset_start() + self.get_zone_offset_end()
    }

    fn get_lines(&self) -> Vec<String> {
        // let lines = self.lines.read().unwrap();
        // return lines.to_vec();
        let lines = self.lines.read().unwrap().to_vec();
        let lines_len = lines.len();
        let lines_index = self.lines_index as usize;
        let lines_start = cmp::min(lines_len, lines_index);
        let lines_end = cmp::min(lines_len, lines_index + self.get_length() as usize);

        // println!("lines_start {:#?}", lines_start);
        // println!("lines_end {:#?}", lines_end);
        // println!("self.get_length() {:#?}", self.get_length());

        return (lines[lines_start as usize..lines_end as usize]).to_vec();
    }

    fn use_keystroke(&mut self, key: Key) {
        match key {
            Key::Down => {
                self.lines_index = self.lines_index + 1;
            }
            Key::Up => {
                if self.lines_index > 0 {
                    self.lines_index = self.lines_index - 1;
                }
            }
            _ => {}
        }
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

// fn write_screen_msg<W: Write>(screen: &mut W, msg: &str) {
//     write!(screen, "{}", msg).unwrap();
// }

fn main() {
    let screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    let mut zones = Zones::new(screen);
    let mut middle_zone = Box::new(MiddleZone::new());

    middle_zone.set_zone_offset_end(-1);
    middle_zone.set_zone_offset_start(1);
    // middle_zone.set_length(10);
    middle_zone.refresh_start();
    zones.add_zone(middle_zone);
    zones.display();
    zones.flush();

    // let mut zone = DirectoryZone::new();
    // zone.set_start(0);
    // zone.start_refresh();
    // zones.add_zone(zone);

    // let path: PathBuf = [r"/home", "bonnemay", "downloads", "aa_inbox", "Adrien"].iter().collect();
    // let config = get_config();

    // let mut timer = FileList::new(config, path);
    // timer.start_refresh();

    // display_zones(&zones, &mut screen);

    let stdin = stdin();
    for c in stdin.keys() {
        let key = c.unwrap();
        match key {
            Key::Char('q') => break,
            _ => {
                zones.send_keystroke(key);
                zones.display();
            }
            // Key::Char('1') => {
            //     zones.display();
            // }
            // Key::Char('2') => {
            //     zones.display();
            // }
            // Key::Char('i') => {
            //     zones.display();
            // }
            // Key::Down => {
            //     zones.display();
            // }
            // Key::Up => {
            //     // write_alt_screen_msg(&mut screen, &mut timer, -1i16);
            //     // display_zones(&zones, &mut screen);
            // }
            // Key::Down => {
            //     zones.display();
            // }
        }
        zones.flush();
    }
}
