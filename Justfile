set shell := ["bash", "-lc"]

fmt:
  cargo fmt --all

clippy:
  cargo clippy --all-targets --all-features -- -D warnings

test:
  cargo test --all

run-full FILE:
  cargo run --bin quickview -- {{FILE}}

run-quick FILE:
  cargo run --bin quickview -- --quick-preview {{FILE}}

