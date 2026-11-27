#!/usr/bin/env bash
set -euo pipefail

HERMES_HOME="${HERMES_HOME:-$HOME/.hermes}"
WEBUI_EXT_ROOT="${HERMES_WEBUI_EXTENSION_DIR:-$HERMES_HOME/webui/extensions}"

if command -v systemctl >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
  systemctl disable --now hermes-teamsd.service 2>/dev/null || true
  rm -f /etc/systemd/system/hermes-teamsd.service
  systemctl daemon-reload
fi

rm -f /usr/local/bin/hermes-teamsd
rm -rf "$HERMES_HOME/plugins/hermes-agent-teams"
rm -rf "$WEBUI_EXT_ROOT/hermes-agent-teams"

echo "Uninstalled hermes-agent-teams cleanly (team archives under <workspace>/.agent-teams/ were preserved)."
