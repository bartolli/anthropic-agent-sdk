//! MCP Server Demo
//!
//! This example demonstrates creating an MCP server with custom tools using rmcp.
//! It shows:
//! 1. Defining tools with the `#[tool]` macro
//! 2. Listing available tools with descriptions
//! 3. Calling tools directly to verify they work
//!
//! Run with: cargo run --example mcp_server --features rmcp

#[cfg(feature = "rmcp")]
mod server {
    use anthropic_agent_sdk::mcp::{
        Parameters, ServerCapabilities, ServerHandler, ServerInfo, ToolRouter, tool, tool_handler,
        tool_router,
    };
    use schemars::JsonSchema;
    use serde::Deserialize;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // ========================================================================
    // Tool Parameter Types
    // ========================================================================

    #[derive(Deserialize, JsonSchema)]
    pub struct CalcParams {
        /// First number
        pub a: f64,
        /// Second number
        pub b: f64,
    }

    #[derive(Deserialize, JsonSchema)]
    pub struct WeatherParams {
        /// City name to get weather for
        pub city: String,
    }

    #[derive(Deserialize, JsonSchema)]
    pub struct NoteParams {
        /// Note content to save
        pub content: String,
    }

    // ========================================================================
    // Demo MCP Server
    // ========================================================================

    /// A demo MCP server with calculator, weather, and notes tools
    #[derive(Clone)]
    pub struct DemoServer {
        notes: Arc<Mutex<Vec<String>>>,
        tool_router: ToolRouter<Self>,
    }

    impl DemoServer {
        pub fn new() -> Self {
            Self {
                notes: Arc::new(Mutex::new(Vec::new())),
                tool_router: Self::tool_router(),
            }
        }
    }

    #[tool_router]
    impl DemoServer {
        #[tool(description = "Add two numbers together")]
        fn add(&self, Parameters(params): Parameters<CalcParams>) -> String {
            format!("{} + {} = {}", params.a, params.b, params.a + params.b)
        }

        #[tool(description = "Subtract two numbers (a - b)")]
        fn subtract(&self, Parameters(params): Parameters<CalcParams>) -> String {
            format!("{} - {} = {}", params.a, params.b, params.a - params.b)
        }

        #[tool(description = "Multiply two numbers")]
        fn multiply(&self, Parameters(params): Parameters<CalcParams>) -> String {
            format!("{} × {} = {}", params.a, params.b, params.a * params.b)
        }

        #[tool(description = "Divide two numbers (a / b). Returns error if b is zero.")]
        fn divide(&self, Parameters(params): Parameters<CalcParams>) -> Result<String, String> {
            if params.b == 0.0 {
                return Err("Cannot divide by zero".to_string());
            }
            Ok(format!(
                "{} ÷ {} = {}",
                params.a,
                params.b,
                params.a / params.b
            ))
        }

        #[tool(description = "Get current weather for a city (simulated data)")]
        fn get_weather(&self, Parameters(params): Parameters<WeatherParams>) -> String {
            let temp = (params.city.len() as f64 * 3.7) % 35.0 + 10.0;
            let conditions = match params.city.len() % 4 {
                0 => "sunny",
                1 => "cloudy",
                2 => "rainy",
                _ => "partly cloudy",
            };
            format!("Weather in {}: {:.1}°C, {}", params.city, temp, conditions)
        }

        #[tool(description = "Save a note to the notebook")]
        async fn save_note(&self, Parameters(params): Parameters<NoteParams>) -> String {
            let mut notes = self.notes.lock().await;
            notes.push(params.content.clone());
            format!("Note saved! Total notes: {}", notes.len())
        }

        #[tool(description = "List all saved notes")]
        async fn list_notes(&self) -> String {
            let notes = self.notes.lock().await;
            if notes.is_empty() {
                "No notes saved yet.".to_string()
            } else {
                let mut result = format!("Notes ({}):\n", notes.len());
                for (i, note) in notes.iter().enumerate() {
                    result.push_str(&format!("{}. {}\n", i + 1, note));
                }
                result
            }
        }

        #[tool(description = "Clear all saved notes")]
        async fn clear_notes(&self) -> String {
            let mut notes = self.notes.lock().await;
            let count = notes.len();
            notes.clear();
            format!("Cleared {} notes.", count)
        }
    }

    #[tool_handler]
    impl ServerHandler for DemoServer {
        fn get_info(&self) -> ServerInfo {
            ServerInfo {
                instructions: Some(
                    "A demo MCP server with calculator, weather, and notes tools.".into(),
                ),
                capabilities: ServerCapabilities::builder().enable_tools().build(),
                ..Default::default()
            }
        }
    }

    fn print_separator(title: &str) {
        let line = "─".repeat(58);
        println!("\n┌{}┐", line);
        println!("│ {:<56} │", title);
        println!("└{}┘", line);
    }

    fn print_tool(name: &str, description: &str, params: &[(&str, &str)]) {
        println!("  ▸ {}", name);
        println!("    {}", description);
        if !params.is_empty() {
            print!("    Parameters: ");
            let param_strs: Vec<_> = params
                .iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect();
            println!("{}", param_strs.join(", "));
        }
        println!();
    }

