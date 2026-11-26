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

## Development

Game-neutral localization, sprite and direct-texture loading live in
`src/common/`; the reusable searchable master-detail browser is in
`src/gui/common/`. Game-specific models, extraction, and view composition stay
under their respective game modules.

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

If [just](https://just.systems/) is installed, `just --list` shows shortcuts
for common build, run, test, and lint commands. CI runs the checks above.
