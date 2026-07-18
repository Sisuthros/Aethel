//! Aethel CLI - Command line interface for Aethel compiler.
//!
//! All commands share one compilation pipeline:
//!   source → parse → HIR lowering → resolution → semantic checking → checked IrModule

use aethel_check::checker;
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

/// Compile a source file through the unified pipeline.
/// Returns the checked IrModule for any downstream use (run, emit-ir).
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

    // Unified pipeline: AST → HIR → resolve → semantic check → IR
    let mut hir_module = aethel_hir::lower::lower_module(&module, file_id);
    let resolve_errors = aethel_hir::resolve::resolve_module(&mut hir_module);
    if !resolve_errors.is_empty() {
        eprintln!("{}", "Name resolution errors:".red().bold());
        for err in &resolve_errors {
            eprintln!("  {}", err);
        }
        std::process::exit(1);
    }

    // Semantic check on HIR, producing checked IrModule
    let (ir_module, check_diagnostics) = checker::check_hir_module(&hir_module, file_id);

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
    let (ir_module, _file_id) = compile_and_check(file)?;

    // Serialize the checked IrModule through a deterministic DTO.
    // We do NOT walk the AST — only the checked IR is authoritative.
    let output = ir_module_to_json(&ir_module, file);
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Convert a checked IrModule to a deterministic JSON value.
/// This is the sole source of truth for emit-ir output.
fn ir_module_to_json(
    module: &aethel_ir::lower::IrModule,
    file: &PathBuf,
) -> serde_json::Value {
    use aethel_ir::lower::*;

    let source = std::fs::read_to_string(file).unwrap_or_default();

    let mut functions = Vec::new();
    let mut effects = Vec::new();
    let mut policies = Vec::new();
    let mut types = Vec::new();  // struct/enum definitions

    for item in &module.items {
        match item {
            IrItem::Fn(f) => {
                let mut params = Vec::new();
                for p in &f.params {
                    params.push(serde_json::json!({
                        "name": p.name,
                        "type": ir_type_to_string(&p.ty)
                    }));
                }
                let mut effect_names: Vec<String> = Vec::new();
                for ef in &f.effects.effects {
                    let name = ef.path.segments.last()
                        .map(|s| s.name.clone())
                        .unwrap_or_default();
                    effect_names.push(name);
                }
                functions.push(serde_json::json!({
                    "name": f.name,
                    "params": params,
                    "returns": ir_type_to_string(&f.ret_type),
                    "effects": effect_names,
                    "has_body": f.body.is_some()
                }));
            }
            IrItem::Policy(p) => {
                let mut claims = Vec::new();
                for c in &p.claims {
                    let ev: Vec<String> = c.evidence.iter()
                        .map(|ev| format!("{:?}", ev.kind))
                        .collect();
                    claims.push(serde_json::json!({
                        "name": c.name,
                        "type": ir_type_to_string(&c.ty),
                        "evidence": ev
                    }));
                }
                policies.push(serde_json::json!({
                    "name": p.name,
                    "claims": claims
                }));
            }
            IrItem::Struct(s) => {
                let mut fields = Vec::new();
                for f in &s.fields {
                    fields.push(serde_json::json!({
                        "name": f.name,
                        "type": ir_type_to_string(&f.ty)
                    }));
                }
                types.push(serde_json::json!({
                    "name": s.name,
                    "fields": fields
                }));
            }
            IrItem::Effect(e) => {
                let mut ops = Vec::new();
                for op in &e.operations {
                    let mut params = Vec::new();
                    for p in &op.params {
                        params.push(format!("{}: {}", p.name, ir_type_to_string(&p.ty)));
                    }
                    ops.push(serde_json::json!({
                        "name": op.name,
                        "params": params,
                        "returns": op.ret_type.as_ref().map(ir_type_to_string)
                    }));
                }
                effects.push(serde_json::json!({
                    "name": e.name,
                    "operations": ops
                }));
            }
            IrItem::Enum(e) => {
                let mut variants = Vec::new();
                for v in &e.variants {
                    variants.push(serde_json::json!({
                        "name": v.name,
                    }));
                }
                types.push(serde_json::json!({
                    "name": e.name,
                    "variants": variants
                }));
            }
            _ => {} // Use, Mod, TypeAlias have no semantic IR representation
        }
    }

    // Sort deterministically for stable output
    functions.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    effects.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    policies.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    types.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));

    serde_json::json!({
        "ir_version": "0.1",
        "schema": "https://aethel-lang.dev/ir-schema-v0.1",
        "checksum": simple_hash(&source),
        "semantic": {
            "effects": effects,
            "policies": policies,
            "types": types,
            "functions": functions
        },
        "diagnostics": []
    })
}

/// Format an IR type as a human-readable string (same format as the old AST-based one).
fn ir_type_to_string(ty: &aethel_ir::lower::IrType) -> String {
    use aethel_ir::lower::IrType;
    match ty {
        IrType::Unit { .. } => "()".into(),
        IrType::Never { .. } => "!".into(),
        IrType::Bool { .. } => "bool".into(),
        IrType::Int { .. } => "int".into(),
        IrType::Float { .. } => "float".into(),
        IrType::String { .. } => "string".into(),
        IrType::Path { path, .. } => path.segments.iter().map(|s| s.name.clone()).collect::<Vec<_>>().join("::"),
        IrType::Claim { ty, .. } => format!("Claim<{}>", ir_type_to_string(ty)),
        IrType::Verified { ty, policy, .. } => format!("Verified<{}, {}>", ir_type_to_string(ty), ir_type_to_string(policy)),
        IrType::Ref { is_mut, ty, .. } => format!("&{}{}", if *is_mut { "mut " } else { "" }, ir_type_to_string(ty)),
        IrType::Owned { ty, .. } => format!("owned {}", ir_type_to_string(ty)),
        IrType::Tuple { types, .. } => format!("({})", types.iter().map(ir_type_to_string).collect::<Vec<_>>().join(", ")),
        IrType::Array { ty, .. } => format!("[{}]", ir_type_to_string(ty)),
        IrType::Fn { params, ret, .. } => format!("fn({}) -> {}", params.iter().map(ir_type_to_string).collect::<Vec<_>>().join(", "), ir_type_to_string(ret)),
    }
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
