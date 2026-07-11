//! Diagnostics and error reporting.

use crate::span::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle, Severity};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::{self, Config};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A unique diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticCode(pub &'static str);

impl DiagnosticCode {
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }

    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Diagnostic codes for Aethel.
pub mod codes {
    use super::DiagnosticCode;

    // Parse errors
    pub const PARSE_ERROR: DiagnosticCode = DiagnosticCode::new("AE-PARSE-001");
    pub const UNEXPECTED_TOKEN: DiagnosticCode = DiagnosticCode::new("AE-PARSE-002");
    pub const UNTERMINATED_STRING: DiagnosticCode = DiagnosticCode::new("AE-PARSE-003");
    pub const INVALID_NUMBER: DiagnosticCode = DiagnosticCode::new("AE-PARSE-004");
    pub const INVALID_ESCAPE: DiagnosticCode = DiagnosticCode::new("AE-PARSE-005");

    // Type errors
    pub const TYPE_MISMATCH: DiagnosticCode = DiagnosticCode::new("AE-TYPE-001");
    pub const TYPE_NOT_FOUND: DiagnosticCode = DiagnosticCode::new("AE-TYPE-002");
    pub const UNEXPECTED_TYPE_ARGS: DiagnosticCode = DiagnosticCode::new("AE-TYPE-003");
    pub const MISSING_TYPE_ARGS: DiagnosticCode = DiagnosticCode::new("AE-TYPE-004");
    pub const RECURSIVE_TYPE: DiagnosticCode = DiagnosticCode::new("AE-TYPE-005");
    pub const INVALID_SELF_TYPE: DiagnosticCode = DiagnosticCode::new("AE-TYPE-006");
    pub const TYPE_ANNOTATION_REQUIRED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-007");
    pub const UNIFIED_TYPE_MISMATCH: DiagnosticCode = DiagnosticCode::new("AE-TYPE-008");
    pub const INFINITE_TYPE: DiagnosticCode = DiagnosticCode::new("AE-TYPE-009");
    pub const UNSUPPORTED_TYPE_OP: DiagnosticCode = DiagnosticCode::new("AE-TYPE-010");
    pub const LINEAR_TYPE_MISUSE: DiagnosticCode = DiagnosticCode::new("AE-TYPE-011");
    pub const LINEAR_USE_AFTER_MOVE: DiagnosticCode = DiagnosticCode::new("AE-TYPE-012");
    pub const LINEAR_NOT_CONSUMED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-013");
    pub const CAPABILITY_REQUIRED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-014");
    pub const CAPABILITY_DUPLICATED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-015");
    pub const CAPABILITY_ESCAPED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-016");
    pub const EFFECT_NOT_HANDLED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-017");
    pub const EFFECT_NOT_DECLARED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-018");
    pub const TYPE_COMMIT_ONCE_REQUIRED: DiagnosticCode = DiagnosticCode::new("AE-TYPE-019");

    // Epistemic type errors (the key guarantee)
    pub const EPISTEMIC_CLAIM_NOT_VERIFIED: DiagnosticCode =
        DiagnosticCode::new("AE-EPISTEMIC-001");
    pub const EPISTEMIC_VERIFIED_REQUIRED: DiagnosticCode = DiagnosticCode::new("AE-EPISTEMIC-002");
    pub const EPISTEMIC_POLICY_MISMATCH: DiagnosticCode = DiagnosticCode::new("AE-EPISTEMIC-003");
    pub const EPISTEMIC_CLAIM_ESCAPE: DiagnosticCode = DiagnosticCode::new("AE-EPISTEMIC-004");
    pub const EPISTEMIC_VERIFY_FAILED: DiagnosticCode = DiagnosticCode::new("AE-EPISTEMIC-005");
    pub const EPISTEMIC_BUDGET_EXCEEDED: DiagnosticCode = DiagnosticCode::new("AE-EPISTEMIC-006");

    // Name resolution errors
    pub const UNDEFINED_VAR: DiagnosticCode = DiagnosticCode::new("AE-NAME-001");
    pub const UNDEFINED_TYPE: DiagnosticCode = DiagnosticCode::new("AE-NAME-002");
    pub const UNDEFINED_EFFECT: DiagnosticCode = DiagnosticCode::new("AE-NAME-003");
    pub const UNDEFINED_MODULE: DiagnosticCode = DiagnosticCode::new("AE-NAME-004");
    pub const AMBIGUOUS_NAME: DiagnosticCode = DiagnosticCode::new("AE-NAME-005");
    pub const SHADOWED_NAME: DiagnosticCode = DiagnosticCode::new("AE-NAME-006");
    pub const UNUSED_IMPORT: DiagnosticCode = DiagnosticCode::new("AE-NAME-007");

    // Other errors
    pub const RECURSION_LIMIT: DiagnosticCode = DiagnosticCode::new("AE-OTHER-001");
    pub const INTERNAL_ERROR: DiagnosticCode = DiagnosticCode::new("AE-OTHER-002");
    pub const NOT_IMPLEMENTED: DiagnosticCode = DiagnosticCode::new("AE-OTHER-003");
    pub const DEPRECATED: DiagnosticCode = DiagnosticCode::new("AE-OTHER-004");
    pub const UNREACHABLE_CODE: DiagnosticCode = DiagnosticCode::new("AE-OTHER-005");
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
    Help,
}

impl From<DiagnosticSeverity> for Severity {
    fn from(s: DiagnosticSeverity) -> Self {
        match s {
            DiagnosticSeverity::Error => Severity::Error,
            DiagnosticSeverity::Warning => Severity::Warning,
            DiagnosticSeverity::Note => Severity::Note,
            DiagnosticSeverity::Help => Severity::Help,
        }
    }
}

/// A diagnostic message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticMessage {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub labels: Vec<LabelInfo>,
    pub notes: Vec<String>,
    pub help: Option<String>,
}

