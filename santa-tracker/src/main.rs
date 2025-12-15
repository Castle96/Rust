mod santa;
mod renderer;
mod effects;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, stdout};
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide
    )?;

    // Run the tracker
    let result = run_tracker().await;

    // Cleanup terminal
    execute!(
        stdout,
        terminal::LeaveAlternateScreen,
        cursor::Show
    )?;
    terminal::disable_raw_mode()?;

    result
}

async fn run_tracker() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = santa::SantaTracker::new();
    let mut renderer = renderer::Renderer::new()?;
    let mut interval = time::interval(Duration::from_millis(100));

    loop {
        // Check for quit event
        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Char('q') || key_event.code == KeyCode::Esc {
                    break;
                }
            }
        }

        // Update state
        tracker.update();
        renderer.update();

        // Render frame
        renderer.render(&tracker)?;

        interval.tick().await;
    }

    Ok(())
}
