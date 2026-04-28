# crater task runner — `brew install just` then `just <target>`
# List all targets: just --list

# ── Dev ───────────────────────────────────────────────────────────────────────

# Start the server (falls back to client_id scrape if no SC creds set)
run:
    cargo run -p crater

# Start the server with local dev defaults (data in ./data, port 8080, debug logging)
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p data
    CRATER_BIND=127.0.0.1:8080 \
    CRATER_DATA_DIR=./data \
    CRATER_LOG=crater=debug,crater_core=debug,sc_client=debug \
    cargo run -p crater

# Type-check the whole workspace without building
check:
    cargo check --workspace

# Build all crates
build:
    cargo build --workspace

# Lint
clippy:
    cargo clippy --workspace -- -D warnings

# ── Tests ─────────────────────────────────────────────────────────────────────

# All non-live tests (no network required)
test:
    cargo test --workspace

# Server smoke tests only (spins up a real server, no SC)
test-smoke:
    cargo test -p crater --test smoke

# sc_client unit tests
test-client:
    cargo test -p sc_client

# crater-core unit tests
test-core:
    cargo test -p crater-core

# Live server tests — hits real SoundCloud (requires network)
test-live:
    cargo test -p crater --features live-tests --test live -- --nocapture

# Live sc_client tests — hits real SoundCloud (requires network)
test-client-live:
    cargo test -p sc_client --features live-tests -- --nocapture
