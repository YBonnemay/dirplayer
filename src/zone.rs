use crossterm::event::{KeyCode, KeyModifiers};
use tui::layout::Constraint;
use tui::widgets::Paragraph;

pub trait Zone {
    fn get_displayable(&self) -> Paragraph;

    fn get_constraints(&self) -> Constraint;
    fn process_event(&mut self, key_code: KeyCode, key_modifiers: KeyModifiers);
    // fn get_frame(&self) -> Constraint;
}
