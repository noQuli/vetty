# Runtime Arguments and Environment Variables

## `vetty` CLI arguments

`vetty` is the binary from `crates/vetty-cli`.

```bash
vetty --help
```

| Argument | Required | Default | Description |
| --- | --- | --- | --- |
| `--dir <DIR>` | Yes | - | Directory to package into the VM code disk |
| `--rootfs <ROOTFS>` | No | `image/rootfs.ext4` | Root filesystem image path |
| `--kernel <KERNEL>` | No | `image/vmlinux` | Firecracker kernel image path |
| `--memory <MEMORY>` | No | `128` | Guest memory in MiB (must be `>= 64`) |
| `--cpus <CPUS>` | No | `1` | Guest vCPU count (must be `>= 1`) |
| `--firecracker <FIRECRACKER>` | No | `firecracker` | Firecracker binary path/name |
| `--no-serial` | No | `false` | Do not attach serial console; wait for Ctrl+C |
| `-h`, `--help` | No | - | Show help |

## `vetty-daemon` environment variables

`vetty-daemon` currently uses environment variables instead of CLI flags.

| Variable | Default | Description |
| --- | --- | --- |
| `VETTY_DAEMON_PORT` | `9876` | HTTP/WebSocket bind port on `127.0.0.1` |
| `VETTY_VSOCK_PATH` | `/tmp/vetty_v.sock` | Base vsock Unix socket path used for guest event ingress |

## `vetty-agent` environment variables

| Variable | Default | Description |
| --- | --- | --- |
| `VETTY_SANDBOX_ID` | auto-generated | Optional sandbox ID sent in agent handshake |

## Logging

All Rust binaries use `tracing_subscriber` with `EnvFilter::from_default_env()`, so you can control log verbosity with `RUST_LOG`.

Example:

```bash
RUST_LOG=info cargo run -p vetty-cli -- --dir ./sample-code
```
