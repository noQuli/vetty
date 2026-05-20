# Vetty — Project Overview & Architecture

## What is Vetty?

Vetty is a security sandbox tool that runs untrusted code inside Firecracker micro-VMs while monitoring all syscalls, file access, network activity, and HTTP traffic in real time. A host-side daemon collects events from the guest agent and streams them to an Electron+React GUI.

## High-Level Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Host Machine                                            │
│                                                          │
│  ┌────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ vetty CLI   │───▶│ Disk Builder │    │  Electron    │  │
│  │ (Rust)      │    │ (Rust)       │    │  GUI (React) │  │
│  └──────┬─────┘    └──────────────┘    └──────┬───────┘  │
│         │                                     │ WS       │
│         ▼                                     ▼          │
│  ┌──────────────┐              ┌──────────────────────┐  │
│  │  Firecracker  │◀── vsock ──▶│  vetty-daemon        │  │
│  │  VM Launcher  │             │  (Rust, host-side)   │  │
│  └──────┬───────┘              │  - vsock listener    │  │
│         │                      │  - REST API          │  │
│         │                      │  - WebSocket stream  │  │
│         ▼                      └──────────────────────┘  │
│  ┌─────────────────────────────────┐                     │
│  │  Firecracker VM (Guest)         │                     │
│  │                                 │                     │
│  │  ┌───────────┐  ┌───────────┐   │                     │
│  │  │ vetty-run  │─▶│ strace    │   │                     │
│  │  │ (wrapper)  │  │ (syscall  │   │                     │
│  │  └───────────┘  │  capture) │   │                     │
│  │                 └─────┬─────┘   │                     │
│  │                       ▼         │                     │
│  │              ┌──────────────┐   │                     │
│  │              │ vetty-agent  │   │                     │
│  │              │ (Rust)       │───┼── vsock ──▶ host    │
│  │              └──────────────┘   │                     │
│  │                                 │                     │
│  │  /sandbox (mounted code disk)   │                     │
│  └─────────────────────────────────┘                     │
└──────────────────────────────────────────────────────────┘
```

## Technology Stack

| Component         | Language / Framework       |
|-------------------|----------------------------|
| vetty CLI         | Rust                       |
| Disk Builder      | Rust (uses `mkfs.ext4`)    |
| VM Launcher       | Rust (Firecracker API)     |
| vetty-agent       | Rust (guest binary)        |
| vetty-run         | Bash script (guest)        |
| Boot script       | Bash (guest init)          |
| vetty-daemon      | Rust (host daemon)         |
| GUI               | Electron + React + TypeScript |

## Repo Structure (Target)

```
vetty/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── vetty-cli/              # Host CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── vetty-daemon/           # Host daemon (vsock + REST + WS)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── vsock.rs
│   │       ├── events.rs
│   │       ├── rest.rs
│   │       └── ws.rs
│   ├── vetty-agent/            # Guest agent binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── strace_parser.rs
│   │       └── vsock_client.rs
│   ├── vetty-disk/             # Disk image builder library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── vetty-vm/               # Firecracker launcher library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs
│   │       └── serial.rs
│   └── vetty-common/           # Shared types (events, protocol)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── guest/
│   ├── vetty-run.sh            # Guest-side wrapper script
│   └── init.sh                 # Boot/init script
├── image/
│   └── build-rootfs.sh         # Script to build the rootfs
├── gui/
│   ├── package.json
│   ├── electron/
│   │   └── main.ts
│   └── src/
│       ├── App.tsx
│       ├── components/
│       │   ├── Sidebar.tsx
│       │   ├── EventTimeline.tsx
│       │   ├── FilterBar.tsx
│       │   └── DetailPane.tsx
│       └── hooks/
│           └── useEventStream.ts
└── docs/
    └── *.md
```

## Build Order (Dependency Graph)

```
Step 1: vetty-common       (no deps — shared types)
Step 2: vetty-disk          (depends on vetty-common)
Step 3: vetty-agent         (depends on vetty-common) — cross-compile for guest
Step 4: vetty-vm            (depends on vetty-common)
Step 5: vetty-daemon        (depends on vetty-common)
Step 6: vetty-cli           (depends on vetty-disk, vetty-vm, vetty-daemon)
Step 7: Guest scripts       (vetty-run.sh, init.sh)
Step 8: Root filesystem     (build-rootfs.sh — bundles agent + scripts)
Step 9: GUI                 (Electron + React — connects to daemon)
```

## Prerequisites

- Linux host with KVM enabled (`/dev/kvm` accessible)
- Rust toolchain (stable) + `x86_64-unknown-linux-musl` target for static linking
- Firecracker binary (v1.x) on PATH
- Node.js 18+ and npm for the GUI
- `e2fsprogs` package (for `mkfs.ext4`, `mount`, etc.)
- `strace` (will be inside guest image)
- `debootstrap` or Alpine `apk` for building the rootfs
