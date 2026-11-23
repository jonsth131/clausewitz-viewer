# Copilot instructions for clausewitz-viewer

## Build, test, and lint

- Build: `cargo build --locked`
- Run the GUI: `cargo run --locked` (or `cargo run --release --locked`)
- Tests: `cargo test --locked`
- Formatting: `cargo fmt -- --check`
- Lint: `cargo clippy --locked --all-targets -- -D warnings`
- See `Justfile` for shortcuts and `README.md` for Linux build dependencies.

The Iced GUI requires a graphical desktop; tests do not require one.

## Architecture

- `src/main.rs` initializes logging and starts the Iced application.
- `src/gui/mod.rs` handles the folder picker, keyboard shortcuts, and debug panel.
- `src/gui/data_view.rs` starts and polls a cancellable background loader, filters
  relative file paths, and displays ordered, expandable parsed values.
- `src/game.rs` discovers `.txt` files and parses them eagerly, reporting
  progress, cancellation, and errors. `src/file.rs` reads UTF-8 without
  discarding non-ASCII text.
- `src/parser.rs` and `src/config.pest` define the Pest grammar and parsed
  values. `ConfigPair` sequences retain source order and duplicate keys.
- `src/logging.rs` collects application logs at Debug level and above for the
  debug panel, retaining up to 10,000 records.

## Parsing and UI notes

- Only `.txt` files are discovered; empty parsed files are omitted.
- Parse and I/O failures are reported in the UI rather than silently discarded.
- Date tokens preserve their source spelling, including optional suffixes.
- Expand/collapse IDs are generated per displayed node and are not stable
  across file selections.
