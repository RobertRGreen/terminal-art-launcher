# terminal-art-launcher

An **Arch Linux** terminal candy gallery, installer, and workspace preset manager,
written in Rust with ratatui and crossterm.
Discover installed art programs, explore their descriptions, save favorites,
and turn your terminal into a rotating canvas.

Command: `art`. Source checkout can live anywhere; local builds install to `~/.local/bin`.

- [Workspace presets and Spotify layout](docs/presets.md)
- [Architecture, development, and testing](docs/development.md)
- [Arch packaging and first AUR submission](docs/packaging.md)

## Install

Supported platform: **Arch Linux**. Source builds require Rust 1.88+ / Cargo and a UTF-8 terminal. A true-color terminal at least
100 × 28 is recommended; the interface works down to 64 × 18.

```sh
git clone https://github.com/RobertRGreen/terminal-art-launcher.git
cd terminal-art-launcher
./install.sh
art
```

For a pacman-managed installation, build the provided Arch package using
[the packaging guide](docs/packaging.md). An AUR submission recipe is included;
the package is **not yet published to AUR**.

The installer builds from `Cargo.lock` and places the executable at
`~/.local/bin/art`. If needed, add `export PATH="$HOME/.local/bin:$PATH"` to
your shell profile. `ART_BIN_DIR` overrides the install/uninstall destination.
No terminal art packages are installed by the installer.

```sh
art --list        # Plain catalog and live install status; no TTY needed
art --no-splash   # Skip the animated startup screen
art --help
./uninstall.sh
```

Uninstall keeps preferences and third-party programs.

## Controls

| Key | Action |
| --- | --- |
| ↑ / ↓ | Select a program |
| ← / → / Tab | Switch sidebar collection |
| Enter | Launch the selected installed program |
| Esc / Q | Quit (Esc first closes search or configuration) |
| / | Search titles, categories, executables, descriptions, and packages |
| Enter in search | Keep filter and return to navigation |
| Esc in search | Clear filter |
| I | Install the selected missing program |
| F | Toggle favorite |
| R | Launch a random available program |
| A | Run each available terminal animation once |
| M | Cycle randomly between cmatrix and unimatrix |
| S | Cycle random terminal effects indefinitely |
| C | Open configuration |
| P | Open saved workspace presets |
| U | Refresh discovery and re-enable failed install attempts |

A random installed program is featured at startup (cbonsai is the fallback
when none are installed). Favorites and the 20 most recently launched entries
have dedicated sidebar collections.

Ordinary launches give the child its normal terminal input. Quit it with its
own controls (usually Q or Ctrl-C) to return. Static text output stays visible
until Enter. During A/M/S playback, **Esc, Q, or Ctrl-C stops** and **Space or →
skips**. The controller owns input during playback; effects receive no stdin.
Effect process groups are terminated and reaped between transitions.

## Catalog

| Category | Programs |
| --- | --- |
| Animations | cbonsai, cmatrix, unimatrix, pipes.sh, asciiquarium, aafire, oneko |
| System | fastfetch, btop, htop, nvtop |
| Text | figlet, toilet, cowsay, ponysay, fortune, lolcat |
| Visualizers | cava |
| Utilities | wttr.in, tty-clock |
| Space | astroterm, globe, starfetch, terrascope |

Discovery checks executable permissions on PATH every launch and refresh.
The gallery intentionally retains missing entries with **Not Installed**,
their descriptions, and package names. This is a curated catalog, not a scan
that attempts to classify arbitrary executables on your machine.

Recipes include demo arguments for typography tools and input for lolcat.
`wttr.in` uses curl with HTTPS and a 15-second request timeout; it contacts the
public weather service. `aafire` comes from `aalib`, `fortune` from
`fortune-mod`, and `unimatrix` uses the `unimatrix-git` package recipe.
Package names and repository availability may differ between distributions.

`oneko` is a graphical X11 cat, so launching requires DISPLAY and it is
excluded from terminal playback. Cava requires working audio capture; nvtop
requires compatible GPU drivers. A/M/S only offer available compatible
programs. Screensaver includes terminal animations, animated Space entries, cava, and tty-clock.
It is an entertainment mode, not a session lock or an idle detector.

## Package installation

Press I to run the first available manager in this order: **paru, yay,
pacman**. Pacman uses sudo unless already root. AUR helpers run as the current
regular user. Native package confirmation, password, and AUR review prompts
remain interactive. There is no automatic system upgrade or `--noconfirm`.
Pacman alone cannot install AUR-only packages; install an AUR helper yourself
or install the program manually.

After installation, PATH is checked again. A failed/cancelled installation
disables that entry's install action until U refreshes discovery. Missing
programs are excluded from launch pools; failing playback effects are removed
from the active sequence. Empty playback modes disappear from the shortcut
bar. Missing entries stay visible so you can still inspect and install them.

## Workspace presets

After saving a preset (for example **Spotify**), press **P**, select it, and
press **Enter**. You can also run:

