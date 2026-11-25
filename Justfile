default:
    @just --list

build:
    cargo build --locked

release:
    cargo build --release --locked
    target/release/clausewitz-viewer

run:
    cargo run --locked

test:
    cargo test --locked

fmt:
    cargo fmt

fmt-check:
    cargo fmt -- --check

clippy:
    cargo clippy --locked --all-targets -- -D warnings
