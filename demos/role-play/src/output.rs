//! Styled output and spinners for role-play demo
//!
//! Provides:
//! - Thinking spinner with random messages
//! - Markdown rendering with syntax highlighting
//! - Error/warning display

use crate::config;
use crate::thinking;
use console::style;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Spinner state with message cycling
struct SpinnerState {
    spinner: Arc<cliclack::ProgressBar>,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

static THINKING_SPINNER: Mutex<Option<SpinnerState>> = Mutex::new(None);

/// Message cycling interval in seconds
const MESSAGE_CYCLE_INTERVAL: u64 = 2;

/// Show the thinking spinner with cycling messages
pub fn show_thinking() {
    if !io::stdout().is_terminal() {
        return;
    }

    let mut guard = THINKING_SPINNER.lock().unwrap();
    if guard.is_some() {
        return; // Already showing
    }

    let spinner = Arc::new(cliclack::spinner());
    spinner.start(format!("{}...", thinking::get_random_message()));

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    let spinner_clone = spinner.clone();

    // Spawn thread to cycle messages every few seconds
    let handle = thread::spawn(move || {
        while !shutdown_clone.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(MESSAGE_CYCLE_INTERVAL));
            if shutdown_clone.load(Ordering::Relaxed) {
                break;
            }
            spinner_clone.set_message(format!("{}...", thinking::get_random_message()));
        }
    });

    *guard = Some(SpinnerState {
        spinner,
        shutdown,
        handle: Some(handle),
    });
}

/// Hide the thinking spinner
pub fn hide_thinking() {
    let mut guard = THINKING_SPINNER.lock().unwrap();
    if let Some(mut state) = guard.take() {
        state.shutdown.store(true, Ordering::Relaxed);
        state.spinner.stop("");
        if let Some(handle) = state.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Print markdown with syntax highlighting using bat
/// Falls back to plain text if terminal doesn't support it or NO_COLOR is set
pub fn print_markdown(content: &str) {
    if !io::stdout().is_terminal() || config::no_color() {
        print!("{}", content);
        let _ = io::stdout().flush();
        return;
    }

    let theme = config::current_theme();

    // Use bat for syntax highlighting
    match bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()))
        .language("Markdown")
        .theme(theme.bat_theme())
        .print()
    {
        Ok(_) => {}
        Err(_) => {
            // Fallback to plain output
            print!("{}", content);
            let _ = io::stdout().flush();
        }
    }
}

/// Display elapsed time
pub fn display_elapsed(elapsed: std::time::Duration) {
    let secs = elapsed.as_secs_f64();
    println!("\n{}", style(format!("Elapsed: {:.1}s", secs)).dim());
}

/// Display an error message
pub fn display_error(msg: &str) {
    eprintln!("{} {}", style("error:").red().bold(), msg);
}

/// Display a warning message
pub fn display_warning(msg: &str) {
    eprintln!("{} {}", style("warning:").yellow().bold(), msg);
}

/// Display goodbye message
pub fn display_goodbye() {
    println!();
    println!("{}", style("Goodbye!").cyan());
}
