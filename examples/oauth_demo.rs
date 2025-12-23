//! OAuth Authentication Demo
//!
//! Demonstrates the OAuth authentication flow for Claude:
//! 1. Check for cached token
//! 2. If no valid token, start OAuth flow
//! 3. Open browser to authorization URL
//! 4. User authenticates and copies authorization code
//! 5. Exchange code for access token
//! 6. Cache token for future use
//!
//! Run with: cargo run --example oauth_demo

use anthropic_agent_sdk::auth::{OAuthClient, TokenStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "anthropic_agent_sdk=debug".parse().unwrap()),
        )
        .init();

    println!("╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                        Claude OAuth Authentication Demo                        ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Check command line args
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "logout" | "--logout" => {
                return logout();
            }
            "status" | "--status" => {
                return status();
            }
            "help" | "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                print_help();
                return Ok(());
            }
        }
    }

    // Create OAuth client with default settings
    let client = OAuthClient::new()?;

    // Check current authentication status
    println!("Checking authentication status...");
    println!();

    if client.is_authenticated() {
        if let Some(token) = client.current_token() {
            println!("✓ Already authenticated!");
            println!();
            print_token_info(&token);

            println!();
            println!("To log out: cargo run --example oauth_demo -- logout");
            return Ok(());
        }
    }

    // Start OAuth flow
    println!("No valid token found. Starting OAuth flow...");
    println!();

    let token = client.authenticate().await?;

    println!();
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!();
    print_token_info(&token);

    println!();
    println!("You can now use this token for Claude API calls.");
    println!("The token is automatically cached and will be reused on future runs.");

    Ok(())
}

fn logout() -> Result<(), Box<dyn std::error::Error>> {
    println!("Logging out...");
    println!();

    let client = OAuthClient::new()?;

    if client.is_authenticated() {
        client.logout()?;
    } else {
        println!("Not currently authenticated.");
    }

    Ok(())
}

fn status() -> Result<(), Box<dyn std::error::Error>> {
    println!("Authentication Status");
    println!("─────────────────────");
    println!();

    let storage = TokenStorage::new();

    match storage.load() {
        Ok(token) => {
            if token.is_expired() {
                println!("Status: Expired");
                println!();
                println!("Token exists but has expired.");
                if token.refresh_token.is_some() {
                    println!(
                        "A refresh token is available - run 'cargo run --example oauth_demo' to refresh."
                    );
                } else {
                    println!("No refresh token - you'll need to re-authenticate.");
                }
            } else {
                println!("Status: Authenticated ✓");
                println!();
                print_token_info(&token);
            }
        }
        Err(_) => {
            println!("Status: Not authenticated");
            println!();
            println!("No cached token found.");
            println!("Run 'cargo run --example oauth_demo' to authenticate.");
        }
    }

    println!();
    println!("Token storage: {}", storage.path().display());

    Ok(())
}

fn print_token_info(token: &anthropic_agent_sdk::auth::TokenInfo) {
    println!("Token Information:");
    println!("  Type: {}", token.token_type);
    println!(
        "  Access Token: {}...",
        &token.access_token[..20.min(token.access_token.len())]
    );

    if let Some(ref refresh) = token.refresh_token {
        println!("  Refresh Token: {}...", &refresh[..20.min(refresh.len())]);
    }

    if let Some(ref scope) = token.scope {
        println!("  Scopes: {}", scope);
    }

    if let Some(remaining) = token.remaining_validity() {
        let hours = remaining.as_secs() / 3600;
        let minutes = (remaining.as_secs() % 3600) / 60;
        println!("  Expires in: {}h {}m", hours, minutes);
    }
}

fn print_help() {
    println!("Usage: cargo run --example oauth_demo [COMMAND]");
    println!();
    println!("Commands:");
    println!("  (none)     Start OAuth flow or show cached token");
    println!("  status     Show current authentication status");
    println!("  logout     Delete cached token");
    println!("  help       Show this help message");
    println!();
    println!("OAuth Flow:");
    println!("  1. Run without arguments to start authentication");
    println!("  2. Browser opens to Claude authentication page");
    println!("  3. Log in and authorize the application");
    println!("  4. Copy the authorization code shown");
    println!("  5. Paste the code in the terminal");
    println!("  6. Token is cached for future use");
    println!();
    println!("Token Storage:");
    let storage = TokenStorage::new();
    println!("  {}", storage.path().display());
}
