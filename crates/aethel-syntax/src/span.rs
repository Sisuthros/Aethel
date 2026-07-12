//! Source code span and position tracking.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

/// A unique identifier for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FileId(pub u32);

impl FileId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A byte offset from the start of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ByteOffset(pub u32);

impl ByteOffset {
    pub fn new(offset: u32) -> Self {
        Self(offset)
    }

    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl From<u32> for ByteOffset {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<usize> for ByteOffset {
    fn from(v: usize) -> Self {
        Self(v as u32)
    }
}

impl std::ops::Add<u32> for ByteOffset {
    type Output = ByteOffset;

    fn add(self, rhs: u32) -> Self::Output {
        ByteOffset(self.0 + rhs)
    }
}

impl std::ops::Sub<u32> for ByteOffset {
    type Output = ByteOffset;

    fn sub(self, rhs: u32) -> Self::Output {
        ByteOffset(self.0 - rhs)
    }
}

impl std::ops::Sub<ByteOffset> for ByteOffset {
    type Output = u32;

    fn sub(self, rhs: ByteOffset) -> Self::Output {
        self.0 - rhs.0
    }
}

/// A span in a source file: [start, end).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub file: FileId,
    pub start: ByteOffset,
    pub end: ByteOffset,
}

impl Span {
    pub fn new(file: FileId, start: ByteOffset, end: ByteOffset) -> Self {
        debug_assert!(start <= end);
        Self { file, start, end }
    }

    pub fn single(file: FileId, offset: ByteOffset) -> Self {
        Self::new(file, offset, offset + 1)
    }

    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, offset: ByteOffset) -> bool {
        self.start <= offset && offset < self.end
    }

    pub fn merge(&self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file);
        Span::new(
            self.file,
            self.start.min(other.start),
            self.end.max(other.end),
        )
    }

    pub fn zero() -> Self {
        Self {
            file: FileId::new(0),
            start: ByteOffset::new(0),
            end: ByteOffset::new(0),
        }
    }

    pub fn range(&self) -> Range<ByteOffset> {
        self.start..self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}..{}", self.file.0, self.start.0, self.end.0)
    }
}

/// Trait for things that have a span.
pub trait Spanned {
    fn span(&self) -> Span;
}

impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

impl<T: Spanned> Spanned for &T {
    fn span(&self) -> Span {
        (*self).span()
    }
}

impl<T: Spanned> Spanned for Box<T> {
    fn span(&self) -> Span {
        (**self).span()
    }
}

/// A source file with its content.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: FileId,
    pub name: String,
    pub content: String,
    /// Line start byte offsets (including 0 at the start).
    line_starts: Vec<ByteOffset>,
}

impl SourceFile {
    pub fn new(id: FileId, name: String, content: String) -> Self {
        let mut line_starts = vec![ByteOffset(0)];
        for (i, c) in content.bytes().enumerate() {
            if c == b'\n' {
                line_starts.push(ByteOffset((i + 1) as u32));
            }
        }
        Self {
            id,
            name,
            content,
            line_starts,
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_start(&self, line: usize) -> Option<ByteOffset> {
        self.line_starts.get(line).copied()
    }

    /// Find the line and column for a byte offset.
    pub fn location(&self, offset: ByteOffset) -> Option<(usize, usize)> {
        let offset = offset.as_usize();
        let line = self
            .line_starts
            .binary_search(&ByteOffset(offset as u32))
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line_start = self.line_starts[line].as_usize();
        let column = offset - line_start;
        Some((line + 1, column + 1))
    }

    /// Get the source text for a span.
    pub fn span_text(&self, span: Span) -> &str {
        &self.content[span.start.as_usize()..span.end.as_usize()]
    }

    /// Get the line containing the span start.
    pub fn line_text(&self, span: Span) -> &str {
        if let Some((line, _)) = self.location(span.start) {
            let start = self.line_start(line - 1).unwrap_or(ByteOffset(0));
            let end = self.line_start(line).unwrap_or(ByteOffset(self.content.len() as u32));
            &self.content[start.as_usize()..end.as_usize()]
        } else {
            ""
        }
    }
}

/// A file database for looking up source files by ID.
#[derive(Debug, Default)]
pub struct FileDb {
    files: Vec<SourceFile>,
}

impl FileDb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: String, content: String) -> FileId {
        let id = FileId::new(self.files.len() as u32);
        let file = SourceFile::new(id, name, content);
        self.files.push(file);
        id
    }

    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_operations() {
        let file = FileId::new(0);
        let span = Span::new(file, ByteOffset(0), ByteOffset(10));
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
        assert!(span.contains(ByteOffset(5)));
        assert!(!span.contains(ByteOffset(10)));
    }

    #[test]
    fn test_source_file_location() {
        let content = "line 1\nline 2\nline 3";
        let file = SourceFile::new(FileId::new(0), "test.aet".into(), content.into());
        let loc = file.location(ByteOffset(7)).unwrap(); // start of "line 2"
        assert_eq!(loc, (2, 1));
        let loc = file.location(ByteOffset(10)).unwrap(); // 'e' in "line 2"
        assert_eq!(loc, (2, 4));
    }
}