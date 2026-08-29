//! Pulling plain text out of the formats we support.
//!
//! Everything here is pure Rust. The previous version shelled out to
//! `pdftotext` and `pandoc`, which are almost never present on a stock macOS
//! machine, so opening a PDF just printed an error to a terminal nobody was
//! looking at.

use std::fmt;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum LoadError {
    Io(String),
    Unsupported(String),
    /// Only the optional format parsers can produce this.
    #[cfg_attr(
        not(any(feature = "pdf", feature = "docx", feature = "epub")),
        allow(dead_code)
    )]
    Parse(String),
    Empty,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "Could not read the file: {e}"),
            LoadError::Unsupported(ext) => {
                write!(f, "Sxarty does not know how to read .{ext} files yet")
            }
            LoadError::Parse(e) => write!(f, "Could not parse the document: {e}"),
            LoadError::Empty => write!(f, "That document has no readable text in it"),
        }
    }
}

/// Extensions the file picker offers and [`extract`] accepts.
pub fn supported_extensions() -> &'static [&'static str] {
    &[
        "txt",
        "md",
        "markdown",
        "text",
        "log",
        #[cfg(feature = "pdf")]
        "pdf",
        #[cfg(feature = "docx")]
        "docx",
        #[cfg(feature = "epub")]
        "epub",
    ]
}

/// Reads `path` and returns its plain text.
pub fn extract(path: &Path) -> Result<String, LoadError> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    let text = match extension.as_str() {
        "txt" | "md" | "markdown" | "text" | "log" | "" => plain(path)?,
        #[cfg(feature = "pdf")]
        "pdf" => pdf(path)?,
        #[cfg(feature = "docx")]
        "docx" => docx(path)?,
        #[cfg(feature = "epub")]
        "epub" => epub(path)?,
        other => return Err(LoadError::Unsupported(other.to_string())),
    };

    if text.split_whitespace().next().is_none() {
        return Err(LoadError::Empty);
    }

    Ok(text)
}

/// Reads a text file, tolerating a BOM and invalid UTF-8.
fn plain(path: &Path) -> Result<String, LoadError> {
    let bytes = std::fs::read(path).map_err(|e| LoadError::Io(e.to_string()))?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);

    Ok(match String::from_utf8(bytes.to_vec()) {
        Ok(text) => text,
        // Better to show slightly mangled text than to refuse the file.
        Err(_) => String::from_utf8_lossy(bytes).into_owned(),
    })
}

#[cfg(feature = "pdf")]
fn pdf(path: &Path) -> Result<String, LoadError> {
    pdf_extract::extract_text(path).map_err(|e| LoadError::Parse(e.to_string()))
}

#[cfg(feature = "docx")]
fn docx(path: &Path) -> Result<String, LoadError> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| LoadError::Io(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| LoadError::Parse(e.to_string()))?;

    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|_| LoadError::Parse("not a Word document (no word/document.xml)".into()))?
        .read_to_string(&mut xml)
        .map_err(|e| LoadError::Parse(e.to_string()))?;

    Ok(text_from_xml(&xml, "w:t", &["w:p"], &["w:br", "w:tab"]))
}

#[cfg(feature = "epub")]
fn epub(path: &Path) -> Result<String, LoadError> {
    let mut doc = epub::doc::EpubDoc::new(path).map_err(|e| LoadError::Parse(e.to_string()))?;

    let mut out = String::new();
    for chapter in 0..doc.get_num_chapters() {
        if !doc.set_current_chapter(chapter) {
            continue;
        }
        if let Some((html, _mime)) = doc.get_current_str() {
            out.push_str(&strip_html(&html));
            out.push_str("\n\n");
        }
    }

    Ok(out)
}

/// Collects the character data of `text_tag`, inserting a paragraph break after
/// each `break_tags` element and a space after each `space_tags` element.
#[cfg(feature = "docx")]
fn text_from_xml(xml: &str, text_tag: &str, break_tags: &[&str], space_tags: &[&str]) -> String {
    use quick_xml::Reader;
    use quick_xml::escape;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == text_tag {
                    in_text = true;
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let name = name.as_ref();
                if name == text_tag {
                    in_text = false;
                } else if break_tags.contains(&name) {
                    out.push_str("\n\n");
                }
            }
            Ok(Event::Empty(e)) => {
                if space_tags.contains(&e.name().as_ref()) {
                    out.push(' ');
                }
            }
            Ok(Event::Text(e)) if in_text => out.push_str(e.as_ref()),
            Ok(Event::CData(e)) if in_text => out.push_str(e.as_ref()),
            // quick-xml reports `&amp;` and `&#8212;` as their own event rather
            // than inlining them into the surrounding text, so an unhandled
            // arm here silently eats every escaped character in the document.
            Ok(Event::GeneralRef(e)) if in_text => {
                let entity = e.as_ref();
                if let Some(resolved) = escape::resolve_predefined_entity(entity) {
                    out.push_str(resolved);
                } else if let Some(c) = numeric_entity(entity) {
                    out.push(c);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    out
}

/// Resolves `#8212` and `#x2014` style character references.
#[cfg(any(feature = "docx", feature = "epub"))]
fn numeric_entity(entity: &str) -> Option<char> {
    let digits = entity.strip_prefix('#')?;

    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };

    char::from_u32(code)
}

/// A deliberately small HTML-to-text pass: enough for EPUB chapter bodies.
#[cfg(feature = "epub")]
fn strip_html(html: &str) -> String {
    const BLOCKS: &[&str] = &[
        "p",
        "div",
        "br",
        "li",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "tr",
        "blockquote",
    ];

    let mut out = String::with_capacity(html.len() / 2);
    let mut rest = html;

    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        rest = &rest[open + 1..];

        let closing = rest.starts_with('/');
        let name: String = rest
            .trim_start_matches('/')
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase();

        rest = after_tag(rest);

        // `<script>` and `<style>` hold raw text, not markup, and their bodies
        // can contain a bare `<` (`if (i < n)`), so they have to be skipped by
        // searching for the literal closing tag rather than by parsing onward.
        // `<title>` is skipped for a different reason: it is chapter metadata,
        // and reading it aloud in the middle of the prose is just noise.
        if !closing && matches!(name.as_str(), "script" | "style" | "title") {
            let close = format!("</{name}");
            match find_ascii_case_insensitive(rest, &close) {
                Some(at) => rest = after_tag(&rest[at + 1..]),
                None => return decode_entities(&out),
            }
            continue;
        }

        if BLOCKS.contains(&name.as_str()) {
            out.push_str("\n\n");
        }
    }

    out.push_str(rest);
    decode_entities(&out)
}

