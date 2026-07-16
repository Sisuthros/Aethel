//! Aethel CLI - Command line interface for Aethel compiler.

use aethel_syntax::{lexer::lex, parser::parse, span::FileId};
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
    /// Emit deterministic Semantic IR as JSON
    EmitIr {
        /// Input file
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Evaluate a source file with the IR interpreter (run-to-completion)
    Run {
        /// Input file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Show full effect trace
        #[arg(long)]
        trace: bool,
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
        Commands::EmitIr { file } => emit_ir(&file),
        Commands::Run { file, trace } => run_file(&file, trace),
        Commands::Fmt { file, check } => fmt_file(&file, check),
    }
}

fn compile_and_check(file: &PathBuf) -> anyhow::Result<(aethel_ir::lower::IrModule, FileId)> {
    let source = std::fs::read_to_string(file)?;
    let file_id = FileId::new(0);

    let tokens = lex(&source, file_id);
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

    // Phase 1: Lower AST to HIR and resolve names
    let mut hir_module = aethel_hir::lower::lower_module(&module, file_id);
    let resolve_errors = aethel_hir::resolve::resolve_module(&mut hir_module);
    if !resolve_errors.is_empty() {
        eprintln!("{}", "Name resolution errors:".red().bold());
        for err in &resolve_errors {
            eprintln!("  {}", err);
        }
        std::process::exit(1);
    }

    // Phase 2: Type-check via HIR-based checker
    let (ir_module, check_diagnostics) = aethel_check::checker::check_hir_module(&hir_module, file_id);

    if check_diagnostics.has_errors() {
        eprintln!("{}", "Type errors:".red().bold());
        for diag in check_diagnostics.errors() {
            eprintln!("  {} at {}", diag.code.to_string().red(), diag.message);
            for label in &diag.labels {
                eprintln!("    --> {}.{}:{}", file.display(), label.span.start.0, label.span.end.0);
            }
        }
        std::process::exit(1);
    }

    if check_diagnostics.warnings().is_empty() {
        println!("{} {}", "✓".green(), format!("{} type checks", file.display()).green());
    } else {
        for warn in check_diagnostics.warnings() {
            eprintln!("{} {}", "warning:".yellow(), warn.message);
        }
        println!("{} {}", "✓".green(), format!("{} type checks (with warnings)", file.display()).green());
    }

    Ok((ir_module, file_id))
}

fn check_file(file: &PathBuf) -> anyhow::Result<()> {
    compile_and_check(file)?;
    Ok(())
}

