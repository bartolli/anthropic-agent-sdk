//! REPL input handling with rustyline
//!
//! Provides readline-style input with history, Ctrl+C handling,
//! and slash command support.

use console::style;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{
    Cmd, Config, Context, EditMode, Editor, EventHandler, Helper, KeyCode, KeyEvent, Modifiers,
};
use std::borrow::Cow;

/// Result of user input
#[derive(Debug)]
pub enum InputResult {
    /// A message to send to Claude
    Message(String),
    /// A slash command (e.g., /help, /quit)
    Command(String),
    /// User wants to exit
    Exit,
    /// Interrupt signal (Ctrl+C with empty line)
    Interrupt,
}

/// Helper struct for rustyline with history hints
pub struct InputHelper {
    hinter: HistoryHinter,
}

impl Default for InputHelper {
    fn default() -> Self {
        Self {
            hinter: HistoryHinter::new(),
        }
    }
}

impl Completer for InputHelper {
    type Candidate = String;

    fn complete(
        &self,
        _line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        // No completion for now
        Ok((0, vec![]))
    }
}

impl Hinter for InputHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for InputHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // Dim the hint text
        Cow::Owned(format!("{}", style(hint).dim()))
    }
}

impl Validator for InputHelper {}

impl Helper for InputHelper {}

/// Create a configured rustyline editor
pub fn create_editor() -> anyhow::Result<Editor<InputHelper, DefaultHistory>> {
    let config = Config::builder()
        .history_ignore_space(true)
        .history_ignore_dups(true)?
        .edit_mode(EditMode::Emacs)
        .auto_add_history(false) // We'll add manually after validation
        .build();

    let mut editor = Editor::with_config(config)?;
    editor.set_helper(Some(InputHelper::default()));

    // Ctrl+J for newlines (multi-line input)
    editor.bind_sequence(
        KeyEvent(KeyCode::Char('j'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::Newline),
    );

    Ok(editor)
}

/// Get the styled prompt string
/// Uses a distinct visual style to separate user input from Claude's output
fn get_prompt_string() -> String {
    if cfg!(target_os = "windows") {
        ">> ".to_string()
    } else {
        // Green arrow prompt for user input (distinct from cyan Claude output)
        format!("{} ", style(">>").green().bold())
    }
}

/// Read input from the user
pub fn get_input(editor: &mut Editor<InputHelper, DefaultHistory>) -> anyhow::Result<InputResult> {
    let prompt = get_prompt_string();

    match editor.readline(&prompt) {
        Ok(line) => {
            let trimmed = line.trim();

            // Empty input - just continue
            if trimmed.is_empty() {
                return Ok(InputResult::Message(String::new()));
            }

            // Check for exit commands
            if trimmed.eq_ignore_ascii_case("quit")
                || trimmed.eq_ignore_ascii_case("exit")
                || trimmed.eq_ignore_ascii_case("/quit")
                || trimmed.eq_ignore_ascii_case("/exit")
            {
                return Ok(InputResult::Exit);
            }

            // Check for slash commands
            if trimmed.starts_with('/') {
                let _ = editor.add_history_entry(&line);
                return Ok(InputResult::Command(trimmed.to_string()));
            }

            // Regular message
            let _ = editor.add_history_entry(&line);
            Ok(InputResult::Message(line))
        }
        Err(ReadlineError::Interrupted) => {
            // Ctrl+C
            Ok(InputResult::Interrupt)
        }
        Err(ReadlineError::Eof) => {
            // Ctrl+D
            Ok(InputResult::Exit)
        }
        Err(e) => Err(e.into()),
    }
}

/// Save history to a file
pub fn save_history(
    editor: &mut Editor<InputHelper, DefaultHistory>,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    editor.save_history(path)?;
    Ok(())
}

/// Load history from a file
pub fn load_history(
    editor: &mut Editor<InputHelper, DefaultHistory>,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    if path.exists() {
        let _ = editor.load_history(path); // Ignore errors on load
    }
    Ok(())
}
