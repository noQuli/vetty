# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-16

### Added

- **Firecracker VM sandbox** — Launch untrusted code in hardware-isolated micro-VMs
- **Real-time syscall monitoring** — Capture file access, network, and process events via strace
- **Guest agent** (`vetty-agent`) — Parses strace output and streams structured events over vsock
- **Host daemon** (`vetty-daemon`) — Ingests events, serves REST API and WebSocket stream
- **CLI** (`vetty`) — One-command sandbox orchestration: disk build → VM launch → network setup
- **Electron GUI** — Real-time event timeline with filtering, search, and HTTP detail inspection
- **HTTPS interception** — mitmproxy integration for capturing TLS traffic from the sandbox
- **Disk builder** (`vetty-disk`) — Packages host directories into ext4 images for the VM
- **Alpine rootfs builder** — Script to build minimal guest images with agent baked in
- **Sample code** — Demo scripts for testing sandbox monitoring
- **CI/CD** — GitHub Actions for Rust stable/nightly, musl cross-compile, GUI build, and automated releases

[Unreleased]: https://github.com/noQuli/vetty/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/noQuli/vetty/releases/tag/v0.1.0
