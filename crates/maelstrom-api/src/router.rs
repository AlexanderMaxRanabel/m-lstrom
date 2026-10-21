use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use maelstrom_core::matrix::error::MatrixError;

use crate::handlers;
use crate::state::AppState;

/// Build the complete Axum router with all middleware and routes.
pub fn build(state: AppState) -> Router {
    let client_api = Router::new()
        .merge(handlers::versions::routes())
        .merge(handlers::wellknown::routes())
        .merge(handlers::health::routes())
        .merge(handlers::register::routes())
        .merge(handlers::auth::routes())
        .merge(handlers::capabilities::routes())
        .merge(handlers::account::routes())
        .merge(handlers::profile::routes())
        .merge(handlers::rooms::routes())
        .merge(handlers::directory::routes())
        .merge(handlers::events::routes())
        .merge(handlers::search::routes())
        .merge(handlers::sync::routes())
        .merge(handlers::typing::routes())
        .merge(handlers::receipts::routes())
        .merge(handlers::presence::routes())
        .merge(handlers::keys::routes())
        .merge(handlers::to_device::routes())
        .merge(handlers::media::routes())
        .merge(handlers::relations::routes())
        .merge(handlers::threads::routes())
        .merge(handlers::spaces::routes())
        .merge(handlers::knock::routes())
        .merge(handlers::reporting::routes())
        .merge(handlers::room_summary::routes())
        .merge(handlers::appservice::routes())
        // Unknown paths must be answered with a JSON `M_UNRECOGNIZED` body
        // rather than axum's empty 404 (spec requirement, checked by
        // Complement's `TestUnknownEndpoints`).
        .fallback(unrecognized_endpoint)
        // Likewise for a known path used with an unsupported HTTP method.
        .method_not_allowed_fallback(method_not_allowed);

    Router::new()
        .merge(client_api)
        .layer(axum::middleware::from_fn(
            crate::middleware::rate_limit::rate_limit_auth,
        ))
        .layer(RequestDecompressionLayer::new())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Fallback handler for unknown paths: **404 / M_UNRECOGNIZED**.
///
/// The Matrix spec requires every error response to carry a JSON body with an
/// `errcode`; axum's built-in 404 has an empty body, so we install our own.
pub async fn unrecognized_endpoint() -> MatrixError {
    MatrixError::unrecognized_endpoint("Unrecognized request")
}

/// Fallback handler for unsupported HTTP methods: **405 / M_UNRECOGNIZED**.
pub async fn method_not_allowed() -> MatrixError {
    MatrixError::method_not_allowed("Unrecognized request")
}
