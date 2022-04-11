use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;

static CONFIG_PATH: &str = "dirplayer/config.json";

fn get_config_file() -> PathBuf {
    let mut config_file = dirs::config_dir().expect("Could not find home directory.");
    config_file.push(CONFIG_PATH);
    config_file
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub extensions: Vec<String>,
    pub tick_rate: String,
    pub working_directories: VecDeque<String>,
    pub working_directory_line_index: HashMap<String, i32>,
    pub path: String,
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
                    .expect("Could not find Document directory.")
                    .to_str()
                    .unwrap(),
            )]),
            working_directory_line_index: HashMap::default(),
            path: String::from(PathBuf::default().to_str().unwrap()),
        }
    }
}

pub fn set_defaut_config() -> Config {
    let config_file = get_config_file();
    let config = Config::default();
    let path_to_config = config_file.parent().unwrap();
    std::fs::create_dir_all(path_to_config).expect("Could not crate path to config file.");
    let file = File::create(config_file).expect("Could not create config file.");
    serde_json::to_writer(file, &config).expect("Could not save to config file.");
    config
}

pub fn get_set_config() -> Config {
    let config_file = get_config_file();

    if config_file.is_file() {
        let file = File::open(config_file).expect("Could not find config file.");
        match serde_json::from_reader(file) {
            Ok(config) => config,
            Err(_) => set_defaut_config(),
        }
    } else {
        set_defaut_config()
    }
}

pub fn get_config() -> Config {
    let config_file = get_config_file();
    let file = File::open(config_file).expect("Could not find config file.");
    serde_json::from_reader(file).unwrap()
}

pub fn update_config(config: Config) {
    let config_file = get_config_file();
    let path_to_config = config_file.parent().unwrap();
    std::fs::create_dir_all(path_to_config).expect("Could not crate path to config file.");
    let file = File::create(config_file).expect("Could not create config file.");
    serde_json::to_writer(file, &config).expect("Could not save to config file.");
}
