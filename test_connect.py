import socket
import json

s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect("/tmp/vetty_v.sock_5123")

msg = {
    "type": "handshake",
    "sandbox_id": "00000000-0000-0000-0000-000000000000",
    "agent_version": "0.1.0",
    "hostname": "test"
}
s.sendall((json.dumps(msg) + "\n").encode())

event = {
    "type": "event",
    "timestamp": "2026-05-09T14:47:41.025662197Z",
    "pid": 123,
    "event_type": "syscall",
    "syscall_name": "test",
    "return_value": 0
}
s.sendall((json.dumps(event) + "\n").encode())

print("Sent!")
try:
    print(s.recv(1024))
except Exception as e:
    print("Error:", e)
