#!/usr/bin/env bash
set -euo pipefail

# questlog installer.
# Builds the release binary, installs it to ~/.local/bin, and ensures the data
# directory exists.
#
# Overridable env vars:
#   BIN_DIR       install target  (default: ~/.local/bin)
#   QUESTLOG_DIR  data root       (default: ~/.local/tui-live-workprogress)

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${QUESTLOG_DIR:-$HOME/.local/tui-live-workprogress}"
BIN_NAME="questlog"

echo "==> building release binary"
cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"

echo "==> installing to $BIN_DIR/$BIN_NAME"
mkdir -p "$BIN_DIR"
install -m 0755 "$REPO_DIR/target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"

echo "==> installing scratchpad toggle to $BIN_DIR/$BIN_NAME-toggle"
install -m 0755 "$REPO_DIR/contrib/questlog-toggle.sh" "$BIN_DIR/$BIN_NAME-toggle"

echo "==> ensuring data dir $DATA_DIR"
mkdir -p "$DATA_DIR"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "WARN: $BIN_DIR is not in \$PATH — add it to your shell profile" ;;
esac

cat <<MSG

Done. Installed:
  binary: $BIN_DIR/$BIN_NAME
  toggle: $BIN_DIR/$BIN_NAME-toggle   (scratchpad show/hide)

Hyprland config — add to ~/.config/hypr/hyprland.conf:

  # questlog: live project map scratchpad (SUPER+ALT+Q)
  bind = SUPER ALT, Q, exec, $BIN_DIR/$BIN_NAME-toggle
  windowrule = float on,                  match:class questlog
  windowrule = center on,                 match:class questlog
  windowrule = size 1800 1000,            match:class questlog
  windowrule = workspace special:questlog, match:class questlog

Claude Code — make every session report progress:

  ln -sf $REPO_DIR/docs/QUESTLOG-CLAUDE.md ~/.claude/QUESTLOG.md
  # then add "@QUESTLOG.md" as the first line of ~/.claude/CLAUDE.md
  # and a SessionStart hook running: $BIN_DIR/$BIN_NAME brief
MSG