```sh
art --preset Spotify
art preset list
art preset save "Music Night" --workspace 3
```

In the presets menu, **N** saves the current workspace under a new name.
Presets capture window geometry, art commands, terminal colors/opacity, and
Cava configuration. Restore requires Hyprland and Kitty. Already-correct windows
are left untouched. Spotify’s mini-player must already be open to be restored.
See [the preset guide](docs/presets.md) for all commands and limitations.

## Install terminal candy in bulk

```sh
art install --all --dry-run   # Preview missing packages
art install --all             # Install all supported missing catalog packages
art install space             # Install a category
art install cmatrix           # Install one program
```

The available manager still asks for confirmation. With only pacman, AUR entries
are listed and skipped; install yay or paru to include them. Programs requiring a
manual source build are also reported and skipped. Already-installed programs
are left alone. Installation does not perform a full system upgrade.

## Configuration

Press C to change Aurora/Ember/Ocean colors, animation speed, splash, startup
collection or random launch, and manage/clear favorites. Changes save
automatically. Speed sets splash timing and the 30/15/7-second automatic
playback interval; third-party rendering speed remains that program's own
setting. Select Manage favorites, then F on entries to remove them; add new
favorites with F in any other collection.

Preferences and recent history live at
`${XDG_CONFIG_HOME:-~/.config}/terminal-art-launcher/config.toml`. Writes use
an atomic rename. A malformed configuration produces an actionable error
without overwriting the original file. Multiple launcher instances use
last-writer-wins preferences.

```toml
theme = 0          # 0 Aurora, 1 Ember, 2 Ocean
speed = 1          # 0 slow, 1 normal, 2 fast
splash = true
startup = 0        # 0 all, 1 favorites, 2 recent, 3 random
favorites = ["cmatrix", "cbonsai"]
recent = []
```

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
python3 scripts/smoke_test.py
python3 scripts/presets_smoke_test.py
python3 scripts/install_smoke_test.py
# Optional: requires all three Space programs installed
python3 scripts/space_smoke_test.py
cargo run -- --no-splash
```

`catalog.rs` owns launch recipes and discovery; `config.rs` owns persistence;
`app.rs` owns filtering and state; `runner.rs` owns processes and installation;
`ui.rs` renders the responsive gallery; `main.rs` handles events and terminal
lifecycle. `presets.rs` captures and restores workspace compositions. The launcher uses direct command arguments, never shell command
interpolation. Panic and normal-exit guards restore the terminal.

Tests cover catalog invariants, search/history behavior, playback eligibility,
configuration compatibility, and rendering at several terminal sizes. The PTY
smoke test uses isolated fake programs and a fake package manager to verify
keyboard interaction, launching, persistence, process cleanup, failed installs,
and terminal restoration without installing packages.

Licensed under MIT.

## Space

The **Space** sidebar collection includes these programs:

- [astroterm](https://github.com/da-luce/astroterm): an animated planetarium
  with stars, planets, and constellation lines. The launcher uses a colorful,
  accelerated equatorial sky view (`--speed 10000`), not your geographic
  location. Package: `astroterm` in Arch's extra repository.
- [globe](https://github.com/adamsky/globe): a rotating ASCII Earth, night-side
  shading, and an orbiting camera. Executable: `globe`; AUR package:
  `globe-cli`. Upstream also supports `cargo install globe-cli`.
- [starfetch](https://github.com/Haruno19/starfetch): a random cyan constellation
  card with astronomy facts. It is static output, held until Enter. No current
  Arch/AUR package was found, so the launcher shows a source-build hint instead
  of offering a known-invalid package-manager installation. Upstream provides
  source and `make` instructions. Resource files must be installed alongside
  the program at the resource path compiled into `src/starfetch.cpp`.

- [Terrascope](https://github.com/a-shygun/Terrascope): an interactive Braille
  Earth map with weather, aircraft, earthquakes, and day/night layers. First-run
  maps and live layers need internet; upstream supports `--offline` for cached
  data. Quit with Q. It stays out of automatic playback because it is interactive.
  The launch recipe disables terminal-palette redefinition to preserve your colors.
  No published Arch/AUR package was found when this entry was added. Use
  upstream's [Arch PKGBUILD](https://github.com/a-shygun/Terrascope/tree/main/packaging/arch),
  or `pipx install terrascope` after installing Arch's `python-pipx` package.
  The gallery discovers either installation on PATH and provides a manual-install hint.
  If the portable build reports `CERTIFICATE_VERIFY_FAILED`, use the Python
  installation above so it uses the system certificate store. Check
  `~/.cache/terrascope/terrascope.log` for download errors; keep TLS verification
  enabled. After switching installations, close and reopen Terrascope.

Search `/space` from All to find all four. Astroterm and globe participate in
A (animation sequence) and S (screensaver); static starfetch cards do not.

Upstream installation options and package mappings are listed above. Source-only
programs stay discoverable, with a build hint instead of an invalid install action.
