# Sxarty

A focused RSVP speed reader for the desktop. One word at a time, held in the
place your eye already wants to look.

Built in Rust with [iced](https://iced.rs). Single binary, no webview, no
runtime dependencies.

```
                    │
      the quick br o wn fox jumped
                    │
                    └── the pivot never moves
```

## What RSVP actually does

Normal reading spends most of its time on *saccades* - the small jumps your eyes
make between words - not on recognising the words themselves. Rapid Serial
Visual Presentation removes the jumps by bringing the words to a fixed point
instead.

The catch is that a fixed point only works if the *right part* of each word
lands there. Every word has an optimal recognition point (ORP) slightly left of
centre - not the middle - and that is the character the eye uses to identify the
whole word. Sxarty pins that character to the same column every time, marked by
the two guide ticks. Punctuation is ignored when finding it, so a trailing comma
does not shift the pivot a place to the right.

The alignment is what does the work. Marking the pivot is a separate choice, and
recolouring it does split the word into three visual pieces, so **Focus mark**
in settings offers a coloured letter, a rule underneath, or nothing at all.

Uniform timing is the other thing that makes RSVP tiring: you get no time to
close a clause before the next one starts. Smart pacing (on by default) holds
longer on commas, sentence endings, paragraph breaks and long words, so the
rhythm follows the prose rather than a metronome.

## Install

```sh
git clone https://github.com/beqaabu/Sxarty
cd Sxarty
cargo build --release
./target/release/sxarty
```

Needs Rust 1.85 or newer.

Document parsing is entirely in-process; there is nothing to install alongside
it. If you only ever read plain text and want a faster build, skip the parsers:

```sh
cargo build --release --no-default-features
```

## Use it

```sh
sxarty                 # start empty
sxarty notes.md        # open a file straight away
```

You can also drag a file onto the window, or paste text with `Cmd`/`Ctrl` + `V`.

**Formats:** `txt`, `md`, `pdf`, `docx`, `epub`.

### Keys

| Key | Does |
| --- | --- |
| `Space` | Play / pause |
| `←` `→` | One word back / forward |
| `Shift` + `←` `→` | One paragraph back / forward |
| `↑` `↓` | Speed ±25 wpm |
| `+` `-` | Font size |
| `1` `2` `3` | Words shown at a time |
| `Home` `End` | Jump to start / end |
| `F` | Cycle the focus mark |
| `C` | Context panel |
| `S` | Settings panel |
| `Z` | Zen mode |
| `Esc` | Leave zen, close a panel, dismiss an error |
| `O` | Open a file |
| `Cmd`/`Ctrl` + `V` | Read from the clipboard |

### Panels

**Context** (left) shows the text around the word you are on, muted, with the
current word lit. RSVP hides structure - you cannot see a paragraph ending
coming, and you lose your place on the page - and this puts it back. It pages
rather than scrolls, so the text only reflows when you cross a page boundary
instead of shuffling 300 times a minute.

**Settings** (right) holds speed, font size, chunk size, smart pacing, the focus
mark, four themes and five accent colours. Everything is saved as you change it.

**Recent** (right) lists what you have opened, with how far through you got.
Reopening a document resumes from where you stopped; so does closing and
reopening the app.

**Zen** hides everything but the word and a hairline of progress.

## Georgian on macOS

Georgian text rendered fine on Linux and came out as empty boxes on macOS. The
cause is not in this app:

`cosmic-text`, the shaper underneath iced, keeps a per-platform table mapping a
Unicode script to a system font that can render it. Its **unix** table contains
`Script::Georgian => "Noto Sans Georgian"`. Its **macOS** table has no Georgian
entry at all, and macOS ships Georgian only inside the private `.SF Georgian`
family that the generic fallback never reaches. On top of that, iced 0.12's
`text` widget defaulted to `Shaping::Basic`, which is documented as doing *no
font fallback whatsoever* - so the seven characters of `ქართული` all shaped to
glyph id 0, `.notdef`, the empty box.

Sxarty embeds Noto Sans Georgian in the binary and selects it explicitly for any
string containing Georgian codepoints, rather than hoping a fallback chain finds
something. Deterministic on every platform, and `cargo test` asserts it against
iced's own font system so it cannot regress.

Worth knowing if you hit this elsewhere: enabling advanced shaping alone *does*
make Georgian appear on macOS, but it falls back to **Menlo**, a monospace
terminal font, which looks wrong beside Latin text. Embedding the font is the
better fix.

## Configuration

Settings and reading positions live in one JSON file:

| Platform | Path |
| --- | --- |
| macOS | `~/Library/Application Support/Sxarty/config.json` |
| Linux | `~/.config/sxarty/config.json` |
| Windows | `%APPDATA%\Sxarty\config.json` |

Delete it to reset. A corrupt or older file is ignored rather than fatal.

## Layout

```
src/
  main.rs           entry point and runtime wiring
  fonts.rs          embedded fonts and script detection
  config.rs         persisted settings and recent documents
  reader.rs         playback: position, timing, pacing
  document/
    mod.rs          the loaded document and its paragraph structure
    tokenize.rs     text to tokens, ORP, per-word hold times
    extract.rs      txt / pdf / docx / epub to plain text
  app/
    mod.rs          application state
    message.rs      every event in the app
    update.rs       the update loop
  ui/
    mod.rs          layout
    skin.rs         the colour system
    style.rs        widget styling
    reader_pane.rs  the word and its guides
    context.rs      the context rail
    chrome.rs       header, footer, panels
assets/fonts/       Noto Sans Georgian (OFL 1.1)
```

The split is by concern rather than by widget: `reader.rs` knows about time and
position but nothing about drawing, `ui/` knows about drawing but holds no
state, and `document/` is pure text processing that is straightforward to test.

## Development

```sh
cargo test          # unit tests, including the Georgian shaping regression
cargo fmt
cargo clippy
```

## Credits

Noto Sans Georgian by the Noto Project Authors, under the SIL Open Font License
1.1 (see `assets/fonts/LICENSE`). Sample text is from Rustaveli's *The Knight in
the Panther's Skin*.
