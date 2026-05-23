<div align="center">

# 🛡️ Vetty

**Run untrusted code safely. See everything it does.**

[![CI](https://github.com/noQuli/vetty/actions/workflows/ci.yml/badge.svg)](https://github.com/noQuli/vetty/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Firecracker](https://img.shields.io/badge/firecracker-v1.x-ff9900.svg)](https://github.com/firecracker-microvm/firecracker)

Vetty is a security sandbox that runs untrusted code inside **Firecracker micro-VMs** while monitoring all syscalls, file access, network activity, and HTTP traffic in real time. A host-side daemon collects events from the guest agent and streams them to an **Electron + React GUI**.

[Quick Start](#-quick-start) •
[Documentation](#-documentation) •
[Contributing](#contributing)

</div>

---

## ✨ Features

- **Hardware-level isolation** — Code runs inside Firecracker micro-VMs with KVM, not containers
- **Real-time syscall monitoring** — Every `open`, `connect`, `exec`, `write` is captured via `strace` and streamed live
- **HTTP/HTTPS interception** — Full request/response inspection via mitmproxy, including TLS traffic
- **Desktop GUI** — Electron + React dashboard with live event timeline, filtering, and detail inspection
- **Single command launch** — One `make run` starts the daemon, GUI, and VM together
- **Minimal footprint** — Micro-VMs boot in under a second with ~128 MB memory

---

## 📋 Prerequisites

| Requirement | Details |
|---|---|
| **OS** | Linux (x86_64) with KVM enabled |
| **KVM** | `/dev/kvm` must exist and be writable by your user |
| **Firecracker** | `firecracker` binary on `PATH` ([install guide](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md)) |
| **Rust** | 1.75+ with `x86_64-unknown-linux-musl` target |
| **Node.js** | 18+ with npm |
| **System packages** | `e2fsprogs` (`mkfs.ext4`), `curl`, `sudo` |
| **Python** | 3.8+ (for mitmproxy HTTPS interception, optional) |
| **mitmproxy** | Optional, for HTTPS traffic inspection |

<details>
<summary><strong>Quick dependency install (Debian/Ubuntu)</strong></summary>

```bash
# System packages
sudo apt update && sudo apt install -y e2fsprogs curl qemu-system-x86

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-unknown-linux-musl

# Node.js (via nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
nvm install 20

# Firecracker
ARCH="$(uname -m)"
release_url="https://github.com/firecracker-microvm/firecracker/releases"
latest=$(basename $(curl -fsSLI -o /dev/null -w %{url_effective} ${release_url}/latest))
curl -L ${release_url}/download/${latest}/firecracker-${latest}-${ARCH}.tgz | tar -xz
sudo mv release-${latest}-${ARCH}/firecracker-${latest}-${ARCH} /usr/local/bin/firecracker
```

</details>

---

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/noQuli/vetty.git
cd vetty

# Download VM assets (kernel + rootfs)
make setup

# Build everything and run (daemon + GUI + VM) with the example code
make run DIR=./sample-code
```

The GUI will open automatically, the daemon starts in the background, and a sandbox VM launches with the directory from `DIR` mounted inside the VM at `/sandbox`.

### Where to Put Code

Vetty runs whatever host directory you pass as `DIR`.

For a first run, use the included example:

```bash
make run DIR=./sample-code
```

Inside the VM, that directory appears here:

```text
/sandbox
```

So this host file:

```text
sample-code/my-file.py
```

is available in the VM as:

```text
/sandbox/my-file.py
```

To run your own code, create or choose any directory on the host and pass it to `DIR`:

```bash
mkdir -p ./my-sandbox-code
cp ./some-script.py ./my-sandbox-code/
make run DIR=./my-sandbox-code
```

### Inside the Sandbox (Guest VM)

Once the VM boots, you are dropped into a root shell inside Alpine Linux.
Here are some examples of what you can do:

```bash
# Since it's Alpine Linux, you can install packages
apk add python3 

# Check the mounted code from DIR
ls -la /sandbox

# Run commands with tracing enabled
vetty-run python3 /sandbox/my-file.py
vetty-run curl ifconfig.me
```

Any commands prefixed with `vetty-run` will be monitored, and their syscalls, network events, and file accesses will immediately appear in the desktop GUI!

| Step | What happens |
|---|---|
| `make setup` | Downloads the Firecracker kernel and builds the Alpine rootfs with the agent baked in |
| `make run DIR=./sample-code` | Builds Rust crates, installs GUI deps, packages `DIR` into the VM code disk, then launches daemon → GUI → VM in parallel |

---

## 🏗️ Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  Host Machine                                                │
│                                                              │
│  ┌────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │ vetty CLI  │───▶│ Disk Builder │    │  Electron GUI    │  │
│  │ (Rust)     │    │ (Rust)       │    │  (React + TS)    │  │
│  └──────┬─────┘    └──────────────┘    └────────┬─────────┘  │
│         │                                       │ WebSocket  │
│         ▼                                       ▼            │
│  ┌──────────────┐              ┌──────────────────────────┐  │
│  │  Firecracker │◀── vsock ──▶│  vetty-daemon             │  │
│  │  VM Launcher │             │  - vsock listener         │  │
│  └──────┬───────┘             │  - REST API (:9876)       │  │
│         │                     │  - WebSocket stream       │  │
│         ▼                     │  - mitmproxy integration  │  │
│  ┌─────────────────────┐      └──────────────────────────┘  │
│  │  Firecracker VM     │                                     │
│  │                     │                                     │
│  │  ┌───────────┐      │                                     │
│  │  │ vetty-run │──┐   │                                     │
│  │  │ (wrapper) │  │   │                                     │
│  │  └───────────┘  │   │                                     │
│  │       ┌─────────▼┐  │                                     │
│  │       │  strace  │  │                                     │
│  │       └─────┬────┘  │                                     │
│  │             ▼       │                                     │
│  │  ┌──────────────┐   │                                     │
│  │  │ vetty-agent  │───┼── vsock ──▶ host daemon             │
│  │  │ (Rust)       │   │                                     │
│  │  └──────────────┘   │                                     │
│  │                     │                                     │
│  │  /sandbox (code)    │                                     │
│  └─────────────────────┘                                     │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **CLI** packages your source directory into an ext4 disk image
2. **VM Launcher** boots a Firecracker micro-VM with the rootfs, kernel, and code disk
3. **Guest init** mounts the code disk, starts the agent, and drops to a shell
4. User runs `vetty-run <command>` — wraps execution with `strace`
5. **Agent** parses strace output and streams structured events over vsock to the host
6. **Daemon** ingests events, stores them, and pushes them over WebSocket
7. **GUI** renders events in real-time with filtering, search, and detail inspection

---

## 📁 Project Structure

```
vetty/
├── crates/
│   ├── vetty-common/      # Shared protocol types and event definitions
│   ├── vetty-disk/        # Builds ext4 code images from host directories
│   ├── vetty-agent/       # Guest-side strace parser + vsock sender
│   ├── vetty-vm/          # Firecracker VM launcher and serial relay
│   ├── vetty-daemon/      # Host daemon: vsock + REST API + WebSocket
│   └── vetty-cli/         # CLI entrypoint orchestrating all components
├── guest/
│   ├── init.sh            # Guest boot/init script
│   └── vetty-run.sh       # Wrapper for traced execution
├── image/
│   ├── build-rootfs.sh    # Builds Alpine rootfs with agent baked in
│   ├── download-kernel.sh # Downloads pre-built Firecracker kernel
│   └── download-rootfs.sh # Downloads pre-built rootfs (quickstart)
├── gui/
│   ├── electron/          # Electron main process
│   └── src/               # React + TypeScript frontend
├── scripts/
│   └── vetty-mitmproxy-addon.py  # mitmproxy addon for HTTPS interception
├── docs/                  # Detailed technical documentation
├── sample-code/           # Example code for testing the sandbox
├── Makefile               # Build and run orchestration
└── Cargo.toml             # Rust workspace configuration
```

---

## 🔧 Development

### Manual Build

```bash
# Build all host crates
cargo build

# Build guest agent (static musl binary)
rustup target add x86_64-unknown-linux-musl
cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent

# Install GUI dependencies
cd gui && npm install
```

### Manual Run (separate terminals)

```bash
# Terminal 1: Start daemon
cargo run -p vetty-daemon

# Terminal 2: Start GUI
cd gui && npm run electron:dev

# Terminal 3: Launch sandbox
cargo run -p vetty-cli -- --dir ./sample-code --rootfs ./image/rootfs.ext4 --kernel ./image/vmlinux
```

### Build Targets

| Command | Description |
|---|---|
| `make build` | Build all Rust crates (host + guest agent) |
| `make build-host` | Build host crates only (debug) |
| `make build-agent` | Cross-compile guest agent (musl, release) |
| `make gui-install` | Install GUI npm dependencies |
| `make setup` | Download kernel + build rootfs |
| `make run` | Build and run everything |
| `make clean` | Remove all build artifacts |
| `make lint` | Run clippy and eslint |
| `make test` | Run all tests |

### CLI Options

```
vetty --dir <PATH>        Source directory to sandbox (required)
      --rootfs <PATH>     Path to rootfs image        [default: image/rootfs.ext4]
      --kernel <PATH>     Path to kernel binary        [default: image/vmlinux]
      --memory <MB>       VM memory in MB              [default: 128]
      --cpus <N>          Number of vCPUs              [default: 1]
      --firecracker <P>   Path to firecracker binary   [default: firecracker]
      --no-serial         Don't attach serial console
```

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `VETTY_DAEMON_PORT` | `9876` | Port for the daemon REST/WS API |
| `VETTY_VSOCK_PATH` | `/tmp/vetty_v.sock` | Unix socket path for vsock proxy |
| `VETTY_DAEMON_BIN` | auto-detect | Override path to daemon binary |
| `RUST_LOG` | — | Standard Rust log filtering |

---

## 📖 Documentation

Detailed technical documentation is available in the [`docs/`](docs/) directory:

| Document | Description |
|---|---|
| [Overview](docs/00-overview.md) | Architecture and design overview |
| [Workspace & Common](docs/01-workspace-and-common.md) | Shared types and workspace setup |
| [Disk Builder](docs/02-disk-builder.md) | Code disk image creation |
| [Guest Agent](docs/03-guest-agent.md) | Strace parser and vsock client |
| [VM Launcher](docs/04-vm-launcher.md) | Firecracker API integration |
| [Host Daemon](docs/05-host-daemon.md) | Event ingestion and API server |
| [CLI](docs/06-cli.md) | Command-line interface |
| [Guest Scripts & Rootfs](docs/07-guest-scripts-and-rootfs.md) | Boot scripts and image building |
| [GUI](docs/08-gui.md) | Electron + React frontend |
| [Integration & Testing](docs/09-integration-and-testing.md) | End-to-end testing |
| [Running](docs/10-running.md) | Execution guide |
| [Arguments](docs/11-arguments.md) | CLI arguments reference |
| [HTTPS Interception](docs/12-https-interception.md) | mitmproxy setup |

---

## 🔒 Security

Vetty is designed for analyzing untrusted code. The isolation model relies on:

- **Firecracker micro-VMs** with KVM hardware virtualization
- **Minimal guest rootfs** (Alpine Linux, ~300 MB)
- **No host filesystem access** — code is mounted via a separate ext4 disk image
- **Network through NAT** — all guest traffic routes through the host's tap interface

See [SECURITY.md](SECURITY.md) for our security policy and responsible disclosure process.

> **⚠️ Warning:** Vetty is under active development. While Firecracker provides strong isolation guarantees, the overall system has not been audited. Do not rely on it as a sole security boundary for highly adversarial workloads.

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
