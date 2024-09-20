use crate::constants::ECHO_SIZE;
use chrono::offset;
// use crossbeam::channel::{unbounded, Receiver, Sender};
use crossbeam::channel::unbounded;
use log::debug;
use ratatui::{
    layout::Constraint,
    style::Style,
    widgets::{Cell, Row, Table},
};
use std::{
    sync::{Arc, RwLock},
    thread,
};

pub struct EchoArea {
    // pub message: Arc<RwLock<String>>,
    // pub receiver: Receiver<String>,
    // pub sender: Sender<String>,
    pub messages: Arc<RwLock<Vec<String>>>,
}

impl EchoArea {
    pub fn new() -> EchoArea {
        let (_, receiver) = unbounded::<String>();
        let closure_receiver = receiver.clone();
        let message = Arc::new(RwLock::new(String::default()));
        // let message_build = message.clone();
        let messages = Arc::new(RwLock::new(vec![String::from(""); ECHO_SIZE as usize]));
        let messages_build = messages.clone();

        thread::spawn(move || loop {
            match closure_receiver.recv() {
                Ok(message_event) => {
                    let mut message = message.write().unwrap();
                    *message = message_event.clone();
                    let mut messages = messages.write().unwrap();

                    let now = offset::Local::now();
                    let formatted_now = now.format("%d/%m/%Y %H:%M:%S");
                    let string_now = formatted_now.to_string();

                    (*messages).push(format!("[{string_now}] {message_event}"));

                    let messages_length = messages.len() as i32 - ECHO_SIZE;

                    if messages_length > 0 {
                        messages.drain(0..(messages_length as usize));
                    }

                    debug!("ECHO {}", *message);
                }
                Err(e) => println!("watch error: {e}"),
            };
        });

        EchoArea {
            // message: message_build,
            // receiver,
            // sender,
            messages: messages_build,
        }
    }

    pub fn draw(&self, f: &mut ratatui::Frame, chunk: ratatui::layout::Rect) {
        let messages = self.messages.read().unwrap();

        let list_items: Vec<Row> = messages
            .iter()
            .map(|message| {
                Row::new(vec![Cell::from((*message).to_string()).style(
                    Style::default().fg(ratatui::style::Color::Rgb(255, 255, 0)),
                )])
            })
            .collect();

        let constraints = &[Constraint::Percentage(80), Constraint::Length(30)];
        let displayables = Table::new(list_items, constraints);

        f.render_widget(displayables, chunk);
    }
}