/// A labeled source range for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelInfo {
    pub style: LabelStyle,
    pub span: Span,
    pub message: String,
}

impl LabelInfo {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            style: LabelStyle::Primary,
            span,
            message: message.into(),
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            style: LabelStyle::Secondary,
            span,
            message: message.into(),
        }
    }
}

/// A collection of diagnostics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Diagnostics {
    pub items: Vec<DiagnosticMessage>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diag: DiagnosticMessage) {
        self.items.push(diag);
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.items.extend(other.items);
    }

    pub fn has_errors(&self) -> bool {
        self.items
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }

    pub fn errors(&self) -> Vec<&DiagnosticMessage> {
        self.items
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&DiagnosticMessage> {
        self.items
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Convert internal diagnostics to codespan-reporting format for rendering.
pub fn render_diagnostics(
    writer: &mut StandardStream,
    config: &Config,
    files: &dyn codespan_reporting::files::Files<'_, FileId = FileId, Name = String, Source = str>,
    diagnostics: &Diagnostics,
) -> Result<(), codespan_reporting::files::Error> {
    for diag in &diagnostics.items {
        let diagnostic = Diagnostic::new(diag.severity.into())
            .with_code(diag.code.to_string())
            .with_message(diag.message.clone())
            .with_labels(
                diag.labels
                    .iter()
                    .map(|l| {
                        Label::new(
                            l.style,
                            l.span.file,
                            l.span.start.0 as usize..l.span.end.0 as usize,
                        )
                        .with_message(l.message.clone())
                    })
                    .collect::<Vec<_>>(),
            )
            .with_notes(diag.notes.clone())
            .with_help(diag.help.clone().unwrap_or_default());

        term::emit(writer, config, files, &diagnostic)?;
    }
    Ok(())
}

/// A diagnostic builder for convenient construction.
pub struct DiagnosticBuilder {
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    message: String,
    labels: Vec<LabelInfo>,
    notes: Vec<String>,
    help: Option<String>,
}

impl DiagnosticBuilder {
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn note(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Note,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn label(mut self, style: LabelStyle, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(LabelInfo {
            style,
            span,
            message: message.into(),
        });
        self
    }

    pub fn primary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(LabelInfo::primary(span, message));
        self
    }

    pub fn secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(LabelInfo::secondary(span, message));
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn build(self) -> DiagnosticMessage {
        DiagnosticMessage {
            code: self.code,
            severity: self.severity,
            message: self.message,
            labels: self.labels,
            notes: self.notes,
            help: self.help,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::{ByteOffset, FileId, Span};

    #[test]
    fn test_diagnostic_builder() {
        let span = Span::new(FileId::new(0), ByteOffset(0), ByteOffset(10));
        let diag = DiagnosticBuilder::error(codes::TYPE_MISMATCH, "type mismatch")
            .primary_label(span, "expected `int`, found `string`")
            .note("help: try converting the string to an int")
            .build();

        assert_eq!(diag.code, codes::TYPE_MISMATCH);
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.notes.len(), 1);
    }
}