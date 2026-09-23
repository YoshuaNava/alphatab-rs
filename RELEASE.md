# Release checklist

Before publishing a release:

- [x] License source code under MPL-2.0 and include the complete license text.
- [x] Set the public repository and homepage to
      `https://github.com/YoshuaNava/alphatab-rs`.
- [ ] Review public API changes and update `CHANGELOG.md`.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo clippy --all-targets -- -D warnings`.
- [ ] Run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`.
- [ ] Run `cargo test --all-targets`.
- [ ] Run `cargo package --list` and confirm the font license and documentation
      are present.
- [ ] Run `cargo package` after filling the license and repository fields.
- [ ] Run `cargo deny check` when cargo-deny is available.
