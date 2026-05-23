use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use vetty_common::http::parse_http_message;
use vetty_common::{SandboxEvent, SandboxId, SandboxInfo};

use crate::events::{EventStore, TaggedEvent};

pub fn rest_router() -> Router<Arc<EventStore>> {
    Router::new()
        .route("/api/sandboxes", get(list_sandboxes))
        .route("/api/sandboxes/{id}/events", get(get_events))
        .route("/api/events", get(list_all_events))
        .route("/api/proxy-events", post(push_proxy_event))
}

async fn list_sandboxes(State(store): State<Arc<EventStore>>) -> Json<Vec<SandboxInfo>> {
    Json(store.list_sandboxes())
}

async fn list_all_events(State(store): State<Arc<EventStore>>) -> Json<Vec<TaggedEvent>> {
    Json(store.list_events())
}

async fn get_events(
    State(store): State<Arc<EventStore>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<SandboxEvent>>, StatusCode> {
    let parsed = SandboxId::from_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let events = store.get_events(&parsed).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(events))
}

async fn push_proxy_event(
    State(store): State<Arc<EventStore>>,
    Json(mut tagged): Json<TaggedEvent>,
) -> StatusCode {
    normalize_proxy_event(&mut tagged.event);
    store.push_event(&tagged.sandbox_id, tagged.event);
    StatusCode::ACCEPTED
}

fn normalize_proxy_event(event: &mut SandboxEvent) {
    if let Some(message) = event.http_message.as_deref() {
        let parsed = parse_http_message(message);
        event.http_method = parsed.method.or_else(|| event.http_method.take());
        event.http_url = parsed.url.or_else(|| event.http_url.take());
        event.hostname = parsed.hostname.or_else(|| event.hostname.take());
        event.http_status = parsed.status.or(event.http_status);
        event.http_headers = parsed.headers.or_else(|| event.http_headers.take());
        event.http_body = parsed.body.or_else(|| event.http_body.take());
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use vetty_common::EventType;

    use super::*;

    #[test]
    fn proxy_response_normalization_preserves_proxy_supplied_request_metadata() {
        let mut event = SandboxEvent {
            timestamp: Utc::now(),
            pid: 1000,
            event_type: EventType::HttpResponse,
            syscall_name: Some("mitmproxy".to_string()),
            path: None,
            hostname: Some("example.com".to_string()),
            port: Some(443),
            flags: None,
            return_value: Some(2),
            http_method: Some("GET".to_string()),
            http_url: Some("https://example.com/".to_string()),
            http_status: Some(200),
            http_headers: Some(json!({ "content-type": "text/plain" })),
            http_body: Some("ok".to_string()),
            http_message: Some("HTTP/2.0 200 OK\r\ncontent-type: text/plain\r\n\r\nok".to_string()),
            raw: Some("200 https://example.com/".to_string()),
        };

        normalize_proxy_event(&mut event);

        assert_eq!(event.hostname.as_deref(), Some("example.com"));
        assert_eq!(event.http_method.as_deref(), Some("GET"));
        assert_eq!(event.http_url.as_deref(), Some("https://example.com/"));
        assert_eq!(event.http_status, Some(200));
        assert_eq!(event.http_body.as_deref(), Some("ok"));
    }

    #[test]
    fn proxy_request_normalization_uses_parsed_message_fields() {
        let mut event = SandboxEvent {
            timestamp: Utc::now(),
            pid: 1000,
            event_type: EventType::HttpRequest,
            syscall_name: Some("mitmproxy".to_string()),
            path: None,
            hostname: None,
            port: Some(443),
            flags: None,
            return_value: Some(0),
            http_method: None,
            http_url: None,
            http_status: None,
            http_headers: None,
            http_body: None,
            http_message: Some(
                "POST https://api.example.test/v1 HTTP/2.0\r\nhost: ignored.test\r\n\r\n{}"
                    .to_string(),
            ),
            raw: None,
        };

        normalize_proxy_event(&mut event);

        assert_eq!(event.hostname.as_deref(), Some("api.example.test"));
        assert_eq!(event.http_method.as_deref(), Some("POST"));
        assert_eq!(
            event.http_url.as_deref(),
            Some("https://api.example.test/v1")
        );
        assert_eq!(event.http_body.as_deref(), Some("{}"));
    }
}
