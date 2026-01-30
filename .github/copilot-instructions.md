# Copilot instructions for clausewitz-viewer

This file documents repository-specific build/test/lint commands, high-level architecture, and key conventions to help Copilot sessions work effectively in this project.

---

## Build, run, test, and lint commands

- Build (debug):
  - cargo build
- Run (debug, launches GUI):
  - cargo run
  - For better performance, use: cargo run --release
- Run tests (full suite):
  - cargo test
- Run a single unit test by name (matches test function name):
  - cargo test test_parse_config_file -- --nocapture
  - or to match exactly: cargo test --exact test_parse_config_file
- Format and lint (if installed):
  - cargo fmt
  - cargo clippy -- -D warnings

Notes:
- The GUI uses iced and requires a graphical environment (X11/Wayland) to display.
- Parsing and file I/O are performed asynchronously via iced Commands; running the binary in a headless CI will not show dialogs.

---

## High-level architecture

- Binary entry: src/main.rs delegates to gui::run() which starts the iced Application.
- GUI layer (src/gui/*):
  - mod.rs: Application implementation (ClausewitzViewer) that opens a folder dialog (rfd) and switches to DataView.
  - data_view.rs: Renders file selection combo box and a collapsible tree of parsed config values; performs parsing on a background task via Command::perform(parse(...)).
- Parsing layer (src/parser.rs + src/config.pest):
  - config.pest contains the grammar used by pest to parse Clausewitz-style config files.
  - parse_config_file returns Vec<ConfigPair> where ConfigPair { identifier, sign, value } and ConfigValue is an enum (Object, Array, String, Number, Identifier, Date, Named).
  - Many unit tests live inside parser.rs validating grammar behavior.
- File discovery and I/O (src/game.rs, src/file.rs):
  - parse_game(path: &PathBuf) recursively searches the provided folder for files with the `.txt` extension, parses each using parse_config_file, and returns a HashMap mapping file stem -> Vec<ConfigPair>.
  - read_file in src/file.rs reads the file as UTF-8, filters to ASCII characters only, and returns the string used by the parser.
- Data mapping: data_view::map_values converts ConfigPair/ConfigValue into a UI-friendly DataValue tree; each node gets a UUID string id used to identify expand/collapse actions.

---

## Key conventions and repo-specific gotchas

- Only `.txt` files are considered: find_txt_files in src/game.rs filters by extension == "txt"; files with other extensions are ignored.
- Non-ASCII characters are stripped: read_file() filters to ASCII only; expect non-ASCII characters to be lost before parsing.
- Parser grammar specifics (see src/config.pest):
  - number rule expects a trailing space or newline (the grammar uses a terminal that includes NEWLINE or space). This can make some numeric tokens parse-fail if not separated.
  - date format is `YYYY.M[.]D[.]` (e.g., 1990.1.31) and allows an optional extra `.` + digits.
  - sign operators supported: <=, >=, !=, =, <, >, +
  - arrays and objects both use braces `{ ... }` but are distinguished by contents (array = sequence of values, object = sequence of pairs).
  - named constructs are `IDENT ASCII_ALPHA+` followed by an array (e.g., `terrain { ... }`).
- Error handling behavior:
  - parse_config_file returns an Err on parse failure; parse_game logs the error to stderr and silently skips files that fail to parse (see src/game.rs).
- UI behaviors:
  - DataView loads parsing asynchronously and populates a combo box with file stems; selecting a file constructs a tree of DataValue items grouped by identifier.
  - Expand/collapse actions use generated UUIDs per DataValue; these are not stable across reloads.

---

## Related files and AI assistant configs

- No dedicated AI assistant config files (CLAUDE.md, AGENTS.md, .cursorrules, .windsurfrules, etc.) were detected in the repository root.

---

If anything in this file should be expanded (for example CI commands, environment variables, or platform-specific notes), say which area to expand and Copilot will add it.
