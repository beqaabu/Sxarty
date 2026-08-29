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
rhythm follows the prose rather than a metronome. At 300 wpm an ordinary word
gets 200ms, a word ending in a comma 260ms and one ending a sentence 320ms; no
single word is ever held longer than twice the base, so a pause reads as a
breath rather than a stall. Turn it off in settings for a flat pace.

## Install

Grab a build from [releases](https://github.com/beqaabu/Sxarty/releases).

| | |
| --- | --- |
| **macOS** | `Sxarty-<version>.dmg`, universal (Apple silicon and Intel), macOS 11+ |
| **Windows** | `Sxarty-<version>-setup.exe`, 64-bit |
| **Linux** | `.AppImage` to run anywhere, `.deb` for Debian and Ubuntu, or a `.tar.gz` |

Nothing else is needed alongside it. Document parsing happens in-process, so
there is no `pdftotext` or `pandoc` to install.

### From source

```sh
git clone https://github.com/beqaabu/Sxarty
cd Sxarty
cargo build --release
./target/release/sxarty
```

Needs Rust 1.85 or newer. If you only read plain text and want a faster build,
skip the document parsers with `--no-default-features`.

### Building the installers

```sh
scripts/bundle-macos.sh    # .app and .dmg  (add --host-only to skip Intel)
scripts/bundle-linux.sh    # .tar.gz, .AppImage and .deb
```

Windows installers are built with [NSIS](https://nsis.sourceforge.io):
`makensis -DVERSION=0.2.0 packaging/windows/sxarty.nsi`.

Signing is opt-in on both signed platforms. The macOS script signs and notarises
only when `MACOS_SIGN_IDENTITY` and `MACOS_NOTARY_PROFILE` are set, and produces
an unsigned build otherwise, so it works the same on a laptop with no Apple
account and in CI with one. Pushing a `v*` tag builds and publishes all three
platforms.

## Use it

```sh
sxarty                 # start empty
sxarty notes.md        # open a file straight away
```

You can also drag a file onto the window, or paste text with `Cmd`/`Ctrl` + `V`.

**Formats:** `txt`, `md`, `pdf`, `docx`, `epub`.

On Windows and Linux, Sxarty is offered under "Open with" for those formats
without taking them over from whatever opens them now. **Not on macOS**: Finder
delivers an opened document as a `kAEOpenDocuments` Apple Event, and winit (under
iced) neither handles it nor exposes its `NSApplicationDelegate` for us to. An
association that silently opened an empty window would be worse than none, so
Sxarty does not register one there. Drag-and-drop and `sxarty FILE` both work.

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

**Every word in it is a jump target.** Click one and the reader moves there,
which is the answer to the thing RSVP is worst at: half-catching a clause and
wanting to go back to it. The panel follows along while you are reading so the
current word is always on screen, and leaves your scrolling alone while you are
paused, so you can look around and click without it snapping back.

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
  build.rs          embeds the Windows icon and version metadata
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
assets/icon/        generated by scripts/make_icons.py
packaging/          per-platform manifests and installer scripts
scripts/            icon generation and the bundlers
```

The split is by concern rather than by widget: `reader.rs` knows about time and
position but nothing about drawing, `ui/` knows about drawing but holds no
state, and `document/` is pure text processing that is straightforward to test.

## Development

```sh
cargo test                     # includes the Georgian shaping regression
cargo fmt
cargo clippy --all-targets
python3 scripts/make_icons.py  # only when the icon changes; output is committed
```

The icon is generated rather than drawn: one geometric description in
`scripts/make_icons.py` emits the SVG, every PNG size, the Windows `.ico` and the
macOS iconset, using nothing but the standard library. The mark is the reading
pane itself, with the right-hand bar longer than the left because the pivot sits
left of centre.

## Credits

Noto Sans Georgian by the Noto Project Authors, under the SIL Open Font License
1.1 (see `assets/fonts/LICENSE`). Sample text is from Rustaveli's *The Knight in
the Panther's Skin*.
