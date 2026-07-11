# ADR 0001: Parser Architecture

## Status
Accepted

## Context
We need to choose a parsing strategy for Aethel source code. The parser must:
- Produce high-quality error messages with precise spans
- Be fast enough for interactive use
- Handle Aethel's indentation-sensitive syntax
- Support the epistemic type syntax (`Claim<T>`, `Verified<T, Policy>`)
- Be maintainable by a small team

## Decision
We use a **handwritten recursive-descent parser with a Pratt parser for expressions**, and **logos for lexing**.

### Lexing: logos
- Fast, compile-time generated lexer
- Good error recovery
- Produces `Token` with `Span` metadata
- No runtime regex compilation

### Parsing: Handwritten Recursive Descent + Pratt
- **Declarations/Statements**: Recursive descent with explicit functions per syntactic category
- **Expressions**: Pratt parser (top-down operator precedence) for clean handling of precedence and associativity
- **Error Recovery**: Panic mode synchronization at statement boundaries (`;`, `}`, `)`)

### Not Using
- **Chumsky**: API instability between 0.x and 1.x; combinator overhead
- **LR/LALR generators (lalrpop)**: Poor error messages; hard to customize recovery
- **Tree-sitter**: Designed for incremental editing, not batch compilation

## Consequences
### Positive
- Full control over error messages and recovery
- Zero dependencies beyond logos
- Easy to add Aethel-specific syntax (epistemic types, effect clauses)
- Fast compilation and parsing
- Team can debug parser with standard debugger

### Negative
- More code to maintain (~2000 lines vs ~500 for combinator approach)
- Manual precedence management in Pratt parser
- Risk of precedence bugs (mitigated by exhaustive tests)

## Implementation Notes
- `Lexer` in `aethel-syntax/src/lexer.rs` using `logos::Logos`
- `Parser` in `aethel-syntax/src/parser.rs` with `Precedence` enum
- Tokens carry `Span` (file_id, start, end) for diagnostics
- Parse errors use `DiagnosticBuilder` with primary/secondary labels

## Testing
- Snapshot tests for token streams (`insta`)
- Snapshot tests for AST round-trip (parse → format → parse)
- Diagnostic snapshot tests for known error patterns
- Property tests for precedence (all operator combinations)