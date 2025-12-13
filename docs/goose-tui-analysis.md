# Goose CLI TUI Analysis Report

A comprehensive analysis of the Goose agent's terminal UI mechanics for building a similar demo with the Claude Agent SDK for Rust.

## TUI Libraries Used

| Library | Version | Purpose |
|---------|---------|---------|
| **cliclack** | 0.3.5 | Interactive prompts, spinners, selects, confirms, password inputs |
| **console** | 0.15.8 | Text styling (colors, bold, dim), terminal size, text width measurement |
| **indicatif** | 0.17.11 | Progress bars, MultiProgress for multiple concurrent bars |
| **bat** | 0.24.0 | Markdown rendering with syntax highlighting |
| **rustyline** | 15.0.0 | REPL input with history, completion, hints, Ctrl+C handling |
| **anstream** | 0.6.18 | ANSI-aware println! for cross-platform output |
| **is-terminal** | 0.4.16 | Detect if stdout is a TTY (for conditional formatting) |
| **rand** | 0.8.5 | Random selection for thinking messages |

**Note:** `dialoguer` is **NOT used** - Goose uses `cliclack` for all interactive prompts.

## Architecture Overview

```
goose-cli/src/session/
├── mod.rs              # CliSession orchestration, main loop
├── input.rs            # User input handling, slash commands (rustyline)
├── output.rs           # Message rendering, spinners, styling (cliclack/console)
├── completion.rs       # Tab completion for prompts/commands (rustyline)
├── thinking.rs         # Random "thinking" messages
├── elicitation.rs      # Dynamic form input from schemas (cliclack)
└── task_execution_display/
    └── mod.rs          # ANSI-based task dashboard with live updates
```

## Component Deep Dive

### 1. Thinking Spinner (`output.rs:93-125`)

Uses `cliclack::ProgressBar` as a spinner with random messages:

```rust
pub struct ThinkingIndicator {
    spinner: Option<cliclack::ProgressBar>,
}

impl ThinkingIndicator {
    pub fn show(&mut self) {
        let spinner = cliclack::spinner();
        spinner.start(format!(
            "{}...",
            super::thinking::get_random_thinking_message()
        ));
        self.spinner = Some(spinner);
    }

    pub fn hide(&mut self) {
        if let Some(spinner) = self.spinner.take() {
            spinner.stop("");
        }
    }
}
```

**Key pattern:** Thread-local global spinner accessed via `show_thinking()` / `hide_thinking()`.

### 2. Text Styling (`output.rs`)

Uses `console::style()` for all text decoration:

```rust
use console::{style, Color, Term};

// Colored output
println!("{}", style("error:").red().bold());
println!("{}", style("session id:").dim());
println!("{}", style(provider).cyan().dim());

// Conditional styling
let styled_text = style(text);
if dim { styled_text = styled_text.dim(); }
if let Some(color) = color {
    styled_text = styled_text.fg(color);
}
```

### 3. Markdown Rendering (`output.rs:561-574`)

Uses `bat` for syntax-highlighted markdown:

```rust
fn print_markdown(content: &str, theme: Theme) {
    if std::io::stdout().is_terminal() {
        bat::PrettyPrinter::new()
            .input(bat::Input::from_bytes(content.as_bytes()))
            .theme(theme.as_str())  // "GitHub", "zenburn", "base16"
            .colored_output(env_no_color())
            .language("Markdown")
            .wrapping_mode(WrappingMode::NoWrapping(true))
            .print()
            .unwrap();
    } else {
        print!("{}", content);  // Plain text fallback
    }
}
```

### 4. Progress Bars (`output.rs:899-961`)

Uses `indicatif::MultiProgress` for concurrent progress tracking:

```rust
pub struct McpSpinners {
    bars: HashMap<String, ProgressBar>,
    log_spinner: Option<ProgressBar>,
    multi_bar: MultiProgress,
}

impl McpSpinners {
    pub fn log(&mut self, message: &str) {
        let spinner = self.log_spinner.get_or_insert_with(|| {
            let bar = self.multi_bar.add(
                ProgressBar::new_spinner()
                    .with_style(
                        ProgressStyle::with_template("{spinner:.green} {msg}")
                            .unwrap()
                            .tick_chars("⠋⠙⠚⠛⠓⠒⠊⠉"),
                    )
            );
            bar.enable_steady_tick(Duration::from_millis(100));
            bar
        });
        spinner.set_message(message.to_string());
    }

    pub fn update(&mut self, token: &str, value: f64, total: Option<f64>, message: Option<&str>) {
        let bar = self.bars.entry(token.to_string()).or_insert_with(|| {
            if let Some(total) = total {
                self.multi_bar.add(
                    ProgressBar::new((total * 100_f64) as u64).with_style(
                        ProgressStyle::with_template("[{elapsed}] {bar:40} {pos:>3}/{len:3} {msg}")
                    )
                )
            } else {
                self.multi_bar.add(ProgressBar::new_spinner())
            }
        });
        bar.set_position((value * 100_f64) as u64);
    }
}
```