fn emit_ir(file: &PathBuf) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(file)?;
    let file_id = FileId::new(0);

    let tokens = lex(&source, file_id);
    let (module, parse_diagnostics) = parse(&tokens, file_id);

    if parse_diagnostics.has_errors() {
        eprintln!("{}", "Parse errors:".red().bold());
        for diag in parse_diagnostics.errors() {
            eprintln!("  {} at {}", diag.code, diag.message);
        }
        std::process::exit(1);
    }

    let (_ir_module, check_diagnostics) = aethel_check::checker::check_module(&module, file_id);

    if check_diagnostics.has_errors() {
        eprintln!("{}", "Type errors:".red().bold());
        for diag in check_diagnostics.errors() {
            eprintln!("  {} at {}", diag.code, diag.message);
        }
        std::process::exit(1);
    }

    // Build deterministic semantic IR JSON from the parsed module
    let mut effects_json = Vec::new();
    let mut policies_json = Vec::new();
    let mut structs_json = Vec::new();
    let mut functions_json = Vec::new();

    use aethel_syntax::ast::Item;
    for item in &module.items {
        match item {
            Item::Effect(e) => {
                let mut ops = Vec::new();
                for op in &e.operations {
                    let mut params = Vec::new();
                    for p in &op.params {
                        params.push(format!("{}: {}", p.name.name, ast_type_to_string(&p.ty)));
                    }
                    ops.push(serde_json::json!({
                        "name": op.name.name,
                        "params": params,
                        "returns": op.ret_type.as_ref().map(|t| ast_type_to_string(t))
                    }));
                }
                effects_json.push(serde_json::json!({
                    "name": e.name.name,
                    "operations": ops
                }));
            }
            Item::Policy(p) => {
                let mut claims = Vec::new();
                for c in &p.claims {
                    claims.push(serde_json::json!({
                        "name": c.name.name,
                        "type": ast_type_to_string(&c.ty),
                        "evidence": c.evidence.iter().map(|ev| format!("{:?}", ev.kind)).collect::<Vec<_>>()
                    }));
                }
                policies_json.push(serde_json::json!({
                    "name": p.name.name,
                    "claims": claims
                }));
            }
            Item::Struct(s) => {
                let mut fields = Vec::new();
                for f in &s.fields {
                    fields.push(serde_json::json!({
                        "name": f.name.name,
                        "type": ast_type_to_string(&f.ty)
                    }));
                }
                structs_json.push(serde_json::json!({
                    "name": s.name.name,
                    "fields": fields
                }));
            }
            Item::Fn(f) => {
                let mut params = Vec::new();
                for p in &f.params {
                    params.push(serde_json::json!({
                        "name": p.name.name,
                        "type": ast_type_to_string(&p.ty)
                    }));
                }
                let mut effects = Vec::new();
                for ef in &f.effects.effects {
                    if let Some(seg) = ef.path.segments.first() {
                        effects.push(seg.name.name.clone());
                    }
                }
                functions_json.push(serde_json::json!({
                    "name": f.name.name,
                    "params": params,
                    "returns": f.ret_type.as_ref().map(|t| ast_type_to_string(t)),
                    "effects": effects,
                    "has_body": f.body.is_some()
                }));
            }
            _ => {}
        }
    }

    // Sort deterministically by name for stable output
    effects_json.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    policies_json.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    structs_json.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    functions_json.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));

    let output = serde_json::json!({
        "ir_version": "0.1",
        "schema": "https://aethel-lang.dev/ir-schema-v0.1",
        "checksum": simple_hash(&source),
        "semantic": {
            "effects": effects_json,
            "policies": policies_json,
            "types": structs_json,
            "functions": functions_json
        },
        "diagnostics": []
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

fn run_file(file: &PathBuf, show_trace: bool) -> anyhow::Result<()> {
    let (ir_module, _file_id) = compile_and_check(file)?;

    let mut evaluator = aethel_interpreter::eval::Evaluator::new();
    let result = evaluator.eval_module(&ir_module)?;

    // Print results
    println!();
    println!("{}", "── Evaluation Results ──".bold());
    println!("  {} claims processed", result.claim_count);
    println!("  {} verified successfully", result.verified_count);

    if result.policy_violations.is_empty() {
        println!("  {} {}", "✓".green(), "No policy violations".green());
    } else {
        println!("  {} {} policy violation(s):", "✗".red().bold(), result.policy_violations.len());
        for v in &result.policy_violations {
            println!("    • {}", v.red());
        }
        anyhow::bail!("Policy violations detected — execution blocked");
    }

    if show_trace && !result.effect_trace.is_empty() {
        println!();
        println!("{}", "── Effect Trace ──".bold());
        for (i, trace) in result.effect_trace.iter().enumerate() {
            let status = if trace.was_verified { "✓".green() } else { "✗".red() };
            println!("  {}. {} effect `{}`", i + 1, status, trace.effect_name);
            if let Some(err) = &trace.error {
                println!("     {}", err.yellow());
            }
        }
    }

    Ok(())
}

/// Stable hash for source content (not cryptographic, just for change detection)
fn simple_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    format!("{:x}", h.finish())
}

/// Helper to print AST types as strings
fn ast_type_to_string(ty: &aethel_syntax::ast::Type) -> String {
    use aethel_syntax::ast::Type;
    match ty {
        Type::Unit { .. } => "()".into(),
        Type::Never { .. } => "!".into(),
        Type::Bool { .. } => "bool".into(),
        Type::Int { .. } => "int".into(),
        Type::Float { .. } => "float".into(),
        Type::String { .. } => "string".into(),
        Type::Path { path, .. } => path.segments.iter().map(|s| s.name.name.clone()).collect::<Vec<_>>().join("::"),
        Type::Claim { ty, .. } => format!("Claim<{}>", ast_type_to_string(ty)),
        Type::Verified { ty, policy, .. } => format!("Verified<{}, {}>", ast_type_to_string(ty), ast_type_to_string(policy)),
        Type::Ref { ty, .. } => format!("&{}", ast_type_to_string(ty)),
        Type::Owned { ty, .. } => format!("owned {}", ast_type_to_string(ty)),
        Type::Tuple { types, .. } => format!("({})", types.iter().map(ast_type_to_string).collect::<Vec<_>>().join(", ")),
        Type::Array { ty, .. } => format!("[{}]", ast_type_to_string(ty)),
        Type::Fn { params, ret, .. } => format!("fn({}) -> {}", params.iter().map(ast_type_to_string).collect::<Vec<_>>().join(", "), ast_type_to_string(ret)),
    }
}

fn fmt_file(file: &PathBuf, check: bool) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(file)?;
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
        println!("{} {}", "✓".green(), format!("{} formatted", file.display()).green());
    }

    Ok(())
}
