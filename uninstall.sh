#!/usr/bin/env bash
set -euo pipefail
install_dir="${ART_BIN_DIR:-$HOME/.local/bin}"
if [[ -e "$install_dir/art" ]]; then
    rm -- "$install_dir/art"
    printf 'Removed %s/art\n' "$install_dir"
else
    echo 'art is not installed in the selected directory.'
fi
printf 'Preferences retained in %s/terminal-art-launcher/\n' "${XDG_CONFIG_HOME:-$HOME/.config}"
echo 'Terminal art packages are retained.'
