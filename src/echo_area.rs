use crossbeam::channel::{unbounded, Receiver, Sender};
use std::{
    sync::{Arc, RwLock},
    thread,
};
use tui::{
    style::{Color, Style},
    text::Span,
    widgets::Block,
};

pub struct EchoArea {
    pub message: Arc<RwLock<String>>,
    pub receiver: Receiver<String>,
    pub sender: Sender<String>,
}

impl<'a> EchoArea {
    pub fn new() -> EchoArea {
        let (sender, receiver) = unbounded::<String>();
        let closure_receiver = receiver.clone();
        let message = Arc::new(RwLock::new(String::default()));
        let message_build = message.clone();

        thread::spawn(move || loop {
            match closure_receiver.recv() {
                Ok(message_event) => {
                    let mut message = message.write().unwrap();
                    *message = message_event;
                    crate::deprintln!("ECHO {}", *message);
                }
                Err(e) => println!("watch error: {:?}", e),
            };
        });

        EchoArea {
            message: message_build,
            receiver,
            sender,
        }
    }

    pub fn draw<B: tui::backend::Backend>(&self, f: &mut tui::Frame<B>, chunk: tui::layout::Rect) {
        // let message = self.message.clone();
        let message = self.message.read().unwrap();
        crate::deprintln!("DRAWING {}", (*message));
        let block = Block::default().title(Span::styled(
            (*message).to_string(),
            Style::default().fg(Color::Yellow),
        ));
        f.render_widget(block, chunk);
    }
}
