#!/usr/bin/env bash
set -euo pipefail

OUT="${OUT:-rootfs.ext4}"
URL="${URL:-https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/x86_64/rootfs/bionic.rootfs.ext4}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Downloading ${URL}"
curl -fsSL "$URL" -o "$SCRIPT_DIR/$OUT"
echo "Saved to $SCRIPT_DIR/$OUT"
