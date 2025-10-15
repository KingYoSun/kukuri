#!/bin/bash
set -euo pipefail

echo "=== Running Rust tests ==="
cd /app/kukuri-tauri/src-tauri
cargo test --workspace --all-features

echo "=== Running Rust clippy ==="
cargo clippy --workspace --all-features -- -D warnings

echo "=== Running TypeScript tests ==="
cd /app/kukuri-tauri
pnpm test

echo "=== Running TypeScript type check ==="
pnpm type-check

echo "=== Running ESLint ==="
pnpm lint

echo "=== All tests passed! ==="
