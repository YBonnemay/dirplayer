use crate::directories::Directories;
use crate::directories::State;
use crate::files::Files;
use crate::switch_play_mode;
use crate::KeyCode;
use crate::KeyModifiers;
use crossterm::event::Event;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::fs;

pub type Chord = (KeyModifiers, KeyCode);

pub fn get_path_completions(path: &String) -> Vec<String> {
    match fs::read_dir(path) {
        Ok(paths) => paths
            .filter_map(|e| {
                let e_ref = e.as_ref().ok()?;
                let metadata = e_ref.metadata().ok()?;
                if !metadata.is_dir() {
                    return None;
                }
                Some(e_ref.file_name().to_string_lossy().to_string())
            })
            .collect::<Vec<String>>(),
        Err(_) => vec![],
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Tab {
    Directories,
    Files,
}

pub struct App<'a> {
    pub current_place: Tab,
    pub directories: Directories<'a>,
    pub files: Files,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let mut files = Files::new();
        let directories = Directories::new();
        files.update_paths(&directories.working_directories);
        files.update_lines();
        files.update_lines_filtered();
        files.watch_archives();
        files.current_file = String::default();
        files.play_file();

        App {
            directories,
            files,
            current_place: Tab::Directories,
        }
    }

    pub fn handle_tick(&mut self) {
        self.files.on_tick();
    }

    pub fn cycle_tab(&mut self) {
        self.current_place = match self.current_place {
            Tab::Directories => Tab::Files,
            Tab::Files => Tab::Directories,
        };
    }

    pub fn handle_event_movement(&mut self, frame: &Frame, chord: Chord) {
        match &self.current_place {
            Tab::Directories => match chord {
                (_, KeyCode::Tab) => match self.directories.state {
                    State::Base => self.cycle_tab(),
                    State::Editing => self.directories.handle_event(chord),
                },
                _ => self.directories.handle_event(chord),
            },
            Tab::Files => match chord {
                (_, KeyCode::Tab) => self.cycle_tab(),
                (KeyModifiers::CONTROL, KeyCode::Char('t')) => switch_play_mode(),
                _ => self.files.handle_event(frame, chord),
            },
        }
    }

    pub fn handle_event(&mut self, frame: &Frame, event: &Event) -> Result<(), ()> {
        let &Event::Key(
            KeyEvent {
                code, modifiers, ..
            },
            ..,
        ) = event
        else {
            return Err(());
        };

        let chord = (modifiers, code);

        if chord == (KeyModifiers::CONTROL, KeyCode::Char('c'))
            || chord == (KeyModifiers::CONTROL, KeyCode::Char('d'))
        {
            return Err(());
        }

        self.handle_event_movement(frame, chord);
        Result::Ok(())
    }
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self::new()
    }
}
