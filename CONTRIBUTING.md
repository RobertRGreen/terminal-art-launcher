# Contributing

`art` targets Arch Linux. Bug reports, verified catalog recipes, usability fixes,
and documentation improvements are welcome. Open an issue before broad changes
such as another desktop backend or package manager.

1. Fork and clone the repository.
2. Install Rust, Cargo, Python, and Git on an up-to-date Arch system.
3. Make a focused change and update the relevant documentation.
4. Run the checks in [development.md](docs/development.md).
5. Submit a pull request explaining the user-visible behavior and validation.

Use the simulated preset backend for tests. Never move a user's desktop windows
or apply their personal preset as part of routine validation. Do not commit personal
presets, screen captures, credentials, build output, or package archives.

For catalog entries, verify the upstream executable, supported arguments, and
current Arch/AUR package name. Mark source-only programs honestly. Programs
must be tested separately for normal launch and timed playback eligibility.

Please report your Arch version, `art --version`, terminal emulator, steps to
reproduce, and relevant non-sensitive errors when filing a bug. Avoid attaching
full process environments or personal preset files without checking their contents.

By contributing, you agree that your contributions use the project's MIT license.
