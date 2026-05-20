# Step 8 — Electron + React GUI

## Goal
Build a desktop GUI using Electron + React + TypeScript that connects to `vetty-daemon`'s WebSocket and displays live sandbox events with filtering and detail views.

---

## 8.1 Project Setup

Create the GUI in `gui/` using Vite + React + TypeScript with Electron.

### `gui/package.json`

```json
{
  "name": "vetty-gui",
  "version": "0.1.0",
  "private": true,
  "main": "electron/main.js",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "electron:dev": "concurrently \"vite\" \"wait-on http://localhost:5173 && electron .\"",
    "electron:build": "vite build && electron-builder"
  },
  "dependencies": {
    "react": "^18.3.0",
    "react-dom": "^18.3.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "concurrently": "^9.0.0",
    "electron": "^33.0.0",
    "electron-builder": "^25.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0",
    "wait-on": "^8.0.0"
  }
}
```

---

## 8.2 Electron Main Process — `gui/electron/main.ts`

```typescript
import { app, BrowserWindow } from 'electron';
import { spawn, ChildProcess } from 'child_process';
import path from 'path';

let mainWindow: BrowserWindow | null = null;
let daemonProcess: ChildProcess | null = null;

function startDaemon() {
  // Spawn vetty-daemon as a child process
  daemonProcess = spawn('vetty-daemon', [], {
    env: {
      ...process.env,
      VETTY_DAEMON_PORT: '9876',
    },
    stdio: 'pipe',
  });

  daemonProcess.stdout?.on('data', (data) => {
    console.log(`[daemon] ${data}`);
  });

  daemonProcess.stderr?.on('data', (data) => {
    console.error(`[daemon] ${data}`);
  });
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 1000,
    minHeight: 600,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
    },
    titleBarStyle: 'hiddenInset',
    backgroundColor: '#0a0a0f',
  });

  // In development, load from Vite dev server
  if (process.env.NODE_ENV === 'development') {
    mainWindow.loadURL('http://localhost:5173');
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }
}

app.whenReady().then(() => {
  startDaemon();
  createWindow();
});

app.on('window-all-closed', () => {
  daemonProcess?.kill();
  app.quit();
});
```

---

## 8.3 Design System — `gui/src/index.css`

Use a dark theme with a cybersecurity aesthetic:

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');

:root {
  /* Color palette — dark cyber theme */
  --bg-primary: #0a0a0f;
  --bg-secondary: #12121a;
  --bg-tertiary: #1a1a28;
  --bg-hover: #22223a;
  --bg-active: #2a2a45;

  --border-primary: #2a2a3d;
  --border-subtle: #1e1e30;

  --text-primary: #e8e8f0;
  --text-secondary: #8888aa;
  --text-muted: #555570;

  --accent-blue: #4a9eff;
  --accent-purple: #8b5cf6;
  --accent-green: #10b981;
  --accent-orange: #f59e0b;
  --accent-red: #ef4444;
  --accent-cyan: #06b6d4;

  /* Event type colors */
  --event-syscall: #8b5cf6;
  --event-file: #4a9eff;
  --event-network: #f59e0b;
  --event-process: #ef4444;
  --event-http: #06b6d4;

  /* Status colors */
  --status-running: #10b981;
  --status-stopped: #6b7280;
  --status-error: #ef4444;

  /* Spacing */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 12px;
  --space-lg: 16px;
  --space-xl: 24px;
  --space-2xl: 32px;

  /* Radius */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;

  /* Font */
  --font-sans: 'Inter', -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;

  /* Shadows */
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.4);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.5);
  --shadow-glow-blue: 0 0 20px rgba(74, 158, 255, 0.15);
  --shadow-glow-purple: 0 0 20px rgba(139, 92, 246, 0.15);
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: var(--font-sans);
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: var(--border-primary);
  border-radius: 3px;
}
::-webkit-scrollbar-thumb:hover {
  background: var(--text-muted);
}
```

---

## 8.4 WebSocket Hook — `gui/src/hooks/useEventStream.ts`

```typescript
import { useState, useEffect, useCallback, useRef } from 'react';

