#!/usr/bin/env bash
# Launch the ADAM ОТ/ТБ demo: start the portal and open all four pages in
# the browser — pitch, general dialog, worker portal, ИТР dashboard.
#
#   scripts/demo.sh
#
# The portal serves every page (the worker/ИТР pages need the server for
# /api/*), so all four open as http://127.0.0.1:8787/… tabs.  Ctrl+C stops.
set -euo pipefail

cd "$(dirname "$0")/.."
ADDR="127.0.0.1:8787"

echo "==> building adam_portal (release)…"
cargo build --release -p adam-portal --bin adam_portal

echo "==> starting portal on http://$ADDR"
./target/release/adam_portal &
PORTAL_PID=$!
trap 'echo; echo "==> stopping portal ($PORTAL_PID)"; kill "$PORTAL_PID" 2>/dev/null || true' EXIT INT TERM

# wait for the port to accept connections
for _ in $(seq 1 40); do
  if curl -s -o /dev/null "http://$ADDR/"; then break; fi
  sleep 0.25
done

# pick a browser opener
if command -v open >/dev/null 2>&1; then OPEN=open        # macOS
elif command -v xdg-open >/dev/null 2>&1; then OPEN=xdg-open # Linux
else OPEN=""; fi

URLS=(
  "http://$ADDR/pitch"    # 1. презентация (ERG/ССГПО)
  "http://$ADDR/dialog"   # 2. общий диалог (движок ADAM)
  "http://$ADDR/"         # 3. кабинет работника (голос + слайды + камера)
  "http://$ADDR/itr"      # 4. дашборд ИТР (журнал допусков)
)

if [ -n "$OPEN" ]; then
  echo "==> opening 4 pages in the browser…"
  "$OPEN" "${URLS[@]}"
else
  echo "==> open these in your browser:"; printf '   %s\n' "${URLS[@]}"
fi

echo "==> demo running. Press Ctrl+C to stop."
wait "$PORTAL_PID"
