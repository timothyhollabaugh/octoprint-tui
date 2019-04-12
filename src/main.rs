mod octoprint;
mod ui;

use std::io;
use std::time::Duration;

use futures::future::lazy;
use futures::stream::iter;
use futures::sync::mpsc;
use futures::sync::oneshot;
use futures::Future;
use futures::Sink;
use futures::Stream;
use tokio::runtime::Runtime;
use tokio_timer::Interval;

use tui::backend::Backend;
use tui::backend::TermionBackend;
use tui::Terminal;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use octoprint::*;
use ui::*;

// Terminal is 65x177

fn main() -> Result<(), Box<std::error::Error>> {
    println!("Hello, world!");

    let url = "http://localhost:5000".to_string();
    let api_key = "D8F72AC7BBCD4197889E4036B6ACA561".to_string();

    let mut octoprint = OctoprintClient::new(url, api_key);

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    let mut ui = Ui::new(terminal);

    let mut runtime = Runtime::new().unwrap();

    let (tx, rx) = mpsc::channel(1024);

    let mut job_octoprint = octoprint.clone();
    let update_job = Interval::new_interval(Duration::from_secs(1))
        .map_err(UiError::from)
        .and_then(move |now| job_octoprint.load_job().map_err(UiError::from))
        .map_err(|e| eprintln!("Error getting jobs: {:?}", e))
        .fold(tx.clone(), |tx, job_response| {
            tx.send(UiEvent::JobUpdate(job_response))
                .map_err(|e| eprintln!("Could not send event: {:?}", e))
        })
        .map(|_| ());
    runtime.spawn(update_job);

    let mut state_octoprint = octoprint.clone();
    let update_state = Interval::new_interval(Duration::from_secs(1))
        .map_err(UiError::from)
        .and_then(move |now| state_octoprint.load_state().map_err(UiError::from))
        .map_err(|e| eprintln!("Error getting jobs: {:?}", e))
        .fold(tx.clone(), |tx, state_response| {
            tx.send(UiEvent::StateUpdate(state_response))
                .map_err(|e| eprintln!("Could not send event: {:?}", e))
        })
        .map(|_| ());
    runtime.spawn(update_state);

    runtime.spawn(rx.for_each(move |event| {
        ui.draw(event);
        Ok(())
    }));

    iter(io::stdin().keys())
        .map_err(|e| eprintln!("Key error: {:?}", e))
        .filter(|k| *k == Key::Esc)
        .into_future()
        .wait();

    runtime.shutdown_now().wait().expect("Could not showdown");

    Ok(())
}
