#!/usr/bin/env bash
# =============================================================================
# Vetty — Launch all components (daemon + GUI + VM)
#
# Usage:
#   ./scripts/run-all.sh [--dir <path>] [--memory <mb>] [--cpus <n>]
#
# This script manages the lifecycle of all three processes and ensures
# clean shutdown when you press Ctrl+C.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Defaults
DIR="${DIR:-$REPO_ROOT/sample-code}"
ROOTFS="${ROOTFS:-$REPO_ROOT/image/rootfs.ext4}"
KERNEL="${KERNEL:-$REPO_ROOT/image/vmlinux}"
MEMORY="${MEMORY:-128}"
CPUS="${CPUS:-1}"
DAEMON_PORT="${VETTY_DAEMON_PORT:-9876}"

# Parse CLI arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)     DIR="$2";    shift 2 ;;
    --rootfs)  ROOTFS="$2"; shift 2 ;;
    --kernel)  KERNEL="$2"; shift 2 ;;
    --memory)  MEMORY="$2"; shift 2 ;;
    --cpus)    CPUS="$2";   shift 2 ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Usage: $0 [--dir <path>] [--memory <mb>] [--cpus <n>]" >&2
      exit 1
      ;;
  esac
done

# Colors
BOLD='\033[1m'
GREEN='\033[32m'
CYAN='\033[36m'
YELLOW='\033[33m'
RED='\033[31m'
RESET='\033[0m'

DAEMON_PID=""
GUI_PID=""
VM_PID=""

launch_vetty_cli_in_terminal() {
  local vm_command
  printf -v vm_command 'cd %q && cargo run -p vetty-cli --manifest-path %q -- --dir %q --rootfs %q --kernel %q --memory %q --cpus %q' \
    "$REPO_ROOT" \
    "$REPO_ROOT/Cargo.toml" \
    "$DIR" \
    "$ROOTFS" \
    "$KERNEL" \
    "$MEMORY" \
    "$CPUS"

  if command -v x-terminal-emulator >/dev/null 2>&1; then
    x-terminal-emulator -e bash -lc "$vm_command" &
    VM_PID=$!
    return
  fi

  if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal -- bash -lc "$vm_command" &
    VM_PID=$!
    return
  fi

  if command -v konsole >/dev/null 2>&1; then
    konsole -e bash -lc "$vm_command" &
    VM_PID=$!
    return
  fi

  if command -v xterm >/dev/null 2>&1; then
    xterm -hold -e bash -lc "$vm_command" &
    VM_PID=$!
    return
  fi

  echo -e "${YELLOW}⚠ No terminal emulator found; running vetty-cli in the current shell.${RESET}"
  cargo run -p vetty-cli --manifest-path "$REPO_ROOT/Cargo.toml" -- \
    --dir "$DIR" \
    --rootfs "$ROOTFS" \
    --kernel "$KERNEL" \
    --memory "$MEMORY" \
    --cpus "$CPUS" &
  VM_PID=$!
}

cleanup() {
  echo ""
  echo -e "${YELLOW}→ Shutting down Vetty...${RESET}"

  if [[ -n "$VM_PID" ]] && kill -0 "$VM_PID" 2>/dev/null; then
    echo -e "  Stopping VM (PID $VM_PID)..."
    kill "$VM_PID" 2>/dev/null || true
    wait "$VM_PID" 2>/dev/null || true
  fi

  if [[ -n "$GUI_PID" ]] && kill -0 "$GUI_PID" 2>/dev/null; then
    echo -e "  Stopping GUI (PID $GUI_PID)..."
    kill "$GUI_PID" 2>/dev/null || true
    wait "$GUI_PID" 2>/dev/null || true
  fi

  if [[ -n "$DAEMON_PID" ]] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    echo -e "  Stopping daemon (PID $DAEMON_PID)..."
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi

  echo -e "${GREEN}✓ All components stopped.${RESET}"
}

trap cleanup EXIT INT TERM

# ---- Preflight checks -------------------------------------------------------

echo -e "${BOLD}🛡️  Vetty — Starting all components${RESET}"
echo ""

if [[ ! -f "$ROOTFS" ]]; then
  echo -e "${RED}✗ Rootfs not found: $ROOTFS${RESET}"
  echo -e "  Run ${CYAN}make setup${RESET} first to download kernel and build rootfs."
  exit 1
fi

if [[ ! -f "$KERNEL" ]]; then
  echo -e "${RED}✗ Kernel not found: $KERNEL${RESET}"
  echo -e "  Run ${CYAN}make setup${RESET} first."
  exit 1
fi

if [[ ! -d "$DIR" ]]; then
  echo -e "${RED}✗ Source directory not found: $DIR${RESET}"
  exit 1
fi

# ---- Start daemon -----------------------------------------------------------

echo -e "${CYAN}[1/3]${RESET} Starting vetty-daemon on port $DAEMON_PORT..."

# Check if daemon is already running
if curl -sf "http://127.0.0.1:$DAEMON_PORT/api/sandboxes" >/dev/null 2>&1; then
  echo -e "  ${YELLOW}Daemon already running on port $DAEMON_PORT${RESET}"
else
  VETTY_DAEMON_PORT="$DAEMON_PORT" cargo run -p vetty-daemon --manifest-path "$REPO_ROOT/Cargo.toml" &
  DAEMON_PID=$!

  # Wait for daemon to be ready (up to 300 seconds array to allow time for compilation)
  for i in $(seq 1 600); do
    if curl -sf "http://127.0.0.1:$DAEMON_PORT/api/sandboxes" >/dev/null 2>&1; then
      break
    fi
    sleep 0.5
  done

  if ! curl -sf "http://127.0.0.1:$DAEMON_PORT/api/sandboxes" >/dev/null 2>&1; then
    echo -e "${RED}✗ Daemon failed to start within 300 seconds${RESET}"
    exit 1
  fi

  echo -e "  ${GREEN}✓ Daemon ready (PID $DAEMON_PID)${RESET}"
fi

# ---- Start GUI ---------------------------------------------------------------

echo -e "${CYAN}[2/3]${RESET} Starting Electron GUI..."

cd "$REPO_ROOT/gui"
npm run electron:dev &
GUI_PID=$!
cd "$REPO_ROOT"

echo -e "  ${GREEN}✓ GUI starting (PID $GUI_PID)${RESET}"

# Give GUI a moment to open
sleep 2

# ---- Start VM ----------------------------------------------------------------

echo -e "${CYAN}[3/3]${RESET} Launching sandbox VM with $DIR..."
echo ""

launch_vetty_cli_in_terminal

echo -e "${GREEN}${BOLD}🚀 Vetty is running!${RESET}"
echo -e "  Daemon:  http://127.0.0.1:$DAEMON_PORT"
echo -e "  GUI:     Electron window"
echo -e "  CLI:     vetty-cli terminal"
echo -e "  Sandbox: $DIR"
echo -e ""
echo -e "  Press ${BOLD}Ctrl+C${RESET} to stop all components."
echo ""

# Wait for any child to exit
wait -n "$VM_PID" "$GUI_PID" ${DAEMON_PID:+"$DAEMON_PID"} 2>/dev/null || true
