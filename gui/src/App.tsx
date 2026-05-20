import { Component, useCallback, useMemo, useState } from "react";
import type { ReactNode } from "react";

import { DetailPane } from "./components/DetailPane";
import { EventTimeline } from "./components/EventTimeline";
import { FilterBar } from "./components/FilterBar";
import { Sidebar } from "./components/Sidebar";
import { useEventStream } from "./hooks/useEventStream";
import type { EventFilters, TaggedEvent } from "./types";
import "./App.css";

const defaultFilters: EventFilters = {
  types: [],
  query: "",
  path: "",
  hostname: "",
};

const VISIBLE_EVENT_LIMIT = 5000;

function App() {
  const { events, sandboxes, connected, clearEvents } = useEventStream();
  const [selectedSandbox, setSelectedSandbox] = useState<string | null>(null);
  const [selectedEvent, setSelectedEvent] = useState<TaggedEvent | null>(null);
  const [filters, setFilters] = useState<EventFilters>(defaultFilters);
  const closeSelectedEvent = useCallback(() => {
    setSelectedEvent(null);
  }, []);

  const visibleEvents = useMemo(() => {
    return events.slice(-VISIBLE_EVENT_LIMIT);
  }, [events]);

  const filteredEvents = useMemo(() => {
    return visibleEvents.filter((tagged) => {
      if (selectedSandbox && tagged.sandbox_id !== selectedSandbox) {
        return false;
      }
      if (filters.types.length > 0 && !filters.types.includes(tagged.event.event_type)) {
        return false;
      }
      if (filters.query) {
        const query = filters.query.toLowerCase();
        
        // Advanced filtering keys
        if (query.startsWith("status:")) {
          const status = query.split(":")[1];
          return tagged.event.http_status?.toString() === status;
        }
        if (query.startsWith("method:")) {
          const method = query.split(":")[1];
          return tagged.event.http_method?.toLowerCase() === method;
        }
        if (query.startsWith("id:")) {
          const id = query.split(":")[1];
          return tagged.sandbox_id.toLowerCase().includes(id);
        }

        const searchable = [
          tagged.event.event_type,
          tagged.event.syscall_name,
          tagged.event.path,
          tagged.event.hostname,
          tagged.event.http_method,
          tagged.event.http_url,
          tagged.event.http_status?.toString(),
          tagged.event.return_value?.toString(),
          tagged.event.raw,
        ]
          .filter(Boolean)
          .join(" ")
          .toLowerCase();

        if (!searchable.includes(query)) {
          return false;
        }
      }
      if (
        filters.path &&
        !(tagged.event.path ?? "").toLowerCase().includes(filters.path.toLowerCase())
      ) {
        return false;
      }
      if (
        filters.hostname &&
        !(tagged.event.hostname ?? "")
          .toLowerCase()
          .includes(filters.hostname.toLowerCase())
      ) {
        return false;
      }
      return true;
    });
  }, [filters, selectedSandbox, visibleEvents]);

  return (
    <div className="app-layout">
      <Sidebar
        sandboxes={sandboxes}
        selectedId={selectedSandbox}
        onSelect={setSelectedSandbox}
        connected={connected}
      />
      <main className="main-content">
        <FilterBar
          filters={filters}
          onChange={setFilters}
          totalCount={events.length}
          visibleCount={filteredEvents.length}
          onClearEvents={() => {
            clearEvents();
            setSelectedEvent(null);
          }}
        />
        <EventTimeline
          events={filteredEvents}
          selectedEvent={selectedEvent}
          onSelect={setSelectedEvent}
        />
      </main>
      {selectedEvent && (
        <DetailErrorBoundary
          key={`${selectedEvent.sandbox_id}-${selectedEvent.event.timestamp}-${selectedEvent.event.pid}`}
          onClose={closeSelectedEvent}
        >
          <DetailPane event={selectedEvent} onClose={closeSelectedEvent} />
        </DetailErrorBoundary>
      )}
    </div>
  );
}

class DetailErrorBoundary extends Component<{
  onClose: () => void;
  children: ReactNode;
}, { hasError: boolean }> {
  state = { hasError: false };

  static getDerivedStateFromError() {
    return { hasError: true };
  }

  componentDidCatch(error: unknown) {
    console.error("Detail pane failed to render:", error);
  }

  render() {
    if (this.state.hasError) {
      return (
        <aside className="detail-pane detail-pane-error">
          <div className="detail-header">
            <div>
              <h2>Unable to render event</h2>
              <span>One HTTP event contained data this view could not display.</span>
            </div>
            <div className="header-actions">
              <button type="button" className="action-button" onClick={this.props.onClose}>
                Close
              </button>
            </div>
          </div>
          <div className="detail-content">
            <section className="detail-section">
              <p className="detail-error-message">
                The event details were blocked by an unexpected payload shape. Select another event or
                close this pane to continue.
              </p>
            </section>
          </div>
        </aside>
      );
    }

    return this.props.children;
  }
}

export default App;