export interface SandboxEvent {
  timestamp: string;
  pid: number;
  event_type: 'syscall' | 'file_access' | 'network_connect' | 'process_spawn' | 'http_request' | 'http_response';
  syscall_name?: string;
  path?: string;
  hostname?: string;
  port?: number;
  flags?: string;
  return_value?: number;
  http_method?: string;
  http_url?: string;
  http_status?: number;
  http_headers?: Record<string, string>;
  http_body?: string;
  raw?: string;
}

export interface TaggedEvent {
  sandbox_id: { inner: string };
  event: SandboxEvent;
}

export interface SandboxInfo {
  id: { inner: string };
  name: string;
  status: 'starting' | 'running' | 'stopped' | 'error';
  started_at: string;
  event_count: number;
}

const DAEMON_URL = 'ws://localhost:9876/ws/events';
const REST_URL = 'http://localhost:9876/api';

export function useEventStream() {
  const [events, setEvents] = useState<TaggedEvent[]>([]);
  const [sandboxes, setSandboxes] = useState<SandboxInfo[]>([]);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);

  // Fetch initial sandbox list
  const fetchSandboxes = useCallback(async () => {
    try {
      const res = await fetch(`${REST_URL}/sandboxes`);
      const data = await res.json();
      setSandboxes(data);
    } catch (e) {
      console.error('Failed to fetch sandboxes:', e);
    }
  }, []);

  // Connect to WebSocket
  useEffect(() => {
    const ws = new WebSocket(DAEMON_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      fetchSandboxes();
    };

    ws.onmessage = (msg) => {
      try {
        const tagged: TaggedEvent = JSON.parse(msg.data);
        setEvents((prev) => [...prev, tagged]);
      } catch (e) {
        console.error('Failed to parse event:', e);
      }
    };

    ws.onclose = () => setConnected(false);
    ws.onerror = () => setConnected(false);

    return () => ws.close();
  }, [fetchSandboxes]);

  return { events, sandboxes, connected, fetchSandboxes };
}
```

---

## 8.5 Components

### `gui/src/components/Sidebar.tsx`

The sidebar lists active sandboxes with their name and status indicator.

```typescript
// Key features:
// - List of SandboxInfo items
// - Color-coded status dot (green=running, gray=stopped, red=error)
// - Click to select/filter events by sandbox
// - Connection status indicator at the bottom
// - Subtle glassmorphism panel style
```

**Design specs:**
- Width: 260px fixed
- Background: `var(--bg-secondary)` with a subtle right border
- Each sandbox item: hover effect with `var(--bg-hover)`
- Status dot: 8px circle with the appropriate status color
- Active sandbox: left accent border in `var(--accent-blue)`

---

### `gui/src/components/FilterBar.tsx`

A horizontal filter bar above the event timeline.

```typescript
// Key features:
// - Toggle buttons for event types: Syscall, File, Network, Process, HTTP
// - Each button uses the event type color as its accent
// - Text input for path substring filter
// - Text input for hostname filter
// - Active filters shown as pills/chips
// - "Clear all" button
```

**Design specs:**
- Height: 48px
- Background: `var(--bg-secondary)` with bottom border
- Toggle buttons: pill-shaped, outlined when inactive, filled when active
- Inputs: compact, monospace font, subtle border

---

### `gui/src/components/EventTimeline.tsx`

The main event table that live-updates as events arrive.

```typescript
// Key features:
// - Virtualized list (for performance with thousands of events)
// - Columns: Time | Type | PID | Summary
// - Type column: colored badge with event type
// - Summary: smart summary based on event type:
//   - FileAccess: "open /etc/passwd → 3"
//   - NetworkConnect: "connect 93.184.216.34:443"
//   - ProcessSpawn: "execve /usr/bin/curl"
//   - HttpRequest: "GET https://evil.com → 200"
// - New rows animate in from the bottom with a subtle fade
// - Click a row to select it (opens DetailPane)
// - Auto-scroll to bottom (with a "pinned to bottom" toggle)
```

**Design specs:**
- Font: monospace for the table body
- Row height: 32px
- Alternating row backgrounds: `var(--bg-primary)` / `var(--bg-secondary)`
- Selected row: `var(--bg-active)` with left accent border
- Type badge: small pill with the event type color
- Timestamp: `var(--text-muted)`, compact format (HH:MM:SS.mmm)
- New event animation: `@keyframes slideIn` — translateY(10px) → 0, opacity 0 → 1, 200ms

> **Performance note:** Use a simple virtual scrolling approach — only render rows visible in the viewport plus a small overscan. Keep all events in state but only render a window.

---

### `gui/src/components/DetailPane.tsx`

A slide-out panel showing full structured data for a selected event.

```typescript
// Key features:
// - Slides in from the right (300px wide)
// - Header: event type badge + timestamp
// - Sections (collapsible):
//   - General: PID, syscall name, return value, flags
//   - Path: file path (if applicable)
//   - Network: hostname, port, IP
//   - HTTP: method, URL, status, headers (key-value table), body (code block)
//   - Raw: the original strace line in a code block
// - Close button (X) in the top right
// - Sections only appear if data is present
```

**Design specs:**
- Background: `var(--bg-secondary)` with a left border and shadow
- Section headers: `var(--text-secondary)`, uppercase, small font
- Values: monospace font
- HTTP body: syntax-highlighted code block with `var(--bg-tertiary)` background
- Slide animation: 250ms ease-out

---

## 8.6 Main App — `gui/src/App.tsx`

```typescript
// Layout:
// ┌──────────┬──────────────────────────────┬─────────┐
// │          │         FilterBar            │         │
// │          ├──────────────────────────────┤         │
// │ Sidebar  │                              │ Detail  │
// │          │       EventTimeline          │  Pane   │
// │          │                              │         │
// │          │                              │         │
// └──────────┴──────────────────────────────┴─────────┘
//
// - Sidebar: fixed left
// - FilterBar: top of main content
// - EventTimeline: fills remaining space
// - DetailPane: conditionally shown on the right

