//! Serves the visualization frontend, embedded into the binary at compile time.

use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// The frontend bundle produced by `just prod`.
///
/// Empty when the crate is built without running the frontend build first.
#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

const INDEX: &str = "index.html";

/// Assets are content-hashed by the bundler, so they can be cached indefinitely.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

const NOT_BUNDLED: &str = "The Tierkreis visualization frontend was not bundled into this build. \
     Run `just prod` and rebuild to enable it.";

/// Serves an embedded asset, falling back to `index.html` so the SPA can route client-side.
pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if !path.is_empty() && path != INDEX {
        if let Some(file) = Assets::get(path) {
            let cache_control = if path.starts_with("assets/") {
                IMMUTABLE
            } else {
                "no-cache"
            };
            return (
                [
                    (header::CONTENT_TYPE, file.metadata.mimetype()),
                    (header::CACHE_CONTROL, cache_control),
                ],
                file.data,
            )
                .into_response();
        }

        // Don't serve the SPA shell in place of a missing API or asset response.
        if path.starts_with("api/") || path.starts_with("assets/") {
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    let Some(index) = Assets::get(INDEX) else {
        return (StatusCode::SERVICE_UNAVAILABLE, NOT_BUNDLED).into_response();
    };

    (
        [
            (header::CONTENT_TYPE, "text/html"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        index.data,
    )
        .into_response()
}
