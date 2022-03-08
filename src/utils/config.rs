use dirs::home_dir;
use serde_derive::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub extensions: Vec<String>,
    pub tick_rate: String,
    pub working_directories: VecDeque<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            extensions: vec!["mp3", "mp4", "avi", "ogg", "m4a", "opus", "flac"]
                .into_iter()
                .map(String::from)
                .collect(),
            tick_rate: String::from("1000"),
            working_directories: VecDeque::from([String::from(
                dirs::document_dir()
                    .expect("Could not find home directory.")
                    .to_str()
                    .unwrap(),
            )]),
        }
    }
}

pub fn get_set_config() -> Config {
    let mut config_file = dirs::home_dir().expect("Could not find home directory.");
    config_file.push(".config/dirplayer/config.json");

    if config_file.is_file() {
        let file = File::open(config_file).expect("Could not find config file.");
        let config: Config = serde_json::from_reader(file).unwrap();
        config
    } else {
        let config = Config::default();
        let path_to_config = config_file.parent().unwrap();
        std::fs::create_dir_all(path_to_config).expect("Could not crate path to config file.");
        let file = File::create(config_file).expect("Could not create config file.");
        serde_json::to_writer(file, &config).expect("Could not save to config file.");
        config
    }
}

pub fn get_config() -> Config {
    let mut config_file = dirs::home_dir().expect("Could not find home directory.");
    config_file.push(".config/dirplayer/config.json");
    let file = File::open(config_file).expect("Could not find config file.");
    serde_json::from_reader(file).unwrap()
}

pub fn update_config(config: Config) {
    let mut config_file = dirs::home_dir().expect("Could not find home directory.");
    config_file.push(".config/dirplayer/config.json");
    let path_to_config = config_file.parent().unwrap();
    std::fs::create_dir_all(path_to_config).expect("Could not crate path to config file.");
    let file = File::create(config_file).expect("Could not create config file.");
    serde_json::to_writer(file, &config).expect("Could not save to config file.");
}

// let json = include_str!("/home/bonnemay/github/dirplayer/src/config/config.json");
// serde_json::from_str::<Config>(json).unwrap()

// pub fn get_config() -> Config {
//     let json = include_str!("/home/bonnemay/github/dirplayer/src/config/config.json");
//     serde_json::from_str::<Config>(json).unwrap()
// }

// pub fn get_config() -> Config {
//     fs::create_dir_all(path);
//     let json = include_str!("/home/bonnemay/github/dirplayer/src/config/config.json");
//     serde_json::from_str::<Config>(json).unwrap()
// }
