# Pre-publication validation

Validation on the maintainer's Arch Linux x86_64 workstation, 2026-10-08.
The tested launcher includes the post-0.2.1 toilet recipe fix; existing 0.2.1
release assets do not include that fix. Build a new release before AUR submission.

## Real program smoke checks

Twenty-two catalog entries passed launch, output/render, and return-to-gallery
checks through the installed `art` in isolated PTYs:

- Animations: cbonsai, cmatrix, unimatrix, pipes.sh, asciiquarium, aafire.
- System: fastfetch, btop, htop, nvtop.
- Text: figlet, toilet, cowsay, ponysay, fortune, lolcat.
- Visualizers: cava.
- Utilities: wttr.in (real network request).
- Space: astroterm, globe, starfetch, terrascope.

Toilet initially failed because its Arch package now names the color filter
`rainbow`. The launch recipe was corrected, rebuilt, installed, and retested.
Terrascope uses its Python installation after the portable binary's certificate
failure; verified map downloads and map rendering were checked separately.

Pending: tty-clock installation and test; Oneko test with an isolated Xvfb display.
These remain release-validation blockers. Oneko itself is installed.

## Launcher checks

Formatting, clippy with warnings denied, eight Rust unit tests, build, simulated
UI/installation/preset integration tests, and real Space playback tests passed.
No live desktop windows or saved presets were modified for testing.

## Scope

These checks establish basic launch/render/return behavior on one machine.
They do not validate every program feature, audio response, GPU model, live data
provider, compositor, or aarch64. Test logs contain local machine details and are
excluded from the repository. See development.md for repeatable commands.
