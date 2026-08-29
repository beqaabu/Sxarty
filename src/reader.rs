//! Playback: where we are in the document and when to move on.

use std::time::{Duration, Instant};

use crate::document::Document;

/// The clock granularity. Fine enough that per-word hold times land within a
/// frame, coarse enough to stay cheap when idle.
pub const TICK: Duration = Duration::from_millis(8);

/// Guard against runaway catch-up if the app was suspended or the window was
/// dragged: never advance more than this many tokens in a single tick, and
/// never credit more than this much wall-clock time to one tick.
const MAX_CATCH_UP: usize = 4;
const MAX_CREDIT: Duration = Duration::from_millis(500);

#[derive(Debug, Default)]
pub struct Reader {
    pub index: usize,
    pub playing: bool,
    /// Time credited toward the current token but not yet spent.
    carry: Duration,
    last_tick: Option<Instant>,
}

/// Pacing settings, read straight off the config each tick.
#[derive(Debug, Clone, Copy)]
pub struct Pacing {
    pub wpm: u32,
    pub chunk: usize,
    pub smart: bool,
}

impl Reader {
    /// Base time per word, before any per-token weighting.
    pub fn base_word_time(wpm: u32) -> Duration {
        Duration::from_secs_f32(60.0 / wpm.max(1) as f32)
    }

    /// How long the chunk starting at `index` should stay on screen.
    pub fn hold_time(document: &Document, index: usize, pacing: Pacing) -> Duration {
        let base = Self::base_word_time(pacing.wpm);
        let end = (index + pacing.chunk.max(1)).min(document.len());

        let weight: f32 = if pacing.smart {
            document.tokens[index..end].iter().map(|t| t.weight).sum()
        } else {
            (end - index) as f32
        };

        base.mul_f32(weight.max(0.1))
    }

    pub fn play(&mut self) {
        self.playing = true;
        self.carry = Duration::ZERO;
        self.last_tick = None;
    }

    pub fn pause(&mut self) {
        self.playing = false;
        self.carry = Duration::ZERO;
        self.last_tick = None;
    }

    pub fn toggle(&mut self) {
        if self.playing {
            self.pause()
        } else {
            self.play()
        }
    }

    /// Moves to `index`, cancelling any partially elapsed hold.
    pub fn seek(&mut self, index: usize) {
        self.index = index;
        self.carry = Duration::ZERO;
        self.last_tick = None;
    }

    /// Advances by `delta` tokens, clamped to the document.
    pub fn step(&mut self, delta: isize, len: usize) {
        let last = len.saturating_sub(1);
        let next = (self.index as isize + delta).clamp(0, last as isize) as usize;
        self.seek(next);
    }

    /// Consumes elapsed wall-clock time and advances as far as it pays for.
    ///
    /// Driven by real timestamps rather than a fixed tick count so that a
    /// dropped frame does not silently slow the reader down.
    pub fn tick(&mut self, now: Instant, document: &Document, pacing: Pacing) {
        if !self.playing || document.is_empty() {
            return;
        }

        let elapsed = match self.last_tick {
            Some(last) => now.saturating_duration_since(last),
            // First tick after a resume: charge one tick, not the whole gap.
            None => TICK,
        };
        self.last_tick = Some(now);
        self.carry += elapsed.min(MAX_CREDIT);

        let chunk = pacing.chunk.max(1);
        for _ in 0..MAX_CATCH_UP {
            let hold = Self::hold_time(document, self.index, pacing);
            if self.carry < hold {
                break;
            }
            self.carry -= hold;

            if self.index + chunk >= document.len() {
                self.index = document.len().saturating_sub(1);
                self.pause();
                return;
            }
            self.index += chunk;
        }
    }

    /// Fraction of the document consumed, in `0.0..=1.0`.
    pub fn progress(&self, len: usize) -> f32 {
        if len <= 1 {
            return 0.0;
        }
        (self.index as f32 / (len - 1) as f32).clamp(0.0, 1.0)
    }

    /// Estimated time left at the current pace.
    pub fn remaining(&self, document: &Document, pacing: Pacing) -> Duration {
        let len = document.len();
        if self.index >= len {
            return Duration::ZERO;
        }

        let base = Self::base_word_time(pacing.wpm);
        let weight: f32 = if pacing.smart {
            document.tokens[self.index..].iter().map(|t| t.weight).sum()
        } else {
            (len - self.index) as f32
        };

        base.mul_f32(weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, Source};

    fn doc(text: &str) -> Document {
        Document::from_text("t", Source::Clipboard, text)
    }

    fn pacing(wpm: u32, chunk: usize, smart: bool) -> Pacing {
        Pacing { wpm, chunk, smart }
    }

    #[test]
    fn steady_pacing_matches_the_requested_wpm() {
        let document = doc("one two three four five six");
        let mut reader = Reader::default();
        reader.play();

        // 600 wpm is exactly 100ms per word.
        let pacing = pacing(600, 1, false);
        let start = Instant::now();
        reader.tick(start, &document, pacing);
        reader.tick(start + Duration::from_millis(350), &document, pacing);

        assert_eq!(reader.index, 3);
    }

    #[test]
    fn smart_pacing_holds_a_sentence_end_longer() {
        let document = doc("plain stop. next");
        let plain = Reader::hold_time(&document, 0, pacing(300, 1, true));
        let stop = Reader::hold_time(&document, 1, pacing(300, 1, true));
        assert!(stop > plain);
    }

    #[test]
    fn playback_stops_and_parks_on_the_last_token() {
        let document = doc("one two");
        let mut reader = Reader::default();
        reader.play();

        let pacing = pacing(600, 1, false);
        let start = Instant::now();
        reader.tick(start, &document, pacing);
        reader.tick(start + Duration::from_secs(5), &document, pacing);

        assert_eq!(reader.index, 1);
        assert!(!reader.playing);
    }

    #[test]
    fn chunking_advances_by_whole_chunks() {
        let document = doc("a b c d e f g h");
        let mut reader = Reader::default();
        reader.play();

        let pacing = pacing(600, 2, false);
        let start = Instant::now();
        reader.tick(start, &document, pacing);
        // A 2-word chunk at 600 wpm costs 200ms.
        reader.tick(start + Duration::from_millis(250), &document, pacing);

        assert_eq!(reader.index, 2);
    }

    #[test]
    fn a_long_stall_does_not_skip_the_whole_document() {
        let document = doc("a b c d e f g h i j k l m n o p q r s t");
        let mut reader = Reader::default();
        reader.play();

        let pacing = pacing(600, 1, false);
        let start = Instant::now();
        reader.tick(start, &document, pacing);
        reader.tick(start + Duration::from_secs(30), &document, pacing);

        assert!(reader.index <= 4, "advanced to {}", reader.index);
    }
}