### 5. REPL Input (`input.rs`)

Uses `rustyline` with custom `GooseCompleter`:

```rust
use rustyline::Editor;

pub fn get_input(
    editor: &mut Editor<GooseCompleter, rustyline::history::DefaultHistory>,
) -> Result<InputResult> {
    // Ctrl+J for newlines
    editor.bind_sequence(
        rustyline::KeyEvent(rustyline::KeyCode::Char('j'), rustyline::Modifiers::CTRL),
        rustyline::EventHandler::Simple(rustyline::Cmd::Newline),
    );

    // Custom Ctrl+C handler
    editor.bind_sequence(
        rustyline::KeyEvent(rustyline::KeyCode::Char('c'), rustyline::Modifiers::CTRL),
        rustyline::EventHandler::Conditional(Box::new(CtrlCHandler)),
    );

    let prompt = get_input_prompt_string();  // "( O)> " styled
    let input = editor.readline(&prompt)?;
    // ...
}
```

**Prompt string:**
```rust
fn get_input_prompt_string() -> String {
    let goose = "( O)>";
    if cfg!(target_os = "windows") {
        format!("{goose} ")
    } else {
        format!("{} ", console::style(goose).cyan().bold())
    }
}
```

### 6. Tab Completion (`completion.rs`)

Implements `rustyline::Completer` for slash commands and prompts:

```rust
impl Completer for GooseCompleter {
    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>)
        -> Result<(usize, Vec<Pair>)>
    {
        if line.starts_with('/') {
            if !line.contains(' ') {
                return self.complete_slash_commands(line);
            }
            if line.starts_with("/prompt") {
                return self.complete_prompt_names(line);
            }
            // ...
        }
        // Fallback to file path completion
        self.complete_file_path(line, ctx)
    }
}
```

### 7. Interactive Configuration (`configure.rs`)

Heavy use of `cliclack` components:

```rust
// Selection menu
let provider_name = cliclack::select("Which model provider should we use?")
    .initial_value(&default_provider)
    .items(&provider_items)
    .interact()?;

// Password input
let value: String = cliclack::password(format!("Enter new value for {}", key.name))
    .mask('▪')
    .interact()?;

// Text input with validation
let name: String = cliclack::input("What would you like to call this extension?")
    .placeholder("my-extension")
    .validate(|input: &String| {
        if input.is_empty() { Err("Please enter a name") }
        else { Ok(()) }
    })
    .interact()?;

// Confirmation
let add_env = cliclack::confirm("Would you like to add environment variables?")
    .interact()?;

// Multi-select with toggles
let selected = cliclack::multiselect("enable extensions:")
    .required(false)
    .items(&extension_items)
    .initial_values(enabled_extensions)
    .interact()?;

// Intro/outro for sections
cliclack::intro(style(" goose-configure ").on_cyan().black())?;
cliclack::outro("Configuration saved successfully")?;

// Spinner for async operations
let spin = cliclack::spinner();
spin.start("Checking your configuration...");
// ... async work ...
spin.stop(style("Model fetch complete").green());
```

### 8. Task Execution Dashboard (`task_execution_display/mod.rs`)

Raw ANSI escape codes for live-updating task display:

```rust
const CLEAR_SCREEN: &str = "\x1b[2J\x1b[H";
const MOVE_TO_PROGRESS_LINE: &str = "\x1b[4;1H";
const CLEAR_TO_EOL: &str = "\x1b[K";
const CLEAR_BELOW: &str = "\x1b[J";

fn format_tasks_update_from_event(event: &TaskExecutionNotificationEvent) -> String {
    let mut display = String::new();

    if !INITIAL_SHOWN.swap(true, Ordering::SeqCst) {
        display.push_str(CLEAR_SCREEN);
        display.push_str("🎯 Task Execution Dashboard\n");
        display.push_str("═══════════════════════════\n\n");
    } else {
        display.push_str(MOVE_TO_PROGRESS_LINE);
    }

    display.push_str(&format!(
        "📊 Progress: {} total | ⏳ {} pending | 🏃 {} running | ✅ {} completed | ❌ {} failed",
        stats.total, stats.pending, stats.running, stats.completed, stats.failed
    ));
    display.push_str(&format!("{}\n\n", CLEAR_TO_EOL));

    // Task details with status icons
    for task in sorted_tasks {
        let status_icon = match task.status {
            TaskStatus::Pending => "⏳",
            TaskStatus::Running => "🏃",
            TaskStatus::Completed => "✅",
            TaskStatus::Failed => "❌",
        };
        // ...
    }
    display
}
```

