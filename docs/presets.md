# Workspace presets

Presets save a composition of terminal art and desktop windows. The current
backend targets **Hyprland 0.56's Lua API and Kitty**. The basic art launcher
works without this backend.

## Use a saved preset

Open a new `art` session, press **P**, select **Spotify**, then press **Enter**.
Or run:

```sh
art --preset Spotify
# Equivalent, with an optional destination override:
art preset launch Spotify
art preset launch Spotify --workspace 4
```

Launching restores the saved windows and focuses the destination workspace.
Existing matching windows are reused; missing supported programs are started.
A workspace override moves matching saved windows to the new workspace rather
than duplicating them. Unrelated windows are not closed.

## Save and inspect

```sh
art preset list
art preset show Spotify
art preset save "Music Night" --workspace 3
# Explicitly replace a saved preset after rearranging it:
art preset save "Music Night" --workspace 3 --replace
```

In the P menu, **N** saves the current workspace under a new name. Type a name,
press Enter to save, or Esc to cancel. Names accept letters, digits, spaces,
hyphens, and underscores. Existing names are not silently overwritten.

Capturing reads the desktop and terminal metadata; it does not reposition windows.

## Spotify compositions

A Spotify preset can combine a Matrix backdrop, bonsai, aquarium, Cava, the
Spotify desktop app, and its mini-player. Arrange them, then capture the workspace:

```sh
art preset save Spotify --workspace 3
```

Presets are machine-local; this repository does not ship anyone's personal layout.
Spotify's mini-player must already be open to be repositioned. If it is closed,
open it in Spotify and reapply the preset. The main Spotify app can be started
automatically. Song, playlist, and playback position are not changed by a preset.

## What capture preserves

- Workspace number and source monitor's logical dimensions and origin.
- Window positions, sizes, floating/tiled state, and identities for reuse.
- Recognized art commands with their arguments and working directories.
- Kitty's current color palette and opacity; configured font and padding values.
- Cava's configuration file contents, replayed through a private `-p` config copy.

Kitty capture currently supports one tab and one terminal pane per OS window.
Unsupported split/tabbed windows produce a clear error instead of a lossy preset.
Only recognized catalog commands are automatically captured from terminals;
launcher-only terminals, shells, and unrelated commands are skipped with notes.
Other desktop windows are reuse-only, except the supported main Spotify app.

Kitty needs remote control enabled and a PID-suffixed socket, for example:

```conf
allow_remote_control yes
listen_on unix:@mykitty
```

This workstation already has that configuration. Capture locates each terminal's
socket through `/proc/net/unix`; it does not copy process environments or credentials.

## Restore behavior

1. Validate the saved file and detect the source or fallback monitor.
2. Match saved terminal instances or terminals previously launched by that preset.
3. Start missing Kitty windows floating, with an internal launch gate.
4. Apply only necessary workspace, floating-state, size, and position changes.
5. Release the gate so an effect starts after its terminal has been placed.
6. Focus the target workspace and report restored/missing windows.

Already-correct windows receive **no** move, resize, or floating-state commands.
Moving a window without resizing it does not emit a resize command. Coordinates
scale proportionally when the target monitor has different logical dimensions.
The tiled background follows the compositor's tiling layout; exact tiled geometry
assumes no additional tiled windows have been added to the destination workspace.

Effects use fresh process state when started: a bonsai or aquarium animation does
not resume the exact frame from capture. Resizing a currently running third-party
program can still cause that program to exit, depending on its own resize handling.
Reapplying an unchanged layout avoids those unnecessary resize events.

## Storage format

Presets are versioned JSON files inside
`${XDG_CONFIG_HOME:-~/.config}/terminal-art-launcher/presets/`.
The filename is the lowercased preset name with spaces changed to hyphens.
Each file contains `version`, `name`, `workspace`, `monitor`, `windows`, and `notes`.
A window stores `command` as an argv array, never shell syntax. Empty `command`
means reuse-only. Resource copies are materialized in `<preset-name>-assets/`.

Presets contain executable commands and local paths. Treat imported preset files
as executable configuration. User presets are separate from project source and
survive installation, upgrades, and uninstallation.

## Troubleshooting

- **P does nothing in an old launcher:** exit that launcher and start `art` again;
  running processes keep the version they originally loaded.
- **Remote socket not found:** check Kitty's `allow_remote_control` and `listen_on`;
  restart only the affected terminal when convenient.
- **Missing program:** install its package, then restore again. Successful windows
  are retained and already-correct ones are left alone.
- **Mini-player missing:** open it from Spotify, then restore again.
- **Different monitor:** coordinates scale to the matching monitor if connected,
  otherwise to the focused or first available monitor.
- **Preset capture fails:** the existing saved file remains intact. Inspect
  `art preset show NAME` before explicitly replacing it.
