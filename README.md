# Clausewitz Viewer

A desktop viewer for Clausewitz-style configuration files. Open a game or mod
directory to browse its `.txt` files and inspect parsed values as a tree.

## Setup

Install the stable Rust toolchain from [rustup](https://rustup.rs/). On Linux,
install the native X11/Wayland build dependencies, for example on Ubuntu:

```sh
sudo apt install build-essential pkg-config libx11-dev libxcursor-dev \
  libxrandr-dev libxi-dev libxinerama-dev libxkbcommon-dev libwayland-dev
```

Run `cargo run --locked` (or `cargo run --release --locked` for larger folders).
The GUI needs a graphical desktop; the folder picker uses the desktop's native
dialog or portal.

## Usage

- Choose **Open Clausewitz Game Path...** or press **Ctrl+O** and select a
  folder. Discovery and parsing happen in a background worker; progress is
  shown and loading can be cancelled.
- The viewer identifies Hearts of Iron IV and Stellaris base-game folders by
  their directory structure. Unknown or ambiguous folders use the generic
  browser; mod overlays are not yet supported.
- For Hearts of Iron IV, select **National Focus** and choose a tree to display
  its locally defined focuses and available base-game icons in a scrollable
  layout. Missing or unsupported images fall back to text. Positions use `x`,
  `y`, and relative-position references; prerequisite IDs appear on each
  focus. Overlapping focuses are grouped, and unresolved positions are listed
  separately. Names and descriptions use English localization where available;
  hover over a focus to see its script information. Arrows connect positioned
  prerequisites with right-angle routes; explicitly hidden focuses and their
  arrows are muted. Drag with the left mouse button to pan the tree; use the
  mouse wheel over the focus tree to zoom in or out. Zoom is bounded, and
  scrollbars remain available.
- Select **Decisions** to browse decisions by their source file. Each row
  shows the localized title and available icon; hover for the description and
  script details. Missing localization or images fall back to the ID or text.
- Select **Events** to choose a source file and search events by title, ID, or
  type. Select an event to inspect localized text, available pictures, options,
  and script details. Conditional text is shown without evaluating conditions.
- Select **Ideas** to browse all idea families by source file and search by
  name, ID, or family. Select an idea to inspect its localized description,
  available picture, and ordered script without evaluating conditions.
- Filter files by their relative paths, then select a file. Expand or collapse
  objects, arrays, and named values individually or all at once. Repeated keys
  remain in source order.
- File read and parse errors appear above the selected file. Empty files and
  files without a `.txt` extension are omitted. Files must be valid UTF-8.
- Press **Ctrl+`** to toggle the debug panel. Select text and press **Ctrl+C**
  to copy it. It keeps the most recent 10,000 application log records at Debug
  level and above for the current session.

The parser supports the syntax represented by its grammar in `src/config.pest`;
some game- or version-specific constructs may not yet be supported.

## Development

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

If [just](https://just.systems/) is installed, `just --list` shows shortcuts
for common build, run, test, and lint commands. CI runs the checks above.
