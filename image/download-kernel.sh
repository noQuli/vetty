#!/usr/bin/env bash
set -euo pipefail

OUT="${OUT:-vmlinux}"
URL="${URL:-https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/v1.10/x86_64/vmlinux-6.1.102}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Downloading ${URL}"
curl -fsSL "$URL" -o "$SCRIPT_DIR/$OUT"
echo "Saved to $SCRIPT_DIR/$OUT"