import React, { useState, useMemo } from 'react';
import { useEventStream, TaggedEvent } from './hooks/useEventStream';
import { Sidebar } from './components/Sidebar';
import { FilterBar } from './components/FilterBar';
import { EventTimeline } from './components/EventTimeline';
import { DetailPane } from './components/DetailPane';

function App() {
  const { events, sandboxes, connected } = useEventStream();
  const [selectedSandbox, setSelectedSandbox] = useState<string | null>(null);
  const [selectedEvent, setSelectedEvent] = useState<TaggedEvent | null>(null);
  const [filters, setFilters] = useState({ types: [], path: '', hostname: '' });

  const filteredEvents = useMemo(() => {
    return events.filter((e) => {
      if (selectedSandbox && e.sandbox_id.inner !== selectedSandbox) return false;
      if (filters.types.length && !filters.types.includes(e.event.event_type)) return false;
      if (filters.path && !e.event.path?.includes(filters.path)) return false;
      if (filters.hostname && !e.event.hostname?.includes(filters.hostname)) return false;
      return true;
    });
  }, [events, selectedSandbox, filters]);

  return (
    <div className="app-layout">
      <Sidebar
        sandboxes={sandboxes}
        selectedId={selectedSandbox}
        onSelect={setSelectedSandbox}
        connected={connected}
      />
      <main className="main-content">
        <FilterBar filters={filters} onChange={setFilters} />
        <EventTimeline
          events={filteredEvents}
          selectedEvent={selectedEvent}
          onSelect={setSelectedEvent}
        />
      </main>
      {selectedEvent && (
        <DetailPane event={selectedEvent} onClose={() => setSelectedEvent(null)} />
      )}
    </div>
  );
}

export default App;
```

---

## 8.7 App Layout CSS — `gui/src/App.css`

```css
.app-layout {
  display: flex;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
```

---

## Done Criteria

- [ ] `npm install` and `npm run dev` work in `gui/`
- [ ] Electron main process spawns daemon and opens window
- [ ] WebSocket hook connects and receives events
- [ ] Sidebar shows sandbox list with status indicators
- [ ] FilterBar allows filtering by event type, path, hostname
- [ ] EventTimeline renders live-updating event rows
- [ ] DetailPane shows full event data when a row is clicked
- [ ] Dark cyber theme applied consistently
- [ ] Smooth animations on new events and panel transitions
