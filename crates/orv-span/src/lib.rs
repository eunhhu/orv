/// A unique identifier for a source file within the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);

impl FileId {
    /// Creates a new `FileId` from a raw `u32` index.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the underlying raw `u32` value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// A byte range within a specific source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    file: FileId,
    start: u32,
    end: u32,
}

impl Span {
    /// Creates a new `Span` covering `[start, end)` in the given file.
    #[must_use]
    pub const fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    /// Returns the `FileId` this span belongs to.
    #[must_use]
    pub const fn file(self) -> FileId {
        self.file
    }

    /// Returns the start byte offset (inclusive).
    #[must_use]
    pub const fn start(self) -> u32 {
        self.start
    }

    /// Returns the end byte offset (exclusive).
    #[must_use]
    pub const fn end(self) -> u32 {
        self.end
    }

    /// Returns the length of the span in bytes.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Returns `true` if the span covers zero bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Merges two spans from the same file into a span covering both.
    ///
    /// # Panics
    ///
    /// Panics if the two spans belong to different files.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        assert_eq!(
            self.file, other.file,
            "cannot merge spans from different files"
        );
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        Self::new(self.file, start, end)
    }
}

/// A value annotated with a source [`Span`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    node: T,
    span: Span,
}

impl<T> Spanned<T> {
    /// Wraps a value with the given span.
    #[must_use]
    pub const fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    /// Returns a reference to the inner value.
    #[must_use]
    pub const fn node(&self) -> &T {
        &self.node
    }

    /// Returns the span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn file_id_roundtrip() {
        let id = FileId::new(7);
        assert_eq!(id.raw(), 7);
    }

    #[test]
    fn span_len_and_empty() {
        let file = FileId::new(0);
        let span = Span::new(file, 2, 10);
        assert_eq!(span.len(), 8);
        assert!(!span.is_empty());

        let empty = Span::new(file, 5, 5);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn span_merge() {
        let file = FileId::new(0);
        let a = Span::new(file, 2, 5);
        let b = Span::new(file, 8, 12);
        let merged = a.merge(b);
        assert_eq!(merged.start(), 2);
        assert_eq!(merged.end(), 12);
    }

    #[test]
    #[should_panic(expected = "cannot merge spans from different files")]
    fn span_merge_different_files_panics() {
        let a = Span::new(FileId::new(0), 0, 5);
        let b = Span::new(FileId::new(1), 0, 5);
        let _ = a.merge(b);
    }

    #[test]
    fn spanned_wraps_value() {
        let file = FileId::new(0);
        let span = Span::new(file, 0, 5);
        let spanned = Spanned::new("hello", span);
        assert_eq!(*spanned.node(), "hello");
        assert_eq!(spanned.span(), span);
    }
}