/// The remainder of `rest` after the first `>`.
#[cfg(feature = "epub")]
fn after_tag(rest: &str) -> &str {
    match rest.find('>') {
        Some(end) => &rest[end + 1..],
        None => "",
    }
}

#[cfg(feature = "epub")]
fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }

    // `needle` is ASCII and starts with `<`, so any hit is a char boundary.
    (0..=haystack.len() - needle.len())
        .find(|&i| haystack[i..i + needle.len()].eq_ignore_ascii_case(needle))
}

#[cfg(feature = "epub")]
fn decode_entities(input: &str) -> String {
    const NAMED: &[(&str, &str)] = &[
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&nbsp;", " "),
        ("&mdash;", "\u{2014}"),
        ("&ndash;", "\u{2013}"),
        ("&hellip;", "\u{2026}"),
        ("&ldquo;", "\u{201C}"),
        ("&rdquo;", "\u{201D}"),
        ("&lsquo;", "\u{2018}"),
        ("&rsquo;", "\u{2019}"),
    ];

    let mut out = input.to_string();
    for (entity, replacement) in NAMED {
        if out.contains(entity) {
            out = out.replace(entity, replacement);
        }
    }

    if !out.contains("&#") {
        return out;
    }

    // Numeric references, which the named table above cannot cover.
    let mut resolved = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(start) = rest.find("&#") {
        resolved.push_str(&rest[..start]);
        let tail = &rest[start + 1..];

        match tail
            .find(';')
            .and_then(|end| numeric_entity(&tail[..end]).map(|c| (c, end)))
        {
            Some((c, end)) => {
                resolved.push(c);
                rest = &tail[end + 1..];
            }
            None => {
                resolved.push('&');
                rest = tail;
            }
        }
    }
    resolved.push_str(rest);

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "docx")]
    #[test]
    fn docx_text_survives_entities_paragraphs_and_breaks() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="x"><w:body>
<w:p><w:r><w:t>Hello </w:t><w:t>world &amp; friends.</w:t></w:r></w:p>
<w:p><w:r><w:t>Third</w:t><w:br/><w:t>line</w:t></w:r></w:p>
</w:body></w:document>"#;

        let text = text_from_xml(xml, "w:t", &["w:p"], &["w:br", "w:tab"]);

        assert!(text.contains("Hello world & friends."), "got {text:?}");
        assert!(
            text.contains("Third line"),
            "a <w:br/> should become a space"
        );
        assert!(text.contains("\n\n"), "each <w:p> should end a paragraph");
    }

    #[cfg(feature = "epub")]
    #[test]
    fn html_stripping_drops_scripts_and_keeps_paragraphs() {
        let html = "<html><head><style>p{color:red}</style></head><body>\
                    <p>First &amp; foremost</p><script>var x = 1 < 2;</script>\
                    <p>Second</p></body></html>";

        let text = strip_html(html);

        assert!(text.contains("First & foremost"));
        assert!(text.contains("Second"));
        assert!(!text.contains("color:red"), "style body leaked: {text:?}");
        assert!(!text.contains("var x"), "script body leaked: {text:?}");
    }

    #[cfg(feature = "epub")]
    #[test]
    fn numeric_character_references_are_resolved() {
        assert_eq!(numeric_entity("#8212"), Some('\u{2014}'));
        assert_eq!(numeric_entity("#x2014"), Some('\u{2014}'));
        assert_eq!(numeric_entity("#nonsense"), None);
        assert_eq!(
            decode_entities("a &#8212; b &#x2019;s"),
            "a \u{2014} b \u{2019}s"
        );
        assert_eq!(decode_entities("Q&A &#bad; end"), "Q&A &#bad; end");
    }

    #[test]
    fn an_unknown_extension_is_reported_rather_than_guessed() {
        let error = extract(std::path::Path::new("/nope.xyz")).unwrap_err();
        assert!(matches!(error, LoadError::Unsupported(ref e) if e == "xyz"));
    }
}
