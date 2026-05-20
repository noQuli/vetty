import base64
import json
import os
import urllib.request
from datetime import datetime, timezone


DAEMON_URL = os.environ.get("VETTY_DAEMON_URL", "http://127.0.0.1:9876/api/proxy-events")
CLIENT_SANDBOX_IDS = {}
CLIENT_PIDS = {}
NEXT_SYNTHETIC_PID = 1000


def client_connection_keys(flow):
    client_conn = getattr(flow, "client_conn", None)
    if not client_conn:
        return []

    keys = []

    conn_id = getattr(client_conn, "id", None)
    if conn_id:
        keys.append(f"id:{conn_id}")

    peername = getattr(client_conn, "peername", None)
    if peername:
        keys.append(f"peer:{repr(peername)}")

    # Fallback to Python object identity for flows that don't expose a stable id.
    keys.append(f"obj:{id(client_conn)}")
    return keys


def client_pid(flow):
    global NEXT_SYNTHETIC_PID

    for client_key in client_connection_keys(flow):
        pid = CLIENT_PIDS.get(client_key)
        if pid is not None:
            return pid

    pid = NEXT_SYNTHETIC_PID
    NEXT_SYNTHETIC_PID += 1

    for client_key in client_connection_keys(flow):
        CLIENT_PIDS[client_key] = pid

    return pid


def http_connect(flow):
    sandbox_id = sandbox_id_from_headers(flow.request.headers)
    if sandbox_id:
        flow.metadata["vetty_sandbox_id"] = sandbox_id
        for client_key in client_connection_keys(flow):
            CLIENT_SANDBOX_IDS[client_key] = sandbox_id


def request(flow):
    if flow.request.method == "CONNECT":
        return
    if not is_https_flow(flow):
        return

    sandbox_id = sandbox_id_from_flow(flow)
    if not sandbox_id:
        return

    body_text = message_body_text(flow.request)
    request_headers = headers(flow.request.headers)
    message_text = http_message_text(
        f"{flow.request.method} {flow.request.pretty_url} {flow.request.http_version}",
        request_headers,
        body_text,
    )
    post_event(sandbox_id, {
        "timestamp": now(),
        "pid": client_pid(flow),
        "event_type": "http_request",
        "syscall_name": "mitmproxy",
        "path": None,
        "hostname": flow.request.host,
        "port": flow.request.port,
        "flags": None,
        "return_value": len(body_text.encode("utf-8")),
        "http_method": flow.request.method,
        "http_url": flow.request.pretty_url,
        "http_status": None,
        "http_headers": request_headers,
        "http_body": body_text,
        "http_message": message_text,
        "raw": f"{flow.request.method} {flow.request.pretty_url}",
    })


def response(flow):
    if not is_https_flow(flow):
        return

    sandbox_id = sandbox_id_from_flow(flow)
    if not sandbox_id or not flow.response:
        return

    body_text = message_body_text(flow.response)
    message_text = http_message_text(
        f"{flow.response.http_version} {flow.response.status_code} {flow.response.reason or ''}".rstrip(),
        flow.response.headers,
        body_text,
    )
    post_event(sandbox_id, {
        "timestamp": now(),
        "pid": client_pid(flow),
        "event_type": "http_response",
        "syscall_name": "mitmproxy",
        "path": None,
        "hostname": flow.request.host,
        "port": flow.request.port,
        "flags": None,
        "return_value": len(body_text.encode("utf-8")),
        "http_method": flow.request.method,
        "http_url": flow.request.pretty_url,
        "http_status": flow.response.status_code,
        "http_headers": headers(flow.response.headers),
        "http_body": body_text,
        "http_message": message_text,
        "raw": f"{flow.response.status_code} {flow.request.pretty_url}",
    })


def sandbox_id_from_flow(flow):
    sandbox_id = flow.metadata.get("vetty_sandbox_id")
    if sandbox_id:
        return sandbox_id

    for client_key in client_connection_keys(flow):
        sandbox_id = CLIENT_SANDBOX_IDS.get(client_key)
        if sandbox_id:
            return sandbox_id

    sandbox_id = sandbox_id_from_headers(flow.request.headers)
    if sandbox_id:
        return sandbox_id

    return os.environ.get("VETTY_SANDBOX_ID")


def sandbox_id_from_headers(request_headers):
    auth = request_headers.get("Proxy-Authorization", "")
    if auth.lower().startswith("basic "):
        try:
            decoded = base64.b64decode(auth.split(None, 1)[1]).decode("utf-8", "replace")
            username = decoded.split(":", 1)[0]
            if username:
                return username
        except Exception:
            pass

    return None


def headers(message_headers):
    result = dict(message_headers)
    result.pop("Proxy-Authorization", None)
    result.pop("proxy-authorization", None)
    return result


def is_https_flow(flow):
    return getattr(flow.request, "scheme", "").lower() == "https"


def message_body_text(message):
    text = message.get_text(strict=False)
    if text is not None:
        return text

    raw = getattr(message, "raw_content", None)
    if raw is None:
        return ""
    return raw.decode("utf-8", "replace")


def http_message_text(start_line, message_headers, body):
    headers_text = "\r\n".join(f"{name}: {value}" for name, value in dict(message_headers).items())
    if headers_text:
        return f"{start_line}\r\n{headers_text}\r\n\r\n{body}"
    return f"{start_line}\r\n\r\n{body}"


def post_event(sandbox_id, event):
    payload = json.dumps({"sandbox_id": sandbox_id, "event": event}).encode("utf-8")
    request = urllib.request.Request(
        DAEMON_URL,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        urllib.request.urlopen(request, timeout=2).read()
    except Exception as error:
        print(f"failed to send Vetty proxy event: {error}")


def now():
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
