## Summary

What does this change and why?

## Related issues

Closes #...

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [ ] `cargo test --workspace` passes
- [ ] `cargo build -p rustsheng-core --no-default-features` still builds (if the core changed)
- [ ] Commit messages follow Conventional Commits (`feat:`, `fix:`, ...)
- [ ] New source files carry the SPDX / copyright header
- [ ] Docs updated (`docs/en` and `docs/ru`) if behavior changed
- [ ] `CHANGELOG.md` updated under "Unreleased"

## Protocol / hardware impact

- Does this touch the wire protocol or flashing? If so, how was it validated
  (unit vectors, cross-check vs K5TOOL, real hardware)?
