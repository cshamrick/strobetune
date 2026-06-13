//! strobetune — an analog-style strobe tuner for the terminal.
//!
//! The display is driven entirely by the phase interaction between an
//! input-driven virtual lamp and a reference wheel turning at the selected
//! note's frequency. There is no pitch detection, cents calculation, or
//! "in tune" decision anywhere in the signal path.

mod app;
mod audio;
mod config;
mod detector;
mod dsp;
mod notes;
mod strobe;
mod ui;

use std::io;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event};

use app::App;
use audio::AudioHandle;

const HELP: &str = "\
strobetune — an analog-style strobe tuner for the terminal

Usage: strobetune [options]

Options:
  -h, --help       Show this help and exit
  -V, --version    Show version and exit

Controls (while running):
  1-6  select string        a      toggle auto string detect
  t/T  next / prev tuning    [ ]    transpose by a semitone
  0    reset transpose       q      quit
";

fn main() -> io::Result<()> {
    let args = std::env::args().skip(1);
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("strobetune {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }

    // Start audio first so its status (device name or failure reason) is shown.
    let audio = AudioHandle::start();
    let mut app = App::new(audio);

    // ratatui::init() enters the alternate screen + raw mode and installs a
    // panic hook that restores the terminal, so a panic won't leave a mess.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    // Surface the audio status after the terminal is back to normal.
    println!("strobetune exited. {}", app.audio.status);
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    while !app.should_quit {
        app.update();
        terminal.draw(|frame| ui::render(frame, app))?;

        // Block up to one frame budget for input; this also paces the loop.
        if event::poll(Duration::from_millis(config::FRAME_POLL_MS))? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key);
            }
        }
    }
    Ok(())
}
