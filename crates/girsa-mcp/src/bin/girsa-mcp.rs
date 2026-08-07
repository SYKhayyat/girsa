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

use girsa_corpus::argv::{self, Argv};
use std::io::{BufRead, Write};
use std::path::PathBuf;

const USAGE: &str = "\
usage: girsa-mcp <corpus> <personal> <index>

  Speaks MCP on stdin and stdout. All three are required, and the count is
  exact: this is started by another program, and a default that quietly
  opened the wrong shelf would be a program answering confidently about a
  library nobody asked about.";

fn main() -> std::process::ExitCode {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    if Argv::wants_help(&typed) {
        return argv::asked(USAGE);
    }
    let args = match Argv::of(typed, &[], &[]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return argv::refuse(USAGE);
        }
    };
    let [root, personal, index] = args.words() else {
        return argv::refuse(USAGE);
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
