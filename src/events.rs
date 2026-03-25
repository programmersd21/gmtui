use crate::error::Result;
use crate::gmail::models::Email;
use crossterm::event::{self, Event};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

pub enum AppEvent {
    Input(Event),
    Tick,
    InboxLoaded(Result<Vec<Email>>),
    EmailLoaded(Result<Email>, bool),
}

pub struct EventHandler {
    rx: Receiver<AppEvent>,
    _tx: Sender<AppEvent>,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let thread_tx = tx.clone();
        thread::spawn(move || loop {
            if event::poll(Duration::from_millis(250)).unwrap_or(false) {
                if let Ok(event) = event::read() {
                    match event {
                        Event::Key(_) => {
                            let _ = thread_tx.send(AppEvent::Input(event));
                        }
                        _ => {
                            let _ = thread_tx.send(AppEvent::Input(event));
                        }
                    }
                }
            } else {
                let _ = thread_tx.send(AppEvent::Tick);
            }
        });

        Self { rx, _tx: tx }
    }

    pub fn next(&self) -> std::result::Result<AppEvent, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn sender(&self) -> Sender<AppEvent> {
        self._tx.clone()
    }
}
