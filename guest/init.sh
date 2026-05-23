#!/usr/bin/env bash
set -euo pipefail

mkdir -p /sandbox
mountpoint -q /sandbox || mount /dev/vdb /sandbox

export VETTY_SANDBOX_ID
VETTY_SANDBOX_ID="$(cat /proc/sys/kernel/random/uuid)"

FIFO="/tmp/vetty-strace.fifo"
rm -f "$FIFO"
mkfifo "$FIFO"

# Open the FIFO read/write in the init process so the agent can start
# immediately and stay alive even before the first traced command runs.
exec 3<> "$FIFO"

# Keep one vetty-agent process alive for the lifetime of the sandbox.
# It registers the sandbox with the host daemon immediately, then waits
# for strace output to arrive on the FIFO.
/opt/vetty/vetty-agent < "$FIFO" &
AGENT_PID=$!

cleanup() {
  kill "$AGENT_PID" 2>/dev/null || true
  rm -f "$FIFO"
}
trap cleanup EXIT

export PATH="/usr/local/bin:$PATH"

echo "============================================"
echo "  Vetty Sandbox Ready"
echo "  Sandbox ID: $VETTY_SANDBOX_ID"
echo "  Code mounted at: /sandbox"
echo "  Use 'vetty-run <command>' to trace execution"
echo "============================================"

# Setup internet access in the guest
# Wait for eth0 to appear
for i in $(seq 1 10); do
  if ip link show dev eth0 >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

ip addr show eth0 | grep -q 172.16.0.2 || ip addr add 172.16.0.2/30 dev eth0
ip link set dev eth0 up
ip route show default | grep -q default || ip route add default via 172.16.0.1 dev eth0

# Setup DNS resolution in the guest
echo 'nameserver 8.8.8.8' > /etc/resolv.conf

# Configure HTTP(S) interception for guest processes.
export VETTY_PROXY_HOST="${VETTY_PROXY_HOST:-172.16.0.1}"
export VETTY_PROXY_PORT="${VETTY_PROXY_PORT:-8899}"
export HTTP_PROXY="http://${VETTY_SANDBOX_ID}:vetty@${VETTY_PROXY_HOST}:${VETTY_PROXY_PORT}"
export HTTPS_PROXY="$HTTP_PROXY"
export http_proxy="$HTTP_PROXY"
export https_proxy="$HTTPS_PROXY"
export ALL_PROXY="$HTTP_PROXY"
export SSL_CERT_FILE="${SSL_CERT_FILE:-/etc/ssl/certs/vetty-proxy-ca.pem}"
export SSL_CERT_DIR="${SSL_CERT_DIR:-/etc/ssl/certs}"
export CURL_CA_BUNDLE="$SSL_CERT_FILE"
export REQUESTS_CA_BUNDLE="$SSL_CERT_FILE"
export PYTHONPATH="/opt/vetty/overrides/pythonpath${PYTHONPATH:+:$PYTHONPATH}"
export RUBYLIB="/opt/vetty/overrides/gems${RUBYLIB:+:$RUBYLIB}"
export PHP_INI_SCAN_DIR="/opt/vetty/overrides/php${PHP_INI_SCAN_DIR:+:$PHP_INI_SCAN_DIR}"

exec /bin/sh
