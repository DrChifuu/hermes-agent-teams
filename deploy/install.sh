#!/usr/bin/env bash
set -euo pipefail

# Idempotent installer for Hermes Agent Teams (Rust daemon + Hermes Agent plugin + Hermes WebUI extension).
# Leaves upstream hermes-agent and hermes-webui git repositories 100% untouched (zero patches!).

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HERMES_HOME="${HERMES_HOME:-$HOME/.hermes}"
PLUGIN_DST="$HERMES_HOME/plugins/hermes-agent-teams"
WEBUI_EXT_ROOT="${HERMES_WEBUI_EXTENSION_DIR:-$HERMES_HOME/webui/extensions}"
WEBUI_EXT_DST="$WEBUI_EXT_ROOT/hermes-agent-teams"
INSTALL_BIN="${INSTALL_BIN:-/usr/local/bin/hermes-teamsd}"
BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"

echo "==> [1/4] Building hermes-teamsd release binary (CARGO_BUILD_JOBS=$BUILD_JOBS)..."
(cd "$REPO_DIR" && CARGO_BUILD_JOBS="$BUILD_JOBS" cargo build --release --bin hermes-teamsd)

echo "==> [2/4] Installing binary to $INSTALL_BIN..."
install -m 0755 "$REPO_DIR/target/release/hermes-teamsd" "$INSTALL_BIN"

echo "==> [3/4] Installing native Hermes Agent plugin & /agent-teams skill to $PLUGIN_DST..."
mkdir -p "$HERMES_HOME/plugins" "$HERMES_HOME/skills/agent-teams"
rm -rf "$PLUGIN_DST"
cp -a "$REPO_DIR/hermes-plugin" "$PLUGIN_DST"
cp -a "$REPO_DIR/hermes-plugin/skills/agent-teams/SKILL.md" "$HERMES_HOME/skills/agent-teams/SKILL.md"
if command -v hermes >/dev/null 2>&1; then
  hermes plugins enable hermes-agent-teams || true
fi

# Ensure Hermes allows hermes-agent-teams to wake the Captain via inject_message
CFG_YAML="$HERMES_HOME/config.yaml"
if [ -f "$CFG_YAML" ]; then
  python3 - "$CFG_YAML" <<'PY'
import sys
from pathlib import Path

p = Path(sys.argv[1])
txt = p.read_text(encoding="utf-8")
if "allow_gateway_injection" not in txt and "entries:" in txt:
    out = []
    inserted = False
    for line in txt.splitlines(keepends=True):
        out.append(line)
        if not inserted and line.strip() == "entries:":
            indent = line[:len(line) - len(line.lstrip())]
            out.append(f"{indent}  hermes-agent-teams:\n")
            out.append(f"{indent}    allow_gateway_injection: true\n")
            inserted = True
    if inserted:
        p.write_text("".join(out), encoding="utf-8")
PY
fi

echo "==> [4/4] Installing official Hermes WebUI extension to $WEBUI_EXT_DST..."
mkdir -p "$WEBUI_EXT_ROOT"
rm -rf "$WEBUI_EXT_DST"
cp -a "$REPO_DIR/webui-extension" "$WEBUI_EXT_DST"

# Register installed extension in Hermes WebUI gallery manifest so it loads automatically with zero env vars
INSTALL_MANIFEST="$HERMES_HOME/webui/extension-install-manifest.json"
python3 - "$INSTALL_MANIFEST" "$WEBUI_EXT_DST" <<'PY'
import datetime, json, sys
from pathlib import Path

p = Path(sys.argv[1])
ext_dir = Path(sys.argv[2])
p.parent.mkdir(parents=True, exist_ok=True)
data = {"version": 1, "installed": {}}
if p.is_file():
    try:
        loaded = json.loads(p.read_text(encoding="utf-8"))
        if isinstance(loaded, dict):
            data = loaded
    except Exception:
        pass
files = sorted(
    str(f.relative_to(ext_dir)).replace("\\", "/")
    for f in ext_dir.rglob("*")
    if f.is_file()
)
installed = data.setdefault("installed", {})
installed["hermes-agent-teams"] = {
    "version": "0.1.0",
    "files": files,
    "installed_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
}
p.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY

# Pre-approve loopback sidecar consent in extension-overrides.json so the UI proxy works right away
OVERRIDES="$HERMES_HOME/webui/extension-overrides.json"
python3 - "$OVERRIDES" <<'PY'
import json, sys
from pathlib import Path

p = Path(sys.argv[1])
p.parent.mkdir(parents=True, exist_ok=True)
data = {"version": 1, "disabled_extensions": [], "sidecar_proxy_consents": {}}
if p.is_file():
    try:
        loaded = json.loads(p.read_text(encoding="utf-8"))
        if isinstance(loaded, dict):
            data = loaded
    except Exception:
        pass
consents = data.setdefault("sidecar_proxy_consents", {})
consents["hermes-agent-teams"] = "http://127.0.0.1:17890"
disabled = data.setdefault("disabled_extensions", [])
if "hermes-agent-teams" in disabled:
    disabled.remove("hermes-agent-teams")
p.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY

if command -v systemctl >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
  echo "==> Installing and starting systemd service hermes-teamsd.service..."
  install -m 0644 "$REPO_DIR/deploy/hermes-teamsd.service" /etc/systemd/system/hermes-teamsd.service
  systemctl daemon-reload
  systemctl enable hermes-teamsd.service
  systemctl restart hermes-teamsd.service
  systemctl status --no-pager hermes-teamsd.service | head -n 15 || true
else
  echo "==> Non-root or no systemd detected. Start the daemon manually with:"
  echo "    $INSTALL_BIN"
fi

echo ""
echo "Done! Verify health with: curl -s http://127.0.0.1:17890/health"
echo "Then reload Hermes WebUI in your browser (or restart hermes-webui.service if desired)."
