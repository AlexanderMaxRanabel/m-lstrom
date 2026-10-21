//! Supported Matrix spec versions.
//!
//! This is typically the first endpoint a client calls during startup to
//! determine which specification versions and unstable features the server
//! supports. The response guides the client on which API paths and behaviours
//! are available.
//!
//! This endpoint does **not** require authentication.
//!
//! # Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | `GET` | `/_matrix/client/versions` | List supported spec versions and unstable feature flags |
//!
//! # Matrix spec
//!
//! * [GET /_matrix/client/versions](https://spec.matrix.org/v1.18/client-server-api/#get_matrixclientversions)

use axum::Json;
use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/_matrix/client/versions", get(get_versions))
}

#[derive(Serialize)]
struct VersionsResponse {
    versions: Vec<&'static str>,
    unstable_features: BTreeMap<String, bool>,
}

/// The stable spec versions implemented by this server.
///
/// The list is contiguous from `v1.1` up to and including the newest stable
/// release (`v1.19`).
const SUPPORTED_VERSIONS: &[&str] = &[
    "v1.1", "v1.2", "v1.3", "v1.4", "v1.5", "v1.6", "v1.7", "v1.8", "v1.9", "v1.10", "v1.11",
    "v1.12", "v1.13", "v1.14", "v1.15", "v1.16", "v1.17", "v1.18", "v1.19",
];

/// The unstable features (MSCs) this server actually implements.
///
/// Only MSCs with a working implementation are advertised — advertising an
/// MSC that is not implemented is worse than not advertising it at all,
/// because clients will then use the unstable code path.
///
/// * `org.matrix.msc3967` — user-interactive authentication for cross-signing
///   and key backup.
/// * `org.matrix.msc3706` — partial state joins.
/// * `org.matrix.msc4102` — thread-aware receipts.
/// * `org.matrix.msc3391` — account data deletion.
const UNSTABLE_FEATURES: &[(&str, bool)] = &[
    ("org.matrix.msc3967", true),
    ("org.matrix.msc3706", true),
    ("org.matrix.msc4102", true),
    ("org.matrix.msc3391", true),
];

async fn get_versions() -> impl IntoResponse {
    let mut unstable_features = BTreeMap::new();
    for (name, enabled) in UNSTABLE_FEATURES {
        unstable_features.insert((*name).to_string(), *enabled);
    }

    Json(VersionsResponse {
        versions: SUPPORTED_VERSIONS.to_vec(),
        unstable_features,
    })
}
