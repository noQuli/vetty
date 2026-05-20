import { useCallback, useEffect, useRef, useState } from "react";

import type { SandboxInfo, TaggedEvent } from "../types";

const DAEMON_WS_URL =
  window.vettyConfig?.daemon.wsUrl ??
  import.meta.env.VITE_DAEMON_WS_URL ??
  "ws://127.0.0.1:9876/ws/events";
const DAEMON_REST_URL =
  window.vettyConfig?.daemon.restUrl ??
  import.meta.env.VITE_DAEMON_REST_URL ??
  "http://127.0.0.1:9876/api";

const RECONNECT_BASE_DELAY = 1000;
const RECONNECT_MAX_DELAY = 15000;
const MAX_EVENT_HISTORY = 5000;

export function useEventStream() {
  const [events, setEvents] = useState<TaggedEvent[]>([]);
  const [sandboxes, setSandboxes] = useState<SandboxInfo[]>([]);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const refreshTimer = useRef<ReturnType<typeof setInterval> | null>(null);
  const connectWsRef = useRef<() => void>(() => {});
  const reconnectAttempt = useRef(0);
  const unmounted = useRef(false);
  const fetchSandboxes = useCallback(async () => {
    try {
      const response = await fetch(`${DAEMON_REST_URL}/sandboxes`);
      if (!response.ok) {
        return;
      }
      const payload = (await response.json()) as SandboxInfo[];
      setSandboxes(payload);
    } catch {
      // intentionally ignored
    }
  }, []);

  const fetchEvents = useCallback(async () => {
    try {
      const response = await fetch(`${DAEMON_REST_URL}/events`);
      if (!response.ok) {
        return;
      }
      const payload = (await response.json()) as TaggedEvent[];
      setEvents(payload.slice(-MAX_EVENT_HISTORY));
    } catch {
      // intentionally ignored
    }
  }, []);

  const connectWs = useCallback(() => {
    if (unmounted.current) return;

    // ... (existing cleanup code)
    if (wsRef.current) {
      wsRef.current.onopen = null;
      wsRef.current.onmessage = null;
      wsRef.current.onerror = null;
      wsRef.current.onclose = null;
      if (wsRef.current.readyState < WebSocket.CLOSING) {
        wsRef.current.close();
      }
      wsRef.current = null;
    }

    const ws = new WebSocket(DAEMON_WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      reconnectAttempt.current = 0;
      fetchSandboxes();
      fetchEvents();

      if (refreshTimer.current) {
        clearInterval(refreshTimer.current);
      }
      refreshTimer.current = setInterval(() => {
        fetchSandboxes();
      }, 2000);
    };

    ws.onmessage = (message) => {
      try {
        const tagged = JSON.parse(message.data as string) as TaggedEvent;
        setEvents((previous) => {
          const next = [...previous, tagged];
          return next.length > MAX_EVENT_HISTORY
            ? next.slice(-MAX_EVENT_HISTORY)
            : next;
        });

        setSandboxes((prev) => {
          const exists = prev.find((s) => s.id === tagged.sandbox_id);
          if (!exists) {
            // Unknown sandbox, fetch the updated list from the daemon
            fetchSandboxes();
            return prev;
          }
          // Update event count for existing sandbox
          return prev.map((s) =>
            s.id === tagged.sandbox_id
              ? { ...s, event_count: s.event_count + 1 }
              : s,
          );
        });
      } catch {
        // ignore malformed frames to keep stream alive
      }
    };

    ws.onerror = () => {
      setConnected(false);
    };

    ws.onclose = () => {
      setConnected(false);
      wsRef.current = null;

      if (refreshTimer.current) {
        clearInterval(refreshTimer.current);
        refreshTimer.current = null;
      }

      if (unmounted.current) return;

      // Exponential backoff reconnect
      const delay = Math.min(
        RECONNECT_BASE_DELAY * Math.pow(2, reconnectAttempt.current),
        RECONNECT_MAX_DELAY,
      );
      reconnectAttempt.current += 1;
      reconnectTimer.current = setTimeout(() => {
        connectWsRef.current();
      }, delay);
    };
  }, [fetchSandboxes, fetchEvents]);

  useEffect(() => {
    connectWsRef.current = connectWs;
  }, [connectWs]);

  useEffect(() => {
    unmounted.current = false;
    connectWs();

    return () => {
      unmounted.current = true;
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
      if (refreshTimer.current) {
        clearInterval(refreshTimer.current);
        refreshTimer.current = null;
      }
      if (wsRef.current) {
        wsRef.current.onclose = null; // prevent reconnect on intentional close
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [connectWs]);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  return { connected, sandboxes, events, fetchSandboxes, clearEvents, wsRef };
}
