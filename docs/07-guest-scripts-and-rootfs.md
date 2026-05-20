# Step 7 — Guest Scripts & Root Filesystem

## Goal
Create the bash scripts that run inside the guest VM and the build script that assembles the rootfs image.

---

## 7.1 Guest Init Script — `guest/init.sh`

This script runs on VM boot (via `/etc/rc.local` or as PID 1 in a minimal init). It:

1. Mounts the second drive at `/sandbox`
2. Launches `vetty-agent` in the background
3. Drops the user into a shell

```bash
#!/bin/bash
set -e

# Mount the code disk (second virtio block device)
# In Firecracker, drives are /dev/vda (rootfs), /dev/vdb (code disk)
mkdir -p /sandbox
mount /dev/vdb /sandbox

# Generate a sandbox ID and export it for the agent
export VETTY_SANDBOX_ID=$(cat /proc/sys/kernel/random/uuid)

# Start the agent in the background
# The agent connects to the host over vsock and waits for events
/opt/vetty/vetty-agent &
AGENT_PID=$!

echo "============================================"
echo "  Vetty Sandbox Ready"
echo "  Sandbox ID: $VETTY_SANDBOX_ID"
echo "  Code mounted at: /sandbox"
echo "  Use 'vetty-run <command>' to trace execution"
echo "============================================"

# Drop into an interactive shell
# When using serial console, /bin/sh on ttyS0
exec /bin/sh
```

---

## 7.2 vetty-run Wrapper — `guest/vetty-run.sh`

This is the script users call inside the sandbox to run a program under monitoring.

```bash
#!/bin/bash
set -e

if [ $# -eq 0 ]; then
    echo "Usage: vetty-run <command> [args...]"
    echo "Runs a command under strace and pipes events to vetty-agent"
    exit 1
fi

# Set up proxy environment (for future HTTP interception)
# export HTTP_PROXY="http://127.0.0.1:8080"
# export HTTPS_PROXY="http://127.0.0.1:8080"

# Run the target command under strace
# -f          : follow forks
# -tt         : timestamps with microseconds
# -T          : show time spent in each syscall
# -e trace=   : only trace file, network, and process syscalls
# -o          : pipe output to vetty-agent via process substitution
#
# We pipe strace output to vetty-agent's stdin using a FIFO
FIFO="/tmp/vetty-strace-$$"
mkfifo "$FIFO"

# Feed the FIFO to vetty-agent in the background
/opt/vetty/vetty-agent < "$FIFO" &
AGENT_PID=$!

# Run strace, writing output to the FIFO
strace -f -tt -T \
    -e trace=file,network,process \
    -o "$FIFO" \
    -- "$@"

# Wait for agent to finish processing
wait $AGENT_PID 2>/dev/null

# Cleanup
rm -f "$FIFO"
```

---

## 7.3 Root Filesystem Build Script — `image/build-rootfs.sh`

This script builds a minimal Alpine Linux rootfs with all needed tools baked in.

