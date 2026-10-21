//! Room summary endpoint.
//!
//! Implements `GET /_matrix/client/v1/room_summary/{roomIdOrAlias}`, which
//! returns a summary of a room without requiring the caller to be a member:
//! the room's name, topic, avatar, join rule, history visibility, member
//! count and the caller's own membership.
//!
//! # Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | `GET` | `/_matrix/client/v1/room_summary/{roomIdOrAlias}` | Get a room summary |
//!
//! # Matrix spec
//!
//! * [GET /_matrix/client/v1/room_summary/{roomIdOrAlias}](https://spec.matrix.org/v1.19/client-server-api/#get_matrixclientv1room_summaryroomidoralias)

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use maelstrom_core::matrix::error::MatrixError;
use maelstrom_core::matrix::room::event_type as et;

use crate::extractors::AuthenticatedUser;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/_matrix/client/v1/room_summary/{roomIdOrAlias}",
        get(get_room_summary),
    )
}

/// The room summary response body.
///
/// Mirrors the spec's `RoomSummary` schema: the public-rooms chunk fields
/// plus the room type, encryption algorithm and room version.
#[derive(Debug, Serialize)]
struct RoomSummary {
    room_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    canonical_alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    num_joined_members: usize,
    world_readable: bool,
    guest_can_join: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    join_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    room_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encryption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    room_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    membership: Option<String>,
}

/// Get a summary of a room.
///
/// `roomIdOrAlias` may be a room ID (`!opaque:server`) or a room alias
/// (`#alias:server`).
async fn get_room_summary(
    State(state): State<AppState>,
    auth: Option<AuthenticatedUser>,
    Path(room_id_or_alias): Path<String>,
) -> Result<Json<RoomSummary>, MatrixError> {
    let storage = state.storage();

    // Resolve an alias to a room ID.
    let room_id = if room_id_or_alias.starts_with('#') {
        storage
            .get_room_alias(&room_id_or_alias)
            .await
            .map_err(|_| MatrixError::not_found("Room not found"))?
    } else {
        room_id_or_alias
    };

    let room = storage
        .get_room(&room_id)
        .await
        .map_err(|_| MatrixError::not_found("Room not found"))?;

    let state_events = storage
        .get_current_state(&room_id)
        .await
        .map_err(crate::extractors::storage_error)?;

    let mut name = None;
    let mut topic = None;
    let mut avatar_url = None;
    let mut canonical_alias = None;
    let mut join_rule = None;
    let mut room_type = None;
    let mut encryption = None;
    let mut history_visibility = None;

    for event in &state_events {
        let content = &event.content;
        match event.event_type.as_str() {
            t if t == et::NAME => {
                name = content.get("name").and_then(|v| v.as_str()).map(str::to_string);
            }
            t if t == et::TOPIC => {
                topic = content
                    .get("topic")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            t if t == et::AVATAR => {
                avatar_url = content
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            t if t == et::CREATE => {
                room_type = content.get("type").and_then(|v| v.as_str()).map(str::to_string);
            }
            t if t == et::JOIN_RULES => {
                join_rule = content
                    .get("join_rule")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            t if t == et::HISTORY_VISIBILITY => {
                history_visibility = content
                    .get("history_visibility")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            t if t == et::ENCRYPTION => {
                encryption = content
                    .get("algorithm")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            _ => {}
        }
    }

    // The canonical alias lives in m.room.canonical_alias, not in the create
    // event; look it up separately.
    if let Ok(event) = storage.get_state_event(&room_id, et::CANONICAL_ALIAS, "").await {
        canonical_alias = event
            .content
            .get("alias")
            .and_then(|v| v.as_str())
            .map(str::to_string);
    }

    let joined_members = storage
        .get_room_members(&room_id, "join")
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    let world_readable = history_visibility.as_deref() == Some("world_readable");

    // Guest access requires a public join rule *and* guest access enabled.
    let guest_can_join = join_rule.as_deref() == Some("public")
        && state_events.iter().any(|e| {
            e.event_type == et::GUEST_ACCESS
                && e.content.get("guest_access").and_then(|v| v.as_str()) == Some("can_join")
        });

    // The caller's own membership, if we know who the caller is.
    let membership = match auth {
        Some(auth) => storage
            .get_membership(auth.user_id.as_ref(), &room_id)
            .await
            .ok()
            .filter(|m| !m.is_empty() && m != "leave")
            .or(Some("leave".to_string())),
        None => None,
    };

    Ok(Json(RoomSummary {
        room_id: room.room_id,
        name,
        topic,
        canonical_alias,
        avatar_url,
        num_joined_members: joined_members,
        world_readable,
        guest_can_join,
        join_rule,
        room_type,
        encryption,
        room_version: Some(room.version),
        membership,
    }))
}
