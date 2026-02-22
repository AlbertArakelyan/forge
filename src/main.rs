mod app;
mod event;
mod terminal;
mod error;
mod ui;
mod state;
mod actions;
mod http;
mod storage;
mod env;
mod scripting;

use std::time::Duration;
use tokio::sync::mpsc;

use crate::app::App;
use crate::event::Event;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    // Background thread: read crossterm events and feed into channel
    let event_tx = tx.clone();
    std::thread::spawn(move || loop {
        if crossterm::event::poll(Duration::from_millis(16)).unwrap_or(false) {
            match crossterm::event::read() {
                Ok(crossterm::event::Event::Key(key)) => {
                    let _ = event_tx.send(Event::Key(key));
                }
                Ok(crossterm::event::Event::Mouse(mouse)) => {
                    let _ = event_tx.send(Event::Mouse(mouse));
                }
                Ok(crossterm::event::Event::Resize(w, h)) => {
                    let _ = event_tx.send(Event::Resize(w, h));
                }
                _ => {}
            }
        } else {
            let _ = event_tx.send(Event::Tick);
        }
    });

    let mut terminal = terminal::init()?;
    let mut app = App::new(tx);

    let result = run_loop(&mut terminal, &mut app, &mut rx).await;

    terminal::restore()?;
    result
}

async fn run_loop(
    terminal: &mut terminal::Tui,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<Event>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| ui::layout::render(frame, &app.state))?;

        match rx.recv().await {
            Some(event) => app.handle_event(event),
            None => break,
        }

        if app.state.should_quit {
            break;
        }
    }
    Ok(())
}
