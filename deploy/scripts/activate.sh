#!/usr/bin/env bash
# Runs ON THE VPS, piped over SSH by the Deploy/Rollback workflows:
#   ssh vps 'bash -s -- activate <release|previous>' < deploy/scripts/activate.sh
#   ssh vps 'bash -s -- list' < deploy/scripts/activate.sh
# It is never copied to the server.
#
# Layout (each release in its own folder, `current` symlinks switched atomically):
#   $API_ROOT/releases/<release>/server     $API_ROOT/current -> releases/<release>
#   $WEB_ROOT/releases/<release>/index.html $WEB_ROOT/current -> releases/<release>
#
# The API is switched and health-checked first; the web app only switches once the API is
# healthy. If the health check fails, the previous API release is restored.
set -euo pipefail

CMD="${1:?usage: activate.sh activate <release|previous> | list}"
API_ROOT="${API_ROOT:-/opt/havoc}"
WEB_ROOT="${WEB_ROOT:-/var/www/havoc}"
SERVICE="${SERVICE:-havoc-api}"
HEALTH_URL="${HEALTH_URL:-http://127.0.0.1:8080/api/health}"
KEEP="${KEEP:-5}"

current_of() { basename "$(readlink -f "$1/current" 2>/dev/null)" 2>/dev/null || true; }

releases_of() { ls -1 "$1/releases" 2>/dev/null | sort; }

# Atomic symlink swap: readers see either the old or the new target, never nothing.
switch_link() { ln -sfn "$1" "$2.new" && mv -Tf "$2.new" "$2"; }

fail() { echo "::error::$*" >&2; exit 1; }

if [ "$CMD" = list ]; then
  live="$(current_of "$API_ROOT")"
  echo "Releases on $(hostname) (newest last):"
  for r in $(releases_of "$API_ROOT"); do
    [ "$r" = "$live" ] && echo "  $r   <- live" || echo "  $r"
  done
  exit 0
fi

[ "$CMD" = activate ] || fail "unknown command: $CMD"
RELEASE="${2:?release name or 'previous' required}"

if [ "$RELEASE" = previous ]; then
  live="$(current_of "$API_ROOT")"
  RELEASE="$(releases_of "$API_ROOT" | awk -v live="$live" '$0 < live' | tail -n 1)"
  [ -n "$RELEASE" ] || fail "no release older than the live one ($live)"
fi

api_dir="$API_ROOT/releases/$RELEASE"
web_dir="$WEB_ROOT/releases/$RELEASE"
[ -x "$api_dir/server" ] || fail "missing or non-executable $api_dir/server"
[ -f "$web_dir/index.html" ] || fail "missing $web_dir/index.html"

prev_api="$(readlink -f "$API_ROOT/current" 2>/dev/null || true)"

echo "Activating API $RELEASE"
switch_link "$api_dir" "$API_ROOT/current"
sudo -n systemctl restart "$SERVICE"

healthy=""
for _ in $(seq 1 30); do
  if curl -fs -o /dev/null --max-time 2 "$HEALTH_URL"; then healthy=1; break; fi
  sleep 1
done

if [ -z "$healthy" ]; then
  echo "::error::API did not pass $HEALTH_URL within 30s — rolling back" >&2
  sudo -n journalctl -u "$SERVICE" -n 60 --no-pager || true
  if [ -n "$prev_api" ] && [ "$prev_api" != "$api_dir" ]; then
    switch_link "$prev_api" "$API_ROOT/current"
    sudo -n systemctl restart "$SERVICE"
    echo "Restored API $(basename "$prev_api")" >&2
  fi
  exit 1
fi
echo "API healthy"

echo "Activating web $RELEASE"
switch_link "$web_dir" "$WEB_ROOT/current"

# Keep the newest $KEEP releases (and whatever is live), for rollbacks.
prune() {
  local root="$1" live
  live="$(current_of "$root")"
  releases_of "$root" | head -n -"$KEEP" | while read -r r; do
    [ -n "$r" ] && [ "$r" != "$live" ] && rm -rf -- "${root:?}/releases/$r"
  done
}
prune "$API_ROOT"
prune "$WEB_ROOT"

echo "Live: $RELEASE"
