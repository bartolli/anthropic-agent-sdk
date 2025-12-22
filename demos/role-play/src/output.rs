//! Styled output and spinners for role-play demo
//!
//! Provides:
//! - Thinking spinner with random messages
//! - Markdown rendering with syntax highlighting
//! - Scene analysis display
//! - Error/warning display

use crate::config;
use crate::thinking;
use console::style;

/// Result from Haiku scene analyzer
pub struct AnalysisResult {
    pub tension_from: u32,
    pub tension_to: u32,
    pub heat_from: u32,
    pub heat_to: u32,
    pub beat_changed: bool,
    pub beat: String,
}

/// Display scene analysis summary (minimal divider style)
#[allow(clippy::cast_possible_wrap)] // tension (1-10) and heat (1-5) are well within i32 range
pub fn display_scene_analysis(result: &AnalysisResult) {
    let tension_delta = result.tension_to as i32 - result.tension_from as i32;
    let heat_delta = result.heat_to as i32 - result.heat_from as i32;

    // Color-coded deltas
    let tension_change = match tension_delta.cmp(&0) {
        std::cmp::Ordering::Greater => style(format!("+{tension_delta}")).red(),
        std::cmp::Ordering::Less => style(format!("{tension_delta}")).green(),
        std::cmp::Ordering::Equal => style("=".to_string()).dim(),
    };

    let heat_change = match heat_delta.cmp(&0) {
        std::cmp::Ordering::Greater => style(format!("+{heat_delta}")).magenta(),
        std::cmp::Ordering::Less => style(format!("{heat_delta}")).cyan(),
        std::cmp::Ordering::Equal => style("=".to_string()).dim(),
    };

    // Minimal divider style
    println!(
        "\n{}",
        style("─── SCENE ─────────────────────────────────").dim()
    );

    // Tension line
    print!(
        "  {} {} → {} ",
        style("Tension:").dim(),
        result.tension_from,
        result.tension_to
    );
    print!("({tension_change})  ");

    // Heat on same line
    print!(
        "{} {} → {} ",
        style("Heat:").dim(),
        result.heat_from,
        result.heat_to
    );
    println!("({heat_change})");

    // Beat line
    if result.beat_changed {
        println!(
            "  {} {} {}",
            style("Beat:").dim(),
            style(&result.beat).yellow().bold(),
            style("(changed)").yellow()
        );
    } else {
        println!("  {} {}", style("Beat:").dim(), style(&result.beat).dim());
    }
}
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

static ANALYZER_SPINNER: Mutex<Option<SpinnerState>> = Mutex::new(None);

/// Show the analyzer spinner with scene-specific messages
pub fn show_analyzing() {
    if !io::stdout().is_terminal() {
        return;
    }

    let mut guard = ANALYZER_SPINNER.lock().unwrap();
    if guard.is_some() {
        return; // Already showing
    }

    let spinner = Arc::new(cliclack::spinner());
    spinner.start(format!("{}...", thinking::get_analyzer_message()));

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
            spinner_clone.set_message(format!("{}...", thinking::get_analyzer_message()));
        }
    });

    *guard = Some(SpinnerState {
        spinner,
        shutdown,
        handle: Some(handle),
    });
}

/// Hide the analyzer spinner
pub fn hide_analyzing() {
    let mut guard = ANALYZER_SPINNER.lock().unwrap();
    if let Some(mut state) = guard.take() {
        state.shutdown.store(true, Ordering::Relaxed);
        state.spinner.stop("");
        if let Some(handle) = state.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Print markdown with syntax highlighting using bat
/// Falls back to plain text if terminal doesn't support it or `NO_COLOR` is set
pub fn print_markdown(content: &str) {
    if !io::stdout().is_terminal() || config::no_color() {
        print!("{content}");
        let _ = io::stdout().flush();
        return;
    }

    let theme = config::current_theme();

    // Use bat for syntax highlighting
    if bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()))
        .language("Markdown")
        .theme(theme.bat_theme())
        .print()
        .is_err()
    {
        // Fallback to plain output
        print!("{content}");
        let _ = io::stdout().flush();
    }
}

/// Display elapsed time
pub fn display_elapsed(elapsed: std::time::Duration) {
    let secs = elapsed.as_secs_f64();
    println!("\n{}", style(format!("Elapsed: {secs:.1}s")).dim());
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
