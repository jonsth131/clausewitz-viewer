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
