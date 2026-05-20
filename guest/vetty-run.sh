#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "Usage: vetty-run <command> [args...]"
  exit 1
fi

FIFO="/tmp/vetty-strace.fifo"

if [ ! -p "$FIFO" ]; then
  echo "vetty-agent FIFO is not ready: $FIFO" >&2
  exit 1
fi

export VETTY_PROXY_HOST="${VETTY_PROXY_HOST:-172.16.0.1}"
export VETTY_PROXY_PORT="${VETTY_PROXY_PORT:-8899}"
export HTTP_PROXY="http://${VETTY_SANDBOX_ID}:vetty@${VETTY_PROXY_HOST}:${VETTY_PROXY_PORT}"
export HTTPS_PROXY="$HTTP_PROXY"
export http_proxy="$HTTP_PROXY"
export https_proxy="$HTTPS_PROXY"
export ALL_PROXY="$HTTP_PROXY"
export SSL_CERT_FILE="${SSL_CERT_FILE:-/etc/ssl/certs/ca-certificates.crt}"
export SSL_CERT_DIR="${SSL_CERT_DIR:-/etc/ssl/certs}"
export CURL_CA_BUNDLE="$SSL_CERT_FILE"
export REQUESTS_CA_BUNDLE="$SSL_CERT_FILE"

strace -f -tt -T \
  -s 8192 \
  -e trace=file,network,process,write,sendto,recvfrom,sendmsg,recvmsg \
  -o "$FIFO" \
  -- "$@"
