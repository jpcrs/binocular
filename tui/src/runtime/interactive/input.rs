use crate::infra::channel::Sender;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use std::time::Duration;

pub enum InputEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
}

pub fn spawn_input_handler(tx: impl Sender<InputEvent>) {
    std::thread::spawn(move || {
        let tick_rate = Duration::from_millis(16);
        let mut last_tick = std::time::Instant::now();

        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).unwrap_or(false) {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press {
                            let _ = tx.send(InputEvent::Key(key));
                        }
                    }
                    Ok(Event::Resize(w, h)) => {
                        let _ = tx.send(InputEvent::Resize(w, h));
                    }
                    _ => {}
                }
            }

            if last_tick.elapsed() >= tick_rate {
                let _ = tx.send(InputEvent::Tick);
                last_tick = std::time::Instant::now();
            }
        }
    });
}
