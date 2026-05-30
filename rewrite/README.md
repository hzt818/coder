Rewrite workspace scaffold for 'coder' — minimal crates created for iterative rewrite.

Crates:
- coder-core
- coder-cli
- coder-tui
- coder-ai
- coder-tools
- coder-storage

Run from repo root:
  cargo build --workspace --manifest-path rewrite\Cargo.toml
  cargo test --workspace --manifest-path rewrite\Cargo.toml
