//! Theme and configuration settings
//!
//! Provides theme selection and display settings for the TUI.

/// Available color themes for markdown rendering
#[derive(Clone, Copy, Debug, Default)]
pub enum Theme {
    /// Light theme (GitHub style)
    Light,
    /// Dark theme (zenburn style)
    Dark,
    /// ANSI theme (uses terminal colors)
    #[default]
    Ansi,
}

impl Theme {
    /// Get the bat theme name for this theme
    pub fn bat_theme(&self) -> &str {
        match self {
            Theme::Light => "GitHub",
            Theme::Dark => "zenburn",
            Theme::Ansi => "base16",
        }
    }

    /// Parse theme from environment or config string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::Ansi,
        }
    }
}

/// Get the current theme from environment
pub fn current_theme() -> Theme {
    std::env::var("CLAUDE_TUI_THEME")
        .map(|s| Theme::from_str(&s))
        .unwrap_or_default()
}

/// Default context limit for token usage display
pub const DEFAULT_CONTEXT_LIMIT: usize = 200_000;

/// Check if we should show cost information
pub fn show_cost() -> bool {
    std::env::var("CLAUDE_TUI_SHOW_COST")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Check if NO_COLOR environment variable is set
pub fn no_color() -> bool {
    std::env::var("NO_COLOR").is_ok()
}
