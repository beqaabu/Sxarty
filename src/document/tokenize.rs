//! Turning raw text into the stream of tokens the reader steps through.

use std::ops::Range;

/// One unit of display: usually a single word.
#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    /// Length in *characters*, not bytes. Georgian is 3 bytes per character,
    /// so byte lengths would put the pivot in the wrong place.
    pub chars: usize,
    /// Character index of the optimal recognition point (the pivot letter).
    pub orp: usize,
    /// How long to hold this token, as a multiple of the base word duration.
    pub weight: f32,
    /// Index into [`super::Document::paragraphs`].
    pub paragraph: usize,
}

/// The optimal recognition point: the character the eye should land on.
///
/// These thresholds come from the classic RSVP literature (and are what Spritz
/// popularised). Note that this is deliberately *left* of centre - the eye
/// identifies a word from slightly before its middle - and that it stops
/// growing at 4, so the pivot never drifts far into a long word.
pub fn orp_index(letters: usize) -> usize {
    match letters {
        0 | 1 => 0,
        2..=5 => 1,
        6..=9 => 2,
        10..=13 => 3,
        _ => 4,
    }
}

/// The pivot character index within a whole token.
///
/// The pivot has to land on a letter. Counting punctuation would put it on the
/// comma of `"I,"` and on the bracket of `"(the"`, and would shift it a place
/// late on every word that ends in a full stop - which is a lot of words.
pub fn pivot_of(chars: &[char]) -> usize {
    let first = chars.iter().position(|c| c.is_alphanumeric());
    let last = chars.iter().rposition(|c| c.is_alphanumeric());

    match (first, last) {
        (Some(first), Some(last)) => first + orp_index(last - first + 1),
        // A token of pure punctuation has no letter to sit on.
        _ => 0,
    }
}

/// Closing punctuation that should not mask the sentence-ending mark behind it.
const CLOSERS: &[char] = &['"', '\'', ')', ']', '}', '”', '’', '»', '*', '_'];
/// Dashes that appear free-standing and belong to the previous token.
const DASHES: &[char] = &['\u{2014}', '\u{2013}', '-'];
/// Opening punctuation that belongs to the next token.
const OPENERS: &[char] = &['(', '[', '{', '\u{201C}', '\u{2018}', '\u{AB}'];

/// How long to hold a token, relative to the base word duration.
///
/// Uniform pacing is the main reason RSVP feels exhausting: the reader gets no
/// time to close a clause. Weighting by punctuation and word length buys that
/// time back without lowering the average WPM much.
fn weight_of(word: &str, chars: usize, ends_paragraph: bool) -> f32 {
    let mut weight = 1.0;

    // Long words need proportionally more time, but with a ceiling.
    if chars > 8 {
        weight += (((chars - 8) as f32) * 0.06).min(0.6);
    }

    // Look past trailing quotes/brackets for the real terminator.
    let terminator = word.chars().rev().find(|c| !CLOSERS.contains(c));
    match terminator {
        Some('.') | Some('!') | Some('?') | Some('\u{2026}') => weight += 1.0,
        Some(',') | Some(';') | Some(':') | Some('\u{2014}') | Some('\u{2013}') => weight += 0.5,
        _ => {}
    }

    if ends_paragraph {
        weight += 1.2;
    }

    weight
}

fn push_token(out: &mut Vec<Token>, text: String, paragraph: usize) {
    let glyphs: Vec<char> = text.chars().collect();
    if glyphs.is_empty() {
        return;
    }

    out.push(Token {
        orp: pivot_of(&glyphs),
        weight: 1.0, // patched once paragraph boundaries are known
        chars: glyphs.len(),
        text,
        paragraph,
    });
}