### 9. Context Usage Visualization (`output.rs:787-823`)

Dot-based progress indicator:

```rust
pub fn display_context_usage(total_tokens: usize, context_limit: usize) {
    let percentage = ((total_tokens as f64 / context_limit as f64) * 100.0).round() as usize;
    let dot_count = 10;
    let filled_dots = ((percentage as f64 / 100.0) * dot_count as f64).round() as usize;
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
        colored_dots, percentage, total_tokens, context_limit
    );
}
```

### 10. Thinking Messages (`thinking.rs`)

200+ playful random messages:

```rust
const THINKING_MESSAGES: &[&str] = &[
    "Spreading wings",
    "Honking thoughtfully",
    "Waddling to conclusions",
    "Reticulating splines",
    "Calculating meaning of life",
    // ... 200+ more
];

pub fn get_random_thinking_message() -> &'static str {
    THINKING_MESSAGES
        .choose(&mut rand::thread_rng())
        .unwrap_or(&THINKING_MESSAGES[0])
}
```

## Main TUI Layout Structure

**Key insight:** Goose uses a **simple vertical scroll layout** - NOT a split-pane TUI like `vim` or `htop`. There's no fixed header/footer; everything scrolls together.

### Visual Layout Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  goose is running! Enter your instructions...    ← GREETING    │
│                                                                 │
│  Context: ●●●●●○○○○○ 42% (4200/10000 tokens)     ← STATUS LINE │
│  ( O)> [user types here]                         ← INPUT PROMPT│
│                                                                 │
│  ◐ Honking thoughtfully...                       ← SPINNER     │
│                                                                 │
│  ─── shell | developer ──────────────────────    ← TOOL HEADER │
│  command: ls -la                                                │
│                                                                 │
│  [Markdown rendered output from assistant]       ← RESPONSE    │
│                                                                 │
│  ⏱️  Elapsed time: 2.3s                          ← ELAPSED     │
│                                                                 │
│  Context: ●●●●●●○○○○ 58% (5800/10000 tokens)     ← STATUS LINE │
│  ( O)> _                                         ← NEXT PROMPT │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Main Loop Flow (`mod.rs:467-610`)

