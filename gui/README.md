# Vetty GUI

Electron + React desktop application for real-time sandbox monitoring.

## Overview

The Vetty GUI connects to the `vetty-daemon` via WebSocket and displays live syscall, file, network, and HTTP events from running Firecracker sandboxes.

### Features

- **Live event timeline** — Events stream in real-time as they happen in the sandbox
- **Event filtering** — Filter by type (file, network, process, HTTP), path, hostname, or free-text search
- **Sandbox sidebar** — View all active/historical sandboxes
- **Detail pane** — Inspect individual events with full HTTP request/response bodies
- **Dark theme** — Professional dark UI designed for extended monitoring sessions

## Architecture

```
gui/
├── electron/
│   ├── main.cjs          # Electron main process (auto-starts daemon)
│   └── preload.cjs       # Context bridge for secure IPC
├── src/
│   ├── App.tsx            # Root component with layout
│   ├── App.css            # Application styles
│   ├── types.ts           # TypeScript type definitions
│   ├── components/
│   │   ├── Sidebar.tsx        # Sandbox list
│   │   ├── EventTimeline.tsx  # Scrollable event list
│   │   ├── FilterBar.tsx      # Event type and text filters
│   │   └── DetailPane.tsx     # Event detail inspector
│   └── hooks/
│       └── useEventStream.ts  # WebSocket connection + event state
├── index.html
├── package.json
└── vite.config.ts
```

## Development

```bash
# Install dependencies
npm install

# Start in development mode (hot-reload + Electron)
npm run electron:dev

# Build for production
npm run build

# Lint
npm run lint
```

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `VETTY_DAEMON_PORT` | `9876` | Port of the daemon API |
| `VETTY_DAEMON_BIN` | auto-detect | Override path to daemon binary |
| `NODE_ENV` | — | Set to `development` for dev mode |

### How it Works

1. **Electron main process** checks if `vetty-daemon` is already running
2. If not, it auto-starts the daemon (finds the binary in `target/debug`, `target/release`, or falls back to `cargo run`)
3. The React app connects to `ws://127.0.0.1:9876/ws/events` via the `useEventStream` hook
4. Events are displayed in the timeline, with real-time filtering and detail inspection

## Tech Stack

- **Electron** 38+ — Desktop application shell
- **React** 19 — UI framework
- **TypeScript** 6 — Type safety
- **Vite** 8 — Build tool with HMR
