use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask", about = "Workspace automation for worldspace")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all checks: fmt, clippy, tests, deny, doc
    Check,
    /// Run cargo fmt --check on all crates
    Fmt,
    /// Run clippy on all crates
    Clippy,
    /// Run all tests
    Test,
    /// Run cargo deny check
    Deny,
    /// Build rustdoc for the workspace
    Doc,
    /// Build the entire workspace
    Build,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check => {
            run_fmt()?;
            run_clippy()?;
            run_tests()?;
            run_deny()?;
            run_doc()?;
        }
        Commands::Fmt => run_fmt()?,
        Commands::Clippy => run_clippy()?,
        Commands::Test => run_tests()?,
        Commands::Deny => run_deny()?,
        Commands::Doc => run_doc()?,
        Commands::Build => run_build()?,
    }

    Ok(())
}

fn run_fmt() -> Result<()> {
    println!("==> Running cargo fmt --check");
    let status = Command::new("cargo")
        .args(["fmt", "--all", "--", "--check"])
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo fmt check failed");
    }
    Ok(())
}

fn run_clippy() -> Result<()> {
    println!("==> Running cargo clippy");
    let status = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo clippy failed");
    }
    Ok(())
}

fn run_tests() -> Result<()> {
    println!("==> Running cargo test");
    let status = Command::new("cargo")
        .args(["test", "--workspace"])
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo test failed");
    }
    Ok(())
}

fn run_deny() -> Result<()> {
    println!("==> Running cargo deny check (licenses bans sources)");
    let status = Command::new("cargo")
        .args(["deny", "check", "licenses", "bans", "sources"])
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo deny check failed");
    }
    Ok(())
}

fn run_doc() -> Result<()> {
    println!("==> Running cargo doc");
    let status = Command::new("cargo")
        .args(["doc", "--workspace", "--no-deps"])
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo doc failed");
    }
    Ok(())
}

fn run_build() -> Result<()> {
    println!("==> Running cargo build");
    let status = Command::new("cargo")
        .args(["build", "--workspace"])
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo build failed");
    }
    Ok(())
}
