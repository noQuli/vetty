#!/usr/bin/env python3
"""
Sample script for testing Vetty sandbox monitoring.

Run inside the sandbox with:
    vetty-run python3 /sandbox/my-file.py

This will demonstrate file access, network connections, and process
events being captured and streamed to the Vetty GUI.
"""

import os
import json
import socket
import subprocess


def demo_file_operations():
    """Create and read files — triggers file_access events."""
    print("[*] File operations...")

    with open("/tmp/vetty-demo.txt", "w") as f:
        f.write("Hello from the Vetty sandbox!\n")
        f.write(f"PID: {os.getpid()}\n")
        f.write(f"User: {os.environ.get('USER', 'unknown')}\n")

    with open("/tmp/vetty-demo.txt", "r") as f:
        content = f.read()
        print(f"    Wrote and read back: {len(content)} bytes")

    # Read some system files
    for path in ["/etc/hostname", "/etc/os-release", "/proc/version"]:
        try:
            with open(path) as f:
                line = f.readline().strip()
                print(f"    {path}: {line}")
        except FileNotFoundError:
            pass


def demo_network():
    """Make DNS and TCP connections — triggers network_connect events."""
    print("[*] Network operations...")

    try:
        ip = socket.gethostbyname("example.com")
        print(f"    DNS: example.com -> {ip}")
    except socket.gaierror as e:
        print(f"    DNS failed: {e}")

    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(3)
        sock.connect(("93.184.216.34", 80))
        sock.send(b"HEAD / HTTP/1.0\r\nHost: example.com\r\n\r\n")
        response = sock.recv(512).decode("utf-8", errors="replace")
        status_line = response.split("\r\n")[0]
        print(f"    HTTP: {status_line}")
        sock.close()
    except Exception as e:
        print(f"    TCP connect failed: {e}")


def demo_process():
    """Spawn subprocesses — triggers process_spawn events."""
    print("[*] Process operations...")

    result = subprocess.run(["uname", "-a"], capture_output=True, text=True)
    print(f"    uname: {result.stdout.strip()}")

    result = subprocess.run(["ls", "/sandbox"], capture_output=True, text=True)
    print(f"    /sandbox contents: {result.stdout.strip()}")


def demo_environment():
    """Print sandbox environment info."""
    print("[*] Environment...")
    print(f"    Sandbox ID: {os.environ.get('VETTY_SANDBOX_ID', 'not set')}")
    print(f"    Working dir: {os.getcwd()}")
    print(f"    HTTP_PROXY: {os.environ.get('HTTP_PROXY', 'not set')}")


if __name__ == "__main__":
    print("=" * 50)
    print("  Vetty Sandbox Demo")
    print("=" * 50)
    print()

    demo_environment()
    print()
    demo_file_operations()
    print()
    demo_network()
    print()
    demo_process()

    print()
    print("[✓] Demo complete — check the Vetty GUI for captured events!")
