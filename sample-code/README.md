# Sample Code

Example scripts for testing Vetty sandbox monitoring.

## Files

| File | Description |
|---|---|
| `my-file.py` | Python demo: file operations, network connections, process spawning |
| `test-http.sh` | Shell script: tests HTTP and HTTPS requests through the proxy |

## Usage

These files are mounted inside the Firecracker VM at `/sandbox`. Run them with the `vetty-run` wrapper to capture all syscalls:

```bash
# From the sandbox shell:
vetty-run python3 /sandbox/my-file.py
vetty-run bash /sandbox/test-http.sh
```

Or specify this directory when launching Vetty:

```bash
make run DIR=./sample-code
```

## Adding Your Own Code

You can sandbox any directory. Just point `--dir` at it:

```bash
cargo run -p vetty-cli -- --dir /path/to/your/code --rootfs ./image/rootfs.ext4 --kernel ./image/vmlinux
```

Your code will be available at `/sandbox` inside the VM.
