#!/usr/bin/env bash
# Developer rebuild — Omarchy users should prefer:
#   omarchy plugin add https://github.com/cybercore-tech/cyberfleet.git --enable
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo "Building cyberfleet (release)..."
cargo build --release

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  BIN_SRC="${CARGO_TARGET_DIR}/release/cyberfleet"
elif [[ -f "$HOME/.cargo/config.toml" ]] && grep -q "target-dir" "$HOME/.cargo/config.toml"; then
  TARGET_DIR=$(grep "target-dir" "$HOME/.cargo/config.toml" | sed -E 's/.*=\s*"(.*)"/\1/')
  BIN_SRC="${TARGET_DIR}/release/cyberfleet"
else
  BIN_SRC="target/release/cyberfleet"
fi

if [[ ! -f "$BIN_SRC" ]]; then
  echo "Could not find built binary at: $BIN_SRC"
  exit 1
fi

case "$(uname -m)" in
  x86_64 | amd64) ARCH_DIR="linux-x86_64" ;;
  aarch64 | arm64) ARCH_DIR="linux-aarch64" ;;
  *) ARCH_DIR="linux-$(uname -m)" ;;
esac

PLUGIN_BIN_DIR="$ROOT/bin/$ARCH_DIR"
mkdir -p "$PLUGIN_BIN_DIR"
strip -o "$PLUGIN_BIN_DIR/cyberfleet" "$BIN_SRC" 2>/dev/null \
  || cp "$BIN_SRC" "$PLUGIN_BIN_DIR/cyberfleet"
chmod +x "$PLUGIN_BIN_DIR/cyberfleet"
echo "Bundled plugin binary → $PLUGIN_BIN_DIR/cyberfleet"

if [[ "${1:-}" == "--local-bin" ]]; then
  mkdir -p "$HOME/.local/bin"
  cp "$PLUGIN_BIN_DIR/cyberfleet" "$HOME/.local/bin/cyberfleet"
  chmod +x "$HOME/.local/bin/cyberfleet"
  echo "Also installed → $HOME/.local/bin/cyberfleet"
fi

echo "Done. Omarchy install:"
echo "  omarchy plugin add https://github.com/cybercore-tech/cyberfleet.git --enable"
