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
    let events = store
        .get_events(&parsed)
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(events))
}

async fn push_proxy_event(
    State(store): State<Arc<EventStore>>,
    Json(mut tagged): Json<TaggedEvent>,
) -> StatusCode {
    if let Some(message) = tagged.event.http_message.as_deref() {
        let parsed = parse_http_message(message);
        tagged.event.http_method = parsed.method;
        tagged.event.http_url = parsed.url;
        tagged.event.hostname = parsed.hostname;
        tagged.event.http_status = parsed.status;
        tagged.event.http_headers = parsed.headers;
        tagged.event.http_body = parsed.body;
    }

    store.push_event(&tagged.sandbox_id, tagged.event);
    StatusCode::ACCEPTED
}
