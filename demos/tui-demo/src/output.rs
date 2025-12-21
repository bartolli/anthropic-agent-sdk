//! Styled output, spinners, and markdown rendering
//!
//! Provides Goose-style terminal output with:
//! - Thinking spinner with random messages
//! - Styled text output
//! - Markdown rendering with syntax highlighting
//! - Context usage visualization

use crate::config;
use crate::thinking;
use console::{Term, style};
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

/// Display the welcome greeting
pub fn display_greeting() {
    println!();
    println!(
        "{}",
        style("╔════════════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("║     Claude Interactive TUI Demo            ║").cyan()
    );
    println!(
        "{}",
        style("╠════════════════════════════════════════════╣").cyan()
    );
    println!(
        "{}",
        style("║  Type your message and press Enter         ║").cyan()
    );
    println!(
        "{}",
        style("║  Commands: /help, /quit, quit, exit        ║").cyan()
    );
    println!(
        "{}",
        style("╚════════════════════════════════════════════╝").cyan()
    );
    println!();
}

/// Display session information
pub fn display_session_info(session_id: Option<&str>) {
    print!("{} ", style("starting session |").dim());
    print!("{} ", style("provider:").dim());
    print!("{} ", style("claude-code").cyan().dim());

    if let Some(id) = session_id {
        println!();
        println!(
            "    {} {}",
            style("session id:").dim(),
            style(id).cyan().dim()
        );
    } else {
        println!();
    }

    if let Ok(cwd) = std::env::current_dir() {
        println!(
            "    {} {}",
            style("working directory:").dim(),
            style(cwd.display()).cyan().dim()
        );
    }
    println!();
}

/// Display context usage with dot visualization
pub fn display_context_usage(tokens: usize, limit: usize) {
    if limit == 0 {
        return;
    }

    let percentage = ((tokens as f64 / limit as f64) * 100.0).round() as usize;
    let percentage = percentage.min(100);

    let dot_count = 10;
    let filled_dots = ((percentage as f64 / 100.0) * dot_count as f64).round() as usize;
    let filled_dots = filled_dots.min(dot_count);
    let empty_dots = dot_count - filled_dots;

    let filled = "●".repeat(filled_dots);
    let empty = "○".repeat(empty_dots);
    let dots = format!("{}{}", filled, empty);

    let colored_dots = if percentage < 50 {
        style(dots).green()
    } else if percentage < 85 {
        style(dots).yellow()
    } else {
        style(dots).red()
    };

    println!(
        "Context: {} {}% ({}/{} tokens)",
        colored_dots, percentage, tokens, limit
    );
}

/// Display cost information (if enabled)
pub fn display_cost(cost: Option<f64>) {
    if !config::show_cost() {
        return;
    }

    if let Some(cost) = cost {
        println!("  {}", style(format!("[cost: ${:.4}]", cost)).dim());
    }
}

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
    println!(
        "\n{}",
        style(format!("⏱️  Elapsed time: {:.1}s", secs)).dim()
    );
}

/// Display a tool use header (like goose's `─── Read | file.rs ──────`)
pub fn display_tool_header(tool_name: &str, detail: Option<&str>) {
    let detail_str = detail
        .map(|d| format!(" | {}", style(d).dim()))
        .unwrap_or_default();

    println!(
        "{} {}{}",
        style("───").dim(),
        style(tool_name).cyan(),
        detail_str
    );
}

/// Display Claude's response prefix (first line of response)
pub fn display_claude_prefix() {
    println!(); // Empty line for visual separation
    print!("{} ", style("claude>").cyan().bold());
    let _ = io::stdout().flush();
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

/// Display help information
pub fn display_help() {
    println!();
    println!("{}", style("Available Commands:").bold());
    println!("  {}  - Show this help", style("/help").cyan());
    println!("  {}  - Clear the screen", style("/clear").cyan());
    println!(
        "  {}  - Test spinner animation (3s)",
        style("/test-spinner").cyan()
    );
    println!("  {}  - Exit the application", style("/quit").cyan());
    println!("  {}   - Exit the application", style("quit").cyan());
    println!("  {}   - Exit the application", style("exit").cyan());
    println!();
    println!("{}", style("Keyboard Shortcuts:").bold());
    println!(
        "  {}    - Cancel current line or exit",
        style("Ctrl+C").cyan()
    );
    println!("  {}    - Exit the application", style("Ctrl+D").cyan());
    println!(
        "  {}    - Insert newline (multi-line input)",
        style("Ctrl+J").cyan()
    );
    println!();
}

/// Clear the terminal screen
pub fn clear_screen() {
    let _ = Term::stdout().clear_screen();
}

/// Get terminal width for text wrapping
#[allow(dead_code)]
pub fn terminal_width() -> Option<usize> {
    Term::stdout().size_checked().map(|(_h, w)| w as usize)
}
