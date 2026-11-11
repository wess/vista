//! `vistamcp` — Vista's Model Context Protocol server.
//!
//! Speaks MCP over stdio so any coding agent that supports MCP
//! can create machines, power them, snapshot them, look at their screens,
//! type and click, and run commands and move files inside them.
//!
//! `vistamcp job <id>` is the same binary working a machine-creation job in
//! the background, so a download outlives the agent session that asked for
//! it.

mod docs;
mod machine;
mod output;
mod rpc;
mod tools;

#[tokio::main]
async fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.first().map(|a| a == "job").unwrap_or(false) {
    let Some(id) = args.get(1) else {
      eprintln!("usage: vistamcp job <id>");
      std::process::exit(2);
    };
    std::process::exit(tools::jobs::work(id).await);
  }
  if args.first().map(|a| a == "--help" || a == "-h").unwrap_or(false) {
    println!(
      "vistamcp — Vista's MCP server (stdio).\n\nPoint an MCP-capable agent at this command:\n  {}",
      std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "vistamcp".into())
    );
    return;
  }
  rpc::serve().await;
}
