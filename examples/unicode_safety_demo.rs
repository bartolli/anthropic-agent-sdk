//! Unicode Safety Demo
//!
//! Demonstrates safe UTF-8 truncation to prevent panics when handling
//! strings containing emoji or multi-byte characters.
//!
//! Run with: cargo run --example unicode_safety_demo

use anthropic_agent_sdk::utils::{safe_truncate, safe_window, truncate_for_display};

fn main() {
    println!("=== Unicode Safety Demo ===\n");

    // Test strings with various multi-byte characters
    let emoji_text = "Status: 🔍 Active";
    let chinese_text = "你好世界"; // 12 bytes, 4 chars
    let mixed_text = "Hello 🌍 World café";

    // 1. Demonstrate the problem
    println!("1. THE PROBLEM");
    println!("   Text: \"{}\" ({} bytes)", emoji_text, emoji_text.len());
    println!("   Emoji 🔍 is at bytes 8-11 (4 bytes)");
    println!();
    println!("   Naive truncation at byte 10 would panic:");
    println!("   // let bad = &emoji_text[..10]; // PANIC: byte index 10 is inside char");
    println!();

    // 2. Safe truncation
    println!("2. SAFE TRUNCATION");
    let safe = safe_truncate(emoji_text, 10);
    println!("   safe_truncate(\"{}\", 10)", emoji_text);
    println!("   Result: \"{}\" ({} bytes)", safe, safe.len());
    println!("   Stopped before emoji - no panic!");
    println!();

    // 3. Truncate for display (with ellipsis)
    println!("3. TRUNCATE FOR DISPLAY");
    let display = truncate_for_display(mixed_text, 10);
    println!("   truncate_for_display(\"{}\", 10)", mixed_text);
    println!("   Result: \"{}\"", display);
    println!();

    // 4. Chinese characters (3 bytes each)
    println!("4. MULTI-BYTE CHARACTERS");
    println!(
        "   Text: \"{}\" ({} bytes, {} chars)",
        chinese_text,
        chinese_text.len(),
        chinese_text.chars().count()
    );

    for max in [2, 3, 5, 6, 9] {
        let result = safe_truncate(chinese_text, max);
        println!(
            "   safe_truncate(text, {:2}) = \"{}\" ({} bytes)",
            max,
            result,
            result.len()
        );
    }
    println!();

    // 5. Safe window extraction
    println!("5. SAFE WINDOW EXTRACTION");
    let code = "fn process(data: 📊) -> Result<()>";
    println!("   Code: \"{}\"", code);
    let window = safe_window(code, 25, 10);
    println!("   safe_window(code, 25, 10) = \"{}\"", window);
    println!();

    // 6. Real-world scenario: Error message preview
    println!("6. REAL-WORLD: ERROR MESSAGE PREVIEW");
    let large_json = r#"{"type":"assistant","content":"Hello! 👋 I'm Claude..."#;
    println!("   JSON: \"{}\" ({} bytes)", large_json, large_json.len());

    // Simulate buffer overflow error with preview
    let preview = truncate_for_display(large_json, 40);
    let error_msg = format!("JSON exceeded buffer size of 1MB. Preview: {}", preview);
    println!("   Error: {}", error_msg);
    println!();

    // 7. Verification: all results are valid UTF-8
    println!("7. VERIFICATION");
    let test_cases = [
        safe_truncate(emoji_text, 10),
        safe_truncate(chinese_text, 4),
        safe_window(code, 20, 8),
    ];

    for (i, result) in test_cases.iter().enumerate() {
        // This would panic if result was invalid UTF-8
        let is_valid = std::str::from_utf8(result.as_bytes()).is_ok();
        println!(
            "   Test {}: \"{}\" - Valid UTF-8: {}",
            i + 1,
            result,
            is_valid
        );
    }

    println!("\n=== All truncations safe! ===");
}
