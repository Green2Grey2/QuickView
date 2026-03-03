# Contributing

Thanks for contributing to QuickView!

## Development workflow

1. Create a branch
2. Keep changes small and focused
3. Run the local checks before opening a PR:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Coding conventions

- Prefer small modules with clear ownership boundaries.
- Avoid blocking the GTK main thread.
- Keep OCR backends behind traits/interfaces.

## Commit style

- Use imperative, descriptive commit messages.
- Reference issues when applicable.

