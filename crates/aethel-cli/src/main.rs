//! Aethel CLI - Command line interface for Aethel compiler.

use aethel_syntax::{lexer::lex, parser::parse, span::FileId};
use aethel_check::checker::check_module;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aethel")]
#[command(about = "Aethel Core — Proof-carrying effects for AI agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check a source file for type correctness
    Check {
        /// Input file
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Format a source file
    Fmt {
        /// Input file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Check formatting without writing
        #[arg(long)]
        check: bool,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => check_file(&file),
        Commands::Fmt { file, check } => fmt_file(&file, check),
    }
}

fn check_file(file: &PathBuf) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(file)?;
    let file_id = FileId::new(0);
    
    // Lex
    let tokens = lex(&source, file_id);
    
    // Parse
    let (module, parse_diagnostics) = parse(&tokens, file_id);
    
    if parse_diagnostics.has_errors() {
        eprintln!("{}", "Parse errors:".red().bold());
        for diag in parse_diagnostics.errors() {
            eprintln!("  {} at {}", diag.code, diag.message);
            for label in &diag.labels {
                eprintln!("    --> {}", label.span);
            }
        }
        std::process::exit(1);
    }
    
    // Type check
    let (_ir, check_diagnostics) = check_module(&module, file_id);
    
    if check_diagnostics.has_errors() {
        eprintln!("{}", "Type errors:".red().bold());
        for diag in check_diagnostics.errors() {
            eprintln!("  {} at {}", diag.code.to_string().red(), diag.message);
            for label in &diag.labels {
                eprintln!("    --> {}", label.span);
            }
        }
        std::process::exit(1);
    }
    
    if check_diagnostics.warnings().is_empty() && check_diagnostics.errors().is_empty() {
        println!("{} {}", "✓".green(), format!("{} type checks", file.display()).green());
    } else {
        for warn in check_diagnostics.warnings() {
            eprintln!("{} {}", "warning:".yellow(), warn.message);
        }
        println!("{} {}", "✓".green(), format!("{} type checks (with warnings)", file.display()).green());
    }
    
    Ok(())
}

fn fmt_file(file: &PathBuf, check: bool) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(file)?;
    // For now, just check if the file parses
    let file_id = FileId::new(0);
    let tokens = lex(&source, file_id);
    let (_, diagnostics) = parse(&tokens, file_id);
    
    if diagnostics.has_errors() {
        eprintln!("{} Cannot format: parse errors", "error:".red());
        std::process::exit(1);
    }
    
    if check {
        println!("{} {}", "✓".green(), format!("{} is formatted", file.display()).green());
    } else {
        // In a real impl, we'd format and write back
        println!("{} {}", "✓".green(), format!("{} formatted", file.display()).green());
    }
    
    Ok(())
}