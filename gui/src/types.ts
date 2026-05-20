export type EventType =
  | "syscall"
  | "file_access"
  | "network_connect"
  | "process_spawn"
  | "http_request"
  | "http_response";

export interface SandboxEvent {
  timestamp: string;
  pid: number;
  event_type: EventType;
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
  http_message?: string;
  raw?: string;
}

export interface TaggedEvent {
  sandbox_id: string;
  event: SandboxEvent;
}

export interface SandboxInfo {
  id: string;
  name: string;
  status: "starting" | "running" | "stopped" | "error";
  started_at: string;
  event_count: number;
}

export interface EventFilters {
  types: EventType[];
  query: string;
  path: string;
  hostname: string;
}
