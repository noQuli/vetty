# HTTPS Interception

Vetty captures HTTP and HTTPS traffic through a host-side proxy. The default backend is `mitmproxy`; `httptoolkit-server` remains available as an optional reference backend.

## Prerequisites

- `mitmdump` installed on the host
- A rebuilt Vetty rootfs that includes the current `guest/init.sh` and `guest/overrides`
- The host tap interface configured by `vetty-cli`, with the host reachable from the guest at `172.16.0.1`

## Build

```bash
cargo build --release
cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent
cd image
sudo ./build-rootfs.sh
cd ..
```

The rootfs build copies `guest/overrides` into `/opt/vetty/overrides`, so rebuilt sandboxes automatically get the Python HTTP client proxy overrides.

## Run

Terminal 1:

```bash
cargo run --release -p vetty-daemon
```

By default the daemon starts mitmproxy on `0.0.0.0:8899` and loads `scripts/vetty-mitmproxy-addon.py`.

Terminal 2:

```bash
cd gui
npm install
npm run electron:dev
```

Terminal 3:

```bash
cargo run --release -p vetty-cli -- \
  --dir ./sample-code \
  --rootfs ./image/rootfs.ext4 \
  --kernel ./image/vmlinux
```

Inside the sandbox, run a command through `vetty-run`:

```bash
vetty-run curl -sv https://example.com
```

The GUI should show `http_request` and `http_response` rows for the same sandbox as the syscall and network events.

## How It Works

`guest/init.sh` exports proxy variables before the shell starts:

```bash
HTTP_PROXY=http://<sandbox-id>:vetty@172.16.0.1:8899
HTTPS_PROXY=http://<sandbox-id>:vetty@172.16.0.1:8899
SSL_CERT_FILE=/etc/ssl/certs/vetty-proxy-ca.pem
```

The sandbox ID is sent as proxy basic-auth username. The mitmproxy addon reads it and posts request/response events to `vetty-daemon` at `/api/proxy-events`, so the UI can show traffic under the correct sandbox.

## Configuration

| Variable | Default | Description |
| --- | --- | --- |
| `VETTY_PROXY_BACKEND` | `mitmproxy` | Backend: `mitmproxy`, `httptoolkit`, `both`, or `none` |
| `VETTY_MITM_PORT` | `8899` | mitmproxy listen port |
| `VETTY_MITM_CONFDIR` | `./.vetty-mitmproxy` | mitmproxy certificate/config directory |
| `VETTY_MITM_ADDON` | `./scripts/vetty-mitmproxy-addon.py` | Vetty mitmproxy forwarding addon |
| `VETTY_PROXY_HOST` | `172.16.0.1` | Host proxy address used by the guest |
| `VETTY_PROXY_PORT` | `8899` | Host proxy port used by the guest |

## Troubleshooting

If HTTPS rows do not appear in the GUI:

- Confirm the daemon logs show mitmproxy listening on port `8899`.
- Confirm `mitmdump` is installed and available on `PATH`.
- Rebuild `image/rootfs.ext4`; older rootfs images will not export the proxy/CA variables.
- In the sandbox, check `echo "$HTTPS_PROXY"` and confirm it points at `172.16.0.1:8899`.
- Run `curl -sv https://example.com` inside `vetty-run` and check for certificate or proxy connection errors.

To disable interception:

```bash
VETTY_PROXY_BACKEND=none cargo run --release -p vetty-daemon
```
