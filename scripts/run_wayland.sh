#!/usr/bin/env bash
set -euo pipefail

EXAMPLE="${1:-widget_gallery}"

# If a Wayland display is already available, just use it.
if [ -z "${WAYLAND_DISPLAY:-}" ]; then
  echo "WAYLAND_DISPLAY is not set. Start your Wayland session and re-run (e.g., WAYLAND_DISPLAY=wayland-0 ./scripts/run_wayland.sh file_manager_demo)." >&2
  exit 1
fi

cargo run -p erigui-examples --example "$EXAMPLE"
