import { useEffect, useMemo, useRef } from "react";

import type { TaggedEvent } from "../types";

interface EventTimelineProps {
  events: TaggedEvent[];
  selectedEvent: TaggedEvent | null;
  onSelect: (event: TaggedEvent) => void;
}

export function EventTimeline({
  events,
  selectedEvent,
  onSelect,
}: EventTimelineProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [events.length]);

  const rows = useMemo(() => events.slice(-5000), [events]);

  return (
    <div className="timeline" ref={containerRef}>
      {rows.length > 0 && (
        <div className="timeline-header">
          <span>Time</span>
          <span>Type</span>
          <span>PID</span>
          <span>Summary</span>
        </div>
      )}

      {rows.length === 0 && (
        <div className="empty-state">
          <div className="empty-icon" />
          <strong>No matching events</strong>
          <span>Adjust the filters or wait for sandbox activity.</span>
        </div>
      )}

      <div className="timeline-rows">
        {rows.map((tagged, index) => {
          const key = `${tagged.sandbox_id}-${tagged.event.timestamp}-${index}`;
          const isSelected =
            selectedEvent !== null &&
            selectedEvent.sandbox_id === tagged.sandbox_id &&
            selectedEvent.event.timestamp === tagged.event.timestamp &&
            selectedEvent.event.pid === tagged.event.pid;
          
          return (
            <button
              key={key}
              type="button"
              className={`timeline-row ${isSelected ? "selected" : ""}`}
              onClick={() => onSelect(tagged)}
            >
              <span>{formatTime(tagged.event.timestamp)}</span>
              <span>
                <span className={`badge ${tagged.event.event_type}`}>
                  {tagged.event.event_type.replace("_", " ")}
                </span>
              </span>
              <span>{tagged.event.pid}</span>
              <span className="summary">
                <strong>{getSubject(tagged)}</strong>
                <span>{summarize(tagged)}</span>
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function getSubject(tagged: TaggedEvent): string {
  const event = tagged.event;
  if (event.event_type === "http_request") {
    return event.http_method || "HTTP";
  }
  if (event.event_type === "http_response") {
    return event.http_status ? `HTTP ${event.http_status}` : "RESPONSE";
  }
  return event.syscall_name || event.event_type.replace("_", " ");
}

function summarize(tagged: TaggedEvent): string {
  const event = tagged.event;
  const parts: string[] = [];

  if (event.http_url) {
    parts.push(event.http_url);
  } else if (event.path) {
    parts.push(event.path);
  } else if (event.hostname) {
    const host = event.port ? `${event.hostname}:${event.port}` : event.hostname;
    parts.push(host);
  }

  const ret = formatReturnValue(event.return_value);
  if (ret) parts.push(ret);

  if (parts.length === 0 && event.raw) {
    return event.raw.slice(0, 100);
  }

  return parts.join(" ") || "—";
}

function formatReturnValue(value: number | undefined): string {
  return value !== undefined ? `return ${value}` : "";
}

function formatTime(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return timestamp;
  }
  return date.toISOString().slice(11, 23);
}
