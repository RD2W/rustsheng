# Contributing to rustsheng

Thanks for your interest! Contributions are welcome.

## Getting started

```bash
git clone <repo> && cd rustsheng
# Linux: sudo apt-get install -y libudev-dev pkg-config
cargo build --workspace
cargo test --workspace
```

Development happens on the `dev` branch; cut feature branches from it
(`feat/...`, `fix/...`).

## Before you open a PR

All of these must pass (CI enforces them):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p rustsheng-core --no-default-features   # core builds without serial/flash-v5
```

## Conventions

- **English** code comments and commit messages; Conventional Commits
  (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`).
- New source files start with the SPDX header (see any existing `.rs` file).
- Keep the core (`rustsheng-core`) free of CLI/UI dependencies and free of
  `process::exit`/`panic!` on radio input — return typed `Result`s.
- Prefer small, focused files with clear responsibilities.

## Protocol / hardware changes

The wire protocol is reverse-engineered. When adding or changing protocol code:

- Cite a reference (`k5prog`, `k5prog-win`, `K5TOOL`, `uv-k5-firmware`) or attach
  captured datagrams (`rustsheng sniffer` / `-vvv`).
- Add unit tests with concrete byte vectors. For anything that can't be tested on
  hardware (especially **flashing**), cross-validate the generated packets against
  a reference tool and say so in the PR.
- Never weaken the safety gating on destructive operations without discussion.

## Documentation

Update both `docs/en` and `docs/ru` when behavior changes, and add an entry to
`CHANGELOG.md` under "Unreleased".

## License

By contributing you agree that your contributions are licensed under
**GPL-3.0-or-later**, consistent with the rest of the project.
