//! Theme and configuration settings for role-play demo

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
    std::env::var("CLAUDE_THEME")
        .map(|s| Theme::from_str(&s))
        .unwrap_or_default()
}

/// Check if `NO_COLOR` environment variable is set
pub fn no_color() -> bool {
    std::env::var("NO_COLOR").is_ok()
}
