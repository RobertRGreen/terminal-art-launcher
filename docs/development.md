# Architecture and development

Package name: `terminal-art-launcher`; binary name: `art`.
Supported platform: Arch Linux.

## Modules

| File | Responsibility |
| --- | --- |
| `src/main.rs` | CLI, key routing, terminal lifecycle, launch/playback orchestration |
| `src/catalog.rs` | Program metadata, safe argv recipes, PATH discovery |
| `src/app.rs` | Search, collections, selection, favorites, playback eligibility |
| `src/config.rs` | XDG preferences, bounded history, atomic updates |
| `src/runner.rs` | Child processes, package-manager execution, timed effects |
| `src/ui.rs` | Gallery, responsive preview, settings and presets overlays |
| `src/presets.rs` | Capture, versioned storage, Hyprland placement, Kitty launch gate |

The catalog is deliberately curated. Add entries with their actual executable,
verified package name or source-install hint, meaningful arguments, and correct
continuous/playback flags. A program requiring interactive input is not necessarily
compatible with automatic playback.

## Process and terminal ownership

Ordinary launches restore the terminal and give the child inherited input/output.
Static output pauses until Enter. Automatic playback reserves keyboard input for
the launcher, supplies null stdin to effects, and creates a separate process group
so scripts and their children can be stopped together.

Background effect groups ignore SIGTTOU and SIGTTIN so terminal setup does not
suspend them under job control. Drop cleanup terminates and reaps the group.
Normal-exit and panic guards restore cooked mode, cursor visibility, and the
original terminal screen.

Preset terminals are separate desktop windows. A local Unix socket gates effect
startup until placement is complete. Socket names are unique; access is restricted
to the user; cleanup removes the socket. The internal `__preset-exec` command is
an implementation detail, not a public command.

## Verification

Run from the project directory:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
python3 scripts/smoke_test.py
python3 scripts/presets_smoke_test.py
python3 scripts/install_smoke_test.py
```

- Unit tests cover catalog invariants, filtering, history, configuration defaults,
  layout rendering, preset names, coordinate transforms, and placement behavior.
- `smoke_test.py` uses a PTY and isolated fake programs/package manager. It covers
  search, launching, favorites, configuration, playback cancellation, failed
  installation suppression/retry, and terminal restoration.
- `presets_smoke_test.py` supplies fake `hyprctl` and `kitty` executables, isolated
  XDG directories, and a fixture program. It verifies preset listing/launching,
  no-op restoration, movement without resizing, correct floating actions, gated
  startup after placement, and socket cleanup. It never contacts the real desktop.
- `install_smoke_test.py` uses a fake manager to verify bulk planning, dry runs,
  source/AUR skips, exact command arguments, and the root-user guard.
- `space_smoke_test.py` is optional and requires installed astroterm, globe, and
  starfetch. It tests effect rendering/exit in a controlling PTY. It does not
  rearrange desktop windows.

Do not use the user's Spotify desktop as a test fixture. Mocked tests establish
command construction and orchestration; they do not prove every compositor or
third-party application behavior. Keep this distinction in release notes.

## Why the preset regression tests matter

Hyprland 0.56 accepts floating actions `enable`/`disable`. Unknown strings such
as `set`/`unset` fall back to toggling. The first live prototype used the wrong
strings and temporarily changed the user's layout; resizing also interrupted
some effects. The corrected implementation is tested for explicit valid actions,
no operations for already-correct windows, and delayed startup for newly placed
terminals. Future changes must preserve these properties.

## Build, install, and remove

```sh
./install.sh
art --version
art preset list
./uninstall.sh
```

The installer uses the lockfile and installs `~/.local/bin/art` by default.
`ART_BIN_DIR=/some/path ./install.sh` selects a different destination; pass the
same setting to uninstall. Installing does not launch a preset, edit a saved
layout, install art packages, or restart existing launcher processes.

Configuration, preset data, and third-party packages are retained by uninstall.
Commit Cargo.lock when changing dependencies. Rust 1.88 or newer is required by
the current dependency lockfile.
