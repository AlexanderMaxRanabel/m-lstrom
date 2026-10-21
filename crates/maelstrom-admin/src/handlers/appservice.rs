//! Admin application-service management endpoints.
//!
//! Application services (bridges) register themselves with the server so that
//! Maelstrom can push events to them and so that their user namespaces are
//! reserved.  These endpoints live in the **admin** router (not the
//! client-server router) and are guarded by the [`AdminUser`] extractor, so an
//! ordinary user can neither register nor list nor delete an appservice.
//!
//! ## Routes
//!
//! | Method   | Path                                             | Operation             |
//! |----------|--------------------------------------------------|-----------------------|
//! | `POST`   | `/_maelstrom/admin/v1/appservice`                | Register an appservice|
//! | `GET`    | `/_maelstrom/admin/v1/appservices`               | List appservices      |
//! | `DELETE` | `/_maelstrom/admin/v1/appservice/{asId}`         | Unregister by id      |

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::Value;

use maelstrom_core::matrix::error::MatrixError;
use maelstrom_storage::traits::AppServiceRecord;

use crate::AdminState;
use crate::auth::AdminUser;

pub fn routes() -> Router<AdminState> {
    Router::new()
        .route("/_maelstrom/admin/v1/appservice", post(register_appservice))
        .route("/_maelstrom/admin/v1/appservices", get(list_appservices))
        .route(
            "/_maelstrom/admin/v1/appservice/{asId}",
            delete(delete_appservice),
        )
}

/// Register a new application service.
///
/// Accepts a JSON body matching [`AppServiceRecord`].  The `id` field must be
/// unique across all registered ASes.
async fn register_appservice(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Json(record): Json<AppServiceRecord>,
) -> Result<Json<Value>, MatrixError> {
    state
        .storage()
        .register_appservice(&record)
        .await
        .map_err(|e| {
            tracing::error!("Failed to register appservice: {e}");
            MatrixError::unknown(format!("Failed to register appservice: {e}"))
        })?;

    Ok(Json(serde_json::json!({ "id": record.id })))
}

/// List all registered application services.
async fn list_appservices(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> Result<Json<Value>, MatrixError> {
    let appservices = state
        .storage()
        .list_appservices()
        .await
        .map_err(|e| MatrixError::unknown(format!("Failed to list appservices: {e}")))?;

    Ok(Json(serde_json::json!({ "appservices": appservices })))
}

/// Unregister an application service by ID.
async fn delete_appservice(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(as_id): Path<String>,
) -> Result<Json<Value>, MatrixError> {
    state
        .storage()
        .delete_appservice(&as_id)
        .await
        .map_err(|e| MatrixError::unknown(format!("Failed to delete appservice: {e}")))?;

    Ok(Json(serde_json::json!({})))
}
