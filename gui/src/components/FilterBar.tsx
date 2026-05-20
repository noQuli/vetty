import type { EventFilters, EventType } from "../types";

interface FilterBarProps {
  filters: EventFilters;
  onChange: (filters: EventFilters) => void;
  totalCount: number;
  visibleCount: number;
  onClearEvents: () => void;
}

const eventTypeOptions: Array<{ key: EventType; label: string }> = [
  { key: "syscall", label: "Syscall" },
  { key: "file_access", label: "File" },
  { key: "network_connect", label: "Network" },
  { key: "process_spawn", label: "Process" },
  { key: "http_request", label: "HTTP Req" },
  { key: "http_response", label: "HTTP Res" },
];

export function FilterBar({
  filters,
  onChange,
  totalCount,
  visibleCount,
  onClearEvents,
}: FilterBarProps) {
  const toggleType = (type: EventType) => {
    const exists = filters.types.includes(type);
    onChange({
      ...filters,
      types: exists
        ? filters.types.filter((item) => item !== type)
        : [...filters.types, type],
    });
  };

  const hasFilters =
    filters.types.length > 0 || filters.query !== "" || filters.path !== "" || filters.hostname !== "";

  return (
    <div className="filter-bar">
      <div className="filter-top-row">
        <div className="filter-summary">
          <span className="filter-title">Events</span>
          <span className="filter-count">
            {visibleCount.toLocaleString()} / {totalCount.toLocaleString()}
          </span>
          <span className={`filter-state ${hasFilters ? "active" : ""}`}>
            {hasFilters ? "Filtered" : "All traffic"}
          </span>
        </div>

        <div className="filter-actions">
          <button
            type="button"
            className="action-button"
            disabled={!hasFilters}
            onClick={() => onChange({ types: [], query: "", path: "", hostname: "" })}
          >
            Reset
          </button>
          <button type="button" className="action-button danger" onClick={onClearEvents}>
            Clear
          </button>
        </div>
      </div>

      <div className="filter-controls">
        <input
          value={filters.query}
          onChange={(event) => onChange({ ...filters, query: event.target.value })}
          placeholder="Search all fields..."
          className="filter-input"
        />

        <input
          value={filters.path}
          onChange={(event) => onChange({ ...filters, path: event.target.value })}
          placeholder="Path filter..."
          className="filter-input"
        />

        <input
          value={filters.hostname}
          onChange={(event) => onChange({ ...filters, hostname: event.target.value })}
          placeholder="Host filter..."
          className="filter-input"
        />
      </div>

      <div className="type-filters">
        {eventTypeOptions.map((option) => (
          <button
            key={option.key}
            type="button"
            className={`type-pill ${filters.types.includes(option.key) ? "active" : ""}`}
            onClick={() => toggleType(option.key)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}
