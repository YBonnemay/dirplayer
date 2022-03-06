use crate::handlers::selector;

use crate::app::{App, Zone};
use crossterm::event::{KeyCode, KeyModifiers};

pub fn process_event(app: &'static mut App, key_code: KeyCode, key_modifiers: KeyModifiers) {
    if key_modifiers == KeyModifiers::CONTROL {
        app.set_zone(&key_code);
    } else {
        match app.current_zone {
            Zone::Directory => selector::process_event(app, key_code, key_modifiers),
            _ => (),
        }
    }
}
