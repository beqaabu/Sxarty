//! The loaded document: its tokens, its structure, and where it came from.

pub mod extract;
pub mod tokenize;

pub use extract::LoadError;
pub use tokenize::Token;

use std::ops::Range;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    File(PathBuf),
    Clipboard,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub title: String,
    pub source: Source,
    pub tokens: Vec<Token>,
    /// Token index ranges, one per paragraph.
    pub paragraphs: Vec<Range<usize>>,
}

impl Document {
    pub fn from_text(title: impl Into<String>, source: Source, content: &str) -> Self {
        let (tokens, paragraphs) = tokenize::tokenize(content);
        Document {
            title: title.into(),
            source,
            tokens,
            paragraphs,
        }
    }

    /// Loads and tokenizes a file. Runs off the UI thread.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        let content = extract::extract(path)?;
        let title = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        // Recent documents are keyed by path, so a relative one from the
        // command line would fail to match after a change of directory.
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        let document = Document::from_text(title, Source::File(path), &content);
        if document.tokens.is_empty() {
            return Err(LoadError::Empty);
        }

        Ok(document)
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.source {
            Source::File(path) => Some(path),
            Source::Clipboard => None,
        }
    }

    /// The paragraph containing `index`, if any.
    pub fn paragraph_of(&self, index: usize) -> Option<usize> {
        self.tokens.get(index).map(|token| token.paragraph)
    }

    /// The first token of the paragraph `delta` paragraphs away from `index`.
    pub fn paragraph_jump(&self, index: usize, delta: isize) -> usize {
        let Some(current) = self.paragraph_of(index) else {
            return index;
        };

        // Stepping back from mid-paragraph should first go to this paragraph's
        // own start, which is what a reader expects from a "previous" jump.
        let at_start = self
            .paragraphs
            .get(current)
            .is_some_and(|range| range.start == index);
        let target = if delta < 0 && !at_start {
            current as isize + delta + 1
        } else {
            current as isize + delta
        };

        let target = target.clamp(0, self.paragraphs.len().saturating_sub(1) as isize) as usize;
        self.paragraphs
            .get(target)
            .map(|range| range.start)
            .unwrap_or(index)
    }
}
