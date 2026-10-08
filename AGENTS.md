# Working on art

- This is the Rust `terminal-art-launcher` project; its installed command is `art`.
- Work in the repository root. Machine-local notes, if any, live in the ignored
  `docs/local-setup.md`.
- Do not apply presets, move/resize desktop windows, or restart live art programs
  to validate changes unless the user explicitly requests that live operation.
  Use `scripts/presets_smoke_test.py`, which provides a simulated desktop.
- Preserve the user's saved Spotify preset under their XDG configuration directory.
  Do not replace it with test fixtures or recapture it as part of a build/install.
- Use `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo build`, and the isolated smoke tests before installing a release.
- Keep commands as argument arrays. Do not interpolate captured commands into a shell.
- Hyprland 0.56 uses Lua dispatchers. Floating actions are `enable` and `disable`;
  `set` and `unset` fall back to toggling and can scramble layouts.
- An already-correct preset window must produce no placement operations. Position
  new terminals before starting effects; some programs exit when resized.
- Document behavior and limitations in README.md and docs/, including validation
  actually performed. Do not claim a mocked test validated the live compositor.