```bash
#!/bin/bash
#
# Build a minimal Alpine Linux rootfs for Vetty
# Requires: root/sudo, qemu-img, e2fsprogs, wget
#
set -euo pipefail

ROOTFS_SIZE_MB=200
ROOTFS_IMG="rootfs.ext4"
ALPINE_VERSION="3.20"
ALPINE_MIRROR="https://dl-cdn.alpinelinux.org/alpine"
ARCH="x86_64"

AGENT_BIN="../target/x86_64-unknown-linux-musl/release/vetty-agent"
GUEST_SCRIPTS="../guest"

# ── Check prerequisites ──────────────────────────────────────────
if [ ! -f "$AGENT_BIN" ]; then
    echo "ERROR: vetty-agent binary not found at $AGENT_BIN"
    echo "Build it first: cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent"
    exit 1
fi

echo "=== Building Vetty rootfs ==="

# ── Create blank image ───────────────────────────────────────────
echo "[1/5] Creating ${ROOTFS_SIZE_MB}MB image..."
dd if=/dev/zero of="$ROOTFS_IMG" bs=1M count=$ROOTFS_SIZE_MB
mkfs.ext4 -F "$ROOTFS_IMG"

# ── Mount and populate ───────────────────────────────────────────
MOUNT_DIR=$(mktemp -d)
sudo mount -o loop "$ROOTFS_IMG" "$MOUNT_DIR"

# Cleanup trap
cleanup() {
    sudo umount "$MOUNT_DIR" 2>/dev/null || true
    rmdir "$MOUNT_DIR" 2>/dev/null || true
}
trap cleanup EXIT

echo "[2/5] Installing Alpine Linux base..."
# Download and extract Alpine minirootfs
ALPINE_TAR="alpine-minirootfs-${ALPINE_VERSION}.0-${ARCH}.tar.gz"
ALPINE_URL="${ALPINE_MIRROR}/v${ALPINE_VERSION}/releases/${ARCH}/${ALPINE_TAR}"
if [ ! -f "$ALPINE_TAR" ]; then
    wget "$ALPINE_URL"
fi
sudo tar xzf "$ALPINE_TAR" -C "$MOUNT_DIR"

# Configure DNS
sudo cp /etc/resolv.conf "$MOUNT_DIR/etc/resolv.conf"

# Install required packages via chroot
echo "[3/5] Installing packages (strace, curl, bash)..."
sudo chroot "$MOUNT_DIR" /bin/sh -c "
    apk add --no-cache bash strace curl net-tools iproute2
"

echo "[4/5] Installing vetty components..."
# Install vetty-agent
sudo mkdir -p "$MOUNT_DIR/opt/vetty"
sudo cp "$AGENT_BIN" "$MOUNT_DIR/opt/vetty/vetty-agent"
sudo chmod +x "$MOUNT_DIR/opt/vetty/vetty-agent"

# Install vetty-run
sudo cp "$GUEST_SCRIPTS/vetty-run.sh" "$MOUNT_DIR/usr/local/bin/vetty-run"
sudo chmod +x "$MOUNT_DIR/usr/local/bin/vetty-run"

# Install init script
sudo cp "$GUEST_SCRIPTS/init.sh" "$MOUNT_DIR/opt/vetty/init.sh"
sudo chmod +x "$MOUNT_DIR/opt/vetty/init.sh"

# Configure auto-start via /etc/inittab (Alpine uses BusyBox init)
echo "[5/5] Configuring boot..."
sudo bash -c "cat > '$MOUNT_DIR/etc/inittab' << 'INITTAB'
::sysinit:/bin/mount -t proc proc /proc
::sysinit:/bin/mount -t sysfs sys /sys
::sysinit:/bin/mount -t devtmpfs dev /dev
::sysinit:/sbin/ifconfig lo up
ttyS0::respawn:/opt/vetty/init.sh
INITTAB"

echo "=== Rootfs built successfully: $ROOTFS_IMG ==="
echo "Size: $(du -h $ROOTFS_IMG | cut -f1)"
```

---

## 7.4 Kernel

You need a compatible Linux kernel binary (`vmlinux`) for Firecracker. Options:

1. **Use Firecracker's pre-built kernel** — download from the Firecracker releases page
2. **Build your own** — use the kernel config from `firecracker/resources/guest_configs/`

```bash
# Download pre-built kernel from Firecracker
KERNEL_VERSION="5.10"
wget "https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/v1.10/${ARCH}/vmlinux-${KERNEL_VERSION}.bin" \
    -O vmlinux
```

---

## 7.5 Guest Networking (Optional for MVP)

For the MVP, networking inside the guest is **not required** — the agent communicates via vsock, not TCP/IP. If you need network access later (e.g., for the untrusted code to make HTTP requests through a proxy):

1. Configure a TAP device on the host
2. Add a network interface to the Firecracker config
3. Set up NAT/masquerading on the host

This is deferred to post-MVP.

---

## Done Criteria

- [ ] `guest/init.sh` exists and mounts `/dev/vdb`, starts agent, drops to shell
- [ ] `guest/vetty-run.sh` exists and runs commands under strace piped to agent
- [ ] `image/build-rootfs.sh` exists and produces a working `rootfs.ext4`
- [ ] The rootfs contains: bash, strace, curl, vetty-agent, vetty-run, init.sh
- [ ] Boot sequence: init → mount code disk → start agent → shell on ttyS0
