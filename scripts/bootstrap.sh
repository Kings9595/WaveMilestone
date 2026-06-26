#!/usr/bin/env bash
# bootstrap.sh — Install and configure all Soroban development dependencies.
set -euo pipefail

echo "==> Bootstrapping Soroban dependencies..."

# Rust
if ! command -v rustc &>/dev/null; then
    echo "==> Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    source "$HOME/.cargo/env"
fi
echo "  [OK] Rust $(rustc --version)"

# WASM target
echo "==> Adding wasm32-unknown-unknown target..."
rustup target add wasm32-unknown-unknown
echo "  [OK] wasm32-unknown-unknown"

# Stellar CLI (Soroban)
if ! command -v stellar &>/dev/null; then
    echo "==> Installing Stellar CLI..."
    cargo install --locked stellar-cli --features opt
fi
echo "  [OK] Stellar CLI $(stellar --version 2>&1 | head -1)"

# .env
if [ ! -f .env ] && [ -f .env.example ]; then
    cp .env.example .env
    echo "==> Created .env from .env.example. Edit it with your Stellar credentials."
fi

echo ""
echo "==> Bootstrap complete. Run 'make build' to compile the contract."
