# Step 9 — Integration, Testing & Final Assembly

## Goal
Wire everything together, verify the end-to-end flow, and document how to run the full system.

---

## 9.1 End-to-End Flow

Here's what happens when a user runs `vetty --dir ./suspicious-code`:

```
1. vetty CLI validates inputs
2. vetty CLI calls DiskBuilder to create code.img from ./suspicious-code
3. vetty CLI creates VmConfig with rootfs, kernel, code.img paths
4. vetty CLI calls VmInstance::create() which:
   a. Spawns Firecracker process with an API socket
   b. Configures boot source, drives, machine config, vsock via API
5. vetty CLI calls VmInstance::start() which sends InstanceStart
6. Firecracker boots the guest kernel
7. Guest init.sh runs:
   a. Mounts /dev/vdb at /sandbox
   b. Starts vetty-agent in background
   c. Drops to shell on serial console
8. vetty CLI attaches serial console — user sees the shell
9. User types: vetty-run node /sandbox/index.js
10. vetty-run starts strace, pipes output to vetty-agent
11. vetty-agent parses strace lines into SandboxEvents
12. vetty-agent sends events as JSON over vsock to host
13. vetty-daemon receives events on the vsock UDS
14. vetty-daemon pushes events to EventStore + broadcasts via channel
15. GUI receives events via WebSocket and renders in EventTimeline
16. User can filter, click events, see details
17. User presses Ctrl+C → vetty CLI kills VM, removes code.img
```

---

## 9.2 Integration Testing Strategy

### Test 1: Common types serialization
```
- Serialize/deserialize every type in vetty-common
- Ensure WireMessage round-trips correctly
- Test edge cases: empty strings, special characters, large payloads
```

### Test 2: Strace parser
```
- Feed real strace output lines into StraceParser
- Verify each line produces the correct EventType and extracted fields
- Test malformed lines return None
- Test multi-PID output with -f flag
- Include sample lines in test data:
  - open, openat, stat, access → FileAccess
  - connect, bind, socket → NetworkConnect
  - execve, fork, clone → ProcessSpawn
  - unfinished/resumed lines → None
  - signal lines → None
```

### Test 3: Disk builder (requires root)
```
- Create a temp directory with known files
- Build a disk image
- Mount the image and verify contents match
- Test empty directory
- Test directory with symlinks
```

### Test 4: Daemon event store
```
- Register a sandbox, push events, verify list and get
- Test broadcast — subscribe before pushing, verify events received
- Test mark_stopped updates status
- Test concurrent access (multiple spawned tasks)
```

### Test 5: REST + WebSocket API
```
- Start daemon in test mode
- POST events via a mock vsock client
- GET /api/sandboxes — verify sandbox appears
- GET /api/sandboxes/:id/events — verify events appear
- Connect to /ws/events — verify events stream in real time
```

### Test 6: Full integration (requires KVM)
```
- Build rootfs and kernel
- Run vetty --dir <test-dir> in a subprocess
- Verify VM boots (check serial output)
- Run a simple command inside the VM
- Verify events arrive at the daemon
```

---

## 9.3 Running the Full System

### Prerequisites Checklist

```bash
# Check KVM access
[ -w /dev/kvm ] && echo "KVM OK" || echo "KVM NOT AVAILABLE"

# Check Firecracker
which firecracker && echo "Firecracker OK" || echo "Install Firecracker"

# Check Rust toolchain
rustc --version
rustup target list --installed | grep musl

# Check Node.js
node --version
npm --version
```

### Build Everything

```bash
# 1. Build all Rust crates (host)
cargo build --release

# 2. Build guest agent (static musl binary)
cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent

# 3. Build rootfs image (requires sudo)
cd image && sudo bash build-rootfs.sh && cd ..

# 4. Download kernel
cd image && bash download-kernel.sh && cd ..

# 5. Build GUI
cd gui && npm install && npm run build && cd ..
```

### Run

```bash
# Terminal 1: Start the daemon
cargo run --release -p vetty-daemon

# Terminal 2: Start the GUI
cd gui && npm run electron:dev

# Terminal 3: Launch a sandbox
cargo run --release -p vetty-cli -- \
  --dir ./test-code \
  --rootfs ./image/rootfs.ext4 \
  --kernel ./image/vmlinux
```

---

## 9.4 Common Issues & Debugging

| Issue | Cause | Fix |
|-------|-------|-----|
| `Permission denied: /dev/kvm` | User not in `kvm` group | `sudo usermod -aG kvm $USER` then re-login |
| Firecracker exits immediately | Bad kernel or rootfs | Check `firecracker` stderr output |
| No events in GUI | vsock not connected | Verify `VETTY_VSOCK_PATH` matches between daemon and VM |
| Agent can't connect vsock | Wrong CID | Guest always connects to CID 2 (host), ensure `GUEST_CID=3` |
| Disk image too small | Overhead too low | Increase `DiskBuilder.overhead` |
| Strace parse failures | Unexpected format | Check strace flags match parser expectations |
| GUI won't connect | CORS issue | Ensure daemon has `CorsLayer::permissive()` |

---

## 9.5 Project Milestones Checklist

### Phase 1: Foundation ✏️
- [ ] Cargo workspace created
- [ ] `vetty-common` compiles with all shared types
- [ ] `vetty-disk` compiles and can build ext4 images
- [ ] `vetty-agent` compiles (at least `cargo check`)

### Phase 2: VM & Daemon ✏️
- [ ] `vetty-vm` compiles and can launch Firecracker
- [ ] `vetty-daemon` compiles with REST + WS endpoints
- [ ] `vetty-cli` compiles and ties everything together

### Phase 3: Guest ✏️
- [ ] Guest scripts written and tested
- [ ] Rootfs image builds successfully
- [ ] VM boots and reaches shell

### Phase 4: Integration ✏️
- [ ] Agent connects to daemon over vsock
- [ ] Events flow from guest to daemon
- [ ] Daemon broadcasts events over WebSocket

### Phase 5: GUI ✏️
- [ ] Electron app starts and connects to daemon
- [ ] Sidebar shows sandboxes
- [ ] EventTimeline renders live events
- [ ] FilterBar works
- [ ] DetailPane shows event details

### Phase 6: Polish ✏️
- [ ] Error handling is robust everywhere
- [ ] Clean shutdown works (Ctrl+C)
- [ ] No zombie processes or leaked files
- [ ] README written

---

## 9.6 Future Enhancements (Post-MVP)

- Allow/deny rules for syscalls and network access
- HTTP interception via mitmproxy inside the guest
- Persistent event storage (SQLite)
- Multiple concurrent VMs
- Snapshot/restore for fast VM startup
- Network access with NAT and traffic capture
- Export events as JSON/CSV
- Diff view: compare two sandbox runs
