use dirs::audio_dir;
use dirs::cache_dir;
use dirs::config_dir;
use dirs::home_dir;
use log::LevelFilter;
use serde_derive::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

static CONFIG_PATH: &str = "dirplayer/config.json";
static CACHE_PATH: &str = "dirplayer";

#[derive(Serialize, Deserialize)]
pub enum PlayMode {
    Queue,
    Random,
}

fn get_config_file() -> PathBuf {
    let mut config_file = config_dir().expect("Could not find home directory.");
    config_file.push(CONFIG_PATH);
    config_file
}

pub fn get_cache_dir() -> PathBuf {
    let mut cache_dir = cache_dir().expect("Could not find cache directory.");
    cache_dir.push(CACHE_PATH);
    cache_dir
}

pub fn get_audio_dir() -> PathBuf {
    audio_dir().expect("Could not find audio directory.")
}

pub fn get_home_dir() -> PathBuf {
    home_dir().expect("Could not find home directory.")
}

#[derive(Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Active,
    #[default]
    Inactive,
    Cache,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct WorkingPath {
    pub path: String,
    pub status: Status,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub extensions: Vec<ExtensionFile>,
    pub extensions_archives: Vec<ExtensionArchive>,
    pub tick_rate: String,
    pub working_directory: PathBuf,
    pub play_mode: PlayMode,
    pub log_level: LevelFilter,
    pub current_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionFile {
    Mp3,
    Mp4,
    Avi,
    Ogg,
    M4a,
    Opus,
    Flac,
    Zip,
}

impl ExtensionFile {
    pub fn matches_str(&self, s: &str) -> bool {
        match self {
            Self::Mp3 => s.eq_ignore_ascii_case("mp3"),
            Self::Mp4 => s.eq_ignore_ascii_case("av4"),
            Self::Avi => s.eq_ignore_ascii_case("avi"),
            Self::Ogg => s.eq_ignore_ascii_case("ogg"),
            Self::M4a => s.eq_ignore_ascii_case("m4a"),
            Self::Opus => s.eq_ignore_ascii_case("opus"),
            Self::Flac => s.eq_ignore_ascii_case("flac"),
            Self::Zip => s.eq_ignore_ascii_case("zip"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionArchive {
    Zip,
}

impl ExtensionArchive {
    pub fn matches_str(&self, s: &str) -> bool {
        match self {
            Self::Zip => s.eq_ignore_ascii_case("zip"),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            extensions: vec![
                ExtensionFile::Mp3,
                ExtensionFile::Mp4,
                ExtensionFile::Avi,
                ExtensionFile::Ogg,
                ExtensionFile::M4a,
                ExtensionFile::Opus,
                ExtensionFile::Flac,
            ],
            extensions_archives: vec![ExtensionArchive::Zip],
            tick_rate: String::from("500"),
            working_directory: PathBuf::from(get_audio_dir().into_os_string()),
            play_mode: PlayMode::Queue,
            log_level: LevelFilter::Info,
            current_file: "".to_string(),
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
    serde_json::from_reader(file).expect("Could not parse config file as json.")
}

pub fn update_config(config: &Config) {
    let config_file = get_config_file();
    let path_to_config = config_file.parent().unwrap();
    std::fs::create_dir_all(path_to_config).expect("Could not crate path to config file.");
    let file = File::create(config_file).expect("Could not create config file.");
    serde_json::to_writer_pretty(file, &config).expect("Could not save to config file.");
}

pub fn update_working_directory(working_directory: &Path) {
    let config_file = get_config_file();
    let path_to_config = &config_file.parent().unwrap();
    let file = File::open(&config_file).expect("Could not find config file.");
    let mut config: Config =
        serde_json::from_reader(file).expect("Could not parse config file as json.");
    config.working_directory = PathBuf::from(working_directory);
    std::fs::create_dir_all(path_to_config).expect("Could not crate path to config file.");
    let file = File::create(config_file).expect("Could not create config file.");
    serde_json::to_writer_pretty(file, &config).expect("Could not save to config file.");
}
