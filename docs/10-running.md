# Running Vetty

This project has 3 runtime components:

1. `vetty-daemon` (host API + event stream)
2. GUI (`gui/`, optional)
3. `vetty` CLI (launches the Firecracker VM)

## Prerequisites

- Linux host with `/dev/kvm` access
- `firecracker` available on `PATH`
- Rust toolchain
- `mkfs.ext4` and `sudo` (for image/rootfs workflows)
- Node.js + npm (only if you use the GUI)

## Build

```bash
# host-side crates
cargo build

# guest agent (static)
rustup target add x86_64-unknown-linux-musl
cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent
```

## Prepare rootfs and kernel

```bash
cd image
./download-kernel.sh
./download-rootfs.sh
# or: sudo ./build-rootfs.sh
cd ..
```

## Run the full system

Terminal 1:

```bash
cargo run -p vetty-daemon
```

Terminal 2 (optional GUI):

```bash
cd gui
npm install
npm run electron:dev
```

Terminal 3 (launch VM sandbox):

```bash
cargo run -p vetty-cli -- \
  --dir ./sample-code \
  --rootfs ./image/rootfs.ext4 \
  --kernel ./image/vmlinux
```

## If Firecracker API setup fails

If startup fails while configuring the VM, Vetty now includes Firecracker process status and stderr tail in the error output. This is usually caused by invalid kernel/rootfs paths, incompatible artifacts, or missing KVM permissions.
