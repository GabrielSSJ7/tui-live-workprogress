#!/usr/bin/env bash
set -euo pipefail

# questlog scratchpad toggle (Hyprland).
#
# Show/hide the persistent questlog window WITHOUT killing it, so it keeps
# watching the data dir and keeps its selection/zoom between peeks. The process
# only dies when you quit with `q`.
#
# First invocation launches questlog into the `questlog` special workspace and
# reveals it; every later invocation toggles that workspace's visibility.
#
# Requires the companion window rule so the window lands in the scratchpad:
#   windowrule = workspace special:questlog, match:class questlog

BIN="${QUESTLOG_BIN:-$HOME/.local/bin/questlog}"
WORKSPACE="questlog"
CLASS="questlog"

running() { hyprctl clients | grep -q "class: ${CLASS}$"; }
shown()   { hyprctl monitors | grep -q "special workspace:.*(special:${WORKSPACE})"; }

if running; then
    hyprctl dispatch togglespecialworkspace "$WORKSPACE"
    exit 0
fi

# Not running: launch it, wait for the window, then reveal the scratchpad if
# the window rule dropped it into a hidden special workspace.
hyprctl dispatch exec "kitty --class ${CLASS} -o remember_window_size=no -e ${BIN}"
for _ in $(seq 1 50); do
    running && break
    sleep 0.1
done
shown || hyprctl dispatch togglespecialworkspace "$WORKSPACE"