/// Splits `content` into tokens plus the token ranges of each paragraph.
///
/// Blank lines separate paragraphs. Free-standing dashes attach to the previous
/// word and lone opening brackets attach to the next one, so the reader never
/// burns a full beat on a single punctuation mark.
pub fn tokenize(content: &str) -> (Vec<Token>, Vec<Range<usize>>) {
    let mut tokens: Vec<Token> = Vec::new();
    let mut paragraphs: Vec<Range<usize>> = Vec::new();
    let mut paragraph_start = 0usize;
    let mut pending_prefix = String::new();

    let close_paragraph =
        |tokens: &Vec<Token>, paragraphs: &mut Vec<Range<usize>>, start: &mut usize| {
            if tokens.len() > *start {
                paragraphs.push(*start..tokens.len());
                *start = tokens.len();
            }
        };

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            close_paragraph(&tokens, &mut paragraphs, &mut paragraph_start);
            pending_prefix.clear();
            continue;
        }

        let paragraph = paragraphs.len();

        for raw in line.split_whitespace() {
            // A token that is nothing but dashes glues onto the previous word.
            if raw.chars().all(|c| DASHES.contains(&c)) {
                match tokens.last_mut() {
                    Some(last) if last.paragraph == paragraph => {
                        last.text.push_str(raw);
                        let glyphs: Vec<char> = last.text.chars().collect();
                        last.chars = glyphs.len();
                        last.orp = pivot_of(&glyphs);
                    }
                    _ => pending_prefix.push_str(raw),
                }
                continue;
            }

            // A lone opening bracket glues onto the next word.
            if raw.chars().count() == 1 {
                let ch = raw.chars().next().unwrap_or(' ');
                if OPENERS.contains(&ch) {
                    pending_prefix.push(ch);
                    continue;
                }
            }

            let text = if pending_prefix.is_empty() {
                raw.to_string()
            } else {
                let mut text = std::mem::take(&mut pending_prefix);
                text.push_str(raw);
                text
            };

            push_token(&mut tokens, text, paragraph);
        }
    }

    close_paragraph(&tokens, &mut paragraphs, &mut paragraph_start);

    // Now that paragraph boundaries are settled, compute the hold times.
    for range in &paragraphs {
        let last = range.end.saturating_sub(1);
        for i in range.clone() {
            let token = &tokens[i];
            let weight = weight_of(&token.text, token.chars, i == last);
            tokens[i].weight = weight;
        }
    }

    (tokens, paragraphs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pivot_char(word: &str) -> char {
        let (tokens, _) = tokenize(word);
        tokens[0].text.chars().nth(tokens[0].orp).unwrap()
    }

    #[test]
    fn the_pivot_never_lands_on_punctuation() {
        assert_eq!(pivot_char("I,"), 'I');
        assert_eq!(pivot_char("(the"), 'h');
        assert_eq!(pivot_char("\"stop.\""), 't');
        // Trailing punctuation must not push the pivot a place to the right.
        assert_eq!(pivot_char("amet,"), pivot_char("amet"));
        assert_eq!(pivot_char("however."), pivot_char("however"));
    }

    #[test]
    fn a_token_with_no_letters_pivots_on_its_first_character() {
        assert_eq!(pivot_of(&['!', '?']), 0);
    }

    #[test]
    fn orp_is_counted_in_characters_not_bytes() {
        // "\u{10E1}\u{10D0}\u{10DB}\u{10E7}\u{10D0}\u{10E0}\u{10DD}" is 7 characters and 21 bytes.
        let (tokens, _) = tokenize("\u{10E1}\u{10D0}\u{10DB}\u{10E7}\u{10D0}\u{10E0}\u{10DD}");
        assert_eq!(tokens[0].chars, 7);
        assert_eq!(tokens[0].orp, 2);
        assert!(tokens[0].text.len() > tokens[0].chars, "expected multibyte");
    }

    #[test]
    fn free_standing_dashes_attach_to_the_previous_word() {
        let (tokens, _) = tokenize("a star \u{2014} and more");
        assert_eq!(tokens[1].text, "star\u{2014}");
        assert_eq!(tokens.len(), 4);
    }

    #[test]
    fn lone_openers_attach_to_the_next_word() {
        let (tokens, _) = tokenize("see ( this ) now");
        assert_eq!(tokens[1].text, "(this");
    }

    #[test]
    fn blank_lines_split_paragraphs() {
        let (tokens, paragraphs) = tokenize("one two\n\nthree four\n");
        assert_eq!(paragraphs, vec![0..2, 2..4]);
        assert_eq!(tokens[3].paragraph, 1);
    }

    #[test]
    fn sentence_ends_hold_longer_than_plain_words() {
        let (tokens, _) = tokenize("plain end. more here\n\ntail");
        assert!(tokens[1].weight > tokens[0].weight);
    }

    #[test]
    fn closing_quotes_do_not_hide_the_terminator() {
        let (tokens, _) = tokenize("he said \"stop.\" then left");
        assert!(tokens[2].weight > tokens[0].weight);
    }
}
