# Arch Linux packaging and first AUR submission

Repository: https://github.com/RobertRGreen/terminal-art-launcher
Package name: `terminal-art-launcher`. Installed executable: `/usr/bin/art`.
The project supports Arch Linux; no cross-distribution installer is maintained.

## Build a pacman-managed package

After cloning the repository:

```sh
cd packaging/arch
# Review PKGBUILD first.
makepkg -si
```

This compiles the tagged release with the committed Cargo.lock, runs Rust tests,
and installs the resulting package through pacman. The package installs the
launcher, a desktop entry, `man art`, license, and basic documentation. Terminal
candy programs are optional and are installed through art when requested.

If you previously used `install.sh`, `~/.local/bin/art` may take precedence over
`/usr/bin/art`. Run the project's `uninstall.sh` to remove that user-local copy
before switching to the pacman-managed installation. Preferences and presets stay.

`x86_64` and `aarch64` builds are declared; the initial local package verification
was performed on x86_64. The GitHub CI job also uses an Arch x86_64 container.

## AUR status

The repository includes `packaging/arch/PKGBUILD` and `.SRCINFO` for submission.
**It is not published to AUR yet.** A public GitHub repository and release do not
automatically create an AUR package.

## First contribution: account and key

1. Register an account at https://aur.archlinux.org/register.
2. Create an SSH key dedicated to AUR, if you do not already have a suitable key:

   ```sh
   ssh-keygen -t ed25519 -f ~/.ssh/aur -C "AUR package maintenance"
   ```

3. Add the contents of `~/.ssh/aur.pub` to the AUR account's SSH public-key field.
   Keep the private `~/.ssh/aur` file on your machine.
4. Add this entry to `~/.ssh/config`:

   ```sshconfig
   Host aur.archlinux.org
       User aur
       IdentityFile ~/.ssh/aur
   ```

5. Verify the server fingerprint against the AUR documentation before accepting
   a new host key, then test authentication:

   ```sh
   ssh aur@aur.archlinux.org help
   ```

## Submit the package

First read the [AUR submission guidelines](https://wiki.archlinux.org/title/AUR_submission_guidelines)
and confirm that the package name is still available.

```sh
git clone ssh://aur@aur.archlinux.org/terminal-art-launcher.git
cd terminal-art-launcher
cp /path/to/project/packaging/arch/PKGBUILD .
makepkg --printsrcinfo > .SRCINFO
# Review, build, and optionally inspect with namcap before submission.
makepkg
namcap PKGBUILD ./*.pkg.tar.zst
git add PKGBUILD .SRCINFO
git commit -m "Initial release: terminal-art-launcher 0.2.0"
git push
```

An empty-repository warning when cloning a new package is expected. The first
successful push publishes the package. Commit only the PKGBUILD, `.SRCINFO`, and
any necessary small packaging files to AUR—not Rust source or built archives.

Once the AUR package is actually published, users can install it with
`yay -S terminal-art-launcher` or `paru -S terminal-art-launcher`. Until then,
use the source installer or build the included PKGBUILD locally.

## Maintaining releases

1. Update the Cargo version, lockfile, changelog, and manual page.
2. Pass formatting, lint, unit tests, and all isolated integration tests.
3. Push a release tag such as `v0.2.0` and publish its GitHub release.
4. Download that tag's source archive and calculate its SHA-256 checksum.
5. Update `pkgver`, reset `pkgrel` to 1, and update `sha256sums` in PKGBUILD.
6. Regenerate `.SRCINFO`; rebuild and check the package.
7. Commit the packaging update and push it to the AUR repository.

For a packaging-only revision, increment `pkgrel` instead of `pkgver`.
The `.in` file is the recipe template; `PKGBUILD` is the generated, usable recipe.
The source archive must correspond to an immutable release tag.

References: [PKGBUILD manual](https://man.archlinux.org/man/PKGBUILD.5.en),
[AUR guidelines](https://wiki.archlinux.org/title/AUR_submission_guidelines).
