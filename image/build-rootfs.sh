#!/usr/bin/env bash
set -euo pipefail

ROOTFS_SIZE_MB="${ROOTFS_SIZE_MB:-300}"
ROOTFS_IMG="${ROOTFS_IMG:-rootfs.ext4}"
ALPINE_VERSION="${ALPINE_VERSION:-3.20}"
ALPINE_MIRROR="${ALPINE_MIRROR:-https://dl-cdn.alpinelinux.org/alpine}"
ARCH="${ARCH:-x86_64}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MITM_CA_CERT="${MITM_CA_CERT:-$REPO_ROOT/.vetty-mitmproxy/mitmproxy-ca-cert.pem}"
MITM_CONFDIR="$(dirname "$MITM_CA_CERT")"

AGENT_BIN="${AGENT_BIN:-$REPO_ROOT/target/x86_64-unknown-linux-musl/release/vetty-agent}"
GUEST_DIR="${GUEST_DIR:-$REPO_ROOT/guest}"
OUT_PATH="$SCRIPT_DIR/$ROOTFS_IMG"

if [[ ! -f "$AGENT_BIN" ]]; then
  echo "ERROR: vetty-agent binary not found at $AGENT_BIN"
  echo "Build it first:"
  echo "  cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent"
  exit 1
fi

if [[ ! -f "$GUEST_DIR/init.sh" || ! -f "$GUEST_DIR/vetty-run.sh" ]]; then
  echo "ERROR: missing guest scripts in $GUEST_DIR"
  exit 1
fi

if [[ ! -f "$MITM_CA_CERT" ]]; then
  if command -v mitmdump >/dev/null 2>&1; then
    mkdir -p "$MITM_CONFDIR"
    # mitmproxy creates its CA files before binding the listen socket.
    timeout 3 mitmdump --set "confdir=$MITM_CONFDIR" --listen-host 127.0.0.1 --listen-port 0 >/dev/null 2>&1 || true
  fi
fi

echo "=== Building rootfs at $OUT_PATH ==="
dd if=/dev/zero of="$OUT_PATH" bs=1M count="$ROOTFS_SIZE_MB" status=none
mkfs.ext4 -F "$OUT_PATH" >/dev/null

MOUNT_DIR="$(mktemp -d)"
cleanup() {
  sudo umount "$MOUNT_DIR" >/dev/null 2>&1 || true
  rmdir "$MOUNT_DIR" >/dev/null 2>&1 || true
}
trap cleanup EXIT

sudo mount -o loop "$OUT_PATH" "$MOUNT_DIR"

ALPINE_TAR="alpine-minirootfs-${ALPINE_VERSION}.0-${ARCH}.tar.gz"
ALPINE_URL="${ALPINE_MIRROR}/v${ALPINE_VERSION}/releases/${ARCH}/${ALPINE_TAR}"
if [[ ! -f "$SCRIPT_DIR/$ALPINE_TAR" ]]; then
  curl -fsSL "$ALPINE_URL" -o "$SCRIPT_DIR/$ALPINE_TAR"
fi

sudo tar xzf "$SCRIPT_DIR/$ALPINE_TAR" -C "$MOUNT_DIR"
sudo cp /etc/resolv.conf "$MOUNT_DIR/etc/resolv.conf"

sudo chroot "$MOUNT_DIR" /bin/sh -c "apk add --no-cache bash strace curl iproute2 ca-certificates"

if [[ -f "$MITM_CA_CERT" ]]; then
  sudo install -D -m 0644 "$MITM_CA_CERT" "$MOUNT_DIR/usr/local/share/ca-certificates/vetty-proxy-ca.crt"
  sudo install -D -m 0644 "$MITM_CA_CERT" "$MOUNT_DIR/etc/ssl/certs/vetty-proxy-ca.pem"
  sudo chroot "$MOUNT_DIR" /bin/sh -c "update-ca-certificates"
else
  echo "WARNING: missing mitmproxy CA certificate at $MITM_CA_CERT"
  echo "HTTPS interception will fail until the certificate is present and the rootfs is rebuilt."
fi

sudo mkdir -p "$MOUNT_DIR/opt/vetty" "$MOUNT_DIR/usr/local/bin"
sudo cp "$AGENT_BIN" "$MOUNT_DIR/opt/vetty/vetty-agent"
sudo chmod +x "$MOUNT_DIR/opt/vetty/vetty-agent"
sudo cp "$GUEST_DIR/vetty-run.sh" "$MOUNT_DIR/usr/local/bin/vetty-run"
sudo chmod +x "$MOUNT_DIR/usr/local/bin/vetty-run"
sudo cp "$GUEST_DIR/init.sh" "$MOUNT_DIR/opt/vetty/init.sh"
sudo chmod +x "$MOUNT_DIR/opt/vetty/init.sh"
if [[ -d "$GUEST_DIR/overrides" ]]; then
  sudo mkdir -p "$MOUNT_DIR/opt/vetty/overrides"
  sudo cp -a "$GUEST_DIR/overrides/." "$MOUNT_DIR/opt/vetty/overrides/"
fi

sudo bash -c "cat > '$MOUNT_DIR/etc/inittab' << 'EOF'
::sysinit:/bin/mount -t proc proc /proc
::sysinit:/bin/mount -t sysfs sys /sys
::sysinit:/bin/mount -t devtmpfs dev /dev
ttyS0::respawn:/opt/vetty/init.sh
EOF"

sudo umount "$MOUNT_DIR"

if [[ -n "${SUDO_UID:-}" && -n "${SUDO_GID:-}" ]]; then
  sudo chown "$SUDO_UID:$SUDO_GID" "$OUT_PATH"
fi
chmod u+rw "$OUT_PATH"

echo "=== rootfs image ready: $OUT_PATH ==="