```
┌─────────────────────────────────────────────────────────────────┐
│                     MAIN INTERACTIVE LOOP                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   1. display_greeting()        ← One-time banner at start      │
│                                                                 │
│   ┌──────────────────────────────────────────────────────┐     │
│   │  LOOP:                                               │     │
│   │                                                      │     │
│   │  2. display_context_usage()   ← Status bar           │     │
│   │                                                      │     │
│   │  3. get_input(&editor)        ← BLOCKS for input     │     │
│   │     └─ rustyline handles:                            │     │
│   │        • Cursor position in input line               │     │
│   │        • History navigation (↑/↓)                    │     │
│   │        • Tab completion                              │     │
│   │        • Ctrl+C (clear line or exit)                 │     │
│   │        • Ctrl+J (newline)                            │     │
│   │                                                      │     │
│   │  4. show_thinking()           ← Spinner appears      │     │
│   │                                                      │     │
│   │  5. process_agent_response()  ← Stream processing    │     │
│   │     └─ For each event:                               │     │
│   │        • hide_thinking() before output               │     │
│   │        • render_message() for content                │     │
│   │        • progress_bars for MCP notifications         │     │
│   │                                                      │     │
│   │  6. hide_thinking()           ← Clean up spinner     │     │
│   │                                                      │     │
│   │  7. print elapsed time                               │     │
│   │                                                      │     │
│   └──────────────────────────────────────────────────────┘     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### The Input Line Mechanics (`input.rs`)

The user input area is NOT a fixed position at the bottom. It's handled by `rustyline`:

```rust
pub fn get_input(editor: &mut Editor<GooseCompleter, ...>) -> Result<InputResult> {
    // Key bindings
    editor.bind_sequence(
        KeyEvent(KeyCode::Char('j'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::Newline),      // Ctrl+J = newline
    );
    editor.bind_sequence(
        KeyEvent(KeyCode::Char('c'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(CtrlCHandler)),  // Smart Ctrl+C
    );

    let prompt = get_input_prompt_string();  // "( O)> "
    let input = editor.readline(&prompt)?;   // BLOCKS HERE - cursor managed by rustyline
    // ...
}
```

**Ctrl+C Smart Handler:**
```rust
impl ConditionalEventHandler for CtrlCHandler {
    fn handle(&self, ..., ctx: &EventContext) -> Option<Cmd> {
        if !ctx.line().is_empty() {
            Some(Cmd::Kill(Movement::WholeBuffer))  // Clear line if text exists
        } else {
            Some(Cmd::Interrupt)                     // Exit if empty
        }
    }
}
```

### Visual Elements

#### 1. Greeting Banner
```rust
pub fn display_greeting() {
    println!("\ngoose is running! Enter your instructions, or try asking what goose can do.\n");
}
```

#### 2. Context Status Line (NOT Sticky!)

**Key insight: The status line is NOT sticky or fixed-position.** It's a simple `println!` that appears right before each input prompt. It scrolls away with the rest of the output.

**Positioning Flow:**
```
                                    ← previous output scrolls up
Context: ●●●●●○○○○○ 42%            ← status line (plain println!)
( O)> _                             ← input prompt (rustyline takes over)
```

**When it appears:** Called once per turn, immediately before `get_input()`:
```rust
// mod.rs:467-471
loop {
    self.display_context_usage().await?;  // ← Status line printed HERE
    match input::get_input(&mut editor)? {  // ← Then rustyline blocks for input
        // ...
    }
}
```

**The actual implementation** - just a regular `println!`:
```rust
pub fn display_context_usage(total_tokens: usize, context_limit: usize) {
    // Bounds checking
    if context_limit == 0 {
        println!("Context: Error - context limit is zero");
        return;
    }

    // Calculate percentage (capped at 100%)
    let percentage = (((total_tokens as f64 / context_limit as f64) * 100.0)
        .round() as usize)
        .min(100);

    // Dot visualization: ●●●●●○○○○○
    let dot_count = 10;
    let filled_dots = (((percentage as f64 / 100.0) * dot_count as f64)
        .round() as usize)
        .min(dot_count);
    let empty_dots = dot_count - filled_dots;

    let filled = "●".repeat(filled_dots);
    let empty = "○".repeat(empty_dots);
    let dots = format!("{}{}", filled, empty);

    // Color thresholds: green < 50%, yellow < 85%, red >= 85%
    let colored_dots = if percentage < 50 {
        style(dots).green()
    } else if percentage < 85 {
        style(dots).yellow()
    } else {
        style(dots).red()
    };

    // Just a println! - no ANSI positioning, no sticky behavior
    println!(
        "Context: {} {}% ({}/{} tokens)",
        colored_dots, percentage, total_tokens, context_limit
    );
}
```

**Optional Cost Line** (if `GOOSE_CLI_SHOW_COST=true`):
```rust
// Called right after display_context_usage() if cost display is enabled
pub async fn display_cost_usage(provider: &str, model: &str, input_tokens: usize, output_tokens: usize) {
    if let Some(cost) = estimate_cost_usd(provider, model, input_tokens, output_tokens).await {
        eprintln!(
            "Cost: {} USD ({} tokens: in {}, out {})",
            style(format!("${:.4}", cost)).cyan(),
            input_tokens + output_tokens,
            input_tokens,
            output_tokens
        );
    }
}
```

**What you see before each prompt:**
```
Context: ●●●●●○○○○○ 42% (4200/10000 tokens)
Cost: $0.0123 USD (4200 tokens: in 3500, out 700)    ← only if GOOSE_CLI_SHOW_COST=true
( O)> _
```

#### 3. Input Prompt
```rust
fn get_input_prompt_string() -> String {
    let goose = "( O)>";
    if cfg!(target_os = "windows") {
        format!("{goose} ")  // Plain text on Windows
    } else {
        format!("{} ", console::style(goose).cyan().bold())  // Styled on Unix
    }
}
```

#### 4. Tool Call Headers
```rust
fn print_tool_header(call: &CallToolRequestParam) {
    // Split "developer__shell" into ["shell", "developer"]
    let parts: Vec<_> = call.name.rsplit("__").collect();

    let tool_header = format!(
        "─── {} | {} ──────────────────────────",
        style(parts.first().unwrap_or(&"unknown")),      // "shell"
        style(parts[1..].join("__")).magenta().dim(),    // "developer"
    );
    println!();
    println!("{}", tool_header);
}
// Output: ─── shell | developer ──────────────────────────
```

#### 5. Elapsed Time Footer
```rust
let elapsed_str = format_elapsed_time(elapsed);
println!(
    "\n{}",
    console::style(format!("⏱️  Elapsed time: {}", elapsed_str)).dim()
);
```

### No Fixed Layout - Just Sequential Output

**Critical understanding:** There is NO:
- Fixed header region
- Fixed footer/input region
- Split panes
- Viewport management
- Scroll regions

Everything is sequential `println!` statements that scroll naturally. The "layout" is simply:

```
[previous output scrolls up]
↓
[new output appears at bottom]
↓
[context line]
↓
[prompt] [cursor]  ← rustyline manages THIS LINE ONLY
```

### Why This Works

1. **rustyline** takes over the terminal completely during input
   - Manages cursor position within the input line
   - Handles history, completion popup, line editing
   - Returns control when Enter is pressed

2. **cliclack spinner** manages its own line
   - Saves cursor position internally
   - Animates on that line
   - Clears and restores when stopped

3. **indicatif MultiProgress** manages progress bars
   - Tracks positions of all active bars
   - Redraws as needed
   - Coordinates with other output via `hide()` method

4. **Regular output** just uses `println!`
   - Cursor auto-advances to next line
   - No position tracking needed

## Cursor Position Management

**Critical insight:** Goose does NOT manually track cursor position. Instead, it delegates to libraries or uses raw ANSI for specific cases.

### Cursor Handling Strategy

| Component | Who Manages Cursor | Mechanism |
|-----------|-------------------|-----------|
| **Spinner (thinking)** | `cliclack` | Internal cursor save/restore on start/stop |
| **Progress bars** | `indicatif` | MultiProgress manages all bar positions |
| **REPL input** | `rustyline` | Full line editing with cursor tracking |
| **Task dashboard** | Manual ANSI | Absolute positioning with `\x1b[row;colH` |
| **Normal output** | None | Sequential `println!` (cursor auto-advances) |

### ANSI Escape Codes Used

```rust
// task_execution_display/mod.rs
const CLEAR_SCREEN: &str = "\x1b[2J\x1b[H";     // Clear entire screen, cursor to 1,1
const MOVE_TO_PROGRESS_LINE: &str = "\x1b[4;1H"; // Move cursor to row 4, col 1
const CLEAR_TO_EOL: &str = "\x1b[K";            // Clear from cursor to end of line
const CLEAR_BELOW: &str = "\x1b[J";             // Clear from cursor to end of screen
```

### Spinner Cursor Flow

```
User input → show_thinking() → cliclack saves cursor position
                             → spinner animates on current line
Agent responds → hide_thinking() → cliclack restores cursor
                                → clears spinner line
                                → output continues below
```

**Key code pattern (`mod.rs:847-1033`):**
```rust
output::show_thinking();
let start_time = Instant::now();
self.process_agent_response(true, cancel_token).await?;
output::hide_thinking();  // MUST hide before any other output
```

### Progress Bar Cursor Coordination

```rust
// When showing notifications during progress:
if interactive {
    let _ = progress_bars.hide();  // Pause all bars, clear their lines
    if !is_json_mode {
        println!("{}", message);   // Safe to print now
    }
}
// Progress bars auto-restore on next update
```

### Immediate Flush Pattern

```rust
// For live-updating displays that need immediate visibility:
print!("{}", formatted_message);
std::io::stdout().flush().unwrap();  // Force immediate display
```

---

## Layout & Responsive Design

### Terminal Size Detection

```rust
use console::Term;

// Get terminal dimensions (height, width)
let max_width = Term::stdout()
    .size_checked()                    // Returns Option<(height, width)>
    .map(|(_h, w)| (w as usize).saturating_sub(reserve_width));
```

### Text Width Measurement (ANSI-aware)

```rust
use console::measure_text_width;

// Correctly measures visible width, ignoring ANSI escape codes
let prefix_width = measure_text_width(prefix.as_str());
print_value(value, debug, prefix_width);
```

### Responsive Text Truncation

```rust
fn print_value(value: &Value, debug: bool, reserve_width: usize) {
    let max_width = Term::stdout()
        .size_checked()
        .map(|(_h, w)| (w as usize).saturating_sub(reserve_width));

    let formatted = match value {
        Value::String(s) => match (max_width, debug) {
            // Truncate long strings in non-debug mode
            (Some(w), false) if s.len() > w => style(safe_truncate(s, w)),
            _ => style(s.to_string()),
        }.green(),
        // ...
    };
    println!("{}", formatted);
}
```

### Path Shortening for Display

```rust
fn shorten_path(path: &str, debug: bool) -> String {
    if debug { return path.to_string(); }

    let home = etcetera::home_dir().ok();
    // Convert /Users/foo/bar to ~/bar
    if let Some(home) = home {
        if let Ok(relative) = Path::new(path).strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }
    path.to_string()
}
```

### Indentation System

```rust
const INDENT: &str = "    ";  // 4 spaces

fn print_params(value: &Option<JsonObject>, depth: usize, debug: bool) {
    let indent = INDENT.repeat(depth);  // depth * 4 spaces
    // ...
    println!("{}{}:", indent, style(key).dim());
    print_params(&Some(obj.clone()), depth + 1, debug);  // Recurse deeper
}
```

### Platform-Specific Handling

```rust
fn get_input_prompt_string() -> String {
    let goose = "( O)>";
    if cfg!(target_os = "windows") {
        // Windows: Plain text, no ANSI (compatibility)
        format!("{goose} ")
    } else {
        // Unix/macOS: Styled with ANSI colors
        format!("{} ", console::style(goose).cyan().bold())
    }
}
```

### Non-TTY Fallback

```rust
// Pattern used throughout output.rs
if std::io::stdout().is_terminal() {
    // Rich output: colors, spinners, progress bars
    bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()))
        .theme(theme.as_str())
        .print()
} else {
    // Plain output: no formatting, no interactivity
    print!("{}", content);
}
```

### Environment Variable Checks

```rust
// Respect NO_COLOR environment variable
fn env_no_color() -> bool {
    std::env::var("NO_COLOR").is_ok()
}

// Use in bat printer
.colored_output(env_no_color())
```

---

## Key Patterns for SDK Demo

### 1. Terminal Detection
```rust
if std::io::stdout().is_terminal() {
    // Rich output with colors
} else {
    // Plain text fallback
}
```

### 2. Thread-Local State for UI Components
```rust
thread_local! {
    static THINKING: RefCell<ThinkingIndicator> = RefCell::new(ThinkingIndicator::default());
}

pub fn show_thinking() {
    if std::io::stdout().is_terminal() {
        THINKING.with(|t| t.borrow_mut().show());
    }
}
```

### 3. Theme Support
```rust
pub enum Theme { Light, Dark, Ansi }

thread_local! {
    static CURRENT_THEME: RefCell<Theme> = RefCell::new(
        std::env::var("GOOSE_CLI_THEME").ok()
            .map(|val| Theme::from_config_str(&val))
            .unwrap_or(Theme::Ansi)
    );
}
```

### 4. Safe String Truncation
```rust
use goose::utils::safe_truncate;

let max_width = Term::stdout()
    .size_checked()
    .map(|(_h, w)| (w as usize).saturating_sub(reserve_width));

if s.len() > max_width {
    style(safe_truncate(s, max_width))
}
```

## Recommended Cargo.toml for Demo

```toml
[dependencies]
# Core TUI
cliclack = "0.3"           # Interactive prompts, spinners
console = "0.15"           # Styling, terminal info
indicatif = "0.17"         # Progress bars

# Input
rustyline = "15"           # REPL with history, completion

# Rendering
bat = "0.24"               # Markdown with syntax highlighting
anstream = "0.6"           # ANSI-aware output

# Utils
is-terminal = "0.4"        # TTY detection
rand = "0.8"               # Random selection

# Your SDK
claude-agent-sdk = { path = ".." }
```

## Implementation Checklist for Demo

### Core TUI Components
- [ ] **Thinking Spinner** - `cliclack::spinner()` with random messages
- [ ] **Text Styling** - `console::style()` for colors/bold/dim
- [ ] **Markdown Output** - `bat::PrettyPrinter` for assistant messages
- [ ] **REPL Input** - `rustyline::Editor` with custom prompt
- [ ] **Tab Completion** - Implement `rustyline::Completer` for commands
- [ ] **Progress Bars** - `indicatif::MultiProgress` for tool execution
- [ ] **Configuration Dialogs** - `cliclack::select/input/confirm/password`

### Cursor & Layout (Critical)
- [ ] **Hide before print** - Always call `hide_thinking()` / `progress_bars.hide()` before `println!`
- [ ] **Flush for live updates** - Use `stdout().flush()` for immediate display
- [ ] **Terminal size detection** - `Term::stdout().size_checked()` for responsive width
- [ ] **ANSI-aware width** - `console::measure_text_width()` for accurate measurement
- [ ] **Text truncation** - `safe_truncate(s, max_width)` respecting terminal width
- [ ] **Raw ANSI for dashboards** - `\x1b[row;colH` for fixed-position updates

### Responsive Design
- [ ] **Terminal Detection** - Graceful fallback for non-TTY with `is_terminal()`
- [ ] **NO_COLOR support** - Respect `NO_COLOR` env var for accessibility
- [ ] **Windows compatibility** - Plain text prompts on Windows
- [ ] **Path shortening** - Display `~/` instead of full home path

### User Feedback
- [ ] **Context Display** - Dot-based usage visualization with color thresholds
- [ ] **Error Styling** - Red bold prefix with dim details
- [ ] **Elapsed time** - Show response time after completion

## Additional Interesting Components

### 11. Dynamic Form Input / Elicitation (`elicitation.rs`)

Schema-driven form input for collecting structured data from users:

```rust
pub fn collect_elicitation_input(message: &str, schema: &Value) -> io::Result<Option<HashMap<String, Value>>> {
    // Display the prompt message
    if !message.is_empty() {
        println!("\n{}", style(message).cyan());
    }

    // Parse JSON schema properties
    let properties = schema.get("properties").and_then(|p| p.as_object());
    let required: Vec<&str> = schema.get("required")...;

    for (name, field_schema) in properties {
        let field_type = field_schema.get("type").and_then(|t| t.as_str()).unwrap_or("string");

        // Boolean fields get a cliclack toggle
        if field_type == "boolean" {
            match cliclack::confirm(&label).initial_value(default_bool).interact() {
                Ok(v) => data.insert(name.clone(), Value::Bool(v)),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(None),
                // ...
            }
            continue;
        }

        // Other fields get manual input with styling
        print!("{}", style(name).yellow());
        if let Some(desc) = description {
            print!(" {}", style(format!("({})", desc)).dim());
        }
        if is_required {
            print!("{}", style("*").red());
        }
        print!(": ");
        io::stdout().flush()?;
        // ...
    }
}
```

### 12. Theme System (`output.rs:22-91`)

Three-theme system with persistent config:

```rust
#[derive(Clone, Copy)]
pub enum Theme {
    Light,  // bat theme: "GitHub"
    Dark,   // bat theme: "zenburn"
    Ansi,   // bat theme: "base16" (terminal colors)
}

// Thread-local with environment and config fallback
thread_local! {
    static CURRENT_THEME: RefCell<Theme> = RefCell::new(
        std::env::var("GOOSE_CLI_THEME").ok()
            .map(|val| Theme::from_config_str(&val))
            .unwrap_or_else(||
                Config::global().get_param::<String>("GOOSE_CLI_THEME").ok()
                    .map(|val| Theme::from_config_str(&val))
                    .unwrap_or(Theme::Ansi)
            )
    );
}

pub fn set_theme(theme: Theme) {
    // Persists to config file
    config.set_param("GOOSE_CLI_THEME", theme.as_config_string())?;
    CURRENT_THEME.with(|t| *t.borrow_mut() = theme);
}
```

### 13. Session Info Display (`output.rs:713-780`)

Startup banner with provider/model info:

```rust
pub fn display_session_info(resume: bool, provider: &str, model: &str, session_id: &Option<String>, ...) {
    let start_session_msg = if resume { "resuming session |" }
                            else if session_id.is_none() { "running without session |" }
                            else { "starting session |" };

    // Check for lead/worker mode (multi-model)
    if let Some(lead_worker) = provider_instance.as_lead_worker() {
        let (lead_model, worker_model) = lead_worker.get_model_info();
        println!(
            "{} {} {} {} {} {} {}",
            style(start_session_msg).dim(),
            style("provider:").dim(), style(provider).cyan().dim(),
            style("lead model:").dim(), style(&lead_model).cyan().dim(),
            style("worker model:").dim(), style(&worker_model).cyan().dim(),
        );
    }

    if let Some(id) = session_id {
        println!("    {} {}", style("session id:").dim(), style(id).cyan().dim());
    }

    println!("    {} {}", style("working directory:").dim(),
        style(std::env::current_dir().unwrap().display()).cyan().dim());
}
```

### 14. Extension Loading Spinner (`builder.rs:474-489`)

Async loading indicator for multiple extensions:

```rust
let spinner = cliclack::spinner();
spinner.start(get_message(&waiting_on));  // "starting 5 extensions: dev, git, ..."

while let Some(result) = set.join_next().await {
    match result {
        Ok((name, Ok(_))) => {
            waiting_on.remove(&name);
            spinner.set_message(get_message(&waiting_on));  // Updates live
        }
        Ok((name, Err(e))) => offer_debug.push((name, e)),
        // ...
    }
}

spinner.clear();  // Remove spinner when done
```

### 15. Extension Debugging Helper (`builder.rs:107-206`)

Interactive debugging when extensions fail to load:

```rust
async fn offer_extension_debugging_help(extension_name: &str, error_message: &str, ...) {
    if !interactive { return Ok(()); }

    let should_help = cliclack::confirm(format!(
        "Would you like me to help debug the '{}' extension failure?", extension_name
    )).initial_value(false).interact()?;

    if should_help {
        println!("{}", style("🔧 Starting debugging session...").cyan());
        // Spawns a mini-agent to analyze the error
        let debug_prompt = format!(
            "I'm having trouble starting an extension called '{}'. Error:\n\n{}\n\n
            Help diagnose: missing dependencies, config problems, network, permissions...",
            extension_name, error_message
        );
        debug_session.headless(debug_prompt).await?;
        println!("{}", style("✅ Debugging session completed.").green());
    }
}
```

### 16. Path Shortening (`output.rs:664-711`)

Intelligent path truncation for display:

```rust
fn shorten_path(path: &str, debug: bool) -> String {
    if debug { return path.to_string(); }

    // Convert home dir to ~
    let home = etcetera::home_dir().ok();
    if let Some(home) = home {
        if let Ok(relative) = Path::new(path).strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }

    // For long paths: /very/long/path/file.txt → /v/l/p/file.txt
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 4 { return path.to_string(); }

    let mut shortened = Vec::new();
    shortened.push(parts[0].to_string());  // Keep root

    // Abbreviate middle components to first char
    for component in &parts[1..parts.len() - 2] {
        shortened.push(component.chars().next().unwrap_or('?').to_string());
    }

    // Keep last two components full
    shortened.push(parts[parts.len() - 2].to_string());
    shortened.push(parts[parts.len() - 1].to_string());

    shortened.join("/")
}
// "/vvvvvvvvvvvvv/long/path/with/many/components/file.txt" → "/v/l/p/w/m/components/file.txt"
```

### 17. Working Directory Change Warning (`builder.rs:406-427`)

Confirmation dialog when resuming in different directory:

```rust
if current_workdir != session.working_dir {
    let change_workdir = cliclack::confirm(format!(
        "{} The original working directory was {}. Current: {}. Switch back?",
        style("WARNING:").yellow(),
        style(session.working_dir.display()).cyan(),
        style(current_workdir.display()).cyan()
    ))
    .initial_value(true)
    .interact()?;

    if change_workdir {
        std::env::set_current_dir(&session.working_dir)?;
    }
}
```

### 18. Markdown Export (`export.rs`)

Converts conversation to markdown with syntax highlighting:

```rust
pub fn message_to_markdown(message: &Message, export_all_content: bool) -> String {
    for content in &message.content {
        match content {
            MessageContent::Text(text) => md.push_str(&text.text),
            MessageContent::ToolRequest(req) => md.push_str(&tool_request_to_markdown(req)),
            MessageContent::Thinking(thinking) => {
                md.push_str("**Thinking:**\n> ");
                md.push_str(&thinking.thinking.replace("\n", "\n> "));
            }
            // ...
        }
    }
}

// Auto-detects JSON/XML in tool responses
fn tool_response_to_markdown(resp: &ToolResponse) -> String {
    let trimmed = text.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}')) {
        format!("```json\n{}\n```\n", trimmed)
    } else if trimmed.starts_with('<') && trimmed.contains("</") {
        format!("```xml\n{}\n```\n", trimmed)
    } else {
        text.to_string()
    }
}
```

### 19. Logging (No Console Output) (`logging.rs`)

All logs go to files, not the TUI:

```rust
pub fn setup_logging(name: Option<&str>, ...) -> Result<()> {
    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        Rotation::NEVER, log_dir, log_filename
    );

    // JSON file logging layer - NO console output
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)  // No ANSI in files
        .json();

    // Console logging disabled for CLI - all logs go to files only
    let layers = vec![
        file_layer.with_filter(env_filter).boxed(),
    ];
}
```

## Summary: The Goose TUI Philosophy

1. **Delegate cursor management** - Let libraries (cliclack, indicatif, rustyline) handle complexity
2. **Linear layout** - No grid/box layout; just vertical `println!` with indentation
3. **Raw ANSI for special cases** - Only use escape codes for live-updating dashboards
4. **Always hide before output** - Spinner/progress must be hidden before any new text
5. **Graceful degradation** - Every feature has a plain-text fallback
6. **Respect user preferences** - Check `NO_COLOR`, `GOOSE_CLI_THEME`, terminal capabilities
