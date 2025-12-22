//! Security Demo
//!
//! Demonstrates the SDK's security measures:
//! - Dangerous environment variable blocking (strict enforcement)
//! - CLI argument allowlist enforcement (strict enforcement)
//!
//! Run with: cargo run --example `security_demo`

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Security Demo ===\n");

    // ========================================================================
    // 1. Dangerous Environment Variables (strict enforcement)
    // ========================================================================
    println!("--- 1. Dangerous Environment Variables - Rejection ---\n");
    println!("These env vars are BLOCKED (returns error):");
    println!("  - LD_PRELOAD           (Linux dynamic linker injection)");
    println!("  - LD_LIBRARY_PATH      (Linux library path manipulation)");
    println!("  - DYLD_INSERT_LIBRARIES (macOS dylib injection)");
    println!("  - DYLD_LIBRARY_PATH    (macOS library path manipulation)");
    println!("  - PATH                 (Command search path manipulation)");
    println!("  - NODE_OPTIONS         (Node.js runtime injection)");
    println!("  - PYTHONPATH           (Python module injection)");
    println!("  - PERL5LIB             (Perl module injection)");
    println!("  - RUBYLIB              (Ruby library injection)");
    println!();

    // Try to set dangerous env vars - will be rejected with error
    let mut options_with_dangerous_env = ClaudeAgentOptions::builder().max_turns(1).build();

    options_with_dangerous_env
        .env
        .insert("LD_PRELOAD".to_string(), "/tmp/malicious.so".to_string());
    options_with_dangerous_env.env.insert(
        "NODE_OPTIONS".to_string(),
        "--require=/tmp/evil.js".to_string(),
    );

    println!("Attempting to set: LD_PRELOAD, NODE_OPTIONS");

    match ClaudeSDKClient::new(options_with_dangerous_env, None).await {
        Ok(_) => {
            println!("  ERROR: Should have been rejected!");
        }
        Err(e) => {
            println!("  REJECTED: {e}");
            println!("  Security: Dangerous env vars cause connection failure");
        }
    }
    println!();

    // ========================================================================
    // 1b. Safe Environment Variables (allowed)
    // ========================================================================
    println!("--- 1b. Safe Environment Variables - Success ---\n");

    let mut options_safe_env = ClaudeAgentOptions::builder().max_turns(1).build();

    options_safe_env
        .env
        .insert("MY_SAFE_VAR".to_string(), "this is allowed".to_string());
    options_safe_env
        .env
        .insert("CUSTOM_CONFIG".to_string(), "some value".to_string());

    println!("Setting safe env vars: MY_SAFE_VAR, CUSTOM_CONFIG");

    match ClaudeSDKClient::new(options_safe_env, None).await {
        Ok(mut client) => {
            println!("  ACCEPTED: Client connected with safe env vars");
            client.close().await?;
        }
        Err(e) => {
            println!("  Connection failed (unrelated to env vars): {e}");
        }
    }
    println!();

    // ========================================================================
    // 2. CLI Argument Allowlist - REJECTION (strict enforcement)
    // ========================================================================
    println!("--- 2. CLI Argument Allowlist - Rejection ---\n");
    println!("Only these CLI flags are allowed in extra_args:");
    println!("  - timeout");
    println!("  - retries");
    println!("  - log-level");
    println!("  - cache-dir");
    println!();

    let mut options_bad_args = ClaudeAgentOptions::builder().max_turns(1).build();

    // Add a disallowed flag - this will cause an error
    options_bad_args
        .extra_args
        .insert("execute-code".to_string(), Some("rm -rf /".to_string()));

    println!("Attempting to use disallowed flag: --execute-code");

    match ClaudeSDKClient::new(options_bad_args, None).await {
        Ok(_) => {
            println!("  ERROR: Should have been rejected!");
        }
        Err(e) => {
            println!("  REJECTED: {e}");
            println!("  Security: Disallowed flags cause connection failure");
        }
    }
    println!();

    // ========================================================================
    // 3. CLI Argument Allowlist - SUCCESS (allowed flags)
    // ========================================================================
    println!("--- 3. CLI Argument Allowlist - Success ---\n");

    let mut options_good_args = ClaudeAgentOptions::builder().max_turns(1).build();

    // Add allowed flags
    options_good_args
        .extra_args
        .insert("timeout".to_string(), Some("30000".to_string()));
    options_good_args
        .extra_args
        .insert("log-level".to_string(), Some("warn".to_string()));

    println!("Using allowed flags: --timeout 30000 --log-level warn");

    match ClaudeSDKClient::new(options_good_args, None).await {
        Ok(mut client) => {
            println!("  ACCEPTED: Client connected successfully");
            client.close().await?;
        }
        Err(e) => {
            println!("  Connection failed (unrelated to flags): {e}");
        }
    }
    println!();

    // ========================================================================
    // Summary
    // ========================================================================
    println!("=== Security Summary ===\n");
    println!("1. Dangerous env vars: Returns error (strict enforcement)");
    println!("2. Disallowed CLI flags: Returns error (strict enforcement)");
    println!("3. Safe env vars and allowed flags: Work normally");
    println!("4. Session binding: Auto-binds on first Result (see session_binding_demo)");
    println!();
    println!("All security violations are LOUD - developers are alerted immediately.");
    println!("See SECURITY.md for full documentation.");

    Ok(())
}
