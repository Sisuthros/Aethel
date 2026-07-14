//! Diagnostics and error reporting.

use crate::span::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::{self, Config};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Local label style for serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

impl From<LabelStyle> for codespan_reporting::diagnostic::LabelStyle {
    fn from(s: LabelStyle) -> Self {
        match s {
            LabelStyle::Primary => codespan_reporting::diagnostic::LabelStyle::Primary,
            LabelStyle::Secondary => codespan_reporting::diagnostic::LabelStyle::Secondary,
        }
    }
}

/// A unique diagnostic code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticCode(pub String);

impl DiagnosticCode {
    pub fn new(code: &str) -> Self {
        Self(code.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

    macro_rules! code {
        ($name:ident, $value:expr) => {
            pub fn $name() -> DiagnosticCode {
                DiagnosticCode::new($value)
            }
        };
    }

    // Parse errors
    code!(PARSE_ERROR, "AE-PARSE-001");
    code!(UNEXPECTED_TOKEN, "AE-PARSE-002");
    code!(UNTERMINATED_STRING, "AE-PARSE-003");
    code!(INVALID_NUMBER, "AE-PARSE-004");
    code!(INVALID_ESCAPE, "AE-PARSE-005");

    // Type errors
    code!(TYPE_MISMATCH, "AE-TYPE-001");
    code!(TYPE_NOT_FOUND, "AE-TYPE-002");
    code!(UNEXPECTED_TYPE_ARGS, "AE-TYPE-003");
    code!(MISSING_TYPE_ARGS, "AE-TYPE-004");
    code!(RECURSIVE_TYPE, "AE-TYPE-005");
    code!(INVALID_SELF_TYPE, "AE-TYPE-006");
    code!(TYPE_ANNOTATION_REQUIRED, "AE-TYPE-007");
    code!(UNIFIED_TYPE_MISMATCH, "AE-TYPE-008");
    code!(INFINITE_TYPE, "AE-TYPE-009");
    code!(UNSUPPORTED_TYPE_OP, "AE-TYPE-010");
    code!(LINEAR_TYPE_MISUSE, "AE-TYPE-011");
    code!(LINEAR_USE_AFTER_MOVE, "AE-TYPE-012");
    code!(LINEAR_NOT_CONSUMED, "AE-TYPE-013");
    code!(CAPABILITY_REQUIRED, "AE-TYPE-014");
    code!(CAPABILITY_DUPLICATED, "AE-TYPE-015");
    code!(CAPABILITY_ESCAPED, "AE-TYPE-016");
    code!(EFFECT_NOT_HANDLED, "AE-TYPE-017");
    code!(EFFECT_NOT_DECLARED, "AE-TYPE-018");
    code!(TYPE_COMMIT_ONCE_REQUIRED, "AE-TYPE-019");

    // Epistemic type errors (the key guarantee)
    code!(EPISTEMIC_CLAIM_NOT_VERIFIED, "AE-EPISTEMIC-001");
    code!(EPISTEMIC_VERIFIED_REQUIRED, "AE-EPISTEMIC-002");
    code!(EPISTEMIC_POLICY_MISMATCH, "AE-EPISTEMIC-003");
    code!(EPISTEMIC_CLAIM_ESCAPE, "AE-EPISTEMIC-004");
    code!(EPISTEMIC_VERIFY_FAILED, "AE-EPISTEMIC-005");
    code!(EPISTEMIC_BUDGET_EXCEEDED, "AE-EPISTEMIC-006");
    code!(EPISTEMIC_UNVERIFIED_EFFECT, "AE-EPISTEMIC-007");

    // Name resolution errors
    code!(UNDEFINED_VAR, "AE-NAME-001");
    code!(UNDEFINED_TYPE, "AE-NAME-002");
    code!(UNDEFINED_EFFECT, "AE-NAME-003");
    code!(UNDEFINED_MODULE, "AE-NAME-004");
    code!(AMBIGUOUS_NAME, "AE-NAME-005");
    code!(SHADOWED_NAME, "AE-NAME-006");
    code!(UNUSED_IMPORT, "AE-NAME-007");

    // Other errors
    code!(RECURSION_LIMIT, "AE-OTHER-001");
    code!(INTERNAL_ERROR, "AE-OTHER-002");
    code!(NOT_IMPLEMENTED, "AE-OTHER-003");
    code!(DEPRECATED, "AE-OTHER-004");
    code!(UNREACHABLE_CODE, "AE-OTHER-005");
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
pub fn render_diagnostics<'a, F>(
    writer: &mut StandardStream,
    config: &Config,
    files: &'a F,
    diagnostics: &Diagnostics,
) -> Result<(), codespan_reporting::files::Error>
where
    F: codespan_reporting::files::Files<'a, FileId = FileId, Name = String, Source = str>,
{
    for diag in &diagnostics.items {
        let diagnostic = Diagnostic::new(diag.severity.into())
            .with_code(diag.code.to_string())
            .with_message(diag.message.clone())
            .with_labels(
                diag.labels
                    .iter()
                    .map(|l| {
                        let style: codespan_reporting::diagnostic::LabelStyle = l.style.into();
                        Label::new(
                            style,
                            l.span.file,
                            l.span.start.0 as usize..l.span.end.0 as usize,
                        )
                        .with_message(l.message.clone())
                    })
                    .collect::<Vec<_>>(),
            )
            .with_notes({
                let mut notes = diag.notes.clone();
                if let Some(ref help) = diag.help {
                    notes.push(help.clone());
                }
                notes
            });

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

    pub fn note_severity(code: DiagnosticCode, message: impl Into<String>) -> Self {
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
                let diag = DiagnosticBuilder::error(codes::TYPE_MISMATCH(), "type mismatch")
                    .primary_label(span, "expected `int`, found `string`")
                    .note("help: try converting the string to an int")
                    .build();

                assert_eq!(diag.code, codes::TYPE_MISMATCH());
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.notes.len(), 1);
    }
}