//! Random thinking messages for the spinner
//!
//! Displays playful messages while waiting for Claude's response.

use rand::prelude::IndexedRandom;

/// Collection of thinking messages (inspired by Goose CLI)
const THINKING_MESSAGES: &[&str] = &[
    "Thinking deeply",
    "Processing request",
    "Analyzing context",
    "Formulating response",
    "Considering options",
    "Pondering the question",
    "Consulting knowledge",
    "Reasoning through",
    "Synthesizing thoughts",
    "Crafting response",
    "Evaluating approach",
    "Computing answer",
    "Gathering insights",
    "Connecting ideas",
    "Reflecting carefully",
    "Weighing possibilities",
    "Exploring solutions",
    "Mapping concepts",
    "Building understanding",
    "Composing thoughts",
];

/// Get a random thinking message
pub fn get_random_message() -> &'static str {
    THINKING_MESSAGES
        .choose(&mut rand::rng())
        .unwrap_or(&THINKING_MESSAGES[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_message_returns_valid() {
        let msg = get_random_message();
        assert!(THINKING_MESSAGES.contains(&msg));
    }
}
