import { memo, useMemo, useState } from "react";
import type { TaggedEvent } from "../types";

interface DetailPaneProps {
  event: TaggedEvent;
  onClose: () => void;
}

export const DetailPane = memo(function DetailPane({ event, onClose }: DetailPaneProps) {
  const [showHttpBody, setShowHttpBody] = useState(false);
  const [showHttpHeaders, setShowHttpHeaders] = useState(false);
  const [showRawTrace, setShowRawTrace] = useState(false);
  const [showEventJson, setShowEventJson] = useState(false);

  const fields = useMemo(() => [
    ["Sandbox", toDisplayText(event.sandbox_id)],
    ["Time", formatDateTime(event.event.timestamp)],
    ["Type", formatEventType(event.event.event_type)],
    ["PID", toDisplayText(event.event.pid)],
    ["Syscall", toDisplayText(event.event.syscall_name)],
    ["Path", toDisplayText(event.event.path)],
    ["Host", formatHost(event.event.hostname, event.event.port)],
    ["HTTP", formatHttp(event)],
    ["Return", formatOptional(event.event.return_value)],
    ["Flags", toDisplayText(event.event.flags)],
  ].filter((field): field is [string, string] => Boolean(field[1])), [event]);

  const normalizedHttp = useMemo(() => parseHttpMessage(event), [event]);
  const httpHeaders = normalizedHttp?.headers ?? event.event.http_headers;
  const httpBody = normalizedHttp?.body ?? event.event.http_body;

  const httpBodyPreview = useMemo(
   () => formatPreview(httpBody),
   [httpBody],
  );

  const rawPreview = useMemo(
    () => formatPreview(event.event.raw),
    [event.event.raw],
  );

  const eventJsonPreview = useMemo(
    () => buildEventPreview(event),
    [event],
  );

  const exportAsJson = () => {
    const data = safeStringify(event.event);
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `event-${event.event.event_type}-${event.event.timestamp}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <aside className="detail-pane">
      <div className="detail-header">
        <div>
          <h2>{formatEventType(event.event.event_type)}</h2>
          <span>{formatDateTime(event.event.timestamp)}</span>
        </div>
        <div className="header-actions">
          <button type="button" className="action-button" onClick={exportAsJson}>
            Export JSON
          </button>
          <button type="button" className="close-button" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
      </div>

      <div className="detail-content">
        <section className="detail-section">
          <div className="section-header">
            <h3>Summary</h3>
            <CopyButton
              getText={() => fields.map(([l, v]) => `${l}: ${v}`).join("\n")}
              label="Copy Summary"
            />
          </div>
          <div className="detail-grid">
            {fields.map(([label, value]) => (
              <div key={label} className="detail-row">
                <span className="detail-label">{label}</span>
                <span className="detail-value">{value}</span>
              </div>
            ))}
          </div>
        </section>

        {httpHeaders && Object.keys(httpHeaders).length > 0 && (
          <section className="detail-section">
            <div className="section-header">
              <h3>HTTP Headers</h3>
              <div className="header-actions">
                <button
                  type="button"
                  className="action-button"
                  onClick={() => setShowHttpHeaders((value) => !value)}
                >
                  {showHttpHeaders ? "Hide" : "Show"}
                </button>
                <CopyButton getText={() => safeStringify(httpHeaders)} label="Copy Headers" />
              </div>
            </div>
            {showHttpHeaders ? (
              <JsonViewer data={httpHeaders} />
            ) : (
              <pre className="detail-preview">{formatJson(httpHeaders)}</pre>
            )}
          </section>
        )}

        {hasHttpBody(httpBody) && (
          <section className="detail-section">
            <div className="section-header">
              <h3>HTTP Body</h3>
              <div className="header-actions">
                <button
                  type="button"
                  className="action-button"
                  onClick={() => setShowHttpBody((value) => !value)}
                >
                  {showHttpBody ? "Hide" : "Show"}
                </button>
                <CopyButton getText={() => toDisplayText(httpBody) ?? ""} label="Copy Body" />
              </div>
            </div>
            {showHttpBody ? (
              <JsonViewer data={httpBody} />
            ) : (
              <pre className="detail-preview">{httpBodyPreview}</pre>
            )}
          </section>
        )}

        {event.event.raw && (
          <section className="detail-section">
            <div className="section-header">
              <h3>Raw Trace</h3>
              <div className="header-actions">
                <button
                  type="button"
                  className="action-button"
                  onClick={() => setShowRawTrace((value) => !value)}
                >
                  {showRawTrace ? "Hide" : "Show"}
                </button>
                <CopyButton getText={() => event.event.raw ?? ""} label="Copy Trace" />
              </div>
            </div>
            {showRawTrace ? <pre>{toDisplayText(event.event.raw) ?? ""}</pre> : <pre className="detail-preview">{rawPreview}</pre>}
          </section>
        )}

        <section className="detail-section">
          <div className="section-header">
            <h3>Event Data</h3>
            <div className="header-actions">
              <button
                type="button"
                className="action-button"
                onClick={() => setShowEventJson((value) => !value)}
              >
                {showEventJson ? "Hide" : "Show"}
              </button>
              <CopyButton getText={() => safeStringify(event.event)} label="Copy JSON" />
            </div>
          </div>
          {showEventJson ? <JsonViewer data={event.event} /> : <pre className="detail-preview">{eventJsonPreview}</pre>}
        </section>
      </div>
    </aside>
  );
});

function CopyButton({ getText, label }: { getText: () => string; label: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(getText());
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  };

  return (
    <button
      type="button"
      className={`copy-button ${copied ? "copied" : ""}`}
      onClick={handleCopy}
    >
      {copied ? "Copied!" : label}
    </button>
  );
}

function JsonViewer({ data }: { data: unknown }) {
  return <pre className="json-pre">{formatJson(data)}</pre>;
}

function formatPreview(value: unknown, limit = 400): string {
  if (value === undefined || value === null) {
    return "(empty)";
  }

  const text = toDisplayText(value) ?? "(empty)";
  if (text === "") {
    return "(empty)";
  }
  if (text.length <= limit) {
    return text;
  }

  return `${text.slice(0, limit)}… [truncated ${text.length - limit} chars]`;
}

function parseHttpMessage(event: TaggedEvent): { headers?: Record<string, string>; body?: string } | undefined {
  const message = event.event.http_message ?? event.event.http_body;
  if (!message || !looksLikeHttpMessage(message)) {
    return undefined;
  }

  const separator = message.includes("\r\n\r\n") ? "\r\n\r\n" : "\n\n";
  const [head, body = ""] = message.split(separator, 2);
  const lines = head.split(/\r?\n/);
  if (lines.length < 2) {
    return { body };
  }

  const headers: Record<string, string> = {};
  for (const line of lines.slice(1)) {
    const idx = line.indexOf(":");
    if (idx <= 0) {
      continue;
    }
    headers[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
  }

  return {
    headers: Object.keys(headers).length > 0 ? headers : undefined,
    body,
  };
}

function hasHttpBody(body: string | undefined): boolean {
  return (body ?? "").trim().length > 0;
}

function looksLikeHttpMessage(value: string): boolean {
  const start = value.trimStart();
  return start.startsWith("HTTP/") || /^[A-Z]+\s+\S+\s+HTTP\/\d/.test(start);
}

function buildEventPreview(event: TaggedEvent): string {
  return [
    `sandbox: ${toDisplayText(event.sandbox_id) ?? "<unknown>"}`,
    `type: ${formatEventType(event.event.event_type)}`,
    `time: ${formatDateTime(event.event.timestamp)}`,
    toDisplayText(event.event.http_method) ? `method: ${toDisplayText(event.event.http_method)}` : undefined,
    toDisplayText(event.event.http_url) ? `url: ${toDisplayText(event.event.http_url)}` : undefined,
    toDisplayText(event.event.path) ? `path: ${toDisplayText(event.event.path)}` : undefined,
    event.event.hostname ? `host: ${formatHost(event.event.hostname, event.event.port)}` : undefined,
    event.event.raw ? formatPreview(event.event.raw, 180) : undefined,
  ]
    .filter(Boolean)
    .join("\n");
}

function formatHost(hostname: string | undefined, port: number | undefined): string | undefined {
  if (!hostname && port === undefined) {
    return undefined;
  }
  const safeHost = toDisplayText(hostname) ?? "<unknown>";
  const safePort = toDisplayText(port) ?? "?";
  return `${safeHost}:${safePort}`;
}

function formatHttp(event: TaggedEvent): string | undefined {
  const { http_method, http_url, http_status } = event.event;
  const method = toDisplayText(http_method);
  const url = toDisplayText(http_url);
  if (method || url) {
    return [method, url].filter(Boolean).join(" ");
  }
  const status = toDisplayText(http_status);
  if (status) {
    return `status ${status}`;
  }
  return undefined;
}

function formatOptional(value: number | string | undefined): string | undefined {
  return value === undefined ? undefined : toDisplayText(value);
}

function formatEventType(value: unknown): string {
  const text = toDisplayText(value) ?? "unknown";
  return text.replace("_", " ");
}

function toDisplayText(value: unknown): string | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }

  if (typeof value === "string") {
    return value;
  }

  if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
    return String(value);
  }

  if (value instanceof Date) {
    return value.toISOString();
  }

  return safeStringify(value);
}

function formatJson(value: unknown): string {
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed.length > 0 && trimmed.length <= 50_000 && (trimmed.startsWith("{") || trimmed.startsWith("["))) {
      try {
        return JSON.stringify(JSON.parse(value), null, 2);
      } catch {
        return value;
      }
    }

    return value;
  }

  return safeStringify(value);
}

function safeStringify(value: unknown): string {
  const seen = new WeakSet<object>();

  try {
    return JSON.stringify(
      value,
      (_key, current) => {
        if (typeof current === "bigint") return current.toString();
        if (typeof current === "object" && current !== null) {
          if (seen.has(current)) return "[Circular]";
          seen.add(current);
        }
        return current;
      },
      2,
    ) ?? "null";
  } catch (error) {
    console.error("Failed to serialize JSON:", error);
    return String(value);
  }
}

function formatDateTime(timestamp: unknown): string {
  const text = toDisplayText(timestamp);
  if (!text) {
    return "<unknown>";
  }

  const date = new Date(text);
  if (Number.isNaN(date.getTime())) {
    return text;
  }
  return date.toLocaleString();
}
