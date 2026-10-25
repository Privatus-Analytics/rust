#!/bin/sh
# Runs rustfmt, Clippy, the tests, the docs and cargo package in the official
# Rust image, so your local Rust does not matter. Usage: ./run-docker.sh
set -e
docker run --rm -v "$PWD":/app -w /app -e CARGO_HOME=/app/target/.cargo -e CARGO_TERM_COLOR=never rust:1-slim sh -c '
  set -e
  rustup component add rustfmt clippy >/dev/null 2>&1
  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo clippy --all-targets -- -D warnings
  cargo test --all-features
  cargo test --no-default-features
  cargo test --all-features --release
  RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
  cargo package --allow-dirty
  echo "All checks passed."
'
