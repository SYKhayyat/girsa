//! The MCP server, on a pipe.
//!
//! ```sh
//! cargo run --release -p girsa-mcp -- corpus personal index
//! ```
//!
//! One JSON-RPC request per line on stdin, one response per line on stdout —
//! which is why **nothing but responses may be written to stdout**. Progress,
//! warnings and the greeting all go to stderr; a stray `println!` here would be
//! read by the client as an answer to whatever it asked last.
//!
//! Registered with a client the usual way, as a command it launches:
//!
//! ```json
//! {"mcpServers": {"girsa": {
//!   "command": "girsa-mcp",
//!   "args": ["/path/to/corpus", "/path/to/personal", "/path/to/index"]
//! }}}
//! ```
//!
//! No port and no socket. spec.md §14 makes offline the product, and the only
//! program that can reach this one is the program that started it.

// The greeting goes to stderr. stdout is the protocol.
#![allow(clippy::print_stderr)]

use std::io::{BufRead, Write};
use std::path::PathBuf;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, personal, index] = args.as_slice() else {
        eprintln!("usage: girsa-mcp <corpus-root> <personal-root> <index-dir>");
        return std::process::ExitCode::from(2);
    };

    let mut server = match girsa_mcp::Server::open(
        &PathBuf::from(root),
        &PathBuf::from(personal),
        &PathBuf::from(index),
    ) {
        Ok(server) => server,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    eprintln!(
        "girsa-mcp: {} seforim on the shelf{}",
        server.shelf().works().len(),
        if girsa_link::inbound::built(server.root()) {
            String::new()
        } else {
            format!(
                ", and no inbound cache under {} — trace and links will be short. \
                 Run girsa-link-types.",
                server.root().join("links").display()
            )
        }
    );

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        let Some(answer) = server.serve(&line) else {
            // A notification. No reply, by the protocol.
            continue;
        };
        // Flushed per line: the client is blocking on this pipe, and a buffered
        // answer is an answer that never arrives.
        if writeln!(stdout, "{answer}").is_err() || stdout.flush().is_err() {
            break;
        }
    }
    std::process::ExitCode::SUCCESS
}
