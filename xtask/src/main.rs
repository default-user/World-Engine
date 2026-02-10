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
    /// Run all checks: fmt, clippy, tests
    Check,
    /// Run cargo fmt --check on all crates
    Fmt,
    /// Run clippy on all crates
    Clippy,
    /// Run all tests
    Test,
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
        }
        Commands::Fmt => run_fmt()?,
        Commands::Clippy => run_clippy()?,
        Commands::Test => run_tests()?,
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