    fn print_call(call: &str, result: &str, is_error: bool) {
        println!("  {} {}", if is_error { "✗" } else { "✓" }, call);
        if is_error {
            println!("    └─ Error: {}", result);
        } else {
            println!("    └─ {}", result);
        }
    }

    pub async fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║              MCP Server Demo (using rmcp)                ║");
        println!("╚══════════════════════════════════════════════════════════╝");

        // Create the server
        let server = DemoServer::new();
        let info = server.get_info();

        println!();
        println!(
            "  Server:  {} v{}",
            info.server_info.name, info.server_info.version
        );
        if let Some(ref instructions) = info.instructions {
            println!("  Purpose: {}", instructions);
        }

        // List tools by category
        print_separator("Calculator Tools");
        print_tool(
            "add",
            "Add two numbers together",
            &[("a", "number"), ("b", "number")],
        );
        print_tool(
            "subtract",
            "Subtract two numbers (a - b)",
            &[("a", "number"), ("b", "number")],
        );
        print_tool(
            "multiply",
            "Multiply two numbers",
            &[("a", "number"), ("b", "number")],
        );
        print_tool(
            "divide",
            "Divide two numbers (returns error if b is zero)",
            &[("a", "number"), ("b", "number")],
        );

        print_separator("Weather Tools");
        print_tool(
            "get_weather",
            "Get current weather for a city (simulated)",
            &[("city", "string")],
        );

        print_separator("Notes Tools (Stateful)");
        print_tool(
            "save_note",
            "Save a note to the notebook",
            &[("content", "string")],
        );
        print_tool("list_notes", "List all saved notes", &[]);
        print_tool("clear_notes", "Clear all saved notes", &[]);

        print_separator("Tool Demonstrations");
        println!();

        // Calculator demos
        let params = CalcParams { a: 42.0, b: 17.0 };
        let result = server.add(Parameters(params));
        print_call("add(42, 17)", &result, false);

        let params = CalcParams { a: 100.0, b: 37.0 };
        let result = server.subtract(Parameters(params));
        print_call("subtract(100, 37)", &result, false);

        let params = CalcParams { a: 6.0, b: 7.0 };
        let result = server.multiply(Parameters(params));
        print_call("multiply(6, 7)", &result, false);

        let params = CalcParams { a: 10.0, b: 0.0 };
        match server.divide(Parameters(params)) {
            Ok(r) => print_call("divide(10, 0)", &r, false),
            Err(e) => print_call("divide(10, 0)", &e, true),
        }

        let params = CalcParams { a: 22.0, b: 7.0 };
        match server.divide(Parameters(params)) {
            Ok(r) => print_call("divide(22, 7)", &r, false),
            Err(e) => print_call("divide(22, 7)", &e, true),
        }

        println!();

        // Weather demo
        let params = WeatherParams {
            city: "Tokyo".to_string(),
        };
        let result = server.get_weather(Parameters(params));
        print_call("get_weather(\"Tokyo\")", &result, false);

        let params = WeatherParams {
            city: "London".to_string(),
        };
        let result = server.get_weather(Parameters(params));
        print_call("get_weather(\"London\")", &result, false);

        println!();

        // Notes demo (stateful)
        let params = NoteParams {
            content: "Hello from Rust!".to_string(),
        };
        let result = server.save_note(Parameters(params)).await;
        print_call("save_note(\"Hello from Rust!\")", &result, false);

        let params = NoteParams {
            content: "MCP is awesome".to_string(),
        };
        let result = server.save_note(Parameters(params)).await;
        print_call("save_note(\"MCP is awesome\")", &result, false);

        let result = server.list_notes().await;
        println!("  ✓ list_notes()");
        for line in result.lines() {
            println!("    │ {}", line);
        }

        let result = server.clear_notes().await;
        print_call("clear_notes()", &result, false);

        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║                     Demo Complete                        ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  To use with Claude, see: mcp_integration.rs             ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();

        Ok(())
    }

    /// Run as MCP stdio server (for Claude integration)
    pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
        use anthropic_agent_sdk::mcp::{ServiceExt, stdio};

        let server = DemoServer::new();
        let (stdin, stdout) = stdio();

        // Serve the MCP protocol over stdio and wait for completion
        let running = server.serve((stdin, stdout)).await?;
        running.waiting().await?;

        Ok(())
    }
}

#[cfg(feature = "rmcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::IsTerminal;

    // When spawned by Claude (stdin not a terminal), serve MCP protocol
    // When run interactively (stdin is terminal), show demo
    if std::io::stdin().is_terminal() {
        server::run_demo().await
    } else {
        server::run_stdio_server().await
    }
}

#[cfg(not(feature = "rmcp"))]
fn main() {
    eprintln!("This example requires the 'rmcp' feature.");
    eprintln!("Run with: cargo run --example mcp_server --features rmcp");
    std::process::exit(1);
}
