use crossterm::event::{KeyCode, KeyModifiers};
use tui::layout::Constraint;

pub trait Zone {
    fn get_displayable(&self) -> Vec<String>;
    fn get_constraints(&self) -> Constraint;
    fn process_event(&mut self, key_code: KeyCode, key_modifiers: KeyModifiers);
    // fn get_frame(&self) -> Constraint;
}
