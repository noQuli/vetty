import type { SandboxInfo } from "../types";

interface SidebarProps {
  sandboxes: SandboxInfo[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  connected: boolean;
}

export function Sidebar({
  sandboxes,
  selectedId,
  onSelect,
  connected,
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <h1>VETTY</h1>
        <span className={`connection ${connected ? "up" : "down"}`}>
          {connected ? "Live" : "Offline"}
        </span>
      </div>

      <nav className="sandbox-list">
        <button
          type="button"
          className={`sandbox-item all ${selectedId === null ? "active" : ""}`}
          onClick={() => onSelect(null)}
        >
          <span className="sandbox-name">All Sandboxes</span>
          <span className="sandbox-count">
            {sandboxes.reduce((acc, s) => acc + s.event_count, 0)}
          </span>
        </button>

        {sandboxes.map((sandbox) => (
          <button
            key={sandbox.id}
            type="button"
            className={`sandbox-item ${selectedId === sandbox.id ? "active" : ""}`}
            onClick={() => onSelect(sandbox.id)}
          >
            <span className={`status-dot ${sandbox.status}`} />
            <span className="sandbox-name" title={sandbox.id}>
              {sandbox.name || sandbox.id.slice(0, 12)}
            </span>
            <span className="sandbox-count">{sandbox.event_count}</span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
