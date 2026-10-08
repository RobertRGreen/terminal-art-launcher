#!/usr/bin/env bash
set -euo pipefail
command -v pacman >/dev/null || { echo "art supports Arch Linux; pacman was not found." >&2; exit 1; }
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
command -v cargo >/dev/null || { echo 'Rust/Cargo is required. Install your distribution rust package or use rustup.' >&2; exit 1; }
cargo build --release --locked --manifest-path "$project_dir/Cargo.toml"
install_dir="${ART_BIN_DIR:-$HOME/.local/bin}"
mkdir -p -- "$install_dir"
staged_binary="$(mktemp "$install_dir/.art-install.XXXXXX")"
trap 'rm -f -- "$staged_binary"' EXIT
install -m 755 -- "$project_dir/target/release/art" "$staged_binary"
mv -f -- "$staged_binary" "$install_dir/art"
trap - EXIT
printf 'Installed %s/art\n' "$install_dir"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) printf 'Add this to your shell profile: export PATH="%s:$PATH"\n' "$install_dir" ;;
esac
